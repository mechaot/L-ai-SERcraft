#!/usr/bin/env bash
# Rebuild the protocol oracle against the Mono-built original LaserGRBL and regenerate tests/golden/protocol/*.txt|.scn.
# Usage: run.sh [scenario ...]   (no args = all scenarios/*.scn)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
LASER_ROOT="$(cd "$REPO/.." && pwd)"
# shellcheck disable=SC1091
source "$LASER_ROOT/.mono-build/env.sh"

BUILD="${GOLDEN_BUILD_DIR:-$HERE/.build}"
mkdir -p "$BUILD"
FAKE_HOME="$BUILD/home"
rm -rf "$FAKE_HOME" "$BUILD/tmp"; mkdir -p "$FAKE_HOME" "$BUILD/tmp"

EXE_DIR="$(dirname "$LASERGRBL_EXE")"
mcs -nowarn:0168,0219,0649,0414 -debug \
    -r:"$LASERGRBL_EXE" -r:System.Drawing.dll -r:System.Windows.Forms.dll \
    -out:"$BUILD/Protocol.exe" "$HERE"/*.cs

# kernel32 shim shared with the other golden harness
gcc -shared -fPIC -O2 -o "$BUILD/libwinshim.so" "$HERE/../harness/native/winshim.c"
MCFG="$BUILD/mono.config"
sed "s#</configuration>#<dllmap dll=\"Kernel32.dll\" target=\"$BUILD/libwinshim.so\"/></configuration>#" "$MONO_ENV/etc/mono/config" > "$MCFG"
grep -q winshim "$MCFG"

cd "$BUILD"
GOLDEN_TMP="$BUILD/tmp" HOME="$FAKE_HOME" MONO_CONFIG="$MCFG" MONO_PATH="$EXE_DIR" \
  mono --debug "$BUILD/Protocol.exe" "$HERE/scenarios" "$REPO/tests/golden/protocol" "$@"
