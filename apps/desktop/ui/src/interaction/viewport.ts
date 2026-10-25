// SPDX-License-Identifier: GPL-3.0-or-later
import type { Affine6, Bounds } from "../render/hittest";
import type { Pt } from "../render/affine";

/** screen = world · scale + (tx, ty). World is document mm; screen is CSS px from the canvas's
 *  top-left. Uniform scale only: nobody wants a squashed view of a cut. */
export type View = { scale: number; tx: number; ty: number };
export type Size = { w: number; h: number };

/** "100%" is CSS reference size: 96 CSS px to the CSS inch. That is physical size only when the
 *  display's density and the OS's scaling match the reference pixel; on most screens it is close,
 *  not exact (Copilot on #298). The document is in millimetres.
 *  ponytail: no per-monitor calibration. Ceiling: ⌘1 is "about real size", not ruler-exact.
 *  Upgrade: a one-time "hold a card to the screen" calibration stored per display. */
export const CSS_PX_PER_MM = 96 / 25.4;
export const MAX_SCALE = 64 * CSS_PX_PER_MM; // 6400%
export const FIT_MARGIN_PX = 24;
export const ZOOM_STEP = 1.25;
/** Pinch reports a few px per event and a mouse notch about 100. An exponential keeps a pinch
 *  proportional, and zooming in then out by the same delta lands exactly where it started. */
export const WHEEL_ZOOM_RATE = 0.01;
/** At or above this a vertical delta is a mouse notch rather than a trackpad's stream: a notch is
 *  about 100 px (more under display scaling), a trackpad's events a few px each. */
export const NOTCH_PX = 50;

export const IDENTITY_VIEW: View = { scale: 1, tx: 0, ty: 0 };

export function fitView(artboard: Bounds, size: Size): View {
  if (artboard.w <= 0 || artboard.h <= 0) {
    return {
      scale: CSS_PX_PER_MM,
      tx: FIT_MARGIN_PX - artboard.x * CSS_PX_PER_MM,
      ty: FIT_MARGIN_PX - artboard.y * CSS_PX_PER_MM,
    };
  }
  const availW = Math.max(size.w - 2 * FIT_MARGIN_PX, 1);
  const availH = Math.max(size.h - 2 * FIT_MARGIN_PX, 1);
  const scale = Math.min(availW / artboard.w, availH / artboard.h);
  return {
    scale,
    tx: (size.w - artboard.w * scale) / 2 - artboard.x * scale,
    ty: (size.h - artboard.h * scale) / 2 - artboard.y * scale,
  };
}

/** Far enough out to see the artboard small with room around it, and no further: past that the
 *  canvas is empty space and the way back is long. */
export function minScaleFor(artboard: Bounds, size: Size): number {
  return Math.min(fitView(artboard, size).scale / 4, MAX_SCALE);
}

export function zoomAt(v: View, screen: Pt, factor: number, minScale: number): View {
  const scale = Math.min(Math.max(v.scale * factor, minScale), MAX_SCALE);
  const k = scale / v.scale;
  return { scale, tx: screen.x - (screen.x - v.tx) * k, ty: screen.y - (screen.y - v.ty) * k };
}

export function panBy(v: View, dx: number, dy: number): View {
  return { scale: v.scale, tx: v.tx + dx, ty: v.ty + dy };
}

export function worldToScreen(v: View, p: Pt): Pt {
  return { x: p.x * v.scale + v.tx, y: p.y * v.scale + v.ty };
}

export function screenToWorld(v: View, p: Pt): Pt {
  return { x: (p.x - v.tx) / v.scale, y: (p.y - v.ty) / v.scale };
}

export function viewMatrix(v: View): Affine6 {
  return [v.scale, 0, 0, v.scale, v.tx, v.ty];
}

export function zoomPercent(v: View): number {
  return Math.round((v.scale / CSS_PX_PER_MM) * 100);
}

/** Through the exponential a notch of 100 px zoomed by e (2.7×) in one click, so far that the
 *  operator overshot every time. A notch takes one menu zoom step per notch instead. */
export function wheelFactor(deltaY: number): number {
  if (Math.abs(deltaY) >= NOTCH_PX) {
    const notches = Math.max(1, Math.round(Math.abs(deltaY) / 100));
    return ZOOM_STEP ** (-Math.sign(deltaY) * notches);
  }
  return Math.exp(-deltaY * WHEEL_ZOOM_RATE);
}

/** A mouse wheel: line-mode deltas (Firefox), or one large vertical delta with no sideways part.
 *  A trackpad scrolls in small two-axis deltas and pans; a mouse wheel zooms, as it does in
 *  LightBurn and Silhouette Studio, where the people this is for have learnt it. */
export function isMouseWheel(e: { deltaMode: number; deltaX: number; deltaY: number }, lineMode: number): boolean {
  return e.deltaMode === lineMode || (e.deltaX === 0 && Math.abs(e.deltaY) >= NOTCH_PX);
}
