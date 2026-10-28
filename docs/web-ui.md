<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# cuthulhu web UI on a Raspberry Pi

`cuthulhu-web` runs on the computer the cutter is plugged into — usually a Raspberry Pi — and serves
the cuthulhu editor to any browser on your network. Design, set up layers and cut from a laptop,
tablet or phone; nothing is installed on them.

It is the desktop app's own editor and the desktop's own cutting code: the same planning, the same
preflight checks, the same drivers. Only the window differs.

## Install

Any Pi 2, 3, 4, 5 or Zero 2 W works. The original Pi Zero / Zero W and the Pi 1 do not: their
ARMv6 chip is older than any build here.

Find which package the Pi needs:

```sh
dpkg --print-architecture
```

`arm64` (64-bit Raspberry Pi OS) takes `cuthulhu-web-linux-arm64.tar.gz`; `armhf` (32-bit Raspberry
Pi OS, and any Pi 2) takes `cuthulhu-web-linux-armhf.tar.gz`. Download it from the repository's
Releases page (the `web-v…` releases), then:

```sh
tar xzf cuthulhu-web-linux-<arch>.tar.gz
sudo ./cuthulhu-web/install.sh
```

The installer refuses the wrong package and names the right one, and checks the server is running
before it prints the address.

The installer:

- creates the `cuthulhu` service account and gives it the serial ports (a Puma) and, by udev rule,
  the Cameos' USB devices;
- installs the server to `/usr/local/bin/cuthulhu-web` and the page to
  `/usr/local/share/cuthulhu-web/ui`;
- starts the `cuthulhu-web` service on port 8080 and enables it at boot.

It ends by printing the address, like `http://192.168.1.50:8080/`. Open it in any browser on the
network and bookmark it; there is no sign-in.

If the cutter was already plugged in, unplug and replug it once so the new udev rule applies.

To upgrade, use **Updates…** in the page (below), or over SSH:

```sh
curl -fsSL https://raw.githubusercontent.com/adman234/cuthulhu/main/apps/web/pi/get.sh | sudo sh
```

That downloads the newest `web-v…` release for this Pi and runs its `install.sh`; saved projects
are kept. To see the server's log: `journalctl -u cuthulhu-web -f`.

## Using it

The page is the desktop's editor. What is different in a browser:

- **Save** asks for a name and saves the project on the Pi. **Open** lists the projects saved on
  the Pi, newest first, or uploads one from your computer (which is then kept on the Pi too).
  **Download** saves and puts a copy on your computer. Projects live in
  `/var/lib/cuthulhu-web/files` on the Pi.
- **Trace** uploads the image to trace.
- **Print & Cut** and the usage log's CSV export download their files.
- **Cut Hosts** are not offered: the Pi is the machine the cutter is on.
- Fonts for text are the Pi's. `sudo apt install fonts-…` adds more; restart the service after.

**There is one document, shared by every browser.** Two people with the page open are looking at
the same design, and both their edits land in it. A browser picks up the other's changes on its next
edit or reload. This suits one cutter with one person at it at a time, which is what a shared cutter
is.

## Updates

**Updates…** in the top bar shows the version running and checks GitHub for a newer `web-v…`
release with a package for this Pi. The page also checks once when it loads, and the button turns
into **Update available…** when there is one. **Update to …** installs it: the server restarts, and
the page says when the new version is up. The Pi needs internet access for this.

The server itself runs as an unprivileged account and cannot install anything. It writes a request,
and a root systemd unit (`cuthulhu-web-update.path`) downloads that release from GitHub and runs its
`install.sh`. Its progress is in `/var/lib/cuthulhu-web-update/status.json`, and its log in
`journalctl -u cuthulhu-web-update`.

Updates come from `adman234/cuthulhu`. The page cannot change that: an update runs as root, and
the page has no sign-in, so whoever could choose where it comes from could run anything on the Pi.
To take updates from another repository, change it over SSH:

```sh
echo '{"repo":"OWNER/NAME"}' | sudo -u cuthulhu tee /var/lib/cuthulhu-web/update-settings.json
```

## Security

There is no sign-in: anyone who can reach port 8080 can use the cutter and install the newest
release. Keep the Pi on a network you trust (your home or makerspace LAN), and do not forward the
port to the internet.


## The same Pi as a Cut Host

`cuthulhu-web` and the Cut Host daemon (`docs/cut-host.md`) both want the cutter, and only one
program can hold a USB device at a time. Run one or the other on a Pi, not both.

## Building it yourself

```sh
cross build --release --locked --target aarch64-unknown-linux-gnu -p web --bin cuthulhu-web
# 32-bit Raspberry Pi OS:
cross build --release --locked --target armv7-unknown-linux-gnueabihf -p web --bin cuthulhu-web
```

Building `-p web` on its own leaves out the desktop's Tauri shell, so nothing from GTK or WebKit is
needed. The page it serves is `apps/desktop/ui/dist`, committed in the repository.

Run it by hand with:

```sh
cuthulhu-web --bind 0.0.0.0:8080 --ui apps/desktop/ui/dist --data ./web-data
```
