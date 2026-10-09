#!/bin/sh
# Fails if any path inside the given packages contains "widevine" (case-insensitive).
set -eu
[ "$#" -gt 0 ] || { echo "usage: $0 PKG..." >&2; exit 2; }
for p in "$@"; do
  if bsdtar -tf "$p" | grep -i widevine; then
    echo "FAIL: Widevine file in $p" >&2
    exit 1
  fi
done
echo "ok: no widevine in $*"
