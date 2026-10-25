// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState, type CSSProperties } from "react";
import { ipcErrorMessage } from "../ipc";
import { uploadPicked } from "../transport";
import { listServerFiles, type ServerFile } from "./server";

export const overlay: CSSProperties = {
  position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex",
  alignItems: "center", justifyContent: "center", zIndex: 100,
};
export const dialog: CSSProperties = {
  background: "var(--panel)", border: "1px solid var(--border)", color: "var(--text)", padding: 16,
  width: 440, maxWidth: "calc(100vw - 32px)", maxHeight: "calc(100vh - 64px)", display: "flex",
  flexDirection: "column", gap: 10,
};
export const button: CSSProperties = {
  background: "var(--panel)", color: "var(--text)", border: "1px solid var(--border)", padding: "4px 10px", cursor: "pointer",
};

function when(seconds: number): string {
  return seconds > 0 ? new Date(seconds * 1000).toLocaleString() : "";
}

/** Open, in a browser: the projects saved on the Pi, or one uploaded from this computer (which is
 *  then kept on the Pi too). Resolves the file name the server holds it under. */
export function PiFilesDialog({ onOpen, onClose }: { onOpen: (name: string) => void; onClose: () => void }) {
  const [files, setFiles] = useState<ServerFile[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listServerFiles()
      .then((all) => setFiles(all.filter((f) => f.name.toLowerCase().endsWith(".cut"))))
      .catch((e) => setError(ipcErrorMessage(e)));
  }, []);

  const upload = () =>
    uploadPicked(".cut").then((name) => { if (name) onOpen(name); }, (e) => setError(ipcErrorMessage(e)));

  return (
    <div style={overlay} onKeyDown={(e) => { if (e.key === "Escape") onClose(); }}>
      <div role="dialog" aria-modal="true" aria-label="Open a project" style={dialog}>
        <div style={{ display: "flex", alignItems: "center" }}>
          <strong>Open a project</strong>
          <div style={{ flex: 1 }} />
          <button style={button} onClick={onClose}>Close</button>
        </div>
        <span style={{ fontSize: 12, color: "var(--muted)" }}>Saved on the Pi</span>
        <div style={{ overflowY: "auto", minHeight: 60, border: "1px solid var(--border)" }}>
          {files === null && error === null ? <p style={{ fontSize: 12, padding: 8, margin: 0 }}>Loading…</p> : null}
          {files?.length === 0 ? <p style={{ fontSize: 12, padding: 8, margin: 0, color: "var(--muted)" }}>Nothing saved on the Pi yet.</p> : null}
          {files?.map((f) => (
            <button
              key={f.name}
              data-testid="pi-file"
              onClick={() => onOpen(f.name)}
              style={{ ...button, display: "flex", width: "100%", border: "none", borderBottom: "1px solid var(--border)", textAlign: "left", gap: 8 }}
            >
              <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{f.name}</span>
              <span style={{ color: "var(--muted)", fontSize: 11 }}>{when(f.modified)}</span>
            </button>
          ))}
        </div>
        {error ? <span role="alert" style={{ fontSize: 12, color: "var(--cut)" }}>{error}</span> : null}
        <div style={{ display: "flex" }}>
          <button style={button} onClick={() => void upload()}>Upload from this computer…</button>
        </div>
      </div>
    </div>
  );
}
