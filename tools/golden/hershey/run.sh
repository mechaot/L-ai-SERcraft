#!/usr/bin/env bash
# Regenerate tests/golden/hershey/cases.txt from the original LaserGRBL (Mono).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
source "$ROOT/../.mono-build/env.sh"
OUT="${TMPDIR:-/tmp}/lg-golden-hershey"
mkdir -p "$OUT" "$ROOT/tests/golden/hershey"
mcs -nowarn:1701 -r:"$LASERGRBL_EXE" -out:"$OUT/HersheyGolden.exe" "$HERE/HersheyGolden.cs"
cp "$LASERGRBL_EXE" "$OUT/"
export MONO_PATH="$(dirname "$LASERGRBL_EXE")"
HOME="$OUT/home" mono "$OUT/HersheyGolden.exe" "$ROOT/tests/golden/hershey/cases.txt"
