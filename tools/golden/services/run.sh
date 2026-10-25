#!/usr/bin/env bash
# Regenerate tests/golden/services/* from the original LaserGRBL (Mono).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
source "$ROOT/../.mono-build/env.sh"
OUT="${TMPDIR:-/tmp}/lg-golden-services"
mkdir -p "$OUT/home" "$ROOT/tests/golden/services"
mcs -nowarn:1701 -r:"$LASERGRBL_EXE" -r:System.Drawing -r:System.Windows.Forms -r:System.Data -r:System.Data.DataSetExtensions -out:"$OUT/GenServices.exe" "$HERE/GenServices.cs"
cp "$LASERGRBL_EXE" "$OUT/"
export MONO_PATH="$(dirname "$LASERGRBL_EXE")"
rm -rf "$OUT/home/.config"
HOME="$OUT/home" mono "$OUT/GenServices.exe" "$ROOT/tests/golden/services"
