// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2001-2016 Peter Selinger (Potrace)
// Copyright (C) 2009-2016 Wolfgang Nagl (CsPotrace port)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `CsPotrace/CsPotrace.cs`: Potrace bitmap tracer (bitmap -> paths -> curves).
//!
//! C# keeps all parameters and the working state in `static` fields; here the parameters live in
//! [`PotraceParams`] and the state in [`Tracer`]. Numeric types are kept (f64 / i32).

use image::RgbaImage;
use lg_dotnet::fmt_double_custom;

/// Kind of a [`Curve`]: straight line or cubic bezier (`CurveKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveKind {
    Line,
    Bezier,
}

/// Double point (`dPoint`). C# class, here a plain copy type (the port never mutates shared refs).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DPoint {
    pub x: f64,
    pub y: f64,
}

impl DPoint {
    /// Ctor `dPoint(x, y)`. `x`, `y` are the coordinates. Returns the point.
    pub const fn new(x: f64, y: f64) -> Self {
        DPoint { x, y }
    }
}

/// One output segment (`Curve` struct): line or bezier with start, two control points and end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curve {
    pub kind: CurveKind,
    /// Start point.
    pub a: DPoint,
    pub control_point_a: DPoint,
    pub control_point_b: DPoint,
    /// End point.
    pub b: DPoint,
}

impl Curve {
    /// Ctor `Curve(kind, A, ControlPointA, ControlPointB, B)`. Args map 1:1 to the fields.
    /// Returns the curve.
    pub const fn new(kind: CurveKind, a: DPoint, control_point_a: DPoint, control_point_b: DPoint, b: DPoint) -> Self {
        Curve { kind, a, control_point_a, control_point_b, b }
    }

    /// Property `LinearLenght` (sic): euclid distance A to B. Returns the length.
    pub fn linear_length(&self) -> f64 {
        let dx = self.b.x - self.a.x;
        let dy = self.b.y - self.a.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Ambiguity resolution policy (`TurnPolicy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnPolicy {
    Minority,
    Majority,
    Right,
    Black,
    White,
}

/// The C# static tracer settings (`Potrace.turnpolicy`, `turdsize`, ...). Default = C# initialisers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PotraceParams {
    /// `turnpolicy`.
    pub turnpolicy: TurnPolicy,
    /// `turdsize`: area of largest path to be ignored.
    pub turdsize: i32,
    /// `alphamax`: corner threshold.
    pub alphamax: f64,
    /// `curveoptimizing`: merge bezier runs.
    pub curveoptimizing: bool,
    /// `opttolerance`: curve optimisation tolerance.
    pub opttolerance: f64,
    /// `Treshold` (sic): brightness threshold, 0..1.
    pub treshold: f64,
}

impl Default for PotraceParams {
    fn default() -> Self {
        PotraceParams {
            turnpolicy: TurnPolicy::Minority,
            turdsize: 2,
            alphamax: 1.0,
            curveoptimizing: true,
            opttolerance: 0.2,
            treshold: 0.5,
        }
    }
}

const POTRACE_CORNER: i32 = 1;
const POTRACE_CURVETO: i32 = 2;

/// Integer point (`Point` struct).
#[derive(Debug, Clone, Copy, Default)]
struct Point {
    x: i32,
    y: i32,
}

/// Binary bitmap (`Bitmap_p`).
#[derive(Debug, Clone)]
struct BitmapP {
    w: i32,
    h: i32,
    data: Vec<u8>,
}

impl BitmapP {
    fn size(&self) -> i32 {
        self.w * self.h
    }

    /// Pixel is set and inside bounds.
    fn at(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.w && y >= 0 && y < self.h && self.data[(self.w * y + x) as usize] == 1
    }

    /// Linear index to point.
    fn index(&self, i: i32) -> Point {
        let y = i / self.w;
        Point { x: i - y * self.w, y }
    }

    /// Toggle pixel (no bounds check on write, like C#).
    fn flip(&mut self, x: i32, y: i32) {
        let i = (self.w * y + x) as usize;
        self.data[i] = if self.at(x, y) { 0 } else { 1 };
    }
}

/// Prefix sums over path points (`Sum`).
#[derive(Debug, Clone, Copy, Default)]
struct Sum {
    x: f64,
    y: f64,
    xy: f64,
    x2: f64,
    y2: f64,
}

/// Private curve (`privcurve`).
#[derive(Debug, Clone)]
struct PrivCurve {
    n: i32,
    tag: Vec<i32>,
    vertex: Vec<DPoint>,
    c: Vec<DPoint>,
    alpha: Vec<f64>,
    alpha0: Vec<f64>,
    beta: Vec<f64>,
    alphacurve: i32,
}

impl PrivCurve {
    fn new(count: i32) -> Self {
        let n = count as usize;
        PrivCurve {
            n: count,
            tag: vec![0; n],
            vertex: vec![DPoint::default(); n],
            c: vec![DPoint::default(); n * 3],
            alpha: vec![0.0; n],
            alpha0: vec![0.0; n],
            beta: vec![0.0; n],
            alphacurve: 0,
        }
    }
}

/// Traced path (`Path`).
#[derive(Debug, Clone)]
struct Path {
    m: i32,
    area: i32,
    len: i32,
    /// `sign == "+"`.
    positive: bool,
    pt: Vec<Point>,
    max_x: i32,
    x0: f64,
    y0: f64,
    po: Vec<i32>,
    lon: Vec<i32>,
    sums: Vec<Sum>,
    curve: Option<PrivCurve>,
}

impl Path {
    fn new() -> Self {
        Path {
            m: 0,
            area: 0,
            len: 0,
            positive: false,
            pt: Vec::new(),
            max_x: -1,
            x0: 0.0,
            y0: 0.0,
            po: Vec::new(),
            lon: Vec::new(),
            sums: Vec::new(),
            curve: None,
        }
    }
}

/// Quadratic form (`Quad`).
#[derive(Debug, Clone, Copy, Default)]
struct Quad {
    data: [f64; 9],
}

impl Quad {
    fn at(&self, x: usize, y: usize) -> f64 {
        self.data[x * 3 + y]
    }
}

/// Result of `opti_penalty` (`Opti`).
#[derive(Debug, Clone, Copy, Default)]
struct Opti {
    pen: f64,
    c: [DPoint; 2],
    t: f64,
    s: f64,
    alpha: f64,
}

// ---------------------------------------------------------------------------------------------
// auxiliary functions
// ---------------------------------------------------------------------------------------------

/// Point on segment [a,b] for `lambda` in 0..1.
fn interval(lambda: f64, a: DPoint, b: DPoint) -> DPoint {
    DPoint { x: a.x + lambda * (b.x - a.x), y: a.y + lambda * (b.y - a.y) }
}

/// Direction 90 deg ccw from p2-p0, restricted to a major wind direction.
fn dorth_infty(p0: DPoint, p2: DPoint) -> DPoint {
    DPoint { y: sign(p2.x - p0.x) as f64, x: -(sign(p2.y - p0.y) as f64) }
}

/// ddenom/dpara: square of radius 1 at p1 hits line p0p2 iff |dpara| <= ddenom.
fn ddenom(p0: DPoint, p2: DPoint) -> f64 {
    let r = dorth_infty(p0, p2);
    r.y * (p2.x - p0.x) - r.x * (p2.y - p0.y)
}

/// (p1-p0)x(p2-p0), parallelogram area.
fn dpara(p0: DPoint, p1: DPoint, p2: DPoint) -> f64 {
    let x1 = p1.x - p0.x;
    let y1 = p1.y - p0.y;
    let x2 = p2.x - p0.x;
    let y2 = p2.y - p0.y;
    x1 * y2 - x2 * y1
}

/// (p1-p0)x(p3-p2).
fn cprod(p0: DPoint, p1: DPoint, p2: DPoint, p3: DPoint) -> f64 {
    let x1 = p1.x - p0.x;
    let y1 = p1.y - p0.y;
    let x2 = p3.x - p2.x;
    let y2 = p3.y - p2.y;
    x1 * y2 - x2 * y1
}

/// (p1-p0)*(p2-p0).
fn iprod(p0: DPoint, p1: DPoint, p2: DPoint) -> f64 {
    let x1 = p1.x - p0.x;
    let y1 = p1.y - p0.y;
    let x2 = p2.x - p0.x;
    let y2 = p2.y - p0.y;
    x1 * x2 + y1 * y2
}

/// (p1-p0)*(p3-p2).
fn iprod1(p0: DPoint, p1: DPoint, p2: DPoint, p3: DPoint) -> f64 {
    let x1 = p1.x - p0.x;
    let y1 = p1.y - p0.y;
    let x2 = p3.x - p2.x;
    let y2 = p3.y - p2.y;
    x1 * x2 + y1 * y2
}

/// Distance between two points.
fn ddist(p: DPoint, q: DPoint) -> f64 {
    ((p.x - q.x) * (p.x - q.x) + (p.y - q.y) * (p.y - q.y)).sqrt()
}

/// Integer cross product p1 x p2.
fn xprodi(p1: Point, p2: Point) -> i32 {
    p1.x * p2.y - p1.y * p2.x
}

/// True if a <= b < c < a in a cyclic sense.
fn cyclic(a: i32, b: i32, c: i32) -> bool {
    if a <= c { a <= b && b < c } else { a <= b || b < c }
}

/// Sign of a double: -1, 0, 1.
fn sign(i: f64) -> i32 {
    if i > 0.0 {
        1
    } else if i < 0.0 {
        -1
    } else {
        0
    }
}

/// Apply quadratic form `q` to vector `w` (homogeneous).
fn quadform(q: &Quad, w: DPoint) -> f64 {
    let v = [w.x, w.y, 1.0];
    let mut sum = 0.0;
    for (i, vi) in v.iter().enumerate() {
        for (j, vj) in v.iter().enumerate() {
            sum += vi * q.at(i, j) * vj;
        }
    }
    sum
}

/// Point of a bezier curve at `t`.
fn bezier(t: f64, p0: DPoint, p1: DPoint, p2: DPoint, p3: DPoint) -> DPoint {
    let s = 1.0 - t;
    DPoint {
        x: s * s * s * p0.x + 3.0 * (s * s * t) * p1.x + 3.0 * (t * t * s) * p2.x + t * t * t * p3.x,
        y: s * s * s * p0.y + 3.0 * (s * s * t) * p1.y + 3.0 * (t * t * s) * p2.y + t * t * t * p3.y,
    }
}

/// Point t in 0..1 on bezier (p0..p3) tangent to q1-q0, or -1.0 if none.
fn tangent(p0: DPoint, p1: DPoint, p2: DPoint, p3: DPoint, q0: DPoint, q1: DPoint) -> f64 {
    let a_ = cprod(p0, p1, q0, q1);
    let b_ = cprod(p1, p2, q0, q1);
    let c_ = cprod(p2, p3, q0, q1);

    let a = a_ - 2.0 * b_ + c_;
    let b = -2.0 * a_ + 2.0 * b_;
    let c = a_;

    let d = b * b - 4.0 * a * c;

    if a == 0.0 || d < 0.0 {
        return -1.0;
    }

    let s = d.sqrt();
    let r1 = (-b + s) / (2.0 * a);
    let r2 = (-b - s) / (2.0 * a);

    if (0.0..=1.0).contains(&r1) {
        r1
    } else if (0.0..=1.0).contains(&r2) {
        r2
    } else {
        -1.0
    }
}

/// Center and slope of line i..j (i<j), needs path sums. Writes `ctr` and `dir`.
fn pointslope(path: &Path, i: i32, j: i32, ctr: &mut DPoint, dir: &mut DPoint) {
    let n = path.len;
    let sums = &path.sums;
    let (mut i, mut j) = (i, j);
    // rotations from i to j
    let mut r: i32 = 0;

    while j >= n {
        j -= n;
        r += 1;
    }
    while i >= n {
        i -= n;
        r -= 1;
    }
    while j < 0 {
        j += n;
        r -= 1;
    }
    while i < 0 {
        i += n;
        r += 1;
    }

    let (ju, iu, nu) = ((j + 1) as usize, i as usize, n as usize);
    let rf = r as f64;
    let x = sums[ju].x - sums[iu].x + rf * sums[nu].x;
    let y = sums[ju].y - sums[iu].y + rf * sums[nu].y;
    let x2 = sums[ju].x2 - sums[iu].x2 + rf * sums[nu].x2;
    let xy = sums[ju].xy - sums[iu].xy + rf * sums[nu].xy;
    let y2 = sums[ju].y2 - sums[iu].y2 + rf * sums[nu].y2;
    let k = (j + 1 - i + r * n) as f64;

    ctr.x = x / k;
    ctr.y = y / k;

    let mut a = (x2 - x * x / k) / k;
    let b = (xy - x * y / k) / k;
    let mut c = (y2 - y * y / k) / k;

    // larger eigenvalue
    let lambda2 = (a + c + ((a - c) * (a - c) + 4.0 * b * b).sqrt()) / 2.0;

    a -= lambda2;
    c -= lambda2;

    let l;
    if a.abs() >= c.abs() {
        l = (a * a + b * b).sqrt();
        if l != 0.0 {
            dir.x = -b / l;
            dir.y = a / l;
        }
    } else {
        l = (c * c + b * b).sqrt();
        if l != 0.0 {
            dir.x = -c / l;
            dir.y = b / l;
        }
    }
    if l == 0.0 {
        // can happen when k=4: both eigenvalues coincide
        dir.x = 0.0;
        dir.y = 0.0;
    }
}

/// C# `mod`: a mod n with non-negative result for a >= -n-ish, exact C# formula.
fn mod_(a: i32, n: i32) -> i32 {
    if a >= n {
        a % n
    } else if a >= 0 {
        a
    } else {
        n - 1 - (-1 - a) % n
    }
}

/// Brightness test for the 8 neighbour majority vote (`majority`).
fn majority(bm1: &BitmapP, x: i32, y: i32) -> bool {
    let s = |b: bool| if b { 1 } else { -1 };
    for i in 2..5 {
        let mut ct = 0;
        for a in (-i + 1)..=(i - 1) {
            ct += s(bm1.at(x + a, y + i - 1));
            ct += s(bm1.at(x + i - 1, y + a - 1));
            ct += s(bm1.at(x + a - 1, y - i));
            ct += s(bm1.at(x - i, y + a));
        }
        if ct > 0 {
            return true;
        } else if ct < 0 {
            return false;
        }
    }
    false
}

/// First set pixel at or after `point` in scan order, or None.
fn find_next(bm1: &BitmapP, point: Point) -> Option<Point> {
    let mut i = bm1.w * point.y + point.x;
    while i < bm1.size() && bm1.data[i as usize] != 1 {
        i += 1;
    }
    if i >= bm1.size() {
        return None;
    }
    Some(bm1.index(i))
}

/// Tracer state (the C# statics `bm` and `pathlist`) plus parameters.
#[derive(Debug, Clone)]
pub struct Tracer {
    /// Tracer settings used by [`Tracer::trace`].
    pub params: PotraceParams,
    bm: Option<BitmapP>,
    pathlist: Vec<Path>,
}

impl Tracer {
    /// Create a tracer. `params` are the settings (C# static fields). Returns an empty tracer.
    pub fn new(params: PotraceParams) -> Self {
        Tracer { params, bm: None, pathlist: Vec::new() }
    }

    /// `Clear`: drop bitmap and paths.
    pub fn clear(&mut self) {
        self.bm = None;
        self.pathlist.clear();
    }

    /// `ConvertBitmap`: threshold on R+G+B sum (alpha ignored). `bitmap` is the source image.
    fn convert_bitmap(&mut self, bitmap: &RgbaImage) {
        let limit = self.params.treshold * 255.0 * 3.0;
        let data: Vec<u8> = bitmap
            .pixels()
            .map(|p| if (p.0[0] as f64 + p.0[1] as f64 + p.0[2] as f64) < limit { 1 } else { 0 })
            .collect();
        self.bm = Some(BitmapP { w: bitmap.width() as i32, h: bitmap.height() as i32, data });
    }

    /// `findPath`: trace the outline starting at upper-left corner `point` of bitmap `bm1`.
    fn find_path(&self, bm1: &BitmapP, point: Point) -> Path {
        let bm = self.bm.as_ref().expect("bitmap");
        let tp = self.params.turnpolicy;
        let mut path = Path::new();
        let mut x = point.x;
        let mut y = point.y;
        let mut dirx: i32 = 0;
        let mut diry: i32 = 1;

        // NOTE(upstream): sign is read from the original bitmap, not the xor-ed copy.
        path.positive = bm.at(point.x, point.y);
        let mut min_x = 100000;
        let mut min_y = 100000;
        let mut max_y = -1;

        loop {
            path.pt.push(Point { x, y });
            if x > path.max_x {
                path.max_x = x;
            }
            if x < min_x {
                min_x = x;
            }
            if y > max_y {
                max_y = y;
            }
            if y < min_y {
                min_y = y;
            }
            path.len += 1;

            x += dirx;
            y += diry;
            path.area -= x * diry;

            if x == point.x && y == point.y {
                break;
            }

            let l = bm1.at(x + (dirx + diry - 1) / 2, y + (diry - dirx - 1) / 2);
            let r = bm1.at(x + (dirx - diry - 1) / 2, y + (diry + dirx - 1) / 2);

            if r && !l {
                if tp == TurnPolicy::Right
                    || (tp == TurnPolicy::Black && path.positive)
                    || (tp == TurnPolicy::White && !path.positive)
                    || (tp == TurnPolicy::Majority && majority(bm1, x, y))
                    || (tp == TurnPolicy::Minority && !majority(bm1, x, y))
                {
                    let tmp = dirx;
                    dirx = -diry;
                    diry = tmp;
                } else {
                    let tmp = dirx;
                    dirx = diry;
                    diry = -tmp;
                }
            } else if r {
                let tmp = dirx;
                dirx = -diry;
                diry = tmp;
            } else if !l {
                let tmp = dirx;
                dirx = diry;
                diry = -tmp;
            }
        }
        path
    }

    /// `bmToPathlist`: decompose bitmap into paths, fill `pathlist` (area > turdsize).
    fn bm_to_pathlist(&mut self) {
        let mut bm1 = self.bm.as_ref().expect("bitmap").clone();
        let mut current = Point { x: 0, y: 0 };

        while let Some(p) = find_next(&bm1, current) {
            current = p;
            let path = self.find_path(&bm1, current);
            xor_path(&mut bm1, &path);
            if path.area > self.params.turdsize {
                self.pathlist.push(path);
            }
        }
    }

    /// `adjustVertices`: stage 3, set `path.curve.vertex`.
    fn adjust_vertices(path: &mut Path) {
        let m = path.m;
        let n = path.len;
        let x0 = path.x0;
        let y0 = path.y0;
        let mu = m as usize;
        let mut ctr = vec![DPoint::default(); mu];
        let mut dir = vec![DPoint::default(); mu];
        let mut q = vec![Quad::default(); mu];
        let mut v = [0.0f64; 3];
        let mut s = DPoint::default();
        let mut curve = PrivCurve::new(m);

        // optimal point-slope representation of each line segment
        for i in 0..m {
            let mut j = path.po[mod_(i + 1, m) as usize];
            j = mod_(j - path.po[i as usize], n) + path.po[i as usize];
            let iu = i as usize;
            let (mut c_, mut d_) = (DPoint::default(), DPoint::default());
            pointslope(path, path.po[iu], j, &mut c_, &mut d_);
            ctr[iu] = c_;
            dir[iu] = d_;
        }

        // each segment as singular quadratic form
        for i in 0..mu {
            let d = dir[i].x * dir[i].x + dir[i].y * dir[i].y;
            if d == 0.0 {
                q[i].data = [0.0; 9];
            } else {
                v[0] = dir[i].y;
                v[1] = -dir[i].x;
                v[2] = -v[1] * ctr[i].y - v[0] * ctr[i].x;
                for l in 0..3 {
                    for k in 0..3 {
                        q[i].data[l * 3 + k] = v[l] * v[k] / d;
                    }
                }
            }
        }

        // intersections of consecutive segments, clamped to unit square
        for i in 0..m {
            let iu = i as usize;
            let mut qq = Quad::default();
            let mut w = DPoint::default();
            // vertex relative to x0/y0
            s.x = path.pt[path.po[iu] as usize].x as f64 - x0;
            s.y = path.pt[path.po[iu] as usize].y as f64 - y0;
            let j = mod_(i - 1, m) as usize;
            for l in 0..3 {
                for k in 0..3 {
                    qq.data[l * 3 + k] = q[j].at(l, k) + q[iu].at(l, k);
                }
            }

            loop {
                let det = qq.at(0, 0) * qq.at(1, 1) - qq.at(0, 1) * qq.at(1, 0);
                if det != 0.0 {
                    w.x = (-qq.at(0, 2) * qq.at(1, 1) + qq.at(1, 2) * qq.at(0, 1)) / det;
                    w.y = (qq.at(0, 2) * qq.at(1, 0) - qq.at(1, 2) * qq.at(0, 0)) / det;
                    break;
                }
                // singular: add orthogonal axis through the unit square centre
                if qq.at(0, 0) > qq.at(1, 1) {
                    v[0] = -qq.at(0, 1);
                    v[1] = qq.at(0, 0);
                } else if qq.at(1, 1) != 0.0 {
                    v[0] = -qq.at(1, 1);
                    v[1] = qq.at(1, 0);
                } else {
                    v[0] = 1.0;
                    v[1] = 0.0;
                }
                let d = v[0] * v[0] + v[1] * v[1];
                v[2] = -v[1] * s.y - v[0] * s.x;
                for l in 0..3 {
                    for k in 0..3 {
                        qq.data[l * 3 + k] += v[l] * v[k] / d;
                    }
                }
            }
            let mut dx = (w.x - s.x).abs();
            let mut dy = (w.y - s.y).abs();
            if dx <= 0.5 && dy <= 0.5 {
                curve.vertex[iu] = DPoint::new(w.x + x0, w.y + y0);
                continue;
            }
            // minimum outside unit square: minimise on its boundary
            let mut min = quadform(&qq, s);
            let mut xmin = s.x;
            let mut ymin = s.y;

            if qq.at(0, 0) != 0.0 {
                for z in 0..2 {
                    w.y = s.y - 0.5 + z as f64;
                    w.x = -(qq.at(0, 1) * w.y + qq.at(0, 2)) / qq.at(0, 0);
                    dx = (w.x - s.x).abs();
                    let cand = quadform(&qq, w);
                    if dx <= 0.5 && cand < min {
                        min = cand;
                        xmin = w.x;
                        ymin = w.y;
                    }
                }
            }

            if qq.at(1, 1) != 0.0 {
                for z in 0..2 {
                    w.x = s.x - 0.5 + z as f64;
                    w.y = -(qq.at(1, 0) * w.x + qq.at(1, 2)) / qq.at(1, 1);
                    dy = (w.y - s.y).abs();
                    let cand = quadform(&qq, w);
                    if dy <= 0.5 && cand < min {
                        min = cand;
                        xmin = w.x;
                        ymin = w.y;
                    }
                }
            }
            // four corners
            for l in 0..2 {
                for k in 0..2 {
                    w.x = s.x - 0.5 + l as f64;
                    w.y = s.y - 0.5 + k as f64;
                    let cand = quadform(&qq, w);
                    if cand < min {
                        min = cand;
                        xmin = w.x;
                        ymin = w.y;
                    }
                }
            }

            curve.vertex[iu] = DPoint::new(xmin + x0, ymin + y0);
        }
        path.curve = Some(curve);
    }

    /// `smooth`: stage 4, corner analysis; fills tags and control points.
    fn smooth(&self, path: &mut Path) {
        let alphamax = self.params.alphamax;
        let curve = path.curve.as_mut().expect("curve");
        let m = curve.n;
        if !path.positive {
            // reverse
            let mut i = 0usize;
            let mut j = (m - 1) as usize;
            while i < j {
                curve.vertex.swap(i, j);
                i += 1;
                j -= 1;
            }
        }
        for i in 0..m {
            let j = mod_(i + 1, m) as usize;
            let k = mod_(i + 2, m) as usize;
            let iu = i as usize;
            let p4 = interval(1.0 / 2.0, curve.vertex[k], curve.vertex[j]);

            let denom = ddenom(curve.vertex[iu], curve.vertex[k]);
            let mut alpha;
            if denom != 0.0 {
                let mut dd = dpara(curve.vertex[iu], curve.vertex[j], curve.vertex[k]) / denom;
                dd = dd.abs();
                alpha = if dd > 1.0 { 1.0 - 1.0 / dd } else { 0.0 };
                alpha /= 0.75;
            } else {
                alpha = 4.0 / 3.0;
            }
            // remember "original" alpha
            curve.alpha0[j] = alpha;

            if alpha >= alphamax {
                curve.tag[j] = POTRACE_CORNER;
                curve.c[3 * j + 1] = curve.vertex[j];
                curve.c[3 * j + 2] = p4;
            } else {
                #[allow(clippy::manual_clamp)] // keep C# NaN behaviour
                if alpha < 0.55 {
                    alpha = 0.55;
                } else if alpha > 1.0 {
                    alpha = 1.0;
                }
                let p2 = interval(0.5 + 0.5 * alpha, curve.vertex[iu], curve.vertex[j]);
                let p3 = interval(0.5 + 0.5 * alpha, curve.vertex[k], curve.vertex[j]);
                curve.tag[j] = POTRACE_CURVETO;
                curve.c[3 * j] = p2;
                curve.c[3 * j + 1] = p3;
                curve.c[3 * j + 2] = p4;
            }
            // cropped alpha
            curve.alpha[j] = alpha;
            curve.beta[j] = 0.5;
        }
        curve.alphacurve = 1;
    }

    /// `opti_penalty`: best fit from i+.5 to j+.5. Returns true if impossible (C# returned 1).
    fn opti_penalty(
        &self,
        curve: &PrivCurve,
        i: i32,
        j: i32,
        res: &mut Opti,
        convc: &[i32],
        areac: &[f64],
    ) -> bool {
        let opttolerance = self.params.opttolerance;
        let m = curve.n;
        let vertex = &curve.vertex;
        let vx = |k: i32| vertex[k as usize];
        let cc = |k: i32| curve.c[k as usize];

        // sanity: full loop is never an opticurve
        if i == j {
            return true;
        }

        let mut k = i;
        let i1 = mod_(i + 1, m);
        let mut k1 = mod_(k + 1, m);
        let conv = convc[k1 as usize];
        if conv == 0 {
            return true;
        }
        let d = ddist(vx(i), vx(i1));
        // convexity, corner-freeness, max bend < 179 deg
        k = k1;
        while k != j {
            k1 = mod_(k + 1, m);
            let k2 = mod_(k + 2, m);
            if convc[k1 as usize] != conv {
                return true;
            }
            if sign(cprod(vx(i), vx(i1), vx(k1), vx(k2))) != conv {
                return true;
            }
            if iprod1(vx(i), vx(i1), vx(k1), vx(k2)) < d * ddist(vx(k1), vx(k2)) * -0.999847695156 {
                return true;
            }
            k = k1;
        }
        // the curve we're working in
        let p0 = cc(mod_(i, m) * 3 + 2);
        let mut p1 = vx(mod_(i + 1, m));
        let mut p2 = vx(mod_(j, m));
        let p3 = cc(mod_(j, m) * 3 + 2);
        // its area
        let mut area = areac[j as usize] - areac[i as usize];
        area -= dpara(vx(0), cc(i * 3 + 2), cc(j * 3 + 2)) / 2.0;
        if i >= j {
            area += areac[m as usize];
        }
        // intersection o of p0p1 and p2p3
        let a1 = dpara(p0, p1, p2);
        let a2 = dpara(p0, p1, p3);
        let a3 = dpara(p0, p2, p3);

        let a4 = a1 + a3 - a2;

        // should never happen
        if a2 == a1 {
            return true;
        }

        let mut t = a3 / (a3 - a4);
        let s = a2 / (a2 - a1);
        let a = a2 * t / 2.0;

        // should never happen
        if a == 0.0 {
            return true;
        }

        // relative area
        let r = area / a;
        // overall alpha for p0-o-p3 curve
        let alpha = 2.0 - (4.0 - r / 0.3).sqrt();

        res.c[0] = interval(t * alpha, p0, p1);
        res.c[1] = interval(s * alpha, p3, p2);
        res.alpha = alpha;
        res.t = t;
        res.s = s;

        p1 = res.c[0];
        p2 = res.c[1];

        res.pen = 0.0;
        // tangency with edges
        k = mod_(i + 1, m);
        while k != j {
            k1 = mod_(k + 1, m);
            t = tangent(p0, p1, p2, p3, vx(k), vx(k1));
            if t < -0.5 {
                return true;
            }
            let pt = bezier(t, p0, p1, p2, p3);
            let d = ddist(vx(k), vx(k1));
            if d == 0.0 {
                // should never happen
                return true;
            }
            let d1 = dpara(vx(k), vx(k1), pt) / d;
            if d1.abs() > opttolerance {
                return true;
            }
            if iprod(vx(k), vx(k1), pt) < 0.0 || iprod(vx(k1), vx(k), pt) < 0.0 {
                return true;
            }
            res.pen += d1 * d1;
            k = k1;
        }
        // corners
        k = i;
        while k != j {
            k1 = mod_(k + 1, m);
            t = tangent(p0, p1, p2, p3, cc(k * 3 + 2), cc(k1 * 3 + 2));
            if t < -0.5 {
                return true;
            }
            let pt = bezier(t, p0, p1, p2, p3);
            let d = ddist(cc(k * 3 + 2), cc(k1 * 3 + 2));
            if d == 0.0 {
                // should never happen
                return true;
            }
            let mut d1 = dpara(cc(k * 3 + 2), cc(k1 * 3 + 2), pt) / d;
            let mut d2 = dpara(cc(k * 3 + 2), cc(k1 * 3 + 2), vx(k1)) / d;
            d2 *= 0.75 * curve.alpha[k1 as usize];
            if d2 < 0.0 {
                d1 = -d1;
                d2 = -d2;
            }
            if d1 < d2 - opttolerance {
                return true;
            }
            if d1 < d2 {
                res.pen += (d1 - d2) * (d1 - d2);
            }
            k = k1;
        }

        false
    }

    /// `optiCurve`: stage 5, replace bezier runs by single segments where possible.
    fn opti_curve(&self, path: &mut Path) {
        let curve = path.curve.as_ref().expect("curve");
        let m = curve.n;
        let mu = m as usize;
        let vert = &curve.vertex;
        let mut pt = vec![0i32; mu + 1];
        let mut pen = vec![0.0f64; mu + 1];
        let mut len = vec![0i32; mu + 1];
        let mut opt = vec![Opti::default(); mu + 1];
        let mut o = Opti::default();
        let mut convc = vec![0i32; mu];
        let mut areac = vec![0.0f64; mu + 1];

        // pre-calculate convexity: +1 right turn, -1 left turn, 0 corner
        for i in 0..m {
            convc[i as usize] = if curve.tag[i as usize] == POTRACE_CURVETO {
                sign(dpara(vert[mod_(i - 1, m) as usize], vert[i as usize], vert[mod_(i + 1, m) as usize]))
            } else {
                0
            };
        }
        // pre-calculate areas
        let mut area = 0.0;
        areac[0] = 0.0;
        let p0 = curve.vertex[0];
        for i in 0..m {
            let i1 = mod_(i + 1, m) as usize;
            let iu = i as usize;
            if curve.tag[i1] == POTRACE_CURVETO {
                let alpha = curve.alpha[i1];
                area += 0.3 * alpha * (4.0 - alpha) * dpara(curve.c[iu * 3 + 2], vert[i1], curve.c[i1 * 3 + 2]) / 2.0;
                area += dpara(p0, curve.c[iu * 3 + 2], curve.c[i1 * 3 + 2]) / 2.0;
            }
            areac[iu + 1] = area;
        }

        pt[0] = -1;
        pen[0] = 0.0;
        len[0] = 0;

        // NOTE(upstream): always starts from a fixed point (Fixme in original).
        for j in 1..=m {
            let ju = j as usize;
            // best path from 0 to j
            pt[ju] = j - 1;
            pen[ju] = pen[ju - 1];
            len[ju] = len[ju - 1] + 1;

            let mut i = j - 2;
            while i >= 0 {
                let iu = i as usize;
                let r = self.opti_penalty(curve, i, mod_(j, m), &mut o, &convc, &areac);
                if r {
                    break;
                }
                if len[ju] > len[iu] + 1 || (len[ju] == len[iu] + 1 && pen[ju] > pen[iu] + o.pen) {
                    pt[ju] = i;
                    pen[ju] = pen[iu] + o.pen;
                    len[ju] = len[iu] + 1;
                    opt[ju] = o;
                    o = Opti::default();
                }
                i -= 1;
            }
        }
        let om = len[mu];
        let mut ocurve = PrivCurve::new(om);
        let omu = om as usize;
        let mut s = vec![0.0f64; omu];
        let mut t = vec![0.0f64; omu];

        let mut j = m;
        for i in (0..om).rev() {
            let iu = i as usize;
            let jm = mod_(j, m) as usize;
            if pt[j as usize] == j - 1 {
                ocurve.tag[iu] = curve.tag[jm];
                ocurve.c[iu * 3] = curve.c[jm * 3];
                ocurve.c[iu * 3 + 1] = curve.c[jm * 3 + 1];
                ocurve.c[iu * 3 + 2] = curve.c[jm * 3 + 2];
                ocurve.vertex[iu] = curve.vertex[jm];
                ocurve.alpha[iu] = curve.alpha[jm];
                ocurve.alpha0[iu] = curve.alpha0[jm];
                ocurve.beta[iu] = curve.beta[jm];
                s[iu] = 1.0;
                t[iu] = 1.0;
            } else {
                let oj = &opt[j as usize];
                ocurve.tag[iu] = POTRACE_CURVETO;
                ocurve.c[iu * 3] = oj.c[0];
                ocurve.c[iu * 3 + 1] = oj.c[1];
                ocurve.c[iu * 3 + 2] = curve.c[jm * 3 + 2];
                ocurve.vertex[iu] = interval(oj.s, curve.c[jm * 3 + 2], vert[jm]);
                ocurve.alpha[iu] = oj.alpha;
                ocurve.alpha0[iu] = oj.alpha;
                s[iu] = oj.s;
                t[iu] = oj.t;
            }
            j = pt[j as usize];
        }
        // beta parameters
        for i in 0..om {
            let i1 = mod_(i + 1, om) as usize;
            let iu = i as usize;
            ocurve.beta[iu] = s[iu] / (s[iu] + t[i1]);
        }
        ocurve.alphacurve = 1;
        path.curve = Some(ocurve);
    }

    /// `PotraceTrace`: trace `bitmap` into curve lists (outer paths first, then holes, like Potrace).
    ///
    /// `bitmap` is the source image (RGB summed against the threshold, alpha ignored).
    /// Returns one curve list per accepted path.
    pub fn trace(&mut self, bitmap: &RgbaImage) -> Vec<Vec<Curve>> {
        self.clear();

        self.convert_bitmap(bitmap);
        self.bm_to_pathlist();
        let mut list = std::mem::take(&mut self.pathlist);
        for p in list.iter_mut() {
            calc_sums(p);
            calc_lon(p);
            best_polygon(p);
            Self::adjust_vertices(p);

            self.smooth(p);
            if self.params.curveoptimizing {
                self.opti_curve(p);
            }
        }
        self.pathlist = list;

        self.trace_to_list()
    }

    /// `tracetoList`: convert private curves into the public [`Curve`] lists.
    fn trace_to_list(&self) -> Vec<Vec<Curve>> {
        let mut out = Vec::new();
        for p in &self.pathlist {
            let pc = p.curve.as_ref().expect("curve");
            let mut list = Vec::new();
            let mut l = pc.c[(pc.n - 1) as usize * 3 + 2];
            for j in 0..pc.n as usize {
                let a = pc.c[j * 3 + 1];
                let b = pc.c[j * 3 + 2];

                if pc.tag[j] == POTRACE_CORNER {
                    list.push(Curve::new(CurveKind::Line, l, l, a, a));
                    list.push(Curve::new(CurveKind::Line, a, a, b, b));
                } else {
                    let cp = pc.c[j * 3];
                    list.push(Curve::new(CurveKind::Bezier, l, cp, a, b));
                }
                l = b;
            }
            out.push(list);
        }
        out
    }

    /// `getSVG`: SVG of the last trace (size 1). Returns None if no trace happened yet.
    pub fn get_svg(&self) -> Option<String> {
        let bm = self.bm.as_ref()?;
        let size = 1.0f64;
        let w = (bm.w as f64 * size) as i32;
        let h = (bm.h as f64 * size) as i32;
        let mut svg = format!(
            "<svg id=\"svg\" version=\"1.1\" width=\"{w}\" height=\"{h}\" xmlns=\"http://www.w3.org/2000/svg\">"
        );
        svg.push_str("<path d=\"");
        for p in &self.pathlist {
            svg.push_str(&svg_path(p.curve.as_ref()?, size));
        }
        let strokec = "";
        let fillc = "black";
        let fillrule = " fill-rule=\"evenodd\"";
        svg.push_str(&format!("\"\n  stroke=\"{strokec}\" fill=\"{fillc}\" {fillrule}/></svg>"));
        Some(svg)
    }
}

/// `xorPath`: flip the pixels inside `path` in `bm1`.
fn xor_path(bm1: &mut BitmapP, path: &Path) {
    let mut y1 = path.pt[0].y;
    let len = path.len;
    for i in 1..len {
        let x = path.pt[i as usize].x;
        let y = path.pt[i as usize].y;

        if y != y1 {
            let min_y = if y1 < y { y1 } else { y };
            let max_x = path.max_x;
            for j in x..max_x {
                bm1.flip(j, min_y);
            }
            y1 = y;
        }
    }
}

/// `calcSums`: fill the prefix sums of a path.
fn calc_sums(path: &mut Path) {
    path.x0 = path.pt[0].x as f64;
    path.y0 = path.pt[0].y as f64;

    path.sums.push(Sum::default());
    for i in 0..path.len as usize {
        let x = path.pt[i].x as f64 - path.x0;
        let y = path.pt[i].y as f64 - path.y0;
        let s = path.sums[i];
        path.sums.push(Sum { x: s.x + x, y: s.y + y, xy: s.xy + x * y, x2: s.x2 + x * x, y2: s.y2 + y * y });
    }
}

/// `penalty3`: penalty of an edge i..j (needs lon and sums).
fn penalty3(path: &Path, i: i32, j: i32) -> f64 {
    let n = path.len;
    let pt = &path.pt;
    let sums = &path.sums;
    let mut j = j;
    let mut r = 0;

    if j >= n {
        j -= n;
        r = 1;
    }
    let (ju, iu, nu) = ((j + 1) as usize, i as usize, n as usize);
    let (x, y, x2, xy, y2, k);
    if r == 0 {
        x = sums[ju].x - sums[iu].x;
        y = sums[ju].y - sums[iu].y;
        x2 = sums[ju].x2 - sums[iu].x2;
        xy = sums[ju].xy - sums[iu].xy;
        y2 = sums[ju].y2 - sums[iu].y2;
        k = j + 1 - i;
    } else {
        x = sums[ju].x - sums[iu].x + sums[nu].x;
        y = sums[ju].y - sums[iu].y + sums[nu].y;
        x2 = sums[ju].x2 - sums[iu].x2 + sums[nu].x2;
        xy = sums[ju].xy - sums[iu].xy + sums[nu].xy;
        y2 = sums[ju].y2 - sums[iu].y2 + sums[nu].y2;
        k = j + 1 - i + n;
    }
    let k = k as f64;
    let (pi, pj, p0) = (pt[iu], pt[j as usize], pt[0]);

    let px = (pi.x + pj.x) as f64 / 2.0 - p0.x as f64;
    let py = (pi.y + pj.y) as f64 / 2.0 - p0.y as f64;
    let ey = (pj.x - pi.x) as f64;
    let ex = -(pj.y - pi.y) as f64;

    let a = (x2 - 2.0 * x * px) / k + px * px;
    let b = (xy - x * py - y * px) / k + px * py;
    let c = (y2 - 2.0 * y * py) / k + py * py;

    let s = ex * ex * a + 2.0 * ex * ey * b + ey * ey * c;

    s.sqrt()
}

/// `calcLon`: stage 2, longest straight subpath from each point.
fn calc_lon(path: &mut Path) {
    let n = path.len;
    let nu = n as usize;
    let pt = &path.pt;

    let mut pivk = vec![0i32; nu];
    let mut nc = vec![0i32; nu];
    let mut lon = vec![0i32; nu];

    let mut constraint = [Point::default(); 2];
    let mut cur = Point::default();
    let mut off = Point::default();
    let mut dk;

    // furthest future point connected by a horizontal/vertical segment
    let mut k: i32 = 0;
    // pivot points
    for i in (0..n).rev() {
        if pt[i as usize].x != pt[k as usize].x && pt[i as usize].y != pt[k as usize].y {
            k = i + 1;
        }
        nc[i as usize] = k;
    }

    let p = |i: i32| pt[i as usize];

    for i in (0..n).rev() {
        let mut ct = [0i32; 4];
        let dir = (3 + 3 * (p(mod_(i + 1, n)).x - p(i).x) + (p(mod_(i + 1, n)).y - p(i).y)) / 2;
        ct[dir as usize] += 1;

        constraint[0] = Point::default();
        constraint[1] = Point::default();

        k = nc[i as usize];
        let mut k1 = i;
        let mut foundk;
        loop {
            foundk = 0;
            let dir = (3 + 3 * sign((p(k).x - p(k1).x) as f64) + sign((p(k).y - p(k1).y) as f64)) / 2;
            ct[dir as usize] += 1;

            if ct[0] == 1 && ct[1] == 1 && ct[2] == 1 && ct[3] == 1 {
                pivk[i as usize] = k1;
                foundk = 1;
                break;
            }

            cur.x = p(k).x - p(i).x;
            cur.y = p(k).y - p(i).y;

            if xprodi(constraint[0], cur) < 0 || xprodi(constraint[1], cur) > 0 {
                break;
            }

            if cur.x.abs() <= 1 && cur.y.abs() <= 1 {
                // nothing
            } else {
                off.x = cur.x + if cur.y >= 0 && (cur.y > 0 || cur.x < 0) { 1 } else { -1 };
                off.y = cur.y + if cur.x <= 0 && (cur.x < 0 || cur.y < 0) { 1 } else { -1 };
                if xprodi(constraint[0], off) >= 0 {
                    constraint[0] = off;
                }
                off.x = cur.x + if cur.y <= 0 && (cur.y < 0 || cur.x < 0) { 1 } else { -1 };
                off.y = cur.y + if cur.x >= 0 && (cur.x > 0 || cur.y < 0) { 1 } else { -1 };
                if xprodi(constraint[1], off) <= 0 {
                    constraint[1] = off;
                }
            }
            k1 = k;
            k = nc[k1 as usize];
            if !cyclic(k, i, k1) {
                break;
            }
        }
        if foundk == 0 {
            dk = Point {
                x: sign((p(k).x - p(k1).x) as f64),
                y: sign((p(k).y - p(k1).y) as f64),
            };
            cur.x = p(k1).x - p(i).x;
            cur.y = p(k1).y - p(i).y;

            let a = xprodi(constraint[0], cur);
            let b = xprodi(constraint[0], dk);
            let c = xprodi(constraint[1], cur);
            let d = xprodi(constraint[1], dk);

            let mut j = 10000000;
            if b < 0 {
                j = a / -b;
            }
            if d > 0 {
                j = j.min(-c / d);
            }
            pivk[i as usize] = mod_(k1 + j, n);
        }
    }

    let mut j = pivk[nu - 1];
    lon[nu - 1] = j;
    for i in (0..=(n - 2)).rev() {
        if cyclic(i + 1, pivk[i as usize], j) {
            j = pivk[i as usize];
        }
        lon[i as usize] = j;
    }

    let mut i = n - 1;
    while cyclic(mod_(i + 1, n), j, lon[i as usize]) {
        lon[i as usize] = j;
        i -= 1;
    }
    path.lon = lon;
}

/// `bestPolygon`: optimal polygon, fills `m` and `po`.
fn best_polygon(path: &mut Path) {
    let n = path.len;
    let nu = n as usize;
    let mut clip0 = vec![0i32; nu];
    let mut pen = vec![0.0f64; nu + 1];
    let mut prev = vec![0i32; nu + 1];
    let mut clip1 = vec![0i32; nu + 1];
    let mut seg0 = vec![0i32; nu + 1];
    let mut seg1 = vec![0i32; nu + 1];

    for i in 0..n {
        let mut c = mod_(path.lon[mod_(i - 1, n) as usize] - 1, n);
        if c == i {
            c = mod_(i + 1, n);
        }
        clip0[i as usize] = if c < i { n } else { c };
    }

    let mut j: i32 = 1;
    for i in 0..n {
        while j <= clip0[i as usize] {
            clip1[j as usize] = i;
            j += 1;
        }
    }

    let mut i: i32 = 0;
    j = 0;
    while i < n {
        seg0[j as usize] = i;
        i = clip0[i as usize];
        j += 1;
    }
    seg0[j as usize] = n;
    let m = j;

    i = n;
    j = m;
    while j > 0 {
        seg1[j as usize] = i;
        i = clip1[i as usize];
        j -= 1;
    }
    seg1[0] = 0;

    pen[0] = 0.0;
    // shortest path with m segments, based on penalty3
    for j in 1..=m {
        let ju = j as usize;
        for i in seg1[ju]..=seg0[ju] {
            let mut best = -1.0;
            let mut k = seg0[ju - 1];
            while k >= clip1[i as usize] {
                let thispen = penalty3(path, k, i) + pen[k as usize];
                if best < 0.0 || thispen < best {
                    prev[i as usize] = k;
                    best = thispen;
                }
                k -= 1;
            }
            pen[i as usize] = best;
        }
    }
    path.m = m;
    path.po = vec![0; m as usize];
    // read off shortest path
    let mut i = n;
    let mut j = m - 1;
    while i > 0 {
        i = prev[i as usize];
        path.po[j as usize] = i;
        j -= 1;
    }
}

/// SVG number: `{0:0.000}` en-US.
fn svg_num(v: f64) -> String {
    fmt_double_custom(v, "0.000")
}

/// `segment`: SVG line command for corner i.
fn svg_segment(curve: &PrivCurve, i: usize, size: f64) -> String {
    let c1 = curve.c[i * 3 + 1];
    let c2 = curve.c[i * 3 + 2];
    format!("\nL {} {} {} {} ", svg_num(c1.x * size), svg_num(c1.y * size), svg_num(c2.x * size), svg_num(c2.y * size))
}

/// `bezier` (SVG): SVG cubic command for segment i.
fn svg_bezier(curve: &PrivCurve, i: usize, size: f64) -> String {
    let c0 = curve.c[i * 3];
    let c1 = curve.c[i * 3 + 1];
    let c2 = curve.c[i * 3 + 2];
    format!(
        "\nC {} {} {} {} {} {} ",
        svg_num(c0.x * size),
        svg_num(c0.y * size),
        svg_num(c1.x * size),
        svg_num(c1.y * size),
        svg_num(c2.x * size),
        svg_num(c2.y * size)
    )
}

/// `path` (SVG): SVG path data of one curve.
fn svg_path(curve: &PrivCurve, size: f64) -> String {
    let n = curve.n as usize;
    let last = curve.c[(n - 1) * 3 + 2];
    let mut p = format!("\nM {} {} ", svg_num(last.x * size), svg_num(last.y * size));
    for i in 0..n {
        if curve.tag[i] == POTRACE_CURVETO {
            p.push_str(&svg_bezier(curve, i, size));
        } else if curve.tag[i] == POTRACE_CORNER {
            p.push_str(&svg_segment(curve, i, size));
        }
    }
    p
}

/// `Potrace.PotraceTrace(Bitmap)` convenience: trace `bitmap` with `params` in a fresh tracer.
///
/// `bitmap` is the source image, `params` the settings. Returns the curve lists.
pub fn potrace_trace(bitmap: &RgbaImage, params: &PotraceParams) -> Vec<Vec<Curve>> {
    Tracer::new(*params).trace(bitmap)
}
