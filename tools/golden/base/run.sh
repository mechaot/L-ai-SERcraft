#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Andreas Pösch

# Regenerate tests/golden/base/* from the original LaserGRBL (Mono).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
source "$ROOT/../.mono-build/env.sh"
OUT="${TMPDIR:-/tmp}/lg-golden-base"
mkdir -p "$OUT/home" "$ROOT/tests/golden/base"
mcs -nowarn:1701 -r:"$LASERGRBL_EXE" -r:System.Drawing -r:System.Windows.Forms -out:"$OUT/GenBase.exe" "$HERE/GenBase.cs"
cp "$LASERGRBL_EXE" "$OUT/"
export MONO_PATH="$(dirname "$LASERGRBL_EXE")"
rm -rf "$OUT/home/.config"
HOME="$OUT/home" mono "$OUT/GenBase.exe" "$ROOT/tests/golden/base" "$ROOT/resources/misc/StandardButtons.zbn"
