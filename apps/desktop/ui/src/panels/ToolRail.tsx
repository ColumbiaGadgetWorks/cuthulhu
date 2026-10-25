// SPDX-License-Identifier: GPL-3.0-or-later
import type { CSSProperties } from "react";
import type { BoolOp } from "../App";
import type { DrawKind } from "../interaction/useCanvasInteraction";

type Tool = "select" | DrawKind;

type Props = {
  tool: Tool;
  selectionCount: number;
  onSelectTool: (tool: Tool) => void;
  onAddText: () => void;
  onBoolean: (op: BoolOp) => void;
  onDelete: () => void;
  /** The tools that make new shapes from the selection, in the order they are listed. */
  shapeTools: ShapeTool[];
};

/** `disabled` carries the reason, shown as the button's tooltip, or null when it can act. */
export type ShapeTool = { label: string; disabled: string | null; onClick: () => void };

const btn = (active: boolean): CSSProperties => ({
  background: active ? "var(--accent)" : "var(--panel)",
  // The accent is a light cyan: light text on it was unreadable.
  color: active ? "#0b0b0d" : "var(--text)",
  border: "1px solid var(--border)",
  padding: "6px 8px",
  cursor: "pointer",
  width: "100%",
});

const BOOL_OPS: BoolOp[] = ["Union", "Subtract", "Intersect", "Exclude"];

export function ToolRail({ tool, selectionCount, onSelectTool, onAddText, onBoolean, onDelete, shapeTools }: Props) {
  const canBoolean = selectionCount >= 2;
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 4,
        padding: 6,
        width: 96,
        background: "var(--panel)",
        borderRight: "1px solid var(--border)",
        // The shape tools make the rail taller than a small window; it scrolls rather than
        // pushing the status bar off screen.
        overflowY: "auto",
        minHeight: 0,
      }}
    >
      <button aria-label="Select" title="Select (V)" aria-pressed={tool === "select"} style={btn(tool === "select")} onClick={() => onSelectTool("select")}>
        Select
      </button>
      <button aria-label="Rectangle" title="Rectangle (R): drag on the canvas; Shift for a square" aria-pressed={tool === "rect"} style={btn(tool === "rect")} onClick={() => onSelectTool("rect")}>
        Rectangle
      </button>
      <button aria-label="Ellipse" title="Ellipse (E): drag on the canvas; Shift for a circle" aria-pressed={tool === "ellipse"} style={btn(tool === "ellipse")} onClick={() => onSelectTool("ellipse")}>
        Ellipse
      </button>
      <button aria-label="Text" style={btn(false)} onClick={onAddText}>
        Text
      </button>
      <div style={{ height: 1, background: "var(--border)", margin: "4px 0" }} />
      {BOOL_OPS.map((op) => (
        <button key={op} aria-label={op} disabled={!canBoolean} style={btn(false)} onClick={() => onBoolean(op)}>
          {op}
        </button>
      ))}
      <div style={{ height: 1, background: "var(--border)", margin: "4px 0" }} />
      {shapeTools.map((t) => (
        <button key={t.label} aria-label={t.label} title={t.disabled ?? undefined} disabled={t.disabled !== null} style={btn(false)} onClick={t.onClick}>
          {t.label}
        </button>
      ))}
      <div style={{ height: 1, background: "var(--border)", margin: "4px 0" }} />
      <button aria-label="Delete" disabled={selectionCount === 0} style={btn(false)} onClick={onDelete}>
        Delete
      </button>
    </div>
  );
}
