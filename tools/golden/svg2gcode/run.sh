#!/usr/bin/env bash
# Regenerate tests/golden/svg_unit/* from the original LaserGRBL (Mono). Inputs: gen_inputs.py + inputs/*.svg.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
source "$ROOT/../.mono-build/env.sh"
OUT="${TMPDIR:-/tmp}/lg-golden-svg2gcode"
mkdir -p "$OUT/home" "$ROOT/tests/golden/svg_unit"
python3 "$HERE/gen_inputs.py"
mcs -nowarn:1701 -r:"$LASERGRBL_EXE" -r:WindowsBase -r:System.Drawing -out:"$OUT/Svg2GcodeGolden.exe" "$HERE/Svg2GcodeGolden.cs"
cp "$LASERGRBL_EXE" "$OUT/"
export MONO_PATH="$(dirname "$LASERGRBL_EXE")"
HOME="$OUT/home" mono "$OUT/Svg2GcodeGolden.exe" "$ROOT/tests/golden/svg_unit" "$HERE/inputs"
