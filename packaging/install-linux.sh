#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Andreas Pösch

# install-linux.sh: install L-ai-SERcraft with menu entry (KDE, GNOME, LXDE, XFCE, ...).
#
# Usage: packaging/install-linux.sh [--appimage FILE] [--skip-build] [--force] [--prefix DIR] [--uninstall]
#   --appimage FILE  install this AppImage as the binary (no cargo build, no resources copy)
#   --skip-build     reuse existing target/release/laisercraft (implied as root)
#   --force          install even if binary/AppImage is older than the sources
#   --prefix DIR     install root (default: /usr as root, else ~/.local)
#   --uninstall      remove all installed files
# Root: system-wide to /usr, no cargo build (build as user first); works with noexec home dirs.
# User: per user to ~/.local, needs exec rights there.
# Refuses a missing binary, or one older than any source it is built from (same mtime rule as cargo).
# Layout: PREFIX/bin/laisercraft, PREFIX/share/laisercraft/resources (found via ../share by the app),
#   PREFIX/share/applications/laisercraft.desktop, PREFIX/share/icons/hicolor/*/apps/laisercraft.*,
#   PREFIX/share/metainfo/io.github.mechaot.laisercraft.appdata.xml
set -euo pipefail

APP="laisercraft"
METAINFO="io.github.mechaot.laisercraft.appdata.xml"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
IS_ROOT=0; [ "$(id -u)" = 0 ] && IS_ROOT=1
# Root: system-wide, no build (root-owned target/, root often lacks cargo).
if [ "$IS_ROOT" = 1 ]; then PREFIX="/usr"; SKIP=1; else PREFIX="$HOME/.local"; SKIP=0; fi
APPIMAGE=""; UNINSTALL=0; FORCE=0

# Parse args.
while [ $# -gt 0 ]; do
  case "$1" in
    --appimage) APPIMAGE="$(realpath "${2:?--appimage needs FILE}")"; shift ;;
    --skip-build) SKIP=1 ;;
    --force) FORCE=1 ;;
    --prefix) PREFIX="$(realpath -m "${2:?--prefix needs DIR}")"; shift ;;
    --uninstall) UNINSTALL=1 ;;
    -h|--help) sed -n '5,18p' "$0"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
  shift
done

BIN_DIR="$PREFIX/bin"
SHARE="$PREFIX/share"
DESKTOP="$SHARE/applications/$APP.desktop"
ICON_SVG="$SHARE/icons/hicolor/scalable/apps/$APP.svg"
ICON_PNG="$SHARE/icons/hicolor/256x256/apps/$APP.png"

# Refresh menu + icon caches; tools are optional.
refresh_caches() {
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q "$SHARE/applications" || true
  command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t "$SHARE/icons/hicolor" || true
  # KDE menu cache is per user: skip as root.
  [ "$IS_ROOT" = 1 ] && return 0
  command -v kbuildsycoca6 >/dev/null 2>&1 && kbuildsycoca6 >/dev/null 2>&1 || true
  command -v kbuildsycoca5 >/dev/null 2>&1 && kbuildsycoca5 >/dev/null 2>&1 || true
}

if [ "$UNINSTALL" = 1 ]; then
  rm -f "$BIN_DIR/$APP" "$DESKTOP" "$ICON_SVG" "$ICON_PNG" "$SHARE/metainfo/$METAINFO"
  rm -rf "$SHARE/$APP"
  refresh_caches
  echo "uninstalled from $PREFIX (settings in ~/.config/L-ai-SERcraft kept)"
  exit 0
fi

# Binary to install; build first unless skipped.
if [ -n "$APPIMAGE" ]; then
  SRC_BIN="$APPIMAGE"
  REBUILD="packaging/appimage/build-appimage.sh"
else
  SRC_BIN="$ROOT/target/release/$APP"
  REBUILD="cargo build --release -p $APP (as normal user)"
  [ "$SKIP" = 1 ] || (cd "$ROOT" && cargo build --release -p "$APP")
fi
# -f, not -x: access(X_OK) fails on noexec mounts, even for root.
[ -f "$SRC_BIN" ] || { echo "missing $SRC_BIN; build first: $REBUILD" >&2; exit 1; }

# Up-to-date check: build inputs incl. include_bytes!/include_str! files (resources/, LICENSE.md, NOTICE.md).
if [ "$FORCE" = 0 ]; then
  NEWER="$(cd "$ROOT" && find app crates resources Cargo.toml Cargo.lock LICENSE.md NOTICE.md \
    -type f -newer "$SRC_BIN" -print 2>/dev/null | head -n 5 || true)"  # || true: SIGPIPE from head + pipefail
  if [ -n "$NEWER" ]; then
    echo "$SRC_BIN is older than its sources, e.g.:" >&2
    sed 's/^/  /' <<<"$NEWER" >&2
    echo "rebuild first: $REBUILD (or pass --force)" >&2
    exit 1
  fi
fi

mkdir -p "$BIN_DIR" "$SHARE/applications" "$(dirname "$ICON_SVG")" "$(dirname "$ICON_PNG")" "$SHARE/metainfo"

if [ -n "$APPIMAGE" ]; then
  # AppImage carries its own resources: drop stale source-install copy.
  install -m755 "$SRC_BIN" "$BIN_DIR/$APP"
  rm -rf "$SHARE/$APP"
else
  install -m755 "$SRC_BIN" "$BIN_DIR/$APP"
  # Resources next to binary as ../share/laisercraft/resources.
  rm -rf "$SHARE/$APP"
  mkdir -p "$SHARE/$APP"
  cp -a "$ROOT/resources" "$SHARE/$APP/resources"
fi

# Absolute Exec: menu launchers often lack ~/.local/bin in PATH.
sed -e "s|^Exec=$APP |Exec=$BIN_DIR/$APP |" -e "/^Exec=/a TryExec=$BIN_DIR/$APP" \
  "$HERE/appimage/$APP.desktop" > "$DESKTOP"
chmod 644 "$DESKTOP"
install -m644 "$ROOT/resources/branding/icon.svg" "$ICON_SVG"
install -m644 "$ROOT/resources/branding/icon.png" "$ICON_PNG"
install -m644 "$HERE/appimage/$METAINFO" "$SHARE/metainfo/$METAINFO"
refresh_caches

echo "installed to $PREFIX"
echo "menu entry: $DESKTOP"
# noexec mount (e.g. hardened /home): binary there cannot start.
if [ "$IS_ROOT" = 0 ] && command -v findmnt >/dev/null 2>&1 \
   && findmnt -n -o OPTIONS -T "$BIN_DIR" | tr ',' '\n' | grep -qx noexec; then
  echo "warning: $BIN_DIR is on a noexec mount, the app cannot start from there. Remove with: $0 --uninstall; then install system-wide as root: sudo bash $0" >&2
fi
case ":$PATH:" in *":$BIN_DIR:"*) ;; *) echo "note: $BIN_DIR not in PATH (menu entry works anyway)" ;; esac
