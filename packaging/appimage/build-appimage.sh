#!/usr/bin/env bash
# build-appimage.sh: build LaserGRBL AppImage.
#
# Usage: packaging/appimage/build-appimage.sh [--native] [--install-deps] [--out DIR] [--skip-build]
#   --native        plain `cargo build` (binary needs host-new glibc)
#   --install-deps  install zig + cargo-zigbuild into cache (user space, no root)
#   --out DIR       output dir (default: <repo>/dist)
#   --skip-build    reuse existing release binary
# Env: XDG_CACHE_HOME (cache = $XDG_CACHE_HOME/lasergrbl-appimage, else <repo>/.cache/appimage)
# Output: DIR/LaserGRBL-<version>-x86_64.AppImage, prints max GLIBC needed.
# Default build: cargo-zigbuild vs glibc 2.28 if found; else native with warning.
set -euo pipefail

TOOL_URL="https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage"
TOOL_SHA256="ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0"
GLIBC_BASE="2.28"
TARGET="x86_64-unknown-linux-gnu"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
CACHE="${XDG_CACHE_HOME:+$XDG_CACHE_HOME/lasergrbl-appimage}"
CACHE="${CACHE:-$ROOT/.cache/appimage}"
OUT="$ROOT/dist"
NATIVE=0; INSTALL=0; SKIP=0

# Parse args.
while [ $# -gt 0 ]; do
  case "$1" in
    --native) NATIVE=1 ;;
    --install-deps) INSTALL=1 ;;
    --skip-build) SKIP=1 ;;
    --out) OUT="$(realpath -m "${2:?--out needs DIR}")"; shift ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
  shift
done

mkdir -p "$CACHE" "$OUT"
# Version from workspace Cargo.toml.
VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/{s/^version *= *"\(.*\)"/\1/p}' "$ROOT/Cargo.toml")"
[ -n "$VERSION" ] || { echo "version not found" >&2; exit 1; }
export PATH="$CACHE/cargo/bin:$CACHE/venv/bin:$PATH"

BIN=""
if [ "$SKIP" = 0 ]; then
  # Optional dep install into cache.
  if [ "$NATIVE" = 0 ] && [ "$INSTALL" = 1 ]; then
    if ! command -v zig >/dev/null 2>&1 && ! python3 -m ziglang version >/dev/null 2>&1; then
      python3 -m venv "$CACHE/venv"
      "$CACHE/venv/bin/pip" install --quiet ziglang
    fi
    command -v cargo-zigbuild >/dev/null 2>&1 || cargo install --locked --root "$CACHE/cargo" cargo-zigbuild
  fi
  cd "$ROOT"
  if [ "$NATIVE" = 0 ] && command -v cargo-zigbuild >/dev/null 2>&1 \
     && { command -v zig >/dev/null 2>&1 || python3 -m ziglang version >/dev/null 2>&1; }; then
    echo ">> zigbuild, glibc $GLIBC_BASE baseline"
    cargo zigbuild --release -p lasergrbl --target "$TARGET.$GLIBC_BASE"
    BIN="$ROOT/target/$TARGET/release/lasergrbl"
  else
    echo "WARNING: native build; binary needs host glibc (not portable to older distros)." >&2
    echo "         Use --install-deps for glibc $GLIBC_BASE baseline via zig." >&2
    cargo build --release
    BIN="$ROOT/target/release/lasergrbl"
  fi
else
  # Pick newest existing binary.
  BIN="$(ls -t "$ROOT/target/$TARGET/release/lasergrbl" "$ROOT/target/release/lasergrbl" 2>/dev/null | head -1)"
fi
[ -x "$BIN" ] || { echo "binary not found: $BIN" >&2; exit 1; }

# Fetch appimagetool, verify sha256.
TOOL="$CACHE/appimagetool-x86_64.AppImage"
if [ ! -x "$TOOL" ] || ! echo "$TOOL_SHA256  $TOOL" | sha256sum -c --status; then
  curl -fL --retry 3 -o "$TOOL.part" "$TOOL_URL"
  echo "$TOOL_SHA256  $TOOL.part" | sha256sum -c --status || { rm -f "$TOOL.part"; echo "appimagetool checksum mismatch" >&2; exit 1; }
  mv "$TOOL.part" "$TOOL"; chmod +x "$TOOL"
fi

# Assemble AppDir.
APPDIR="$CACHE/AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/lasergrbl" "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/metainfo" "$APPDIR/usr/share/icons/hicolor/256x256/apps"
install -m755 "$BIN" "$APPDIR/usr/bin/lasergrbl"
cp -a "$ROOT/resources" "$APPDIR/usr/share/lasergrbl/resources"
install -m755 "$HERE/AppRun" "$APPDIR/AppRun"
install -m644 "$HERE/lasergrbl.desktop" "$APPDIR/lasergrbl.desktop"
install -m644 "$HERE/lasergrbl.desktop" "$APPDIR/usr/share/applications/lasergrbl.desktop"
install -m644 "$HERE/io.github.lasergrbl.LaserGRBL.appdata.xml" "$APPDIR/usr/share/metainfo/"
python3 -I "$HERE/ico2png.py" "$ROOT/resources/grafica/LaserGrbl.ico" "$APPDIR/lasergrbl.png" 256
cp "$APPDIR/lasergrbl.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/lasergrbl.png"

# Report max GLIBC symbol version needed.
MAXG="$(objdump -T "$BIN" 2>/dev/null | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -V | tail -1 || true)"
echo ">> max GLIBC required: ${MAXG:-unknown}"

# Pack. Extract-and-run: no FUSE needed.
OUTFILE="$OUT/LaserGRBL-$VERSION-x86_64.AppImage"
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" --no-appstream "$APPDIR" "$OUTFILE"
echo ">> built $OUTFILE ($(du -h "$OUTFILE" | cut -f1))"
