#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Installs cuthulhu-web as a service from the unpacked release folder. Run it again to upgrade:
# the token and the saved files are kept.
set -eu
[ "$(id -u)" -eq 0 ] || { echo "Run this with sudo: sudo ./install.sh" >&2; exit 1; }
cd "$(dirname "$0")"

id cuthulhu >/dev/null 2>&1 || useradd --system --no-create-home --shell /usr/sbin/nologin cuthulhu
# A Puma is a serial device.
usermod -a -G dialout cuthulhu

install -d -o cuthulhu -g cuthulhu -m 0750 /var/lib/cuthulhu-web
install -m 0755 cuthulhu-web /usr/local/bin/cuthulhu-web
rm -rf /usr/local/share/cuthulhu-web/ui
install -d /usr/local/share/cuthulhu-web
cp -r ui /usr/local/share/cuthulhu-web/ui

install -d -o root -g cuthulhu -m 0750 /etc/cuthulhu-web
if [ ! -s /etc/cuthulhu-web/token ]; then
  head -c 24 /dev/urandom | base64 | tr -d '/+=' > /etc/cuthulhu-web/token
fi
chown root:cuthulhu /etc/cuthulhu-web/token
chmod 0640 /etc/cuthulhu-web/token

install -m 0644 60-cuthulhu-web.rules /etc/udev/rules.d/60-cuthulhu-web.rules
udevadm control --reload-rules && udevadm trigger

install -m 0644 cuthulhu-web.service /etc/systemd/system/cuthulhu-web.service
systemctl daemon-reload
systemctl enable cuthulhu-web >/dev/null
systemctl restart cuthulhu-web

addr=$(hostname -I 2>/dev/null | awk '{print $1}')
echo
echo "cuthulhu-web is running. Open this once on each computer, then bookmark http://${addr:-<pi-address>}:8080/"
echo "  http://${addr:-<pi-address>}:8080/?token=$(cat /etc/cuthulhu-web/token)"
echo "Unplug and replug the cutter if it was already connected."
