#!/usr/bin/env bash
# Regenerate tests/golden/clipper/*.txt from the original ClipperLib (LaserGRBL.exe under Mono).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
source "$ROOT/../.mono-build/env.sh"
SCRATCH="${TMPDIR:-/tmp}/lg-golden-clipper"
mkdir -p "$SCRATCH"
cp "$LASERGRBL_EXE" "$SCRATCH/LaserGRBL.exe"
mcs -nowarn:0168,0219,0414,0649 -r:"$SCRATCH/LaserGRBL.exe" -out:"$SCRATCH/GenClipper.exe" "$HERE/GenClipper.cs"
# throw-away HOME so the original never touches real settings
HOME="$SCRATCH/home" MONO_PATH="$SCRATCH" mono "$SCRATCH/GenClipper.exe" "$ROOT/tests/golden/clipper"
