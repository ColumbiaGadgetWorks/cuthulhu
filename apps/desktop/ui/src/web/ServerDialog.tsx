// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useRef, useState } from "react";
import { ipcErrorMessage } from "../ipc";
import { button, dialog, overlay } from "./PiFilesDialog";
import { applyUpdate, checkForUpdate, updateStatus, type UpdateCheck, type UpdateStatus } from "./server";

/** How often the dialog asks how an update is going. The install restarts the server, so some of
 *  these fail, and the first to answer again comes from the new version. */
const POLL_MS = 2000;

/** The Pi server's version and updates: check the repository's releases and install the newest.
 *  It checks once on opening, so "Update" is one click from the top bar. */
export function ServerDialog({ onClose }: { onClose: () => void }) {
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [check, setCheck] = useState<UpdateCheck | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [updating, setUpdating] = useState<string | null>(null);
  const startedFrom = useRef<string | null>(null);

  const load = () =>
    updateStatus().then((s) => {
      setStatus(s);
      return s;
    });

  useEffect(() => {
    load().catch((e) => setError(ipcErrorMessage(e)));
    checkForUpdate().then(setCheck, (e) => setError(ipcErrorMessage(e)));
  }, []);

  // While an update runs: keep asking until the server answers with another version, or says the
  // install failed. A failed poll is the restart, not an error.
  useEffect(() => {
    if (updating === null) return;
    const timer = window.setInterval(() => {
      load()
        .then((s) => {
          if (s.current !== startedFrom.current || s.last?.state === "done") {
            setUpdating(null);
          } else if (s.last?.state === "failed" && !s.pending) {
            setUpdating(null);
            setError(s.last.message);
          }
        })
        .catch(() => {});
    }, POLL_MS);
    return () => window.clearInterval(timer);
  }, [updating]);

  const run = async (f: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await f();
    } catch (e) {
      setError(ipcErrorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const updated = status !== null && startedFrom.current !== null && status.current !== startedFrom.current;

  return (
    <div style={overlay} onKeyDown={(e) => { if (e.key === "Escape") onClose(); }}>
      <div role="dialog" aria-modal="true" aria-label="Pi server" style={dialog}>
        <div style={{ display: "flex", alignItems: "center" }}>
          <strong>Pi server</strong>
          <div style={{ flex: 1 }} />
          <button style={button} onClick={onClose}>Close</button>
        </div>
        {status ? (
          <span style={{ fontSize: 12 }}>
            cuthulhu-web {status.current} ({status.arch})
          </span>
        ) : null}

        {status ? (
          <span style={{ fontSize: 12, color: "var(--muted)" }}>Updates from github.com/{status.repo}</span>
        ) : null}

        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <button style={button} disabled={busy || updating !== null} onClick={() => void run(async () => setCheck(await checkForUpdate()))}>
            Check for updates
          </button>
          {check?.updateAvailable && check.latest ? (
            <button
              style={{ ...button, background: "var(--accent)", color: "#0b0b0d" }}
              disabled={busy || updating !== null}
              onClick={() =>
                void run(async () => {
                  startedFrom.current = status?.current ?? null;
                  setStatus(await applyUpdate(check.latest!));
                  setUpdating(check.latest);
                })
              }
            >
              Update to {check.latest.replace(/^web-v/, "")}
            </button>
          ) : null}
        </div>

        {check && !check.updateAvailable ? (
          <span style={{ fontSize: 12, color: "var(--muted)" }}>
            {check.latest ? `Up to date (newest is ${check.latest.replace(/^web-v/, "")}).` : `No web release for ${status?.arch} in ${status?.repo}.`}
          </span>
        ) : null}
        {updating ? (
          <span style={{ fontSize: 12 }}>
            Installing {updating}… {status?.last?.state === "running" ? status.last.message : "waiting for the installer"}. The page
            loses the server for a few seconds while it restarts.
          </span>
        ) : null}
        {updated ? (
          <span style={{ fontSize: 12, color: "var(--ready)" }}>
            Updated to {status?.current}.{" "}
            <button style={button} onClick={() => window.location.reload()}>Reload the page</button>
          </span>
        ) : null}
        {error ? <span role="alert" style={{ fontSize: 12, color: "var(--cut)" }}>{error}</span> : null}
      </div>
    </div>
  );
}
