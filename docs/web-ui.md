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
- generates an access token in `/etc/cuthulhu-web/token` (kept on reinstall);
- starts the `cuthulhu-web` service on port 8080 and enables it at boot.

It ends by printing the address with the token, like
`http://192.168.1.50:8080/?token=…`. Open that once in each browser: it signs that browser in for a
year. After that, `http://192.168.1.50:8080/` is enough, so bookmark it.

If the cutter was already plugged in, unplug and replug it once so the new udev rule applies.

To upgrade, unpack the newer release and run its `install.sh` again. To see the server's log:
`journalctl -u cuthulhu-web -f`.

## Using it

The page is the desktop's editor. What is different in a browser:

- **Open** uploads a `.cut` project from your computer; **Save** downloads it. A copy of everything
  saved or opened also stays on the Pi, under `/var/lib/cuthulhu-web/files`, which is what
  **Reload** reads.
- **Trace** uploads the image to trace.
- **Print & Cut** and the usage log's CSV export download their files.
- **Cut Hosts** are not offered: the Pi is the machine the cutter is on.
- Fonts for text are the Pi's. `sudo apt install fonts-…` adds more; restart the service after.

**There is one document, shared by every browser.** Two people with the page open are looking at
the same design, and both their edits land in it. A browser picks up the other's changes on its next
edit or reload. This suits one cutter with one person at it at a time, which is what a shared cutter
is.

## Security

Anyone who can reach port 8080 and has the token can cut. The token is a long random string, and
the browser keeps it in a cookie that other websites cannot make it send. The connection is plain
HTTP, so keep the Pi on a network you trust (your home or makerspace LAN), and do not forward the
port to the internet.

To change the token, put a new one in `/etc/cuthulhu-web/token` and
`sudo systemctl restart cuthulhu-web`; every browser then signs in again with the new link.

Running without a token at all (dropping `--token-file` from the service) lets anyone on the network
use the cutter without signing in.

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
cuthulhu-web --bind 0.0.0.0:8080 --ui apps/desktop/ui/dist --data ./web-data --token-file ./token
```
