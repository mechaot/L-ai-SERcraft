#!/usr/bin/env bash
# Regenerate tests/golden/raster/* with the original LaserGRBL built under Mono.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
source "$ROOT/../.mono-build/env.sh"
SCRATCH="${SCRATCH:-${TMPDIR:-/tmp}/lg-golden-raster}"
mkdir -p "$SCRATCH/home" "$SCRATCH/bin"
EXE_DIR="$(dirname "$LASERGRBL_EXE")"
# compile harness next to a copy of the app so Mono finds the reference
mcs -nologo -out:"$SCRATCH/bin/GenRaster.exe" -r:"$LASERGRBL_EXE" -r:System.Drawing "$HERE/GenRaster.cs"
rm -rf "$ROOT/tests/golden/raster"
mkdir -p "$ROOT/tests/golden/raster"
HOME="$SCRATCH/home" MONO_PATH="$EXE_DIR" mono "$SCRATCH/bin/GenRaster.exe" "$ROOT/tests/golden/raster"
