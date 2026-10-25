#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Run as root by cuthulhu-web-update.service when the web server writes an update request
# ("<owner/repo> <web-vX.Y.Z>"). The server is unprivileged and only asks; this downloads the
# release for this machine and runs its install.sh. Both strings come from a process the network
# can talk to, so each is checked again here before it reaches a URL or a shell.
set -u
data=/var/lib/cuthulhu-web
req="$data/update-request"
# Not in $data: that directory is the web server's, and a file root writes there could be a symlink
# the server planted pointing anywhere. This one is root's, and the server only reads it.
statusdir=/var/lib/cuthulhu-web-update
status="$statusdir/status.json"
install -d -m 0755 "$statusdir"

say() { # state, message
  printf '{"state":"%s","message":"%s","at":%s}\n' "$1" "$2" "$(date +%s)" > "$status.tmp" && mv -f "$status.tmp" "$status"
}

[ -f "$req" ] || exit 0
line=$(head -c 200 "$req")
rm -f "$req"
repo=${line%% *}
tag=$(printf '%s' "${line#* }" | tr -d '\n')

case "$repo" in
  */*/*|.*|*/.*|"") say failed "refused: bad repository"; exit 1 ;;
esac
printf '%s' "$repo" | grep -Eq '^[A-Za-z0-9_.-]{1,100}/[A-Za-z0-9_.-]{1,100}$' || { say failed "refused: bad repository"; exit 1; }
printf '%s' "$tag" | grep -Eq '^web-v[0-9]+(\.[0-9]+)*$' || { say failed "refused: bad tag"; exit 1; }

arch=$(dpkg --print-architecture)
url="https://github.com/$repo/releases/download/$tag/cuthulhu-web-linux-$arch.tar.gz"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

say running "downloading $tag"
if ! curl -fsSL --max-time 300 -o "$work/pkg.tar.gz" "$url"; then
  say failed "could not download $url"; exit 1
fi
if ! tar xzf "$work/pkg.tar.gz" -C "$work"; then
  say failed "the download is not a release package"; exit 1
fi
say running "installing $tag"
# install.sh restarts the server, which drops the page's connection for a few seconds.
if "$work/cuthulhu-web/install.sh" > "$work/install.log" 2>&1; then
  say done "updated to $tag"
else
  say failed "install failed: $(tail -n 3 "$work/install.log" | tr '\n"' ' _')"
  exit 1
fi
