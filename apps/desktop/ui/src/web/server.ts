// SPDX-License-Identifier: GPL-3.0-or-later
//
// What only the web server answers: the files it holds and its own updates (`apps/web`). Kept out
// of `ipc.ts`, whose every export is a desktop command checked against the desktop's inventory.
import { invoke } from "../transport";

export type ServerFile = { name: string; size: number; modified: number };

/** Files saved on the server, newest first. */
export async function listServerFiles(): Promise<ServerFile[]> {
  const res = await fetch("/files");
  if (!res.ok) throw await res.text();
  return res.json();
}

export type UpdateStatus = {
  current: string;
  arch: string;
  repo: string;
  /** False while the server runs without sign-in: an update installs as root, so where it comes
   *  from is not something anyone on the network may choose. */
  canChangeRepo: boolean;
  /** An update was asked for and the installer has not picked it up yet. */
  pending: boolean;
  last: { state: "running" | "done" | "failed"; message: string; at: number } | null;
};

export type UpdateCheck = { status: UpdateStatus; latest: string | null; url: string | null; updateAvailable: boolean };

export const updateStatus = () => invoke<UpdateStatus>("web_update_status");
export const checkForUpdate = () => invoke<UpdateCheck>("web_update_check");
export const applyUpdate = (tag: string) => invoke<UpdateStatus>("web_update_apply", { tag });
export const setUpdateRepo = (repo: string) => invoke<UpdateStatus>("web_update_set_repo", { repo });
