<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Project summary: Silhouette Cameo 1 on Linux for the makerspace

A record of the work done with Claude on this project, as of 2026-11-07, and what to know to carry
it on.

## The goal

Columbia Gadget Works has an original Silhouette Cameo (Cameo 1, USB id `0b4d:1121`). Silhouette
dropped Cameo 1 USB support on Windows 10 1809 and later, so it would not work from Windows. The goal
was to run it from Linux with a full, simple UI that makerspace members can use, with nothing to
install on members' own machines.

**Where it ended up:** the Cameo 1 is plugged into a Raspberry Pi running **cuthulhu-web**, and
members use it from a browser on the makerspace network at `http://<pi-address>:8080/`. The Cameo 1
was tested on real hardware and cut correctly.

## Repositories

| Repository | What it holds |
|---|---|
| `adman234/cuthulhu` (fork of `vcolombo/cuthulhu`) | **The project.** All code lives here. Work goes to `main`. |
| `adman234/cuthulhu`, branch `cameo1-support` | The Cameo 1 driver only, kept clean to send upstream as a PR. |
| `columbiagadgetworks/cameo-1-linux`, branch `claude/vigilant-ptolemy-j190cf` | The first plan (`PLAN.md`) and the software research (`docs/SOFTWARE-RESEARCH.md`). Historical. |

Cuthulhu is GPLv3 cutting software: a Rust engine with a Tauri + React desktop app, a CLI, a Cut
Host daemon and (in this fork) a browser UI. Read `CLAUDE.md` and `CONTEXT.md` before changing code.

## What was done, in order

1. **Plan.** `PLAN.md` in `cameo-1-linux`: why Windows fails, and how to drive the Cameo 1 from
   Linux.
2. **Research.** `docs/SOFTWARE-RESEARCH.md`: a comparison of free cutter software. Cuthulhu was
   chosen over Inkscape with the inkscape-silhouette plugin because it already had an editor,
   trace, presets, pass planning and a GPGL driver.
3. **Cameo 1 driver** (branch `cameo1-support`, merged into `main`):
   - `crates/driver-silhouette`: a `Model` enum (Cameo 5 Alpha, Cameo 1), the Cameo 1's GPGL
     dialect, and USB open that detaches the kernel's `usblp` printer driver.
   - The `cameo1` machine id in `driver-registry`, a 295 × 2999 mm profile, and builtin material
     presets.
   - `docs/protocol/silhouette-cameo1.md`: every protocol fact cites inkscape-silhouette
     `silhouette/Graphtec.py` (GPL).
   - `docs/linux/99-silhouette-cameo1.rules`: lets users open the cutter without root.
4. **Simple LightBurn-style UI** (`apps/desktop/ui/src/simple/`, fork only):
   - a colour palette strip under the canvas;
   - a Cuts / Layers dock with material, speed, force, passes, output on/off and order;
   - a Cutter panel with Connect, Test cut, Start, Stop and Resume;
   - a **Classic layout** toggle in the top bar.
5. **Second round of features**, items 2–21, 23–25 and 27 from the suggested list:
   - Machine and cut settings: per-machine speed limit, track enhancing, pen mode, test cut.
   - Print & cut with registration marks; media presets; mirror for HTV.
   - Design tools: weed box and weed lines, copies, offset/contour, text editing with weld, nesting.
   - The dock: cut preview, layer reorder, palette highlight, dimmed Output-off shapes, time estimate,
     beginner mode.
   - Saving and sharing: layer settings saved in the project (project manifest version 3), a usage
     log with CSV export, a shared presets file, and material notes with an advisory blade depth.
   - Packaging and docs: a Linux `.deb` with the udev rule and a menu entry, a USB test that skips
     when there is no USB bus, and a rewritten README.
6. **Releases** (added in a parallel session, PRs #1–#7 on the fork):
   - The `v*` tags build a Windows installer and Linux `.deb`, `.rpm` and AppImage.
   - The `web-v*` tags build the Pi web UI for arm64, armhf (32-bit) and amd64.
7. **Browser UI on the Pi** (`apps/web`, PRs #4–#7):
   - `cuthulhu-web` serves the same editor to any browser, from the computer the cutter is plugged
     into.
   - Projects are saved and opened on the Pi.
   - The Pi updates itself from the page.
8. **Sign-in removed** (`90a28f7`, `6abf2a9`), at the user's request:
   - The web UI opens with no token.
   - The desktop ↔ Cut Host pairing asks only for the address and a fingerprint check.
   - Old config files and old `?token=` bookmarks still work.
9. **Easier updates:**
   - **Updates…** in the web UI's top bar checks GitHub on page load and shows
     **Update available…** when a newer release exists.
   - `apps/web/pi/get.sh` installs or upgrades the Pi in one line over SSH.
   - Current Pi release: **web-v0.1.4**.

## How the makerspace uses it

**On the Pi** (install, or upgrade over SSH):

```sh
curl -fsSL https://raw.githubusercontent.com/adman234/cuthulhu/main/apps/web/pi/get.sh | sudo sh
```

This picks the right package for the Pi (64-bit or 32-bit), installs the `cuthulhu-web` service on
port 8080, adds the USB rule for the Cameo 1 and Cameo 5, and prints the address. If the cutter was
already plugged in, unplug and replug it once.

**Members:** open `http://<pi-address>:8080/` in a browser. There is no sign-in.

**Updating later:** click **Updates…**, then **Update to x.y.z**. The Pi needs internet access.

**The details** are in these docs:
- `docs/web-ui.md`: the Pi web UI.
- `docs/linux/makerspace-quickstart.md`: the desktop app on a Linux PC wired directly to the cutter.
- `docs/cut-host.md`: the alternative Pi daemon that desktop apps send cuts to.

## How to release a change

Releases are what reach the Pi; pushing to `main` alone does not.

1. Make the change on `main`. Then pass the CI gates in `CLAUDE.md`:
   - `cargo test --workspace --locked`
   - If anything in `ui/src` changed, rebuild and commit `apps/desktop/ui/dist`.
   - If an IPC command changed, regenerate `apps/desktop/ipc-inventory.json`.
2. Bump `version` in `apps/web/Cargo.toml` and refresh the lock: `cargo update -p web --offline`.
3. Push, then run the **Release (Pi web UI)** workflow (`release-web.yml`) on `main`, with the
   `tag` input set to `web-v<version>`. A Claude session cannot push tags (the proxy refuses), so
   start the workflow from the GitHub Actions page or with the GitHub tool's `run_workflow`. It
   creates the tag and publishes the arm64, armhf and amd64 packages.
4. The Pi's **Updates…** button then offers the new version.

Desktop app releases use the `v*` tags and `release.yml` the same way, and build the Windows
installer and the Linux `.deb`, `.rpm` and AppImage.

## Notes for continuing

- **Future work is mostly UI changes and improvements.** The UI is shared: `apps/desktop/ui` is
  both the desktop window and what the Pi serves. A UI change needs `dist/` rebuilt and committed,
  and then a `web-v*` release.
- **Open the upstream PR for the Cameo 1 driver.** It now cuts on real hardware, so this is ready
  once the hardware checklist is filled in:
  - Tick the Cameo 1 section of `apps/desktop/MANUAL-CHECKLIST.md` with the device and date.
  - Open a PR from `cameo1-support` to `vcolombo/cuthulhu`.
  - Before opening it, follow the two review gates in `CLAUDE.md`.
  - Keep the simple UI, web UI and sign-in removal out of it; those are fork-only.
- **Still unverified on hardware, even though basic cutting works:** track enhancing, pen mode,
  registration-mark search (print & cut, which waits up to 40 s for the marks), test cut, and the
  print & cut margins. They are listed in the Cameo 1 section of `MANUAL-CHECKLIST.md`.
- **There is no sign-in, on purpose.** Anyone on the network can use the cutter. Keep the Pi on the
  makerspace LAN and never forward port 8080 or 7878 to the internet. The web page cannot change
  which GitHub repository updates come from, because an update runs as root. To change it, use SSH;
  see `docs/web-ui.md`.
- **Keep versions matched.** The Cut Host and the desktops must run the same build: there is no
  version handshake, and a mismatch shows up as "the host could not be reached". A handshake is the
  noted upgrade path.
- **Run either cuthulhu-web or the Cut Host daemon on a Pi, not both.** Only one program can hold
  the USB device.
- **There is one shared document** in the web UI. Every open browser edits the same design, which
  suits one person at the cutter at a time.
- **Conventions** (`CLAUDE.md`):
  - SPDX headers on every file.
  - `CONTEXT.md` vocabulary: no "layer" in code; use DocumentPass / PassKey.
  - Commit subjects are imperative and give the reason.
  - Protocol facts cite their source.
  - `// ponytail:` marks a deliberate shortcut.
- **Ideas not done yet:**
  - a blade-life counter;
  - true-shape nesting (the current Nest packs bounding boxes);
  - manual registration (`TB23`);
  - Cameo 5 registration (`TB124`);
  - a version/capability handshake between desktop and Cut Host.
