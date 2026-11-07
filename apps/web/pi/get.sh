#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Installs or upgrades cuthulhu-web to the newest `web-v…` release, in one line over SSH:
#
#   curl -fsSL https://raw.githubusercontent.com/ColumbiaGadgetWorks/cuthulhu/main/apps/web/pi/get.sh | sudo sh
#
# The same steps as by hand (download the package for this Pi, unpack, run its install.sh), so an
# install that is too old to update itself from the page can be brought up to one that can.
# CUTHULHU_REPO=owner/name takes releases from another repository.
set -eu
[ "$(id -u)" -eq 0 ] || { echo "Run this with sudo." >&2; exit 1; }
repo=${CUTHULHU_REPO:-ColumbiaGadgetWorks/cuthulhu}
arch=$(dpkg --print-architecture)
asset="cuthulhu-web-linux-$arch.tar.gz"

# Every release's download links, filtered to this Pi's package on a web-v tag; sort -V puts the
# newest last. Read with grep rather than jq, which Raspberry Pi OS does not ship. Matched on any
# owner: after a repository moves, GitHub answers under the old name but writes the new one into
# the links, so matching `$repo` there found nothing.
url=$(curl -fsSL --max-time 30 "https://api.github.com/repos/$repo/releases?per_page=50" \
  | grep -o "https://github.com/[A-Za-z0-9_.-]*/[A-Za-z0-9_.-]*/releases/download/web-v[0-9.]*/$asset" \
  | sort -t/ -k8V | tail -n 1) || true
[ -n "$url" ] || { echo "No $asset release found in $repo." >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
echo "Downloading $url"
curl -fsSL --max-time 300 -o "$work/pkg.tar.gz" "$url"
tar xzf "$work/pkg.tar.gz" -C "$work"
"$work/cuthulhu-web/install.sh"
