#!/bin/bash
# Packs the Linux binary into wiggl3-linux-<arch>.tar.gz with a per-user installer (no sudo, no root).
# Usage: bash package_linux.sh <binary> <arch> <output-dir>      e.g.  ... target/release/wiggl3 x86_64 dist
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
BIN="$1"; ARCH="$2"; OUT="$3"
[ -f "$BIN" ] && [ -n "$ARCH" ] && [ -n "$OUT" ] || { echo "usage: package_linux.sh <binary> <arch> <outdir>"; exit 1; }
STAGE="$(mktemp -d)/wiggl3"
mkdir -p "$STAGE" "$OUT"
cp "$BIN" "$STAGE/wiggl3"; chmod 755 "$STAGE/wiggl3"
cp "$HERE/../assets/icon_512.png" "$STAGE/wiggl3.png"
cp "$HERE/linux-install.sh" "$STAGE/install.sh"
cp "$HERE/linux-uninstall.sh" "$STAGE/uninstall.sh"
chmod 755 "$STAGE/install.sh" "$STAGE/uninstall.sh"
cp "$HERE/../../README.md" "$STAGE/README.md"
cp "$HERE/../../LICENSE" "$STAGE/LICENSE"
tar -C "$(dirname "$STAGE")" -czf "$OUT/wiggl3-linux-$ARCH.tar.gz" wiggl3
echo "created $OUT/wiggl3-linux-$ARCH.tar.gz"
