<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Makerspace quick start: Silhouette Cameo 1 on Linux

This fork (`adman234/cuthulhu`) adds two things: Cameo 1 support (`cameo1`), and a simple window
laid out like LightBurn. The Cameo 1 support is meant to go upstream once it has been verified on
hardware. Until the hardware checklist in `apps/desktop/MANUAL-CHECKLIST.md` (§ Cameo 1) is
ticked, treat every cut as a test cut and use scrap media.

## 1. Build the package (once, on any Ubuntu 22.04+ / Debian 12+ machine)

Each release on the repository's GitHub Releases page carries a prebuilt `.deb`, `.rpm` and
AppImage: download the `.deb` and skip to step 2. To build it yourself instead, build once, then
copy the `.deb` to the other computers; they do not need any of the build tools.

```sh
sudo apt-get install -y build-essential curl git pkg-config libssl-dev libudev-dev \
  libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev file
curl https://sh.rustup.rs -sSf | sh && . ~/.cargo/env   # Rust, if not installed
# Node 22: the build runs the UI's `npm run build`, and Vite needs Node 20.19 or newer —
# Debian 12's own nodejs is 18, so take it from NodeSource (or nvm) instead.
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash - && sudo apt-get install -y nodejs
cargo install tauri-cli --version '^2' --locked   # once
git clone https://github.com/adman234/cuthulhu && cd cuthulhu
npm --prefix apps/desktop/ui ci                   # the UI's dependencies, once per checkout
(cd apps/desktop && cargo tauri build --bundles deb,appimage)
# → target/release/bundle/deb/cuthulhu_0.2.1_amd64.deb
# → target/release/bundle/appimage/cuthulhu_0.2.1_amd64.AppImage
```

`--bundles rpm` builds a Fedora package the same way.

## 2. Install it on each computer

```sh
sudo apt install ./cuthulhu_0.2.1_amd64.deb
```

The `.deb` does three things beyond copying the app:

- installs the udev rule `docs/linux/99-silhouette-cameo1.rules` as
  `/etc/udev/rules.d/99-silhouette-cameo1.rules`, so members can use the cutter without root;
- reloads udev after installing (and after removing), so a Cameo that is already plugged in works
  without a replug — if it still is not found, unplug and replug it once;
- adds **Cuthulhu** to the applications menu under Graphics and Engineering.

You do not need to blacklist the kernel printer driver (`usblp`): Cuthulhu detaches it when it
opens the cutter.

### AppImage instead of a .deb

An AppImage cannot install system files, so on a computer that runs the AppImage install the udev
rule by hand, once:

```sh
sudo cp docs/linux/99-silhouette-cameo1.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
chmod +x cuthulhu_0.2.1_amd64.AppImage && ./cuthulhu_0.2.1_amd64.AppImage
```

Then unplug and replug the Cameo.

### Check the setup

From a source checkout:

```sh
cargo run -p cli -- list-devices        # should list a cameo1 device
cargo run -p cli -- cut square.svg --device cameo1 --dry-run   # prints the bytes, cuts nothing
```

## 3. Cutting, in the simple window

1. **Machine:** in the top bar, choose **Silhouette Cameo**. The artboard becomes 295 × 2999 mm,
   which is the area the blade can reach. Artboard (0, 0) is 9 mm from the left edge of the media
   and 1 mm from the top.
2. **Design:** draw, add text, trace, or **Import** an SVG.
3. **Layers:** select shapes and click a colour in the **Layer** strip under the canvas. Each
   colour is one row in **Cuts / Layers**, and the rows are cut in the order shown.
4. **Settings per layer:**
   - **Material** sets speed and force from a preset (Vinyl Sticker, Cardstock…).
   - **Spd**, **Force** and **Passes** override the preset. Leave them blank to use it. The
     Cameo 1's speed goes up to 10.
   - **Out** off skips that layer, and its shapes go faint on the canvas.
   - **Trk** (track enhancing) rolls the media back and forth first so the rollers grip; the Cameo 1
     only does it at force 19 or more. **Pen** draws with a pen instead of cutting.
   - **↑ / ↓** change the cut order. Everything on a layer is saved in the project file.
   - **Beginner** (top right of the panel) hides everything but the material picker. Switch it on
     once on a shared computer; it is remembered there.
5. **Job:** pick the **Media** you loaded (12×12 or 12×24 mat, Letter, A4, roll); the artboard
   follows it, and a cut past its edge is refused. Tick **Mirror (HTV)** for heat-transfer vinyl,
   which is cut face down — it turns on by itself when you pick a material marked as cut
   mirrored. Never flip the design as well, or it is mirrored twice.
6. **Cutter:** type your name in **Operator** (for the usage log), load the mat or media using the
   machine's own button, pick the cutter, and press **Connect cutter**. **Test cut** cuts a 10 mm
   square with a triangle at X/Y using the top layer's settings — weed it to check the depth. Then
   press **Start**; the estimated time is shown above it. **Stop** cancels the cut, but the Cameo
   finishes the moves it has already received. With several layers, **Resume** continues after
   each one.
7. **Show cut preview** under the layers draws the cut order and the head's travel.
   **Preview, presets and Cut Hosts…** opens the full cut dialog: edit presets there, and cut
   through a Raspberry Pi Cut Host.

### Design tools

The tool rail on the left has **Offset…** (a contour around a sticker, or an inset), **Weed box…**
(a border to weed to, with optional weed lines that stop short of the letters), **Copies…** (a
grid of the selection), **Weld** (join overlapping letters into one shape), **To path**, and
**Nest…** (pack the selection along the media to save vinyl). **Edit text…** in the Objects tab,
or double-clicking text on the canvas, changes the words, size and font.

### Print & cut

1. **Registration marks** in the top bar opens Print & cut: add the marks for Letter or A4, then
   **Export for printing** and print the sheet at 100 % (no "fit to page").
2. Put the printed sheet on the mat with its top-left corner at the mat's top-left, load it, and
   cut with **Print & cut** ticked in Cuts / Layers. The Cameo looks for the marks first (up to
   40 s) and refuses the job if it cannot find them.
3. Leave the marks out of any selection you weld, offset or nest; the tools refuse them. Print &
   cut cannot be mirrored.

**Classic layout** in the top bar switches back to the original layers and properties window.
The window remembers which layout you used last.

## 4. Usage log

Every cut started from this computer — on a local cutter or through a Cut Host — is recorded in
`~/.config/cuthulhu/usage.jsonl` with its start and end time, operator, cutter, document, passes
and settings, cut length and how it ended. Type a name in **Operator** in the cut dialog; it is
remembered on that computer. **Usage log…** in the top bar lists recent jobs with totals per
operator and per material, and **Export CSV…** saves the whole log for a spreadsheet.

## 5. One set of material presets for every computer

By default each computer keeps its own presets in `~/.config/cuthulhu/presets.json`. To share
them, put the presets on a network share that every computer mounts at the same path (for
example `/mnt/makerspace/cuthulhu/`), then on each computer open the cut dialog, connect a cutter,
and under **Material presets** press **Change…** next to **Presets file** and choose that folder.
The choice is saved in `~/.config/cuthulhu/settings.json`; **Use default** switches back.

If the share is not mounted, the presets section and any cut that uses a preset say the presets
file cannot be reached — mount the share, or press **Use default**. Cuthulhu never creates the
folder itself, so an unmounted share is not silently replaced by a local copy. Two people saving
a preset in the same second can still overwrite each other's change; edit presets from one
computer at a time.

### Notes, blade depth and mirror

A preset can carry **Notes** (brand, which side up), an advisory **Blade depth** (1–10, for the
Cameo 1's hand-set ratchet blade — Cuthulhu never sends it) and **Cut mirrored (HTV)**. All three
are shown under the material picker when a pass uses that preset. Cut mirrored turns the job's
**Mirror** on when the material is picked in the simple window, and the classic cut dialog has a
**Cut mirrored** box; the design itself is never flipped, so do not flip it yourself.

## Not verified yet

Everything Cameo 1-specific — track enhancing, pen, test cut, finding registration marks and the
`.deb` install — is built from the reference driver's source and tested without hardware, not yet
on a real Cameo 1. `apps/desktop/MANUAL-CHECKLIST.md` lists what to try. A Raspberry Pi Cut Host
must run the same build as the desktops (see `docs/cut-host.md`).
