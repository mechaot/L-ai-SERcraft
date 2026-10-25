#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Andreas Pösch

"""One-off generator for gcode_parse/corpus.txt (checked-in input; run.sh does not call this)."""
import sys, pathlib

L = []
def add(*lines):
    L.extend(lines)

# --- basic motion
add("G0 X0 Y0", "G1 X10 Y20 F1000", "G1 X10.5 Y-20.25", "g1 x10 y20", "G01 X1 Y1", "G00 X1 Y1", "G1X10Y20F500",
    "X5", "Y-5", "Z2", "F3000", "S500", "S0", "G0 Z5", "G1 Z-0.1 F100", "G1 X1 Y1 Z1 F1 S1")
# --- laser
add("M3 S1000", "M4 S500", "M5", "m3", "m4 s255", "M3S0", "M03", "M05", "M3 S1000 G1 X1 Y1", "M5 S0", "M2", "M30", "M0", "M8", "M9", "M7",
    "S1000 M3", "G1 S255 M4 X1")
# --- arcs
add("G2 X10 Y10 I5 J0", "G3 X10 Y10 I0 J5", "G2 X0 Y0 I10 J0", "G3 X0 Y0 I-10 J0", "G2 X10 Y0 R5", "G3 X10 Y0 R-5",
    "G2 I5", "G3 J-5", "G2 X5 Y5 I2.5 J-2.5 F800", "g3 x1.5 y2.5 i.5 j.5", "G2 X1 Y1 R0", "G17 G2 X1 Y1 I1", "G2 X1 Y1 Z1 I1 J1")
# --- modal / coords
add("G90", "G91", "G90 G21", "G21 G90", "G90G21", "G91 G0 X10", "G20", "G21", "G17", "G18", "G19", "G54", "G55", "G94", "G93", "G91.1", "G40", "G49", "G43.1 Z1",
    "G80", "G38.2 Z-10 F10", "G38.3 Z-10", "G28", "G30", "G28.1", "G53 G0 X0")
# --- G92 / G10
add("G92 X0 Y0", "G92 X10", "G92 Y-5.5", "G92 Z0", "G92.1", "G92 X0 Y0 Z0", "G10 L2 P1 X0", "G10 L20 P1 X0 Y0")
# --- dwell
add("G4 P1", "G4 P0.5", "G4 P.5", "G4 S2", "G4 P1 S2", "G4", "G04 P2", "g4 p1000")
# --- grbl commands
add("$H", "$X", "$G", "$#", "$$", "$I", "$N", "$C", "$SLP", "$RST=*", "$RST=$", "$100=250", "$100=250.000", "$30=1000", "$32=1", "$110=6000", "$130=400", "$J=G91 X10 F100",
    "$J=G90 X0 Y0 F1000", "$j=g91 y-5 f200", "$J=G21G91X1F100", "$20=0 (soft limits, bool)", "$ 100=1", "$ =", "$=", "$1=", "$x=1", "  $H  ", "$H ; home", "$100 = 250", "$0=10", "$#=", "$G ($ comment)")
add("$J=G91 X10 F100", "$J=G53 G21 G90 Z0 F500", "$J=", "$J=;", "$J=X1")
# --- comments
add("; comment only", ";", "G1 X1 ; comment", "G1 X1;comment", "G1 (comment) X1", "(comment only)", "(a)(b)", "G1 (x) X(y)1", "G1 X1 (unterminated", "G1 X1 ) stray",
    "(", ")", "()", "G1 ((nested)) X1", "G1 X1 (c1) Y2 (c2) ; c3", "(c) ; (c)", "; (c", "G1 ; X1", "M3 (laser on) S100", "( G1 X1 )", "(msg, hi) G0 X1", "G1 X1 (a;b) Y2", "G1 X1 ;(a) Y2")
# --- spaces and case
add("", " ", "   ", "\t", "G1  X1   Y2", "  G1 X1 Y2  ", "G1 X 1 Y 2", "G 1 X 1", "G1\tX1\tY2", "g1 X1 y2 f3", "G 1", "X 1 0", "X1 0", "G1 X - 1", "G1 X-  1", "gG1", "Ｇ１", "G1　X1")
# --- duplicates (Dictionary.Add throws)
add("G90 G21", "G0 G1 X1", "X1 X2", "G1 X1 Y1 X2", "M3 M4", "G1 G1", "S1 S2", "F1 F2 F3", "G90 G90", "G1 X1 y2 Y3", "G91 G90 G21 G20", "G2 G3 I1", "x1 X2 y3")
# --- letters without numbers
add("G", "X", "G1 X", "G1 X Y", "G X1", "M", "M3 S", "S", "F", "G1 X1 Y", "XYZ", "XYZ1", "X1Y", "G1 F", "ABC", "G1 X1 Y2 Z", "I J", "G2 I J", "G1 N", "N", "N10 G1 X1", "N10", "N0 G0")
# --- numbers
add("G1 X1.", "G1 X.5", "G1 X-.5", "G1 X-1.", "G1 X+1", "G1 X--1", "G1 X1-2", "G1 X1.2.3", "G1 X-", "G1 X.", "G1 X..5", "G1 X1..", "G1 X0", "G1 X-0", "G1 X0.0", "G1 X-0.0", "G1 X00010", "G1 X1.000", "G1 X1.50", "G1 X100.100",
    "G1 X1e5", "G1 X1E-3", "G1 X1,5", "G1 X1_000", "G1 X 1.5 Y .5", "G1 X-1-1", "G1 X1.5-2.5")
add("G1 X123456789012345678901234567890", "G1 X1234567890123456789012345678", "G1 X79228162514264337593543950335", "G1 X79228162514264337593543950336", "G1 X-79228162514264337593543950335",
    "G1 X0.0000000000000000000000000001", "G1 X0.00000000000000000000000000001", "G1 X1.00000000000000000000000000000", "G1 X1.123456789012345678901234567890", "G1 X3.14159265358979323846264338327950288",
    "G1 X99999999999999999999999999999999", "G1 F0.0001", "G1 X-0.00000000000000000000000000000001", "S1000.00000000000000000000", "G1 X10.123456789012345678901234567")
# --- unicode
add("G1 X1 Ä", "G1 É2", "ä1", "ß1", "ǆ1", "ı1", "İ1", "G1 X１２", "G1 X٣", "G1 X²", "G1 X½", "G1 X१", "G1 X1  Y2", "G1 X1", "G1 X1 Y2", "M3 S٣", "日本語", "G1 X1 日 Y2", "ǅ1", "ǈ2", "ʰ3", "ªº", "€5", "G1 X1 😀 Y2", "G😀1", "g1 x1 ÿ2")
# --- misc junk
add("hello", "HELLO WORLD", "?", "!", "~", "ctrl-x", "0x18", "[ESC]", "G1 X1 [Y2]", "{G1}", "G1 X1 # Y2", "#100=5", "G1 X[1+2]", "G1 X#1", "%", "O100", "G1 X1 *42", "N5 G1 X1 *71", "G1 X1 & Y2", "<G1>", "G1 X1 \"Y2\"", "G1 X1 'Y2'", "G1 X1/Y2", "/G1 X1")
# --- real-world style
add("G1 X12.345 Y-6.789 S0", "G0 X0 Y0 S0", "G1 X100.000 Y100.000 S1000 F1500", "G1 Y0.1", "G1 X0.123", "G1 X0.01 S255", "S0 G1 X1", "G90 (use absolute coordinates)", "G0 X0 Y0 Z0 (move back to origin)", "G91 (use relative coordinates)",
    ";(Uncomment if you want to sink Z axis)", ";G91 (use relative coordinates)", "G0 Z-1 (sinks the Z axis, 1mm)", "G1 X1 Y1 F1000 S200 (burn)", "M4 S0", "G1 F1000.5", "G1 F1e3", "M3 S1000.", "G0 X1.0000001")
# --- case variety
for s in ["g0 x1", "G0 x1", "g0 X1", "m3 s1", "M3 s1", "g90", "g91", "g92 x0", "g2 x1 y1 i1", "G3 x1 y1 r1", "m5", "M5", "g4 p1", "$j=g91 x1", "g1X1.5y-2f100s5m3"]:
    L.append(s)
# --- more generated mixes
import itertools
words = ["G0", "G1", "G2", "G3", "G4", "G90", "G91", "G92", "M3", "M4", "M5", "X1.5", "Y-2", "Z0.1", "I1", "J-1", "R2", "F100", "S500", "P1", "T1"]
for a, b in itertools.combinations(["G1", "G2", "G3", "G92", "G91", "M3", "M4", "M5", "G4"], 2):
    L.append(f"{a} {b} X1 Y2")
    L.append(f"{a}{b}X1Y2")
for w in words:
    L.append(f"{w} ; trailing")
    L.append(f"({w}) {w}")
    L.append(f"{w.lower()}")
    L.append(f" {w} {w} ")

seen = []
for x in L:
    seen.append(x)
out = pathlib.Path(__file__).resolve().parent / "gcode_parse" / "corpus.txt"
for x in seen:
    assert "\n" not in x and "\r" not in x
out.write_text("\n".join(seen) + "\n", encoding="utf-8")
print(len(seen), "lines ->", out)
