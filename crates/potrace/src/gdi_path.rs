// SPDX-License-Identifier: GPL-3.0-or-later
// Algorithm ported from Mono libgdiplus, see NOTICE.md
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Emulation of the parts of GDI+ `GraphicsPath` used by `CsPotrace` (AddLine, AddBezier,
//! Flatten, PathPoints), all in f32.
//!
//! The algorithm is a port of Mono's libgdiplus 6.1 (`graphics-path.c`: `append`,
//! `GdipAddPathLine`, `GdipAddPathBezier`, `nr_curve_flatten`, `GdipFlattenPath`), which is the
//! oracle of the golden tests. NOTE: Windows GDI+ is closed source and may flatten slightly
//! differently, so vector fillings can differ in the last digits from the Windows original.

/// Max subdivision depth of `nr_curve_flatten` (`FLATTEN_RECURSION_LIMIT`).
const FLATTEN_RECURSION_LIMIT: i32 = 10;

/// Default flatness of `GraphicsPath.Flatten()`.
pub const DEFAULT_FLATNESS: f32 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PointType {
    Start,
    Line,
    Bezier,
}

/// Minimal GDI+ path: points and point types.
#[derive(Debug, Clone)]
pub struct GraphicsPath {
    points: Vec<(f32, f32)>,
    types: Vec<PointType>,
    start_new_fig: bool,
}

impl Default for GraphicsPath {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphicsPath {
    /// Ctor `GraphicsPath()`: empty path. Returns it.
    pub fn new() -> Self {
        GraphicsPath { points: Vec::new(), types: Vec::new(), start_new_fig: true }
    }

    /// libgdiplus `append`. `x`,`y` point; `ty` type; `compress` drops a point equal to the last.
    fn append(&mut self, x: f32, y: f32, ty: PointType, compress: bool) {
        let mut t = ty;
        if self.start_new_fig {
            t = PointType::Start;
        } else if let Some(&(lx, ly)) = self.points.last() {
            // identical point can be compressed (no closed subpaths here)
            if compress && lx == x && ly == y {
                return;
            }
        }
        self.points.push((x, y));
        self.types.push(t);
        self.start_new_fig = false;
    }

    /// `AddLine(PointF, PointF)`: `p1`, `p2` end points. Only the first point may be compressed.
    pub fn add_line(&mut self, p1: (f32, f32), p2: (f32, f32)) {
        self.append(p1.0, p1.1, PointType::Line, true);
        self.append(p2.0, p2.1, PointType::Line, false);
    }

    /// `AddBezier(p1..p4)`: start, two control points, end.
    pub fn add_bezier(&mut self, p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), p4: (f32, f32)) {
        self.append(p1.0, p1.1, PointType::Line, true);
        self.append(p2.0, p2.1, PointType::Bezier, false);
        self.append(p3.0, p3.1, PointType::Bezier, false);
        self.append(p4.0, p4.1, PointType::Bezier, false);
    }

    /// `PathPoints`. Returns the current points.
    pub fn path_points(&self) -> &[(f32, f32)] {
        &self.points
    }

    /// `Flatten()` with default flatness 0.25.
    pub fn flatten(&mut self) {
        self.flatten_with(DEFAULT_FLATNESS);
    }

    /// `Flatten(null, flatness)`: replace beziers by lines. `flatness` is the tolerance.
    pub fn flatten_with(&mut self, flatness: f32) {
        if !self.types.contains(&PointType::Bezier) {
            return;
        }
        let flatness = flatness.abs();
        let mut flat = GraphicsPath::new();
        let mut i = 0;
        while i < self.points.len() {
            let (x, y) = self.points[i];
            if self.types[i] == PointType::Bezier {
                // bezier has 4 points: previous, current and next two
                let ok = i > 0 && i + 2 < self.points.len() && {
                    let s = self.points[i - 1];
                    let f = self.points[i];
                    let se = self.points[i + 1];
                    let e = self.points[i + 2];
                    let saved = flat.points.len();
                    let r = nr_curve_flatten(s.0, s.1, f.0, f.1, se.0, se.1, e.0, e.1, flatness, 0, &mut flat);
                    if !r {
                        flat.points.truncate(saved);
                        flat.types.truncate(saved);
                    }
                    r
                };
                if !ok {
                    // too complex: mimic MS, degenerate rectangle at origin
                    flat = GraphicsPath::new();
                    flat.append(0.0, 0.0, PointType::Start, false);
                    flat.append(0.0, 0.0, PointType::Line, false);
                    flat.append(0.0, 0.0, PointType::Line, false);
                    flat.append(0.0, 0.0, PointType::Line, false);
                    break;
                }
                i += 2;
            } else {
                let t = self.types[i];
                flat.append(x, y, t, false);
            }
            i += 1;
        }
        self.points = flat.points;
        self.types = flat.types;
    }
}

/// libgdiplus `nr_curve_flatten`. Appends line points to `flat`. Returns false on too deep recursion.
#[allow(clippy::too_many_arguments)]
fn nr_curve_flatten(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    x3: f32,
    y3: f32,
    flatness: f32,
    level: i32,
    flat: &mut GraphicsPath,
) -> bool {
    let dx1_0 = x1 - x0;
    let dy1_0 = y1 - y0;
    let dx2_0 = x2 - x0;
    let dy2_0 = y2 - y0;
    let dx3_0 = x3 - x0;
    let dy3_0 = y3 - y0;
    let dx2_3 = x3 - x2;
    let dy2_3 = y3 - y2;
    let f2 = flatness;
    let d3_0_2 = dx3_0 * dx3_0 + dy3_0 * dy3_0;

    // true = subdivide
    let subdivide = if d3_0_2 < f2 {
        let d1_0_2 = dx1_0 * dx1_0 + dy1_0 * dy1_0;
        let d2_0_2 = dx2_0 * dx2_0 + dy2_0 * dy2_0;
        !(d1_0_2 < f2 && d2_0_2 < f2)
    } else {
        let f2_q = f2 * d3_0_2;
        let s1_q = dx1_0 * dx3_0 + dy1_0 * dy3_0;
        let t1_q = dy1_0 * dx3_0 - dx1_0 * dy3_0;
        let s2_q = dx2_0 * dx3_0 + dy2_0 * dy3_0;
        let t2_q = dy2_0 * dx3_0 - dx2_0 * dy3_0;
        let v2_q = dx2_3 * dx3_0 + dy2_3 * dy3_0;
        (t1_q * t1_q) > f2_q
            || (t2_q * t2_q) > f2_q
            || (s1_q < 0.0 && (s1_q * s1_q) > f2_q)
            || (v2_q < 0.0 && (v2_q * v2_q) > f2_q)
            || s1_q >= s2_q
    };

    if !subdivide {
        flat.append(x3, y3, PointType::Line, false);
        return true;
    }
    if level >= FLATTEN_RECURSION_LIMIT {
        return false;
    }

    let x00t = (x0 + x1) * 0.5;
    let y00t = (y0 + y1) * 0.5;
    let x0tt = (x0 + 2.0 * x1 + x2) * 0.25;
    let y0tt = (y0 + 2.0 * y1 + y2) * 0.25;
    let x1tt = (x1 + 2.0 * x2 + x3) * 0.25;
    let y1tt = (y1 + 2.0 * y2 + y3) * 0.25;
    let x11t = (x2 + x3) * 0.5;
    let y11t = (y2 + y3) * 0.5;
    let xttt = (x0tt + x1tt) * 0.5;
    let yttt = (y0tt + y1tt) * 0.5;

    if !nr_curve_flatten(x0, y0, x00t, y00t, x0tt, y0tt, xttt, yttt, flatness, level + 1, flat) {
        return false;
    }
    nr_curve_flatten(xttt, yttt, x1tt, y1tt, x11t, y11t, x3, y3, flatness, level + 1, flat)
}
