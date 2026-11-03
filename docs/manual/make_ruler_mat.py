#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Andreas Pösch

"""Generate 400x400 mm ruler mat SVG for laser engraving onto base board.

Usage: python3 -I docs/manual/make_ruler_mat.py [out.svg]   (default: ruler-mat-400.svg next to this script)

All geometry is stroke-only paths. Dashes are real segments (no stroke-dasharray).
Text uses Hershey single-stroke glyphs from crates/hershey/src/data.rs.
Coordinates: SVG user unit = 1 mm. Labels use machine coords (origin bottom-left, Y up).
"""

import math
import re
import sys
from dataclasses import dataclass
from pathlib import Path

SIZE = 400.0  # board edge, mm
BAND = 10.0  # ruler band width; grid lives in [BAND, SIZE-BAND]
STROKE = 0.2  # display stroke width, mm
CENTER = (SIZE / 2, SIZE / 2)

# tick lengths by mm position
TICK_1, TICK_5, TICK_10 = 2.0, 3.5, 5.0

# text cap heights
H_EDGE = 2.5
H_LINE = 2.8
H_COORD = 1.8
R_MARK = 1.3  # ring radius around tagged crossings

# dash patterns: alternating on/off lengths, mm
DASH_45 = [8.0, 3.0]
DASH_30_60 = [6.0, 1.5, 1.0, 1.5]
DASH_CIRCLE = [2.5, 1.5]

DIAG_SPACING = 100.0  # 45 deg family spacing along X; 0 = main diagonals only
CIRCLE_D = 50.0

HOLE_PAD = 0.8  # clearance between label and lines cut around it

DATA_RS = Path(__file__).resolve().parents[2] / "crates/hershey/src/data.rs"


# ---------------------------------------------------------------- Hershey font

@dataclass
class Glyph:
    strokes: list[list[tuple[float, float]]]  # polylines, font units, Y up
    advance: float  # x extent, font units


def load_hershey(path: Path) -> dict[str, Glyph]:
    """Parse HOR glyph table from data.rs.

    path: data.rs file.
    Returns map char -> Glyph for ASCII 32..126.
    """
    src = path.read_text()
    # HOR table only, rows are &["..", ..],
    hor = src[src.index("HOR"): src.index("VER")]
    hor = hor[hor.index("= ["):]  # skip type decl
    rows = re.findall(r"&\[(.*?)\]", hor)
    glyphs: dict[str, Glyph] = {}
    for code, row in zip(range(32, 127), rows):
        x = y = 0.0
        prev = (0.0, 0.0)
        on = False
        strokes: list[list[tuple[float, float]]] = []
        cur: list[tuple[float, float]] | None = None
        for cmd in re.findall(r'"(.*?)"', row):
            if cmd.startswith("M3"):
                on = True
                continue
            if cmd.startswith("M5"):
                on, cur = False, None
                continue
            rapid = cmd.startswith("G0")
            for axis, val in re.findall(r"([XY])(-?\d+(?:\.\d+)?)", cmd):
                if axis == "X":
                    x = float(val)
                else:
                    y = float(val)
            # G1 with laser on draws; G0 lifts
            if rapid:
                cur = None
            elif on:
                if cur is None:
                    cur = [prev]
                    strokes.append(cur)
                cur.append((x, y))
            prev = (x, y)
        xs = [0.0] + [p[0] for s in strokes for p in s]
        adv = max(xs) - min(xs) if strokes else x
        glyphs[chr(code)] = Glyph(strokes, adv)
    return glyphs


FONT = load_hershey(DATA_RS)
SPACING = 0.3  # inter-glyph gap, font units (Hershey crate value)
# cap height and baseline from 'H'
_hy = [p[1] for s in FONT["H"].strokes for p in s]
BASE, CAP = min(_hy), max(_hy) - min(_hy)


def degree_glyph() -> Glyph:
    """Build degree sign (not in ASCII table).

    Returns small circle glyph at cap height.
    """
    r = CAP * 0.14
    cy = BASE + CAP - r
    pts = [(r + r * math.cos(a * math.pi / 8), cy + r * math.sin(a * math.pi / 8)) for a in range(17)]
    return Glyph([pts], 2 * r)


FONT["°"] = degree_glyph()


def diameter_glyph() -> Glyph:
    """Build diameter sign from 'O' plus slash.

    Returns glyph.
    """
    o = FONT["O"]
    xs = [p[0] for s in o.strokes for p in s]
    return Glyph(o.strokes + [[(min(xs) - 0.05, BASE - 0.1), (max(xs) + 0.05, BASE + CAP + 0.1)]], o.advance)


FONT["Ø"] = diameter_glyph()


def text_width(s: str, h: float) -> float:
    """Measure text.

    s: text. h: cap height mm.
    Returns width in mm.
    """
    k = h / CAP
    return k * (sum(FONT[c].advance for c in s) + SPACING * (len(s) - 1))


@dataclass
class Label:
    """Placed text = oriented rect; also used to cut lines."""
    text: str
    cx: float
    cy: float
    angle: float  # deg, SVG rotation (clockwise on screen)
    h: float

    @property
    def w(self) -> float:
        return text_width(self.text, self.h)

    def corners(self, pad: float = 0.0) -> list[tuple[float, float]]:
        """Rect corners in SVG coords.

        pad: grow rect by this on all sides.
        Returns 4 corners.
        """
        hw, hh = self.w / 2 + pad, self.h / 2 + pad
        c, s = math.cos(math.radians(self.angle)), math.sin(math.radians(self.angle))
        return [(self.cx + u * c - v * s, self.cy + u * s + v * c) for u, v in ((-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh))]

    def paths(self) -> list[list[tuple[float, float]]]:
        """Glyph polylines in SVG coords.

        Returns polylines, text centered on (cx, cy).
        """
        k = self.h / CAP
        c, s = math.cos(math.radians(self.angle)), math.sin(math.radians(self.angle))
        out = []
        pen = -self.w / 2
        for ch in self.text:
            g = FONT[ch]
            for st in g.strokes:
                pts = []
                for gx, gy in st:
                    # font Y up -> SVG Y down, vertical center on cap height
                    u = pen + gx * k
                    v = -((gy - BASE) * k - self.h / 2)
                    pts.append((self.cx + u * c - v * s, self.cy + u * s + v * c))
                out.append(pts)
            pen += (g.advance + SPACING) * k
        return out


def upright(angle: float) -> float:
    """Normalize text angle so text never reads upside down.

    angle: deg. Returns angle in (-90, 90].
    """
    a = (angle + 180) % 360 - 180
    if a > 90:
        a -= 180
    elif a <= -90:
        a += 180
    return a


# ---------------------------------------------------------------- geometry helpers

def clip_seg_to_poly(p0, p1, poly) -> tuple[float, float] | None:
    """Cyrus-Beck clip of segment against convex polygon.

    p0, p1: segment ends. poly: convex polygon corners (any winding).
    Returns (t_in, t_out) param range inside polygon or None.
    """
    t0, t1 = 0.0, 1.0
    dx, dy = p1[0] - p0[0], p1[1] - p0[1]
    n = len(poly)
    # centroid to orient edge normals inward
    gx = sum(p[0] for p in poly) / n
    gy = sum(p[1] for p in poly) / n
    for i in range(n):
        a, b = poly[i], poly[(i + 1) % n]
        nx, ny = -(b[1] - a[1]), b[0] - a[0]
        if nx * (gx - a[0]) + ny * (gy - a[1]) < 0:
            nx, ny = -nx, -ny
        num = nx * (p0[0] - a[0]) + ny * (p0[1] - a[1])
        den = nx * dx + ny * dy
        if abs(den) < 1e-12:
            if num < 0:
                return None
            continue
        t = -num / den
        if den > 0:
            t0 = max(t0, t)
        else:
            t1 = min(t1, t)
        if t0 >= t1:
            return None
    return t0, t1


def subtract(intervals, cut):
    """Remove interval from interval list.

    intervals: [(a, b)]. cut: (c, d).
    Returns remaining pieces.
    """
    c, d = cut
    out = []
    for a, b in intervals:
        if d <= a or c >= b:
            out.append((a, b))
            continue
        if c > a:
            out.append((a, c))
        if d < b:
            out.append((d, b))
    return out


def dash_intervals(length: float, pattern: list[float] | None, start: float = 0.0):
    """Split [0, length] into dash on-intervals.

    length: line length mm. pattern: on/off list or None for solid.
    start: param where pattern phase 0 sits (dashes grow both ways from there).
    Returns [(a, b)] mm along line.
    """
    if not pattern:
        return [(0.0, length)]
    period = sum(pattern)
    out = []
    # walk periods covering whole line, anchored at start
    k = math.floor(-start / period) - 1
    while start + k * period < length:
        pos = start + k * period
        for i, seg in enumerate(pattern):
            if i % 2 == 0:
                a, b = max(pos, 0.0), min(pos + seg, length)
                if b - a > 0.05:
                    out.append((a, b))
            pos += seg
        k += 1
    return out


def clip_line_to_box(p0, p1, lo, hi):
    """Clip infinite-ish segment to axis box [lo, hi]^2.

    Returns clipped (q0, q1) or None.
    """
    box = [(lo, lo), (hi, lo), (hi, hi), (lo, hi)]
    r = clip_seg_to_poly(p0, p1, box)
    if r is None:
        return None
    t0, t1 = r
    lerp = lambda t: (p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t)
    return lerp(t0), lerp(t1)


# ---------------------------------------------------------------- mat builder

class Mat:
    def __init__(self) -> None:
        self.layers: dict[str, list[str]] = {}
        self.labels: dict[str, list[Label]] = {}
        self.all_pts: list[tuple[float, float]] = []
        self.extra_holes: list[list[tuple[float, float]]] = []  # cut-only rects, already padded

    def seg(self, layer: str, p0, p1) -> None:
        """Add straight segment.

        layer: group id. p0, p1: ends mm.
        """
        self.all_pts += [p0, p1]
        self.layers.setdefault(layer, []).append(f"M{p0[0]:.3f} {p0[1]:.3f}L{p1[0]:.3f} {p1[1]:.3f}")

    def label(self, layer: str, lab: Label) -> None:
        """Queue text label; lines added later get cut around it."""
        self.labels.setdefault(layer, []).append(lab)

    def holes(self):
        """All padded label rects. Returns list of polygons."""
        return [lab.corners(HOLE_PAD) for labs in self.labels.values() for lab in labs] + self.extra_holes

    def line(self, layer: str, p0, p1, pattern=None, anchor=None) -> None:
        """Add (dashed) line with gaps at labels.

        layer: group id. p0, p1: ends. pattern: dash list or None.
        anchor: point on line where dash phase 0 sits, default p0.
        """
        L = math.dist(p0, p1)
        if L < 1e-9:
            return
        start = 0.0 if anchor is None else math.dist(p0, anchor)
        # center first dash on anchor for symmetric look
        if anchor is not None and pattern:
            start -= pattern[0] / 2
        ivs = dash_intervals(L, pattern, start)
        for poly in self.holes():
            r = clip_seg_to_poly(p0, p1, poly)
            if r:
                ivs = subtract(ivs, (r[0] * L, r[1] * L))
        ux, uy = (p1[0] - p0[0]) / L, (p1[1] - p0[1]) / L
        for a, b in ivs:
            if b - a > 0.3:
                self.seg(layer, (p0[0] + ux * a, p0[1] + uy * a), (p0[0] + ux * b, p0[1] + uy * b))

    def dashed_circle(self, layer: str, c, r: float, pattern) -> None:
        """Add dashed circle as arc paths, dashes symmetric about top.

        layer: group id. c: center. r: radius. pattern: dash list.
        """
        circ = 2 * math.pi * r
        period = sum(pattern)
        # integer periods so dashes close evenly
        n = max(1, round(circ / period))
        scale = circ / (n * period)
        holes = self.holes()
        pos = -pattern[0] * scale / 2
        for _ in range(n):
            for i, seg in enumerate(pattern):
                L = seg * scale
                if i % 2 == 0:
                    a0, a1 = pos / r - math.pi / 2, (pos + L) / r - math.pi / 2
                    q0 = (c[0] + r * math.cos(a0), c[1] + r * math.sin(a0))
                    q1 = (c[0] + r * math.cos(a1), c[1] + r * math.sin(a1))
                    am = (a0 + a1) / 2
                    mid = (c[0] + r * math.cos(am), c[1] + r * math.sin(am))
                    # drop dash if it touches a label
                    if not any(clip_seg_to_poly(q0, mid, h) or clip_seg_to_poly(mid, q1, h) for h in holes):
                        self.all_pts += [q0, q1]
                        self.layers.setdefault(layer, []).append(
                            f"M{q0[0]:.3f} {q0[1]:.3f}A{r:.3f} {r:.3f} 0 0 1 {q1[0]:.3f} {q1[1]:.3f}"
                        )
                pos += L

    def ring(self, layer: str, c, r: float) -> None:
        """Add solid circle as two arcs, not cut by labels.

        layer: group id. c: center. r: radius.
        """
        x, y = c
        self.all_pts += [(x - r, y - r), (x + r, y + r)]
        self.layers.setdefault(layer, []).append(
            f"M{x - r:.3f} {y:.3f}A{r:.3f} {r:.3f} 0 0 1 {x + r:.3f} {y:.3f}A{r:.3f} {r:.3f} 0 0 1 {x - r:.3f} {y:.3f}"
        )

    def render_labels(self) -> None:
        """Turn queued labels into glyph paths."""
        for layer, labs in self.labels.items():
            for lab in labs:
                for pts in lab.paths():
                    self.all_pts += pts
                    d = "M" + "L".join(f"{x:.3f} {y:.3f}" for x, y in pts)
                    self.layers.setdefault(layer, []).append(d)

    def check_overlaps(self) -> list[str]:
        """Find label pairs that overlap. Returns descriptions."""
        labs = [lab for ls in self.labels.values() for lab in ls]
        bad = []
        for i, a in enumerate(labs):
            pa = a.corners(0.3)
            for b in labs[i + 1:]:
                pb = b.corners(0.3)
                # SAT on both rects' edges
                sep = False
                for poly in (pa, pb):
                    for k in range(4):
                        e = (poly[(k + 1) % 4][0] - poly[k][0], poly[(k + 1) % 4][1] - poly[k][1])
                        n = (-e[1], e[0])
                        sa = [n[0] * p[0] + n[1] * p[1] for p in pa]
                        sb = [n[0] * p[0] + n[1] * p[1] for p in pb]
                        if max(sa) < min(sb) or max(sb) < min(sa):
                            sep = True
                            break
                    if sep:
                        break
                if not sep:
                    bad.append(f"{a.text}@({a.cx:.1f},{a.cy:.1f}) vs {b.text}@({b.cx:.1f},{b.cy:.1f})")
        return bad

    def svg(self) -> str:
        """Serialize. Returns SVG text."""
        out = [
            '<?xml version="1.0" encoding="UTF-8"?>',
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{SIZE:g}mm" height="{SIZE:g}mm" viewBox="0 0 {SIZE:g} {SIZE:g}">',
            "  <title>Ruler mat 400 x 400 mm</title>",
            f'  <g fill="none" stroke="#000000" stroke-width="{STROKE}" stroke-linecap="round" stroke-linejoin="round">',
        ]
        for name, ds in self.layers.items():
            out.append(f'    <g id="{name}">')
            # one path per layer keeps file small
            out.append(f'      <path d="{"".join(ds)}"/>')
            out.append("    </g>")
        out += ["  </g>", "</svg>", ""]
        return "\n".join(out)


def mx(sx: float) -> float:
    """SVG x -> machine x label value."""
    return sx


def my(sy: float) -> float:
    """SVG y -> machine y label value (Y up, origin bottom-left)."""
    return SIZE - sy


def build() -> Mat:
    """Lay out the whole mat. Returns filled Mat."""
    m = Mat()
    lo, hi = BAND, SIZE - BAND
    cx, cy = CENTER

    # --- labels first so lines can cut around them

    # edge numbers every 10 mm, sitting between tick and grid
    tpos = (TICK_10 + BAND) / 2 + 0.2
    for v in range(int(BAND), int(SIZE - BAND) + 1, 10):
        m.label("ruler_text", Label(f"{mx(v):g}", v, tpos, 0, H_EDGE))  # top
        m.label("ruler_text", Label(f"{mx(v):g}", v, SIZE - tpos, 0, H_EDGE))  # bottom
        # sides skip first/last: corner clash with top/bottom numbers
        if lo < v < hi:
            m.label("ruler_text", Label(f"{my(v):g}", tpos, v, -90, H_EDGE))  # left, reads bottom-up
            m.label("ruler_text", Label(f"{my(v):g}", SIZE - tpos, v, 90, H_EDGE))  # right, reads top-down
    # origin tag in bottom-left band corner
    m.label("ruler_text", Label("0", BAND / 2, SIZE - BAND / 2, 0, H_EDGE))

    # coordinate tags at 50 mm crossings: X over Y in the cell up-right of crossing
    for gx in range(50, int(SIZE), 50):
        for gy in range(50, int(SIZE), 50):
            if (gx, gy) == (cx, cy):
                continue
            pair = ((f"X{mx(gx):g}", gy - 7.0), (f"Y{my(gy):g}", gy - 3.0))
            for t, yy in pair:
                w = text_width(t, H_COORD)
                m.label("grid_text", Label(t, gx + 1.4 + w / 2, yy, 0, H_COORD))
            # one block over both rows, so no dash survives in between
            x0 = gx + 1.4 - HOLE_PAD
            x1 = gx + 1.4 + max(text_width(t, H_COORD) for t, _ in pair) + HOLE_PAD
            y0, y1 = gy - 7.0 - H_COORD / 2 - HOLE_PAD, gy - 3.0 + H_COORD / 2 + HOLE_PAD
            m.extra_holes.append([(x0, y0), (x1, y0), (x1, y1), (x0, y1)])

    # angle line labels along their line, at several radii
    def ray_labels(text: str, ang_machine: float, radii: list[float]) -> None:
        # machine angle CCW -> SVG direction (Y flipped)
        a = math.radians(ang_machine)
        for r in radii:
            for sgn in (1, -1):
                px, py = cx + sgn * r * math.cos(a), cy - sgn * r * math.sin(a)
                m.label("angle_text", Label(text, px, py, upright(-ang_machine), H_LINE))

    ray_labels("45°", 45, [90, 245])
    ray_labels("45°", 135, [90, 245])
    ray_labels("30°", 30, [100, 175])
    ray_labels("30°", 150, [100, 175])
    ray_labels("60°", 60, [95, 165])
    ray_labels("60°", 120, [95, 165])

    # circle tag just above circle, inside its cell row
    r = CIRCLE_D / 2
    m.label("circle_text", Label("Ø50", cx + 5, cy - r - 2.6, 0, 2.2))

    # --- lines

    # border and grid frame
    for a, b in (((0, 0), (SIZE, 0)), ((SIZE, 0), (SIZE, SIZE)), ((SIZE, SIZE), (0, SIZE)), ((0, SIZE), (0, 0))):
        m.seg("border", a, b)

    # 10 mm grid inside band
    for v in range(int(lo), int(hi) + 1, 10):
        m.line("grid", (v, lo), (v, hi))
        m.line("grid", (lo, v), (hi, v))

    # mm ticks on all four edges
    for v in range(1, int(SIZE)):
        L = TICK_10 if v % 10 == 0 else TICK_5 if v % 5 == 0 else TICK_1
        m.seg("ticks", (v, 0), (v, L))
        m.seg("ticks", (v, SIZE), (v, SIZE - L))
        m.seg("ticks", (0, v), (L, v))
        m.seg("ticks", (SIZE, v), (SIZE - L, v))

    # 45 deg family through grid crossings, clipped to grid
    big = 2 * SIZE
    offs = [0.0]
    if DIAG_SPACING > 0:
        k = 1
        while k * DIAG_SPACING < hi - lo:
            offs += [k * DIAG_SPACING, -k * DIAG_SPACING]
            k += 1
    for off in offs:
        for slope in (1, -1):
            # line through (cx + off, cy) with given slope
            p0 = (cx + off - big, cy - slope * big)
            p1 = (cx + off + big, cy + slope * big)
            seg = clip_line_to_box(p0, p1, lo, hi)
            if seg and math.dist(*seg) > 5:
                # anchor dash phase at nearest point to center
                m.line("diag_45", seg[0], seg[1], DASH_45, anchor=(cx + off, cy))

    # 30 / 60 deg lines through center
    for ang in (30, 60, 120, 150):
        a = math.radians(ang)
        d = (math.cos(a), -math.sin(a))
        seg = clip_line_to_box((cx - big * d[0], cy - big * d[1]), (cx + big * d[0], cy + big * d[1]), lo, hi)
        m.line("lines_30_60", seg[0], seg[1], DASH_30_60, anchor=(cx, cy))

    # ring marks the crossing each coordinate tag belongs to
    for gx in range(50, int(SIZE), 50):
        for gy in range(50, int(SIZE), 50):
            if (gx, gy) != (cx, cy):
                m.ring("grid_marks", (gx, gy), R_MARK)

    # center circle + center dot
    m.dashed_circle("center_circle", (cx, cy), r, DASH_CIRCLE)
    m.line("center_circle", (cx - 2, cy - 2), (cx + 2, cy + 2))
    m.line("center_circle", (cx - 2, cy + 2), (cx + 2, cy - 2))

    m.render_labels()
    return m


def main() -> int:
    """Build, validate, write SVG. Returns exit code."""
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).with_name("ruler-mat-400.svg")
    m = build()
    # everything must be engravable: inside [0, SIZE] incl. half stroke
    eps = 1e-6
    bad = [p for p in m.all_pts if not (-eps <= p[0] <= SIZE + eps and -eps <= p[1] <= SIZE + eps)]
    # text must stay clear of the border by half stroke
    txt = [lab for k, ls in m.labels.items() for lab in ls]
    txt_bad = [lab.text for lab in txt if any(not (STROKE <= c <= SIZE - STROKE) for p in lab.corners() for c in p)]
    overlaps = m.check_overlaps()
    # cell-bound labels must not cut grid lines
    for k in ("grid_text", "circle_text", "ruler_text"):
        for lab in m.labels.get(k, []):
            cs = lab.corners(HOLE_PAD)
            for axis in (0, 1):
                a, b = min(c[axis] for c in cs), max(c[axis] for c in cs)
                o0, o1 = min(c[1 - axis] for c in cs), max(c[1 - axis] for c in cs)
                if o1 < BAND or o0 > SIZE - BAND:
                    continue  # in ruler band, no grid there
                if math.floor(b / 10) * 10 > a:
                    overlaps.append(f"{lab.text}@({lab.cx:.1f},{lab.cy:.1f}) cuts grid")
    if bad or txt_bad or overlaps:
        print("out of bounds:", bad[:5], txt_bad[:5])
        print("overlaps:", overlaps[:10])
        return 1
    out.write_text(m.svg())
    xs = [p[0] for p in m.all_pts]
    ys = [p[1] for p in m.all_pts]
    print(f"wrote {out}  bbox x[{min(xs):.2f},{max(xs):.2f}] y[{min(ys):.2f},{max(ys):.2f}]  labels={len(txt)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
