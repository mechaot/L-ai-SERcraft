// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2010-2017 Angus Johnson (Clipper)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)
// Clipper parts: Boost Software License 1.0, see NOTICE.md

//! Port of `ClipperOffset` from `Clipper/Clipper.cs`.

use crate::clipper::{Clipper, near_zero};
use crate::polytree::PolyTree;
use crate::types::{
    ClipType, ClipperException, DoublePoint, EndType, IntPoint, JoinType, Path, Paths,
    PolyFillType, PolyType, round,
};

type Res<T> = Result<T, ClipperException>;

const TWO_PI: f64 = std::f64::consts::PI * 2.0;
const DEF_ARC_TOLERANCE: f64 = 0.25;

/// One input path with its join / end style (C# `PolyNode` children of `m_polyNodes`).
#[derive(Debug, Clone)]
struct OffsetNode {
    polygon: Path,
    join_type: JoinType,
    end_type: EndType,
}

/// Path offsetting (`ClipperOffset`).
#[derive(Debug)]
pub struct ClipperOffset {
    dest_polys: Paths,
    src_poly: Path,
    dest_poly: Path,
    normals: Vec<DoublePoint>,
    delta: f64,
    sin_a: f64,
    sin: f64,
    cos: f64,
    miter_lim: f64,
    steps_per_rad: f64,
    lowest: IntPoint,
    poly_nodes: Vec<OffsetNode>,
    /// `ArcTolerance` property.
    pub arc_tolerance: f64,
    /// `MiterLimit` property.
    pub miter_limit: f64,
}

impl Default for ClipperOffset {
    fn default() -> Self {
        Self::new(2.0, DEF_ARC_TOLERANCE)
    }
}

impl ClipperOffset {
    /// Ctor `ClipperOffset(miterLimit, arcTolerance)`.
    ///
    /// `miter_limit` C# default 2.0. `arc_tolerance` C# default 0.25. Returns an empty offsetter.
    pub fn new(miter_limit: f64, arc_tolerance: f64) -> Self {
        ClipperOffset {
            dest_polys: Paths::new(),
            src_poly: Path::new(),
            dest_poly: Path::new(),
            normals: Vec::new(),
            delta: 0.0,
            sin_a: 0.0,
            sin: 0.0,
            cos: 0.0,
            miter_lim: 0.0,
            steps_per_rad: 0.0,
            lowest: IntPoint::new(-1, 0),
            poly_nodes: Vec::new(),
            arc_tolerance,
            miter_limit,
        }
    }

    /// `Clear`: drop all added paths.
    pub fn clear(&mut self) {
        self.poly_nodes.clear();
        self.lowest.x = -1;
    }

    /// `AddPath`. `path` is the input path, `join_type` its corner style, `end_type` its end
    /// style.
    pub fn add_path(&mut self, path: &[IntPoint], join_type: JoinType, end_type: EndType) {
        let mut high_i = path.len() as isize - 1;
        if high_i < 0 {
            return;
        }
        let mut new_node = OffsetNode { polygon: Vec::new(), join_type, end_type };

        // strip duplicate points from path and also get index to the lowest point
        if end_type == EndType::ClosedLine || end_type == EndType::ClosedPolygon {
            while high_i > 0 && path[0] == path[high_i as usize] {
                high_i -= 1;
            }
        }
        let high_i = high_i as usize;
        new_node.polygon.push(path[0]);
        let mut j = 0usize;
        let mut k = 0usize;
        for i in 1..=high_i {
            if new_node.polygon[j] != path[i] {
                j += 1;
                new_node.polygon.push(path[i]);
                if path[i].y > new_node.polygon[k].y
                    || (path[i].y == new_node.polygon[k].y && path[i].x < new_node.polygon[k].x)
                {
                    k = j;
                }
            }
        }
        if end_type == EndType::ClosedPolygon && j < 2 {
            return;
        }

        self.poly_nodes.push(new_node);
        let last = self.poly_nodes.len() - 1;

        // if this path's lowest pt is lower than all the others then update lowest
        if end_type != EndType::ClosedPolygon {
            return;
        }
        let kp = self.poly_nodes[last].polygon[k];
        if self.lowest.x < 0 {
            self.lowest = IntPoint::new(last as i64, k as i64);
        } else {
            let ip = self.poly_nodes[self.lowest.x as usize].polygon[self.lowest.y as usize];
            if kp.y > ip.y || (kp.y == ip.y && kp.x < ip.x) {
                self.lowest = IntPoint::new(last as i64, k as i64);
            }
        }
    }

    /// `AddPaths`. Adds every path of `paths` with the given styles.
    pub fn add_paths(&mut self, paths: &[Path], join_type: JoinType, end_type: EndType) {
        for p in paths {
            self.add_path(p, join_type, end_type);
        }
    }

    /// `FixOrientations`: make closed paths consistent with the lowest one.
    fn fix_orientations(&mut self) {
        // fixup orientations of all closed paths if the orientation of the
        // closed path with the lowermost vertex is wrong
        if self.lowest.x >= 0
            && !Clipper::orientation(&self.poly_nodes[self.lowest.x as usize].polygon)
        {
            for node in &mut self.poly_nodes {
                if node.end_type == EndType::ClosedPolygon
                    || (node.end_type == EndType::ClosedLine && Clipper::orientation(&node.polygon))
                {
                    node.polygon.reverse();
                }
            }
        } else {
            for node in &mut self.poly_nodes {
                if node.end_type == EndType::ClosedLine && !Clipper::orientation(&node.polygon) {
                    node.polygon.reverse();
                }
            }
        }
    }

    /// `GetUnitNormal`. Returns the unit normal of segment pt1-pt2 (zero for a point).
    fn get_unit_normal(pt1: IntPoint, pt2: IntPoint) -> DoublePoint {
        let mut dx = pt2.x.wrapping_sub(pt1.x) as f64;
        let mut dy = pt2.y.wrapping_sub(pt1.y) as f64;
        if dx == 0.0 && dy == 0.0 {
            return DoublePoint::default();
        }

        let f = 1.0 * 1.0 / (dx * dx + dy * dy).sqrt();
        dx *= f;
        dy *= f;

        DoublePoint::new(dy, -dx)
    }

    /// `DoOffset`: build `dest_polys` (raw offset rings) for distance `delta`.
    fn do_offset(&mut self, delta: f64) {
        self.dest_polys = Paths::new();
        self.delta = delta;

        // if zero offset, just copy any CLOSED polygons to dest and return
        if near_zero(delta) {
            for node in &self.poly_nodes {
                if node.end_type == EndType::ClosedPolygon {
                    self.dest_polys.push(node.polygon.clone());
                }
            }
            return;
        }

        // see offset_triginometry3.svg in the documentation folder
        if self.miter_limit > 2.0 {
            self.miter_lim = 2.0 / (self.miter_limit * self.miter_limit);
        } else {
            self.miter_lim = 0.5;
        }

        let y = if self.arc_tolerance <= 0.0 {
            DEF_ARC_TOLERANCE
        } else if self.arc_tolerance > delta.abs() * DEF_ARC_TOLERANCE {
            delta.abs() * DEF_ARC_TOLERANCE
        } else {
            self.arc_tolerance
        };
        // see offset_triginometry2.svg in the documentation folder
        let steps = std::f64::consts::PI / (1.0 - y / delta.abs()).acos();
        self.sin = (TWO_PI / steps).sin();
        self.cos = (TWO_PI / steps).cos();
        self.steps_per_rad = steps / TWO_PI;
        if delta < 0.0 {
            self.sin = -self.sin;
        }

        for ni in 0..self.poly_nodes.len() {
            let (join_type, end_type) = (self.poly_nodes[ni].join_type, self.poly_nodes[ni].end_type);
            self.src_poly = self.poly_nodes[ni].polygon.clone();

            let len = self.src_poly.len();

            if len == 0 || (delta <= 0.0 && (len < 3 || end_type != EndType::ClosedPolygon)) {
                continue;
            }

            self.dest_poly = Path::new();

            if len == 1 {
                if join_type == JoinType::Round {
                    let mut x = 1.0f64;
                    let mut yy = 0.0f64;
                    let mut j = 1;
                    while (j as f64) <= steps {
                        self.dest_poly.push(IntPoint::new(
                            round(self.src_poly[0].x as f64 + x * delta),
                            round(self.src_poly[0].y as f64 + yy * delta),
                        ));
                        let x2 = x;
                        x = x * self.cos - self.sin * yy;
                        yy = x2 * self.sin + yy * self.cos;
                        j += 1;
                    }
                } else {
                    let mut x = -1.0f64;
                    let mut yy = -1.0f64;
                    for _ in 0..4 {
                        self.dest_poly.push(IntPoint::new(
                            round(self.src_poly[0].x as f64 + x * delta),
                            round(self.src_poly[0].y as f64 + yy * delta),
                        ));
                        if x < 0.0 {
                            x = 1.0;
                        } else if yy < 0.0 {
                            yy = 1.0;
                        } else {
                            x = -1.0;
                        }
                    }
                }
                self.dest_polys.push(std::mem::take(&mut self.dest_poly));
                continue;
            }

            // build normals
            self.normals.clear();
            for j in 0..len - 1 {
                self.normals.push(Self::get_unit_normal(self.src_poly[j], self.src_poly[j + 1]));
            }
            if end_type == EndType::ClosedLine || end_type == EndType::ClosedPolygon {
                self.normals.push(Self::get_unit_normal(self.src_poly[len - 1], self.src_poly[0]));
            } else {
                self.normals.push(self.normals[len - 2]);
            }

            if end_type == EndType::ClosedPolygon {
                let mut k = len - 1;
                for j in 0..len {
                    self.offset_point(j, &mut k, join_type);
                }
                self.dest_polys.push(std::mem::take(&mut self.dest_poly));
            } else if end_type == EndType::ClosedLine {
                let mut k = len - 1;
                for j in 0..len {
                    self.offset_point(j, &mut k, join_type);
                }
                self.dest_polys.push(std::mem::take(&mut self.dest_poly));
                self.dest_poly = Path::new();
                // re-build normals
                let n = self.normals[len - 1];
                for j in (1..len).rev() {
                    self.normals[j] = DoublePoint::new(-self.normals[j - 1].x, -self.normals[j - 1].y);
                }
                self.normals[0] = DoublePoint::new(-n.x, -n.y);
                k = 0;
                for j in (0..len).rev() {
                    self.offset_point(j, &mut k, join_type);
                }
                self.dest_polys.push(std::mem::take(&mut self.dest_poly));
            } else {
                let mut k = 0usize;
                for j in 1..len - 1 {
                    self.offset_point(j, &mut k, join_type);
                }

                if end_type == EndType::OpenButt {
                    let j = len - 1;
                    let pt1 = IntPoint::new(
                        round(self.src_poly[j].x as f64 + self.normals[j].x * delta),
                        round(self.src_poly[j].y as f64 + self.normals[j].y * delta),
                    );
                    self.dest_poly.push(pt1);
                    let pt1 = IntPoint::new(
                        round(self.src_poly[j].x as f64 - self.normals[j].x * delta),
                        round(self.src_poly[j].y as f64 - self.normals[j].y * delta),
                    );
                    self.dest_poly.push(pt1);
                } else {
                    let j = len - 1;
                    k = len - 2;
                    self.sin_a = 0.0;
                    self.normals[j] = DoublePoint::new(-self.normals[j].x, -self.normals[j].y);
                    if end_type == EndType::OpenSquare {
                        self.do_square(j, k);
                    } else {
                        self.do_round(j, k);
                    }
                }

                // re-build normals
                for j in (1..len).rev() {
                    self.normals[j] = DoublePoint::new(-self.normals[j - 1].x, -self.normals[j - 1].y);
                }

                self.normals[0] = DoublePoint::new(-self.normals[1].x, -self.normals[1].y);

                k = len - 1;
                for j in (1..k).rev() {
                    self.offset_point(j, &mut k, join_type);
                }

                if end_type == EndType::OpenButt {
                    let pt1 = IntPoint::new(
                        round(self.src_poly[0].x as f64 - self.normals[0].x * delta),
                        round(self.src_poly[0].y as f64 - self.normals[0].y * delta),
                    );
                    self.dest_poly.push(pt1);
                    let pt1 = IntPoint::new(
                        round(self.src_poly[0].x as f64 + self.normals[0].x * delta),
                        round(self.src_poly[0].y as f64 + self.normals[0].y * delta),
                    );
                    self.dest_poly.push(pt1);
                } else {
                    self.sin_a = 0.0;
                    if end_type == EndType::OpenSquare {
                        self.do_square(0, 1);
                    } else {
                        self.do_round(0, 1);
                    }
                }
                self.dest_polys.push(std::mem::take(&mut self.dest_poly));
            }
        }
    }

    /// Outer rectangle used to turn a negative offset into a hole search (`Execute` tail).
    fn outer_rect(&self) -> Path {
        let r = Clipper::get_bounds(&self.dest_polys);
        vec![
            IntPoint::new(r.left - 10, r.bottom + 10),
            IntPoint::new(r.right + 10, r.bottom + 10),
            IntPoint::new(r.right + 10, r.top - 10),
            IntPoint::new(r.left - 10, r.top - 10),
        ]
    }

    /// `Execute(ref Paths solution, double delta)`.
    ///
    /// `solution` is cleared and receives the offset polygons. `delta` is the offset (units;
    /// negative shrinks). Returns Err only for library exceptions (coordinate range).
    pub fn execute(&mut self, solution: &mut Paths, delta: f64) -> Res<()> {
        solution.clear();
        self.fix_orientations();
        self.do_offset(delta);
        // now clean up 'corners'
        let mut clpr = Clipper::new(0);
        clpr.add_paths(&self.dest_polys, PolyType::Subject, true)?;
        if delta > 0.0 {
            clpr.execute_fill(ClipType::Union, solution, PolyFillType::Positive, PolyFillType::Positive)?;
        } else {
            let outer = self.outer_rect();

            clpr.add_path(&outer, PolyType::Subject, true)?;
            clpr.reverse_solution = true;
            clpr.execute_fill(ClipType::Union, solution, PolyFillType::Negative, PolyFillType::Negative)?;
            if !solution.is_empty() {
                solution.remove(0);
            }
        }
        Ok(())
    }

    /// `Execute(ref PolyTree solution, double delta)`.
    ///
    /// `solution` is cleared and receives the offset tree. `delta` is the offset (units).
    /// Returns Err only for library exceptions (coordinate range).
    pub fn execute_tree(&mut self, solution: &mut PolyTree, delta: f64) -> Res<()> {
        solution.clear();
        self.fix_orientations();
        self.do_offset(delta);

        // now clean up 'corners'
        let mut clpr = Clipper::new(0);
        clpr.add_paths(&self.dest_polys, PolyType::Subject, true)?;
        if delta > 0.0 {
            clpr.execute_tree_fill(
                ClipType::Union,
                solution,
                PolyFillType::Positive,
                PolyFillType::Positive,
            )?;
        } else {
            let outer = self.outer_rect();

            clpr.add_path(&outer, PolyType::Subject, true)?;
            clpr.reverse_solution = true;
            clpr.execute_tree_fill(
                ClipType::Union,
                solution,
                PolyFillType::Negative,
                PolyFillType::Negative,
            )?;
            // remove the outer PolyNode rectangle
            if solution.child_count() == 1 && solution.node(solution.childs()[0]).child_count() > 0 {
                let outer_node = solution.childs()[0];
                let outer_childs: Vec<_> = solution.node(outer_node).childs().to_vec();
                solution.nodes[PolyTree::ROOT].childs[0] = outer_childs[0];
                solution.nodes[outer_childs[0]].parent = Some(PolyTree::ROOT);
                for &c in &outer_childs[1..] {
                    solution.add_child(PolyTree::ROOT, c);
                }
            } else {
                solution.clear();
            }
        }
        Ok(())
    }

    /// `OffsetPoint`: emit the offset vertices for corner `j` (previous corner `k`, updated).
    fn offset_point(&mut self, j: usize, k: &mut usize, jointype: JoinType) {
        // cross product
        self.sin_a = self.normals[*k].x * self.normals[j].y - self.normals[j].x * self.normals[*k].y;

        if (self.sin_a * self.delta).abs() < 1.0 {
            // dot product
            let cos_a = self.normals[*k].x * self.normals[j].x + self.normals[j].y * self.normals[*k].y;
            if cos_a > 0.0 {
                // angle ==> 0 degrees
                self.dest_poly.push(IntPoint::new(
                    round(self.src_poly[j].x as f64 + self.normals[*k].x * self.delta),
                    round(self.src_poly[j].y as f64 + self.normals[*k].y * self.delta),
                ));
                return;
            }
            // else angle ==> 180 degrees
        } else if self.sin_a > 1.0 {
            self.sin_a = 1.0;
        } else if self.sin_a < -1.0 {
            self.sin_a = -1.0;
        }

        if self.sin_a * self.delta < 0.0 {
            self.dest_poly.push(IntPoint::new(
                round(self.src_poly[j].x as f64 + self.normals[*k].x * self.delta),
                round(self.src_poly[j].y as f64 + self.normals[*k].y * self.delta),
            ));
            self.dest_poly.push(self.src_poly[j]);
            self.dest_poly.push(IntPoint::new(
                round(self.src_poly[j].x as f64 + self.normals[j].x * self.delta),
                round(self.src_poly[j].y as f64 + self.normals[j].y * self.delta),
            ));
        } else {
            match jointype {
                JoinType::Miter => {
                    let r = 1.0
                        + (self.normals[j].x * self.normals[*k].x
                            + self.normals[j].y * self.normals[*k].y);
                    if r >= self.miter_lim {
                        self.do_miter(j, *k, r);
                    } else {
                        self.do_square(j, *k);
                    }
                }
                JoinType::Square => self.do_square(j, *k),
                JoinType::Round => self.do_round(j, *k),
            }
        }
        *k = j;
    }

    /// `DoSquare`: squared corner at `j` from previous corner `k`.
    fn do_square(&mut self, j: usize, k: usize) {
        let (nk, nj) = (self.normals[k], self.normals[j]);
        let dx = (self.sin_a.atan2(nk.x * nj.x + nk.y * nj.y) / 4.0).tan();
        self.dest_poly.push(IntPoint::new(
            round(self.src_poly[j].x as f64 + self.delta * (nk.x - nk.y * dx)),
            round(self.src_poly[j].y as f64 + self.delta * (nk.y + nk.x * dx)),
        ));
        self.dest_poly.push(IntPoint::new(
            round(self.src_poly[j].x as f64 + self.delta * (nj.x + nj.y * dx)),
            round(self.src_poly[j].y as f64 + self.delta * (nj.y - nj.x * dx)),
        ));
    }

    /// `DoMiter`: mitered corner at `j`; `r` is 1 + dot of the normals.
    fn do_miter(&mut self, j: usize, k: usize, r: f64) {
        let q = self.delta / r;
        let (nk, nj) = (self.normals[k], self.normals[j]);
        self.dest_poly.push(IntPoint::new(
            round(self.src_poly[j].x as f64 + (nk.x + nj.x) * q),
            round(self.src_poly[j].y as f64 + (nk.y + nj.y) * q),
        ));
    }

    /// `DoRound`: arc corner at `j` from previous corner `k`.
    fn do_round(&mut self, j: usize, k: usize) {
        let (nk, nj) = (self.normals[k], self.normals[j]);
        let a = self.sin_a.atan2(nk.x * nj.x + nk.y * nj.y);
        // (int)Round(..): C# long -> int truncation
        let steps = (round(self.steps_per_rad * a.abs()) as i32).max(1);

        let mut x = nk.x;
        let mut y = nk.y;
        for _ in 0..steps {
            self.dest_poly.push(IntPoint::new(
                round(self.src_poly[j].x as f64 + x * self.delta),
                round(self.src_poly[j].y as f64 + y * self.delta),
            ));
            let x2 = x;
            x = x * self.cos - self.sin * y;
            y = x2 * self.sin + y * self.cos;
        }
        self.dest_poly.push(IntPoint::new(
            round(self.src_poly[j].x as f64 + nj.x * self.delta),
            round(self.src_poly[j].y as f64 + nj.y * self.delta),
        ));
    }
}
