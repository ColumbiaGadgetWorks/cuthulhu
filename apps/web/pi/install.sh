#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Installs cuthulhu-web as a service from the unpacked release folder. Run it again to upgrade:
# the saved files are kept. There is no sign-in: anyone who can reach the Pi can use the cutter.
#
#   sudo ./install.sh              install or upgrade
set -eu
[ "$(id -u)" -eq 0 ] || { echo "Run this with sudo: sudo ./install.sh" >&2; exit 1; }
cd "$(dirname "$0")"

for arg in "$@"; do
  case "$arg" in
    # Sign-in is gone; these are accepted so an old note or script still runs.
    --token|--no-token) ;;
    *) echo "unknown option $arg" >&2; exit 1 ;;
  esac
done

# Checked before anything is installed: a package for another architecture installed cleanly
# and then failed every start with "Exec format error", behind a link that answered nothing.
want=$(cat ARCH 2>/dev/null || echo unknown)
have=$(dpkg --print-architecture 2>/dev/null || echo unknown)
if [ "$have" = armel ] || [ "$(uname -m)" = armv6l ]; then
  echo "This Pi (an original Pi Zero / Zero W or a Pi 1) is not supported: use a Pi 2, 3, 4, 5 or Zero 2 W." >&2
  exit 1
fi
if [ "$want" != "$have" ]; then
  echo "This package is for $want, but this system is $have." >&2
  echo "Download cuthulhu-web-linux-$have.tar.gz from the same release instead." >&2
  exit 1
fi

id cuthulhu >/dev/null 2>&1 || useradd --system --no-create-home --shell /usr/sbin/nologin cuthulhu
# A Puma is a serial device.
usermod -a -G dialout cuthulhu

install -d -o cuthulhu -g cuthulhu -m 0750 /var/lib/cuthulhu-web
install -m 0755 cuthulhu-web /usr/local/bin/cuthulhu-web
rm -rf /usr/local/share/cuthulhu-web/ui
install -d /usr/local/share/cuthulhu-web
cp -r ui /usr/local/share/cuthulhu-web/ui

# An older install's sign-in token and its on/off marker.
rm -rf /etc/cuthulhu-web

install -m 0644 60-cuthulhu-web.rules /etc/udev/rules.d/60-cuthulhu-web.rules
udevadm control --reload-rules && udevadm trigger

install -m 0644 cuthulhu-web.service /etc/systemd/system/cuthulhu-web.service

# Updates from the page: the server writes a request, and this root unit acts on it (update.sh).
install -d /usr/local/lib/cuthulhu-web
install -m 0755 update.sh /usr/local/lib/cuthulhu-web/update.sh
install -m 0644 cuthulhu-web-update.service /etc/systemd/system/cuthulhu-web-update.service
install -m 0644 cuthulhu-web-update.path /etc/systemd/system/cuthulhu-web-update.path

systemctl daemon-reload
systemctl enable cuthulhu-web cuthulhu-web-update.path >/dev/null 2>&1
systemctl restart cuthulhu-web
systemctl start cuthulhu-web-update.path

# `restart` returns once the process is spawned, not once it is serving. A server that exits at
# once (a port already taken, a missing file) shows as not active a moment later.
sleep 3
if ! systemctl is-active --quiet cuthulhu-web; then
  echo "cuthulhu-web did not stay running. Its log:" >&2
  journalctl -u cuthulhu-web -n 20 --no-pager >&2
  exit 1
fi

addr=$(hostname -I 2>/dev/null | awk '{print $1}')
echo
echo "cuthulhu-web is running. Open http://${addr:-<pi-address>}:8080/ on any computer on this network."
echo "Unplug and replug the cutter if it was already connected."
