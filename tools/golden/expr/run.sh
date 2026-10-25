#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Andreas Pösch

# Regenerate tests/golden/expr/cases.tsv from the original LaserGRBL (Mono).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
source "$ROOT/../.mono-build/env.sh"
OUT="${TMPDIR:-/tmp}/lg-golden-expr"
mkdir -p "$OUT" "$ROOT/tests/golden/expr"
mcs -nowarn:1701 -r:"$LASERGRBL_EXE" -out:"$OUT/ExprGolden.exe" "$HERE/ExprGolden.cs"
cp "$LASERGRBL_EXE" "$OUT/"
export MONO_PATH="$(dirname "$LASERGRBL_EXE")"
HOME="$OUT/home" mono "$OUT/ExprGolden.exe" "$ROOT/tests/golden/expr/cases.tsv"
