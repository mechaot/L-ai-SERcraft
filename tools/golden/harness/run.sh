#!/usr/bin/env bash
# Rebuild the golden harness against the Mono-built original LaserGRBL and regenerate tests/golden/**.
# Idempotent: output dirs of each area are wiped and rewritten. Usage: run.sh [area ...]
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"                       # .../LaserGRBL_linux
LASER_ROOT="$(cd "$REPO/.." && pwd)"                        # .../Laser
# shellcheck disable=SC1091
source "$LASER_ROOT/.mono-build/env.sh"

BUILD="${GOLDEN_BUILD_DIR:-$HERE/.build}"          # build products + throw-away HOME (git-ignored)
mkdir -p "$BUILD"
# Throw-away HOME: original Settings persists to ~/.config/LaserGRBL (must never touch the real one).
FAKE_HOME="$BUILD/home"
rm -rf "$FAKE_HOME" "$BUILD/tmp"; mkdir -p "$FAKE_HOME" "$BUILD/tmp"

EXE_DIR="$(dirname "$LASERGRBL_EXE")"
mcs -nowarn:0168,0219,0649,0414 -unsafe -debug \
    -r:"$LASERGRBL_EXE" -r:System.Drawing.dll -r:System.Windows.Forms.dll -r:WindowsBase.dll -r:System.Xml.Linq.dll -r:System.Xml.dll \
    -out:"$BUILD/Golden.exe" "$HERE"/*.cs

# Native shim for kernel32 timer calls + Mono config with dllmap pointing at it.
gcc -shared -fPIC -O2 -o "$BUILD/libwinshim.so" "$HERE/native/winshim.c"
MCFG="$BUILD/mono.config"
sed "s#</configuration>#<dllmap dll=\"Kernel32.dll\" target=\"$BUILD/libwinshim.so\"/></configuration>#" "$MONO_ENV/etc/mono/config" > "$MCFG"
grep -q winshim "$MCFG"

AREAS=("$@")
cd "$BUILD"
# wipe outputs of requested areas (all if none given) so stale files vanish
ALL=(gcode_parse gcode_analyze state_builder l2l potrace potrace_curves potrace_rasterfill svg generators config timespan_fmt custom_code custom_buttons)
if [ ${#AREAS[@]} -eq 0 ]; then
  for a in "${ALL[@]}"; do rm -rf "$REPO/tests/golden/$a"; done
fi
GOLDEN_TMP="$BUILD/tmp" HOME="$FAKE_HOME" MONO_CONFIG="$MCFG" MONO_PATH="$EXE_DIR" mono --debug "$BUILD/Golden.exe" "$REPO" "${AREAS[@]}"
