// SPDX-License-Identifier: GPL-3.0-or-later
//
// How the UI reaches its backend. Inside the desktop app that is Tauri's IPC; served by
// `cuthulhu-web` (on the Pi the cutter is plugged into) it is HTTP to the same commands, by the same
// names and argument names (`apps/web/src/api.rs`). The web server marks its pages by setting
// `window.__CUTHULHU_WEB__` before anything runs, rather than this guessing from the absence of
// Tauri: a test environment has neither, and must keep reaching the Tauri mock it installs.
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";

export function isWeb(): boolean {
  return typeof window !== "undefined" && (window as { __CUTHULHU_WEB__?: boolean }).__CUTHULHU_WEB__ === true;
}

/** Rejects with what the command rejected with — a string, or an `IpcError` object — as Tauri's
 *  `invoke` does, so `ipcErrorMessage` reads both the same. */
export async function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (!isWeb()) return tauriInvoke<T>(cmd, args);
  let res: Response;
  try {
    res = await fetch(`/api/${cmd}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(args),
    });
  } catch (e) {
    throw `cannot reach the cutter's server: ${e instanceof Error ? e.message : String(e)}`;
  }
  const text = await res.text();
  let body: unknown = text;
  try {
    body = JSON.parse(text);
  } catch {
    // Not JSON: a proxy's page or the sign-in message; shown as it came.
  }
  if (!res.ok) throw body;
  return body as T;
}

/** How often a browser asks for the device state. The desktop is pushed every event; a browser
 *  polls the same published `CutStatus`, which never blocks the server to read. */
const STATUS_POLL_MS = 500;

/** Tauri's `listen`. In a browser only `device-event` happens, as a poll of `get_device_state`
 *  delivered in the event's shape; the others (the desktop's close guard) have nothing to say. */
export async function listen<T>(event: string, handler: EventCallback<T>): Promise<UnlistenFn> {
  if (!isWeb()) return tauriListen<T>(event, handler);
  if (event !== "device-event") return () => {};
  let live = true;
  let busy = false;
  const tick = async () => {
    if (busy || !live) return;
    busy = true;
    try {
      const status = await invoke("get_device_state");
      if (live) handler({ event, id: 0, payload: { status } as T });
    } catch {
      // A missed poll is retried on the next tick; the page's own calls report a server that is down.
    } finally {
      busy = false;
    }
  };
  const timer = window.setInterval(() => void tick(), STATUS_POLL_MS);
  void tick();
  return () => {
    live = false;
    window.clearInterval(timer);
  };
}

/** A file name the server accepts (`apps/web/src/files.rs`): its alphabet, never a leading dot. */
export function serverFileName(name: string): string {
  const cleaned = name.replace(/[^A-Za-z0-9._ -]/g, "_").replace(/^\.+/, "").slice(0, 128);
  return cleaned === "" ? "file" : cleaned;
}

/** Asks the browser for one file and stores it on the server; resolves to the name the server
 *  holds it under, which every path argument then names, or null if nothing was chosen. */
export function uploadPicked(accept: string): Promise<string | null> {
  return new Promise((resolve, reject) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.addEventListener("cancel", () => resolve(null));
    input.addEventListener("change", async () => {
      const file = input.files?.[0];
      if (!file) return resolve(null);
      const name = serverFileName(file.name);
      try {
        const res = await fetch(`/upload?name=${encodeURIComponent(name)}`, { method: "POST", body: file });
        if (!res.ok) throw await res.text();
        resolve(name);
      } catch (e) {
        reject(e);
      }
    });
    input.click();
  });
}

/** Hands a file the server just wrote to the browser as a download. */
export function download(name: string): void {
  const a = document.createElement("a");
  a.href = `/files/${encodeURIComponent(name)}`;
  a.download = name;
  document.body.appendChild(a);
  a.click();
  a.remove();
}
