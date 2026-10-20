#!/usr/bin/env python3
"""Generate oracle input lists (deterministic) into tests/golden/svg_unit/."""
import random, pathlib, sys

out = pathlib.Path(__file__).resolve().parents[3] / "tests/golden/svg_unit"
out.mkdir(parents=True, exist_ok=True)
rnd = random.Random(12345)

def num():
    k = rnd.random()
    if k < 0.15: return rnd.choice(["0", "1", "-1", "2", "0.5", "-0.5"])
    if k < 0.5: return repr(round(rnd.uniform(-10, 10), rnd.randint(0, 6)))
    if k < 0.8: return repr(round(rnd.uniform(-500, 500), rnd.randint(0, 4)))
    return repr(rnd.uniform(-1, 1) * 10 ** rnd.randint(-6, 6))

def mat():
    k = rnd.random()
    if k < 0.2: return "1 0 0 1 0 0"
    if k < 0.4: return f"1 0 0 1 {num()} {num()}"
    if k < 0.6: return f"{num()} 0 0 {num()} 0 0"
    if k < 0.75: return f"{num()} 0 0 {num()} {num()} {num()}"
    return " ".join(num() for _ in range(6))

ops = []
def op():
    k = rnd.random()
    if k < 0.10: return "new " + mat()
    if k < 0.16: return "ident"
    if k < 0.22: return f"set {rnd.choice(['M11','M12','M21','M22','OffsetX','OffsetY'])} {num()}"
    if k < 0.34: return f"scale {num()} {num()}"
    if k < 0.40: return f"scaleat {num()} {num()} {num()} {num()}"
    if k < 0.46: return f"rotate {rnd.choice([30,45,90,180,360,400,-45,num()])}"
    if k < 0.54: return f"rotateat {rnd.choice([30,45,90,180,-45,num()])} {num()} {num()}"
    if k < 0.64: return f"translate {num()} {num()}"
    if k < 0.68: return f"skew {rnd.choice([10,30,45,num()])} {rnd.choice([0,10,20,num()])}"
    if k < 0.78: return "mul " + mat()
    if k < 0.86: return "mulr " + mat()
    if k < 0.90: return "invert"
    return f"xf {num()} {num()}"

lines = []
for i in range(600):
    n = rnd.randint(1, 7)
    seq = ["new " + mat() if rnd.random() < 0.6 else "ident"] + [op() for _ in range(n)] + [f"xf {num()} {num()}", f"xv {num()} {num()}"]
    lines.append(";".join(seq))
# hand-written cases mirroring the converter usage
lines += [
    "new 1 0 0 1 0 0;set M11 1;set M22 -1;set OffsetY 100;scale 0.1 0.1;scale 3.7795275 3.7795275;xf 10 20",
    "new 1 0 0 1 0 0;set M11 0.26458333;set OffsetX 0;set M22 -0.26458333;set OffsetY 26.458333;xf 10 20",
    "new 1 0 0 1 0 0;rotateat 45 5 5;xf 1 1",
    "new 1 0 0 1 0 0;set OffsetX 10;set OffsetY 20;xf 3 4",
    "ident;set OffsetX 5;xf 1 1",
    "ident;set M12 2;xf 1 1",
    "ident;invert",
    "new 0 0 0 0 0 0;invert",
    "new 2 0 0 2 0 0;invert;xf 4 4",
    "new 1 0 0 1 3 4;invert;xf 4 4",
    "new 2 0 0 3 4 5;invert;xf 4 4",
    "new 1 2 3 4 5 6;invert;xf 4 4",
]
(out / "matrix_ops.txt").write_text("\n".join(lines) + "\n")

# bezier: kind n x0 y0 ... ; error ; maxsub  (kind F = FlattenTo, O = GetBezierApproximationOld)
b = []
def pt(): return f"{num()} {num()}"
for i in range(150):
    err = rnd.choice(["0.01", "0.2", "0.05", "0.5", "1", "0.001", "5"])
    segs = rnd.choice([1, 1, 1, 2, 3])
    pts = " ".join(pt() for _ in range(1 + 3 * segs))
    b.append(f"F\t{1+3*segs}\t{pts}\t{err}\t{rnd.choice([20,20,20,3,0,6])}")
for i in range(40):
    b.append(f"O\t4\t{pt()} {pt()} {pt()} {pt()}\t0\t{rnd.choice([12,1,2,5,30])}")
b.append("F\t4\t0 0 0 0 0 0 0 0\t0.01\t20")
b.append("F\t4\t0 0 10 0 20 0 30 0\t0.01\t20")
b.append("F\t4\t0 0 0 100 100 100 100 0\t0.0001\t20")
b.append("F\t7\t0 0 1 1 2 2 3 3 4 2 5 1 6 0\t0.01\t20")
b.append("F\t5\t0 0 1 1 2 2 3 3 4 4\t0.01\t20")
b.append("F\t1\t5 5\t0.01\t20")
(out / "bezier_in.txt").write_text("\n".join(b) + "\n")

# parse: px text [ext] | tf text | tb src s1 | color text    (TAB separated; \\t \\n escapes not used)
p = []
px = ["10", "10mm", "10cm", "10in", "10pt", "10pc", "10em", "10em ", "10px", "50%", "0.5", ".5", "-.5", "1e2", "1E2", "1e-2mm", "abc", "", " ", "mm", "10 mm", "10mm ", " 10", "1,5", "1,000", "+5", "-5", "5.", "NaN", "Infinity", "-Infinity", "1e400", "0x10", "1 2", "10in20", "10pt5", "in", "10%%", "10%mm", "1e5mm", "3.14159265358979", "0.1", "33.333333333", "16777217", "1e38", "1e39", "1e-46", "100%", "25.4mm", "2.54cm", "72pt", "6pc", "1e1in", "١٢", "5 ", "10px20"]
for s in px:
    p.append(f"px\t{s}")
for s in ["50%", "12.5%", "200%", "10", "10mm"]:
    for ext in ["100", "123.456", "0", "-5"]:
        p.append(f"px\t{s}\t{ext}")
tfs = ["translate(10,20)", "translate(10 20)", "translate(10)", "translate(10,)", "translate(,10)", "translate(1e1,2e1)", "translate(5mm,1in)", "translate(-5,-5)", "translate( 10 , 20 )", "translate(1.5.5,2)", "translate", "translatee", "translate(", "translate(abc)", "translate(10,20))", "translate(10;20)", "translate(10,20) ",
       "scale(2)", "scale(2,3)", "scale(2 3)", "scale(-1,1)", "scale()", "scale(2,)", "scale(,2)", "scale", "scale(0.5 0.25)", "scale(1e1)",
       "rotate(45)", "rotate(45,5,5)", "rotate(45 5 5)", "rotate(45,5)", "rotate(45,5,5,5)", "rotate(-90)", "rotate(360)", "rotate(720.5)", "rotate()", "rotate", "rotate(30 1e1 2e1)",
       "matrix(1,0,0,1,0,0)", "matrix(1 0 0 1 5 5)", "matrix(0.5,0.5,-0.5,0.5,100,20)", "matrix(1,2,3)", "matrix(1,2,3,4,5)", "matrix()", "matrix", "matrix(2 0 0 2 0 0) ", "matrix(1e-1,0,0,1e-1,0,0)",
       "trans", "foo", "", "translatescale(2)", "scaletranslate(1,2)", "rotate(10)scale(2)"]
for s in tfs:
    p.append(f"tf\t{s}")
tbs = [("translate(10,20)", "translate("), ("translate(10,20) scale(2)", "translate("), ("translate(10", "translate("), ("translate(", "translate("), ("translate(1e1)", "translate("), ("scale(2)x", "scale("), ("scale(2)", "scale("), ("xx", "translate("), ("scale", "scale("), ("matrix(1 2 3 4 5 6)", "matrix("), ("rotate(45 1 2)end", "rotate("), ("rotate(1)", "rotate("), ("translate(1١,2)", "translate(")]
for a, s1 in tbs:
    p.append(f"tb\t{a}\t{s1}")
cols = ["#ff0000", "#FF0000", "#f00", "#F00", "#ffff", "#ff00", "#ff", "#", "#12345678", "#ff00000", "#GGGGGG", "#ggg", "red", "Red", "RED", " red ", "red ", "lime", "blue", "black", "white", "transparent", "lightgrey", "LightGrey", "lightgray", "gray", "grey", "darkgreen", "notacolor", "none", "currentColor", "rgb(255,0,0)", "rgb(255, 0, 0)", "255,0,0", "255, 0, 0", " 255,0,0 ", "1,2,3,4", "1,2", "1,2,3,4,5", "256,0,0", "-1,0,0", "a,b,c", "0,0,0,0", "255,255,255,255", "128,0,0,0", "ff0000", "FF0000", "0xff0000", "0XFF0000", "&Hff0000", "&hFF0000", "16711680", "-1", "0", "12", "2147483647", "2147483648", "4294967295", "0xffffffff", "0x1ffffffff", "#fffffffff", "window", "Window", "buttonface", "threedface", "ButtonFace", "ThreeDShadow", "activeborder", "control", "Control", "controltext", "highlight", "infobackground", "menu", "scrollbar", "windowtext", "captiontext", "appworkspace", "background", "desktop", "", " ", "0x", "#12", "#12345", "-", "+5", "1,2,0x10", "1,2,#10", "1.5,2,3", "(1,2,3)", "red,green,blue", "ActiveCaption", "DarkSlateGray", "RebeccaPurple", "rebeccapurple", "orange", "OrangeRed", "gold", "aqua", "aquamarine", "fuchsia", "magenta", "navy", "silver", "teal", "maroon", "olive", "purple", "yellow", "cyan"]
for c in cols:
    p.append(f"color\t{c}")
(out / "parse_in.txt").write_text("\n".join(p) + "\n")
print(len(lines), len(b), len(p))
