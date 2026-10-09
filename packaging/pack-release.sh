#!/bin/sh
# Build the presto-bin release tarball. Run from the repo root: packaging/pack-release.sh VERSION
# PACK_SKIP_BUILD=1 reuses existing target/release and engine/node_modules (local dry run).
set -eu
V=${1:?usage: pack-release.sh VERSION}
N=presto-$V-x86_64
ROOT=$PWD

if [ "${PACK_SKIP_BUILD:-0}" != 1 ]; then
  RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$PWD=/build" \
    cargo build --release --locked -p presto -p presto-engine-mock
  (cd engine && npm ci --allow-git=root && node node_modules/electron/install.js)
fi
test -f engine/node_modules/electron/path.txt
test -x target/release/presto
test -x target/release/presto-engine-mock

rm -rf "dist/$N" "dist/$N.tar.gz" "dist/$N.tar.gz.sha256"
mkdir -p "dist/$N/bin"
S=dist/$N
install -m755 target/release/presto target/release/presto-engine-mock "$S/bin/"
cp -a engine "$S/engine"
rm -rf "$S/engine/test"
install -m644 LICENSE packaging/presto.desktop packaging/presto.svg "$S/"
# Same prunes as the presto-git package().
E=$S/engine/node_modules
rm -f "$E/electron/vmp-resign.py" "$E/electron/cli.js" "$E/electron/install.js"
find "$E/@electron-internal/extract-zip" -name '*.node' ! -name '*linux-x64-gnu.node' -delete
chmod 755 "$E/electron/dist/chrome-sandbox"  # namespace sandbox, never setuid

tar --owner=0 --group=0 -C dist -czf "dist/$N.tar.gz" "$N"
if tar -tzf "dist/$N.tar.gz" | grep -i widevine; then
  echo 'Widevine file in tarball' >&2
  exit 1
fi
(cd dist && sha256sum "$N.tar.gz" > "$N.tar.gz.sha256")
echo "$ROOT/dist/$N.tar.gz"
