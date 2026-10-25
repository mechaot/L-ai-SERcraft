// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2010-2017 Angus Johnson (Clipper)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)
// Clipper parts: Boost Software License 1.0, see NOTICE.md

//! Port of `ClipperBase` + `Clipper` from `Clipper/Clipper.cs` (Clipper 6.4.2, `use_lines`).
//! C# linked object graph (TEdge / OutRec / OutPt / LocalMinima / Maxima) is held in index arenas.
//! `NIL` is the C# `null`. Traversal order is kept identical to the C# code.

use crate::polytree::{NodeId, PolyTree};
use crate::sort::list_sort;
use crate::types::{
    CInt, ClipType, ClipperException, HI_RANGE, Int128, IntPoint, IntRect, LO_RANGE, Path, Paths,
    PolyFillType, PolyType, round,
};

type Res<T> = Result<T, ClipperException>;

/// C# `null` for arena links.
const NIL: usize = usize::MAX;
const HORIZONTAL: f64 = -3.4E+38;
const SKIP: i32 = -2;
const UNASSIGNED: i32 = -1;
const TOLERANCE: f64 = 1.0E-20;

/// `ClipperBase.near_zero`. `val` is tested against +-1e-20.
pub(crate) fn near_zero(val: f64) -> bool {
    val > -TOLERANCE && val < TOLERANCE
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    RightToLeft,
    LeftToRight,
}

/// `TEdge`.
#[derive(Debug, Clone)]
struct TEdge {
    bot: IntPoint,
    curr: IntPoint,
    top: IntPoint,
    delta: IntPoint,
    dx: f64,
    poly_typ: PolyType,
    side: EdgeSide,
    wind_delta: i32,
    wind_cnt: i32,
    wind_cnt2: i32,
    out_idx: i32,
    next: usize,
    prev: usize,
    next_in_lml: usize,
    next_in_ael: usize,
    prev_in_ael: usize,
    next_in_sel: usize,
    prev_in_sel: usize,
}

impl TEdge {
    fn new() -> Self {
        TEdge {
            bot: IntPoint::default(),
            curr: IntPoint::default(),
            top: IntPoint::default(),
            delta: IntPoint::default(),
            dx: 0.0,
            poly_typ: PolyType::Subject,
            side: EdgeSide::Left,
            wind_delta: 0,
            wind_cnt: 0,
            wind_cnt2: 0,
            out_idx: 0,
            next: NIL,
            prev: NIL,
            next_in_lml: NIL,
            next_in_ael: NIL,
            prev_in_ael: NIL,
            next_in_sel: NIL,
            prev_in_sel: NIL,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct IntersectNode {
    edge1: usize,
    edge2: usize,
    pt: IntPoint,
}

/// `LocalMinima` (singly linked, sorted by Y descending).
#[derive(Debug, Clone)]
struct LocalMinima {
    y: CInt,
    left_bound: usize,
    right_bound: usize,
    next: usize,
}

/// `Maxima` (doubly linked, sorted by X ascending).
#[derive(Debug, Clone)]
struct Maxima {
    x: CInt,
    next: usize,
    prev: usize,
}

/// `OutRec`.
#[derive(Debug, Clone)]
struct OutRec {
    idx: i32,
    is_hole: bool,
    is_open: bool,
    first_left: usize,
    pts: usize,
    bottom_pt: usize,
    poly_node: Option<NodeId>,
}

/// `OutPt`.
#[derive(Debug, Clone)]
struct OutPt {
    idx: i32,
    pt: IntPoint,
    next: usize,
    prev: usize,
}

/// `Join`.
#[derive(Debug, Clone)]
struct Join {
    out_pt1: usize,
    out_pt2: usize,
    off_pt: IntPoint,
}

/// Slopes of two edges equal (`SlopesEqual(TEdge, TEdge)`).
fn slopes_equal_edges(e1: &TEdge, e2: &TEdge, use_full_range: bool) -> bool {
    if use_full_range {
        Int128::mul(e1.delta.y, e2.delta.x) == Int128::mul(e1.delta.x, e2.delta.y)
    } else {
        e1.delta.y.wrapping_mul(e2.delta.x) == e1.delta.x.wrapping_mul(e2.delta.y)
    }
}

/// Three points collinear (`SlopesEqual(pt1, pt2, pt3)`).
fn slopes_equal3(pt1: IntPoint, pt2: IntPoint, pt3: IntPoint, use_full_range: bool) -> bool {
    if use_full_range {
        Int128::mul(pt1.y.wrapping_sub(pt2.y), pt2.x.wrapping_sub(pt3.x))
            == Int128::mul(pt1.x.wrapping_sub(pt2.x), pt2.y.wrapping_sub(pt3.y))
    } else {
        pt1.y
            .wrapping_sub(pt2.y)
            .wrapping_mul(pt2.x.wrapping_sub(pt3.x))
            .wrapping_sub(pt1.x.wrapping_sub(pt2.x).wrapping_mul(pt2.y.wrapping_sub(pt3.y)))
            == 0
    }
}

/// Two segments parallel (`SlopesEqual(pt1, pt2, pt3, pt4)`).
fn slopes_equal4(
    pt1: IntPoint,
    pt2: IntPoint,
    pt3: IntPoint,
    pt4: IntPoint,
    use_full_range: bool,
) -> bool {
    if use_full_range {
        Int128::mul(pt1.y.wrapping_sub(pt2.y), pt3.x.wrapping_sub(pt4.x))
            == Int128::mul(pt1.x.wrapping_sub(pt2.x), pt3.y.wrapping_sub(pt4.y))
    } else {
        pt1.y
            .wrapping_sub(pt2.y)
            .wrapping_mul(pt3.x.wrapping_sub(pt4.x))
            .wrapping_sub(pt1.x.wrapping_sub(pt2.x).wrapping_mul(pt3.y.wrapping_sub(pt4.y)))
            == 0
    }
}

/// `Pt2IsBetweenPt1AndPt3`.
fn pt2_is_between_pt1_and_pt3(pt1: IntPoint, pt2: IntPoint, pt3: IntPoint) -> bool {
    if pt1 == pt3 || pt1 == pt2 || pt3 == pt2 {
        false
    } else if pt1.x != pt3.x {
        (pt2.x > pt1.x) == (pt2.x < pt3.x)
    } else {
        (pt2.y > pt1.y) == (pt2.y < pt3.y)
    }
}

/// `TopX`: X of `edge` at height `current_y`.
fn top_x(edge: &TEdge, current_y: CInt) -> CInt {
    if current_y == edge.top.y {
        return edge.top.x;
    }
    edge.bot.x + round(edge.dx * (current_y.wrapping_sub(edge.bot.y)) as f64)
}

/// `IntersectPoint`: where two edges cross, clamped into the scanbeam.
fn intersect_point(edge1: &TEdge, edge2: &TEdge) -> IntPoint {
    let mut ip = IntPoint::default();
    // nb: with very large coords SlopesEqual may be false while Dx are equal
    if edge1.dx == edge2.dx {
        ip.y = edge1.curr.y;
        ip.x = top_x(edge1, ip.y);
        return ip;
    }

    if edge1.delta.x == 0 {
        ip.x = edge1.bot.x;
        if edge2.delta.y == 0 {
            ip.y = edge2.bot.y;
        } else {
            let b2 = edge2.bot.y as f64 - (edge2.bot.x as f64 / edge2.dx);
            ip.y = round(ip.x as f64 / edge2.dx + b2);
        }
    } else if edge2.delta.x == 0 {
        ip.x = edge2.bot.x;
        if edge1.delta.y == 0 {
            ip.y = edge1.bot.y;
        } else {
            let b1 = edge1.bot.y as f64 - (edge1.bot.x as f64 / edge1.dx);
            ip.y = round(ip.x as f64 / edge1.dx + b1);
        }
    } else {
        let b1 = edge1.bot.x as f64 - edge1.bot.y as f64 * edge1.dx;
        let b2 = edge2.bot.x as f64 - edge2.bot.y as f64 * edge2.dx;
        let q = (b2 - b1) / (edge1.dx - edge2.dx);
        ip.y = round(q);
        if edge1.dx.abs() < edge2.dx.abs() {
            ip.x = round(edge1.dx * q + b1);
        } else {
            ip.x = round(edge2.dx * q + b2);
        }
    }

    if ip.y < edge1.top.y || ip.y < edge2.top.y {
        if edge1.top.y > edge2.top.y {
            ip.y = edge1.top.y;
        } else {
            ip.y = edge2.top.y;
        }
        if edge1.dx.abs() < edge2.dx.abs() {
            ip.x = top_x(edge1, ip.y);
        } else {
            ip.x = top_x(edge2, ip.y);
        }
    }
    // finally, don't allow ip BELOW curr.y (bottom of scanbeam)
    if ip.y > edge1.curr.y {
        ip.y = edge1.curr.y;
        // better to use the more vertical edge to derive X
        if edge1.dx.abs() > edge2.dx.abs() {
            ip.x = top_x(edge2, ip.y);
        } else {
            ip.x = top_x(edge1, ip.y);
        }
    }
    ip
}

/// `Area(OutPt)` over a ring in `pts`. `op` is the start (NIL gives 0). Returns signed area.
fn area_ring(pts: &[OutPt], op: usize) -> f64 {
    if op == NIL {
        return 0.0;
    }
    let op_first = op;
    let mut op = op;
    let mut a = 0.0;
    loop {
        let prev = &pts[pts[op].prev];
        let cur = &pts[op];
        a += (prev.pt.x.wrapping_add(cur.pt.x)) as f64 * (prev.pt.y.wrapping_sub(cur.pt.y)) as f64;
        op = cur.next;
        if op == op_first {
            break;
        }
    }
    a * 0.5
}

/// `PointInPolygon(IntPoint, OutPt)`: 0 outside, +1 inside, -1 on boundary.
fn point_in_ring(pt: IntPoint, pts: &[OutPt], op: usize) -> i32 {
    let mut result = 0;
    let start_op = op;
    let mut op = op;
    let ptx = pt.x;
    let pty = pt.y;
    let mut poly0x = pts[op].pt.x;
    let mut poly0y = pts[op].pt.y;
    loop {
        op = pts[op].next;
        let poly1x = pts[op].pt.x;
        let poly1y = pts[op].pt.y;

        if poly1y == pty && (poly1x == ptx || (poly0y == pty && ((poly1x > ptx) == (poly0x < ptx))))
        {
            return -1;
        }
        if (poly0y < pty) != (poly1y < pty) {
            if poly0x >= ptx {
                if poly1x > ptx {
                    result = 1 - result;
                } else {
                    let d = (poly0x.wrapping_sub(ptx)) as f64 * (poly1y.wrapping_sub(pty)) as f64
                        - (poly1x.wrapping_sub(ptx)) as f64 * (poly0y.wrapping_sub(pty)) as f64;
                    if d == 0.0 {
                        return -1;
                    }
                    if (d > 0.0) == (poly1y > poly0y) {
                        result = 1 - result;
                    }
                }
            } else if poly1x > ptx {
                let d = (poly0x.wrapping_sub(ptx)) as f64 * (poly1y.wrapping_sub(pty)) as f64
                    - (poly1x.wrapping_sub(ptx)) as f64 * (poly0y.wrapping_sub(pty)) as f64;
                if d == 0.0 {
                    return -1;
                }
                if (d > 0.0) == (poly1y > poly0y) {
                    result = 1 - result;
                }
            }
        }
        poly0x = poly1x;
        poly0y = poly1y;
        if start_op == op {
            break;
        }
    }
    result
}

/// `Poly2ContainsPoly1`: is ring `out_pt1` inside ring `out_pt2`?
fn poly2_contains_poly1(pts: &[OutPt], out_pt1: usize, out_pt2: usize) -> bool {
    let mut op = out_pt1;
    loop {
        // nb: PointInPolygon returns 0 if false, +1 if true, -1 if pt on polygon
        let res = point_in_ring(pts[op].pt, pts, out_pt2);
        if res >= 0 {
            return res > 0;
        }
        op = pts[op].next;
        if op == out_pt1 {
            break;
        }
    }
    true
}

/// `Clipper` (+ `ClipperBase`): polygon clipping engine.
#[derive(Debug)]
pub struct Clipper {
    // ClipperBase state
    edges: Vec<TEdge>,
    minima: Vec<LocalMinima>,
    minima_list: usize,
    current_lm: usize,
    scanbeam: Vec<CInt>, // ascending, unique; last = top of C# list
    poly_outs: Vec<OutRec>,
    out_pts: Vec<OutPt>,
    active_edges: usize,
    use_full_range: bool,
    has_open_paths: bool,
    /// `PreserveCollinear` property.
    pub preserve_collinear: bool,
    // Clipper state
    clip_type: ClipType,
    maxima: Vec<Maxima>,
    maxima_head: usize,
    sorted_edges: usize,
    intersect_list: Vec<IntersectNode>,
    intersect_capacity: usize, // List<T> backing array length, drives sort depth limit
    execute_locked: bool,
    clip_fill_type: PolyFillType,
    subj_fill_type: PolyFillType,
    joins: Vec<Join>,
    ghost_joins: Vec<Join>,
    using_poly_tree: bool,
    /// `ReverseSolution` property.
    pub reverse_solution: bool,
    /// `StrictlySimple` property.
    pub strictly_simple: bool,
}

/// `Clipper.ioReverseSolution`.
pub const IO_REVERSE_SOLUTION: i32 = 1;
/// `Clipper.ioStrictlySimple`.
pub const IO_STRICTLY_SIMPLE: i32 = 2;
/// `Clipper.ioPreserveCollinear`.
pub const IO_PRESERVE_COLLINEAR: i32 = 4;

impl Default for Clipper {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Clipper {
    /// Ctor `Clipper(int InitOptions)`.
    ///
    /// `init_options` is an OR of `IO_*` flags. Returns an empty clipper.
    pub fn new(init_options: i32) -> Self {
        Clipper {
            edges: Vec::new(),
            minima: Vec::new(),
            minima_list: NIL,
            current_lm: NIL,
            scanbeam: Vec::new(),
            poly_outs: Vec::new(),
            out_pts: Vec::new(),
            active_edges: NIL,
            use_full_range: false,
            has_open_paths: false,
            preserve_collinear: (IO_PRESERVE_COLLINEAR & init_options) != 0,
            clip_type: ClipType::Intersection,
            maxima: Vec::new(),
            maxima_head: NIL,
            sorted_edges: NIL,
            intersect_list: Vec::new(),
            intersect_capacity: 0,
            execute_locked: false,
            clip_fill_type: PolyFillType::EvenOdd,
            subj_fill_type: PolyFillType::EvenOdd,
            joins: Vec::new(),
            ghost_joins: Vec::new(),
            using_poly_tree: false,
            reverse_solution: (IO_REVERSE_SOLUTION & init_options) != 0,
            strictly_simple: (IO_STRICTLY_SIMPLE & init_options) != 0,
        }
    }

    // ---- small link helpers ----

    fn nx(&self, e: usize) -> usize {
        self.edges[e].next
    }

    fn pv(&self, e: usize) -> usize {
        self.edges[e].prev
    }

    // ---- ClipperBase ----

    /// `Clear`: drop all added paths. Returns nothing.
    pub fn clear(&mut self) {
        self.minima.clear();
        self.minima_list = NIL;
        self.current_lm = NIL;
        self.edges.clear();
        self.use_full_range = false;
        self.has_open_paths = false;
    }

    /// `RangeTest`: flips to full range mode / fails when `pt` is too big.
    fn range_test(&mut self, pt: IntPoint) -> Res<()> {
        if self.use_full_range {
            if pt.x > HI_RANGE
                || pt.y > HI_RANGE
                || pt.x.wrapping_neg() > HI_RANGE
                || pt.y.wrapping_neg() > HI_RANGE
            {
                return Err(ClipperException("Coordinate outside allowed range".into()));
            }
            Ok(())
        } else if pt.x > LO_RANGE
            || pt.y > LO_RANGE
            || pt.x.wrapping_neg() > LO_RANGE
            || pt.y.wrapping_neg() > LO_RANGE
        {
            self.use_full_range = true;
            self.range_test(pt)
        } else {
            Ok(())
        }
    }

    /// `InitEdge`: set links, current point and out index.
    fn init_edge(&mut self, e: usize, e_next: usize, e_prev: usize, pt: IntPoint) {
        let ed = &mut self.edges[e];
        ed.next = e_next;
        ed.prev = e_prev;
        ed.curr = pt;
        ed.out_idx = UNASSIGNED;
    }

    /// `InitEdge2`: set bot / top / dx / poly type.
    fn init_edge2(&mut self, e: usize, poly_type: PolyType) {
        let next_curr = self.edges[self.nx(e)].curr;
        let ed = &mut self.edges[e];
        if ed.curr.y >= next_curr.y {
            ed.bot = ed.curr;
            ed.top = next_curr;
        } else {
            ed.top = ed.curr;
            ed.bot = next_curr;
        }
        self.set_dx(e);
        self.edges[e].poly_typ = poly_type;
    }

    /// `FindNextLocMin`. `e` is the start edge. Returns edge at next local minimum.
    fn find_next_loc_min(&self, mut e: usize) -> usize {
        loop {
            while self.edges[e].bot != self.edges[self.pv(e)].bot
                || self.edges[e].curr == self.edges[e].top
            {
                e = self.nx(e);
            }
            if self.edges[e].dx != HORIZONTAL && self.edges[self.pv(e)].dx != HORIZONTAL {
                break;
            }
            while self.edges[self.pv(e)].dx == HORIZONTAL {
                e = self.pv(e);
            }
            let e2 = e;
            while self.edges[e].dx == HORIZONTAL {
                e = self.nx(e);
            }
            if self.edges[e].top.y == self.edges[self.pv(e)].bot.y {
                continue; // just an intermediate horz
            }
            if self.edges[self.pv(e2)].bot.x < self.edges[e].bot.x {
                e = e2;
            }
            break;
        }
        e
    }

    /// `ProcessBound`. `e` is the first edge of the bound; `left_bound_is_forward` its direction.
    /// Returns the edge just beyond the bound.
    fn process_bound(&mut self, mut e: usize, left_bound_is_forward: bool) -> usize {
        let mut result = e;

        if self.edges[result].out_idx == SKIP {
            // check for edges beyond the skip edge in the bound; if so create another LocMin
            e = result;
            if left_bound_is_forward {
                while self.edges[e].top.y == self.edges[self.nx(e)].bot.y {
                    e = self.nx(e);
                }
                while e != result && self.edges[e].dx == HORIZONTAL {
                    e = self.pv(e);
                }
            } else {
                while self.edges[e].top.y == self.edges[self.pv(e)].bot.y {
                    e = self.pv(e);
                }
                while e != result && self.edges[e].dx == HORIZONTAL {
                    e = self.nx(e);
                }
            }
            if e == result {
                result = if left_bound_is_forward { self.nx(e) } else { self.pv(e) };
            } else {
                // more edges in the bound beyond result starting with e
                e = if left_bound_is_forward { self.nx(result) } else { self.pv(result) };
                let y = self.edges[e].bot.y;
                let loc_min = self.new_lm(y, NIL, e);
                self.edges[e].wind_delta = 0;
                result = self.process_bound(e, left_bound_is_forward);
                self.insert_local_minima(loc_min);
            }
            return result;
        }

        if self.edges[e].dx == HORIZONTAL {
            // careful with open paths: may not be a true local minima (following a skip edge)
            let e_start = if left_bound_is_forward { self.pv(e) } else { self.nx(e) };
            if self.edges[e_start].dx == HORIZONTAL {
                // adjoining horizontal skip edge
                if self.edges[e_start].bot.x != self.edges[e].bot.x
                    && self.edges[e_start].top.x != self.edges[e].bot.x
                {
                    self.reverse_horizontal(e);
                }
            } else if self.edges[e_start].bot.x != self.edges[e].bot.x {
                self.reverse_horizontal(e);
            }
        }

        let e_start = e;
        if left_bound_is_forward {
            while self.edges[result].top.y == self.edges[self.nx(result)].bot.y
                && self.edges[self.nx(result)].out_idx != SKIP
            {
                result = self.nx(result);
            }
            if self.edges[result].dx == HORIZONTAL
                && self.edges[self.nx(result)].out_idx != SKIP
            {
                // at the top of a bound, horizontals join the bound only when the preceding
                // edge attaches to the horizontal's left vertex (unless a Skip edge)
                let mut horz = result;
                while self.edges[self.pv(horz)].dx == HORIZONTAL {
                    horz = self.pv(horz);
                }
                if self.edges[self.pv(horz)].top.x > self.edges[self.nx(result)].top.x {
                    result = self.pv(horz);
                }
            }
            while e != result {
                self.edges[e].next_in_lml = self.nx(e);
                if self.edges[e].dx == HORIZONTAL
                    && e != e_start
                    && self.edges[e].bot.x != self.edges[self.pv(e)].top.x
                {
                    self.reverse_horizontal(e);
                }
                e = self.nx(e);
            }
            if self.edges[e].dx == HORIZONTAL
                && e != e_start
                && self.edges[e].bot.x != self.edges[self.pv(e)].top.x
            {
                self.reverse_horizontal(e);
            }
            result = self.nx(result); // move to the edge just beyond current bound
        } else {
            while self.edges[result].top.y == self.edges[self.pv(result)].bot.y
                && self.edges[self.pv(result)].out_idx != SKIP
            {
                result = self.pv(result);
            }
            if self.edges[result].dx == HORIZONTAL
                && self.edges[self.pv(result)].out_idx != SKIP
            {
                let mut horz = result;
                while self.edges[self.nx(horz)].dx == HORIZONTAL {
                    horz = self.nx(horz);
                }
                if self.edges[self.nx(horz)].top.x == self.edges[self.pv(result)].top.x
                    || self.edges[self.nx(horz)].top.x > self.edges[self.pv(result)].top.x
                {
                    result = self.nx(horz);
                }
            }

            while e != result {
                self.edges[e].next_in_lml = self.pv(e);
                if self.edges[e].dx == HORIZONTAL
                    && e != e_start
                    && self.edges[e].bot.x != self.edges[self.nx(e)].top.x
                {
                    self.reverse_horizontal(e);
                }
                e = self.pv(e);
            }
            if self.edges[e].dx == HORIZONTAL
                && e != e_start
                && self.edges[e].bot.x != self.edges[self.nx(e)].top.x
            {
                self.reverse_horizontal(e);
            }
            result = self.pv(result); // move to the edge just beyond current bound
        }
        result
    }

    /// `AddPath`.
    ///
    /// `pg` is the path. `poly_type` subject or clip (open paths must be subject). `closed` tells
    /// polygon vs polyline. Returns Ok(true) when something was added, Err on bad input.
    pub fn add_path(&mut self, pg: &[IntPoint], poly_type: PolyType, closed: bool) -> Res<bool> {
        if !closed && poly_type == PolyType::Clip {
            return Err(ClipperException("AddPath: Open paths must be subject.".into()));
        }

        let mut high_i = pg.len() as isize - 1;
        if closed {
            while high_i > 0 && pg[high_i as usize] == pg[0] {
                high_i -= 1;
            }
        }
        while high_i > 0 && pg[high_i as usize] == pg[(high_i - 1) as usize] {
            high_i -= 1;
        }
        if (closed && high_i < 2) || (!closed && high_i < 1) {
            return Ok(false);
        }
        let high_i = high_i as usize;

        // create a new edge array (arena slice base..base+high_i+1)
        let base = self.edges.len();
        for _ in 0..=high_i {
            self.edges.push(TEdge::new());
        }
        let ed = |i: usize| base + i;

        let mut is_flat = true;

        // 1. basic (first) edge initialization
        self.edges[ed(1)].curr = pg[1];
        if let Err(err) = self
            .range_test(pg[0])
            .and_then(|_| self.range_test(pg[high_i]))
        {
            self.edges.truncate(base);
            return Err(err);
        }
        self.init_edge(ed(0), ed(1), ed(high_i), pg[0]);
        self.init_edge(ed(high_i), ed(0), ed(high_i - 1), pg[high_i]);
        for i in (1..high_i).rev() {
            if let Err(err) = self.range_test(pg[i]) {
                self.edges.truncate(base);
                return Err(err);
            }
            self.init_edge(ed(i), ed(i + 1), ed(i - 1), pg[i]);
        }
        let mut e_start = ed(0);

        // 2. remove duplicate vertices, and (when closed) collinear edges
        let mut e = e_start;
        let mut e_loop_stop = e_start;
        loop {
            // nb: allows matching start and end points when not closed
            if self.edges[e].curr == self.edges[self.nx(e)].curr
                && (closed || self.nx(e) != e_start)
            {
                if e == self.nx(e) {
                    break;
                }
                if e == e_start {
                    e_start = self.nx(e);
                }
                e = self.remove_edge(e);
                e_loop_stop = e;
                continue;
            }
            if self.pv(e) == self.nx(e) {
                break; // only two vertices
            } else if closed
                && slopes_equal3(
                    self.edges[self.pv(e)].curr,
                    self.edges[e].curr,
                    self.edges[self.nx(e)].curr,
                    self.use_full_range,
                )
                && (!self.preserve_collinear
                    || !pt2_is_between_pt1_and_pt3(
                        self.edges[self.pv(e)].curr,
                        self.edges[e].curr,
                        self.edges[self.nx(e)].curr,
                    ))
            {
                // collinear edges are merged in closed paths (unless PreserveCollinear
                // and not a spike)
                if e == e_start {
                    e_start = self.nx(e);
                }
                e = self.remove_edge(e);
                e = self.pv(e);
                e_loop_stop = e;
                continue;
            }
            e = self.nx(e);
            if e == e_loop_stop || (!closed && self.nx(e) == e_start) {
                break;
            }
        }

        if (!closed && e == self.nx(e)) || (closed && self.pv(e) == self.nx(e)) {
            self.edges.truncate(base);
            return Ok(false);
        }

        if !closed {
            self.has_open_paths = true;
            let p = self.pv(e_start);
            self.edges[p].out_idx = SKIP;
        }

        // 3. second stage of edge initialization
        e = e_start;
        loop {
            self.init_edge2(e, poly_type);
            e = self.nx(e);
            if is_flat && self.edges[e].curr.y != self.edges[e_start].curr.y {
                is_flat = false;
            }
            if e == e_start {
                break;
            }
        }

        // 4. finally, add edge bounds to LocalMinima list
        // totally flat paths are handled differently to avoid endless loops
        if is_flat {
            if closed {
                self.edges.truncate(base);
                return Ok(false);
            }
            let p = self.pv(e);
            self.edges[p].out_idx = SKIP;
            let y = self.edges[e].bot.y;
            let loc_min = self.new_lm(y, NIL, e);
            self.edges[e].side = EdgeSide::Right;
            self.edges[e].wind_delta = 0;
            loop {
                if self.edges[e].bot.x != self.edges[self.pv(e)].top.x {
                    self.reverse_horizontal(e);
                }
                if self.edges[self.nx(e)].out_idx == SKIP {
                    break;
                }
                self.edges[e].next_in_lml = self.nx(e);
                e = self.nx(e);
            }
            self.insert_local_minima(loc_min);
            return Ok(true);
        }

        let mut e_min = NIL;

        // workaround for endless loop when open paths have matching start and end points
        if self.edges[self.pv(e)].bot == self.edges[self.pv(e)].top {
            e = self.nx(e);
        }

        loop {
            e = self.find_next_loc_min(e);
            if e == e_min {
                break;
            } else if e_min == NIL {
                e_min = e;
            }

            // e and e.prev now share a local minima (left aligned if horizontal);
            // compare their slopes to find which starts which bound
            let y = self.edges[e].bot.y;
            let (lb, rb, left_bound_is_forward);
            if self.edges[e].dx < self.edges[self.pv(e)].dx {
                lb = self.pv(e);
                rb = e;
                left_bound_is_forward = false; // Q.nextInLML = Q.prev
            } else {
                lb = e;
                rb = self.pv(e);
                left_bound_is_forward = true; // Q.nextInLML = Q.next
            }
            let loc_min = self.new_lm(y, lb, rb);
            self.edges[lb].side = EdgeSide::Left;
            self.edges[rb].side = EdgeSide::Right;

            self.edges[lb].wind_delta = if !closed {
                0
            } else if self.nx(lb) == rb {
                -1
            } else {
                1
            };
            self.edges[rb].wind_delta = -self.edges[lb].wind_delta;

            e = self.process_bound(lb, left_bound_is_forward);
            if self.edges[e].out_idx == SKIP {
                e = self.process_bound(e, left_bound_is_forward);
            }

            let mut e2 = self.process_bound(rb, !left_bound_is_forward);
            if self.edges[e2].out_idx == SKIP {
                e2 = self.process_bound(e2, !left_bound_is_forward);
            }

            if self.edges[lb].out_idx == SKIP {
                self.minima[loc_min].left_bound = NIL;
            } else if self.edges[rb].out_idx == SKIP {
                self.minima[loc_min].right_bound = NIL;
            }
            self.insert_local_minima(loc_min);
            if !left_bound_is_forward {
                e = e2;
            }
        }
        Ok(true)
    }

    /// `AddPaths`. `ppg` paths, `poly_type`, `closed` as in `add_path`. Returns true if any added.
    pub fn add_paths(&mut self, ppg: &[Path], poly_type: PolyType, closed: bool) -> Res<bool> {
        let mut result = false;
        for p in ppg {
            if self.add_path(p, poly_type, closed)? {
                result = true;
            }
        }
        Ok(result)
    }

    /// `RemoveEdge`: unlink `e` from the ring. Returns the following edge.
    fn remove_edge(&mut self, e: usize) -> usize {
        let (p, n) = (self.pv(e), self.nx(e));
        self.edges[p].next = n;
        self.edges[n].prev = p;
        self.edges[e].prev = NIL; // flag as removed
        n
    }

    /// `SetDx`: delta and slope from top / bot.
    fn set_dx(&mut self, e: usize) {
        let ed = &mut self.edges[e];
        ed.delta.x = ed.top.x.wrapping_sub(ed.bot.x);
        ed.delta.y = ed.top.y.wrapping_sub(ed.bot.y);
        if ed.delta.y == 0 {
            ed.dx = HORIZONTAL;
        } else {
            ed.dx = ed.delta.x as f64 / ed.delta.y as f64;
        }
    }

    /// Allocate a `LocalMinima` node. Returns its arena id.
    fn new_lm(&mut self, y: CInt, left_bound: usize, right_bound: usize) -> usize {
        self.minima.push(LocalMinima { y, left_bound, right_bound, next: NIL });
        self.minima.len() - 1
    }

    /// `InsertLocalMinima`: sorted insert (Y descending, new goes before equal Y).
    fn insert_local_minima(&mut self, new_lm: usize) {
        if self.minima_list == NIL {
            self.minima_list = new_lm;
        } else if self.minima[new_lm].y >= self.minima[self.minima_list].y {
            self.minima[new_lm].next = self.minima_list;
            self.minima_list = new_lm;
        } else {
            let mut tmp = self.minima_list;
            while self.minima[tmp].next != NIL
                && self.minima[new_lm].y < self.minima[self.minima[tmp].next].y
            {
                tmp = self.minima[tmp].next;
            }
            self.minima[new_lm].next = self.minima[tmp].next;
            self.minima[tmp].next = new_lm;
        }
    }

    /// `PopLocalMinima`. `y` is the scanbeam Y. Returns the minima id when one is due at `y`.
    fn pop_local_minima(&mut self, y: CInt) -> Option<usize> {
        let current = self.current_lm;
        if current != NIL && self.minima[current].y == y {
            self.current_lm = self.minima[current].next;
            return Some(current);
        }
        None
    }

    /// `ReverseHorizontal`: swap top and bottom X of a horizontal edge.
    fn reverse_horizontal(&mut self, e: usize) {
        let ed = &mut self.edges[e];
        std::mem::swap(&mut ed.top.x, &mut ed.bot.x);
    }

    /// `Reset`: prepare edges, scanbeam and AEL for `Execute`.
    fn reset(&mut self) {
        self.current_lm = self.minima_list;
        if self.current_lm == NIL {
            return; // nothing to process
        }

        // reset all edges
        self.scanbeam.clear();
        let mut lm = self.minima_list;
        while lm != NIL {
            let y = self.minima[lm].y;
            self.insert_scanbeam(y);
            let e = self.minima[lm].left_bound;
            if e != NIL {
                self.edges[e].curr = self.edges[e].bot;
                self.edges[e].out_idx = UNASSIGNED;
            }
            let e = self.minima[lm].right_bound;
            if e != NIL {
                self.edges[e].curr = self.edges[e].bot;
                self.edges[e].out_idx = UNASSIGNED;
            }
            lm = self.minima[lm].next;
        }
        self.active_edges = NIL;
    }

    /// `GetBounds`. `paths` are scanned. Returns the bounding rect (all zero if no points).
    pub fn get_bounds(paths: &[Path]) -> IntRect {
        let cnt = paths.len();
        let mut i = 0;
        while i < cnt && paths[i].is_empty() {
            i += 1;
        }
        if i == cnt {
            return IntRect::new(0, 0, 0, 0);
        }
        let mut result = IntRect::default();
        result.left = paths[i][0].x;
        result.right = result.left;
        result.top = paths[i][0].y;
        result.bottom = result.top;
        while i < cnt {
            for p in &paths[i] {
                if p.x < result.left {
                    result.left = p.x;
                } else if p.x > result.right {
                    result.right = p.x;
                }
                if p.y < result.top {
                    result.top = p.y;
                } else if p.y > result.bottom {
                    result.bottom = p.y;
                }
            }
            i += 1;
        }
        result
    }

    /// `InsertScanbeam`: sorted unique insert. `y` is the scanline Y.
    fn insert_scanbeam(&mut self, y: CInt) {
        if let Err(pos) = self.scanbeam.binary_search(&y) {
            self.scanbeam.insert(pos, y);
        }
    }

    /// `PopScanbeam`. Returns (found, y); y is 0 when nothing left (as C# `out`).
    fn pop_scanbeam(&mut self) -> (bool, CInt) {
        match self.scanbeam.pop() {
            Some(y) => (true, y),
            None => (false, 0),
        }
    }

    /// `LocalMinimaPending`. Returns true when minima remain.
    fn local_minima_pending(&self) -> bool {
        self.current_lm != NIL
    }

    /// `CreateOutRec`. Returns index of the new record.
    fn create_out_rec(&mut self) -> usize {
        self.poly_outs.push(OutRec {
            idx: UNASSIGNED,
            is_hole: false,
            is_open: false,
            first_left: NIL,
            pts: NIL,
            bottom_pt: NIL,
            poly_node: None,
        });
        let idx = self.poly_outs.len() - 1;
        self.poly_outs[idx].idx = idx as i32;
        idx
    }

    /// Allocate an `OutPt`. Returns its arena id.
    fn new_out_pt(&mut self, idx: i32, pt: IntPoint, next: usize, prev: usize) -> usize {
        self.out_pts.push(OutPt { idx, pt, next, prev });
        self.out_pts.len() - 1
    }

    /// `UpdateEdgeIntoAEL`: replace `*e` by its `NextInLML` in the AEL. `e` is updated.
    fn update_edge_into_ael(&mut self, e: &mut usize) -> Res<()> {
        let cur = *e;
        if self.edges[cur].next_in_lml == NIL {
            return Err(ClipperException("UpdateEdgeIntoAEL: invalid call".into()));
        }
        let ael_prev = self.edges[cur].prev_in_ael;
        let ael_next = self.edges[cur].next_in_ael;
        let nl = self.edges[cur].next_in_lml;
        self.edges[nl].out_idx = self.edges[cur].out_idx;
        if ael_prev != NIL {
            self.edges[ael_prev].next_in_ael = nl;
        } else {
            self.active_edges = nl;
        }
        if ael_next != NIL {
            self.edges[ael_next].prev_in_ael = nl;
        }
        self.edges[nl].side = self.edges[cur].side;
        self.edges[nl].wind_delta = self.edges[cur].wind_delta;
        self.edges[nl].wind_cnt = self.edges[cur].wind_cnt;
        self.edges[nl].wind_cnt2 = self.edges[cur].wind_cnt2;
        *e = nl;
        self.edges[nl].curr = self.edges[nl].bot;
        self.edges[nl].prev_in_ael = ael_prev;
        self.edges[nl].next_in_ael = ael_next;
        if self.edges[nl].delta.y != 0 {
            let ty = self.edges[nl].top.y;
            self.insert_scanbeam(ty);
        }
        Ok(())
    }

    /// `SwapPositionsInAEL`: swap two edges in the active edge list.
    fn swap_positions_in_ael(&mut self, edge1: usize, edge2: usize) {
        // check that one or other edge hasn't already been removed from AEL
        if self.edges[edge1].next_in_ael == self.edges[edge1].prev_in_ael
            || self.edges[edge2].next_in_ael == self.edges[edge2].prev_in_ael
        {
            return;
        }

        if self.edges[edge1].next_in_ael == edge2 {
            let next = self.edges[edge2].next_in_ael;
            if next != NIL {
                self.edges[next].prev_in_ael = edge1;
            }
            let prev = self.edges[edge1].prev_in_ael;
            if prev != NIL {
                self.edges[prev].next_in_ael = edge2;
            }
            self.edges[edge2].prev_in_ael = prev;
            self.edges[edge2].next_in_ael = edge1;
            self.edges[edge1].prev_in_ael = edge2;
            self.edges[edge1].next_in_ael = next;
        } else if self.edges[edge2].next_in_ael == edge1 {
            let next = self.edges[edge1].next_in_ael;
            if next != NIL {
                self.edges[next].prev_in_ael = edge2;
            }
            let prev = self.edges[edge2].prev_in_ael;
            if prev != NIL {
                self.edges[prev].next_in_ael = edge1;
            }
            self.edges[edge1].prev_in_ael = prev;
            self.edges[edge1].next_in_ael = edge2;
            self.edges[edge2].prev_in_ael = edge1;
            self.edges[edge2].next_in_ael = next;
        } else {
            let next = self.edges[edge1].next_in_ael;
            let prev = self.edges[edge1].prev_in_ael;
            self.edges[edge1].next_in_ael = self.edges[edge2].next_in_ael;
            let n1 = self.edges[edge1].next_in_ael;
            if n1 != NIL {
                self.edges[n1].prev_in_ael = edge1;
            }
            self.edges[edge1].prev_in_ael = self.edges[edge2].prev_in_ael;
            let p1 = self.edges[edge1].prev_in_ael;
            if p1 != NIL {
                self.edges[p1].next_in_ael = edge1;
            }
            self.edges[edge2].next_in_ael = next;
            let n2 = self.edges[edge2].next_in_ael;
            if n2 != NIL {
                self.edges[n2].prev_in_ael = edge2;
            }
            self.edges[edge2].prev_in_ael = prev;
            let p2 = self.edges[edge2].prev_in_ael;
            if p2 != NIL {
                self.edges[p2].next_in_ael = edge2;
            }
        }

        if self.edges[edge1].prev_in_ael == NIL {
            self.active_edges = edge1;
        } else if self.edges[edge2].prev_in_ael == NIL {
            self.active_edges = edge2;
        }
    }

    /// `DeleteFromAEL`: unlink `e` from the active edge list.
    fn delete_from_ael(&mut self, e: usize) {
        let ael_prev = self.edges[e].prev_in_ael;
        let ael_next = self.edges[e].next_in_ael;
        if ael_prev == NIL && ael_next == NIL && e != self.active_edges {
            return; // already deleted
        }
        if ael_prev != NIL {
            self.edges[ael_prev].next_in_ael = ael_next;
        } else {
            self.active_edges = ael_next;
        }
        if ael_next != NIL {
            self.edges[ael_next].prev_in_ael = ael_prev;
        }
        self.edges[e].next_in_ael = NIL;
        self.edges[e].prev_in_ael = NIL;
    }

    /// `PointIsVertex`. `pt` is tested against ring `pp`. Returns true if it is a vertex.
    #[allow(dead_code)] // unused in the C# lib too
    fn point_is_vertex(&self, pt: IntPoint, pp: usize) -> bool {
        let mut pp2 = pp;
        loop {
            if self.out_pts[pp2].pt == pt {
                return true;
            }
            pp2 = self.out_pts[pp2].next;
            if pp2 == pp {
                break;
            }
        }
        false
    }

    /// `PointOnLineSegment`. Returns true when `pt` lies on segment `line_pt1`-`line_pt2`.
    /// `use_full_range` picks 128 bit math.
    #[allow(dead_code)] // unused in the C# lib too
    fn point_on_line_segment(
        pt: IntPoint,
        line_pt1: IntPoint,
        line_pt2: IntPoint,
        use_full_range: bool,
    ) -> bool {
        if use_full_range {
            (pt.x == line_pt1.x && pt.y == line_pt1.y)
                || (pt.x == line_pt2.x && pt.y == line_pt2.y)
                || (((pt.x > line_pt1.x) == (pt.x < line_pt2.x))
                    && ((pt.y > line_pt1.y) == (pt.y < line_pt2.y))
                    && Int128::mul(pt.x.wrapping_sub(line_pt1.x), line_pt2.y.wrapping_sub(line_pt1.y))
                        == Int128::mul(
                            line_pt2.x.wrapping_sub(line_pt1.x),
                            pt.y.wrapping_sub(line_pt1.y),
                        ))
        } else {
            (pt.x == line_pt1.x && pt.y == line_pt1.y)
                || (pt.x == line_pt2.x && pt.y == line_pt2.y)
                || (((pt.x > line_pt1.x) == (pt.x < line_pt2.x))
                    && ((pt.y > line_pt1.y) == (pt.y < line_pt2.y))
                    && pt.x.wrapping_sub(line_pt1.x).wrapping_mul(line_pt2.y.wrapping_sub(line_pt1.y))
                        == line_pt2
                            .x
                            .wrapping_sub(line_pt1.x)
                            .wrapping_mul(pt.y.wrapping_sub(line_pt1.y)))
        }
    }

    /// `PointOnPolygon`. Returns true when `pt` lies on an edge of ring `pp`.
    #[allow(dead_code)] // unused in the C# lib too
    fn point_on_polygon(&self, pt: IntPoint, pp: usize, use_full_range: bool) -> bool {
        let mut pp2 = pp;
        loop {
            let next = self.out_pts[pp2].next;
            if Self::point_on_line_segment(
                pt,
                self.out_pts[pp2].pt,
                self.out_pts[next].pt,
                use_full_range,
            ) {
                return true;
            }
            pp2 = next;
            if pp2 == pp {
                break;
            }
        }
        false
    }

    /// `IsMinima`. Returns true when `e` (not NIL) starts a local minimum.
    #[allow(dead_code)] // unused in the C# lib too
    fn is_minima(&self, e: usize) -> bool {
        e != NIL
            && self.edges[self.pv(e)].next_in_lml != e
            && self.edges[self.nx(e)].next_in_lml != e
    }

    /// `DeleteFromSEL`: unlink `e` from the sorted edge list.
    #[allow(dead_code)] // unused in the C# lib too
    fn delete_from_sel(&mut self, e: usize) {
        let sel_prev = self.edges[e].prev_in_sel;
        let sel_next = self.edges[e].next_in_sel;
        if sel_prev == NIL && sel_next == NIL && e != self.sorted_edges {
            return; // already deleted
        }
        if sel_prev != NIL {
            self.edges[sel_prev].next_in_sel = sel_next;
        } else {
            self.sorted_edges = sel_next;
        }
        if sel_next != NIL {
            self.edges[sel_next].prev_in_sel = sel_prev;
        }
        self.edges[e].next_in_sel = NIL;
        self.edges[e].prev_in_sel = NIL;
    }

    /// `SwapPoints`: swap two points in place.
    #[allow(dead_code)] // unused in the C# lib too
    fn swap_points(pt1: &mut IntPoint, pt2: &mut IntPoint) {
        std::mem::swap(pt1, pt2);
    }

    /// `IntersectNodeSort` (private static, unused in C#): Y difference as int.
    #[allow(dead_code)] // unused in the C# lib too
    fn intersect_node_sort(node1: &IntersectNode, node2: &IntersectNode) -> i32 {
        // C# (int) cast of a long
        node2.pt.y.wrapping_sub(node1.pt.y) as i32
    }

    // ---- Clipper ----

    /// `InsertMaxima`: sorted unique insert into the maxima list. `x` is the maxima X.
    fn insert_maxima(&mut self, x: CInt) {
        self.maxima.push(Maxima { x, next: NIL, prev: NIL });
        let new_max = self.maxima.len() - 1;
        if self.maxima_head == NIL {
            self.maxima_head = new_max;
        } else if x < self.maxima[self.maxima_head].x {
            self.maxima[new_max].next = self.maxima_head;
            self.maxima[new_max].prev = NIL;
            self.maxima_head = new_max;
        } else {
            let mut m = self.maxima_head;
            while self.maxima[m].next != NIL && x >= self.maxima[self.maxima[m].next].x {
                m = self.maxima[m].next;
            }
            if x == self.maxima[m].x {
                return; // ignores duplicates
            }
            // insert new_max between m and m.next
            self.maxima[new_max].next = self.maxima[m].next;
            self.maxima[new_max].prev = m;
            let mn = self.maxima[m].next;
            if mn != NIL {
                self.maxima[mn].prev = new_max;
            }
            self.maxima[m].next = new_max;
        }
    }

    /// `Execute(clipType, Paths, FillType)`: same fill type for subject and clip.
    ///
    /// `clip_type` operation, `solution` is cleared and filled, `fill_type` for both polys.
    /// Returns Ok(success); Err on library exceptions.
    pub fn execute(
        &mut self,
        clip_type: ClipType,
        solution: &mut Paths,
        fill_type: PolyFillType,
    ) -> Res<bool> {
        self.execute_fill(clip_type, solution, fill_type, fill_type)
    }

    /// `Execute(clipType, PolyTree, FillType)`: same fill type for subject and clip.
    ///
    /// `clip_type` operation, `polytree` receives the result, `fill_type` for both polys.
    /// Returns Ok(success); Err on library exceptions.
    pub fn execute_tree(
        &mut self,
        clip_type: ClipType,
        polytree: &mut PolyTree,
        fill_type: PolyFillType,
    ) -> Res<bool> {
        self.execute_tree_fill(clip_type, polytree, fill_type, fill_type)
    }

    /// `Execute(clipType, Paths, subjFillType, clipFillType)`.
    ///
    /// `solution` is cleared and filled. Open paths need the PolyTree overload (Err otherwise).
    /// Returns Ok(success).
    pub fn execute_fill(
        &mut self,
        clip_type: ClipType,
        solution: &mut Paths,
        subj_fill_type: PolyFillType,
        clip_fill_type: PolyFillType,
    ) -> Res<bool> {
        if self.execute_locked {
            return Ok(false);
        }
        if self.has_open_paths {
            return Err(ClipperException(
                "Error: PolyTree struct is needed for open path clipping.".into(),
            ));
        }

        self.execute_locked = true;
        solution.clear();
        self.subj_fill_type = subj_fill_type;
        self.clip_fill_type = clip_fill_type;
        self.clip_type = clip_type;
        self.using_poly_tree = false;
        let result = self.execute_internal().inspect(|&succeeded| {
            // build the return polygons
            if succeeded {
                self.build_result(solution);
            }
        });
        self.dispose_all_poly_pts();
        self.execute_locked = false;
        result
    }

    /// `Execute(clipType, PolyTree, subjFillType, clipFillType)`.
    ///
    /// `polytree` is cleared and filled (open paths allowed). Returns Ok(success).
    pub fn execute_tree_fill(
        &mut self,
        clip_type: ClipType,
        polytree: &mut PolyTree,
        subj_fill_type: PolyFillType,
        clip_fill_type: PolyFillType,
    ) -> Res<bool> {
        if self.execute_locked {
            return Ok(false);
        }
        self.execute_locked = true;
        self.subj_fill_type = subj_fill_type;
        self.clip_fill_type = clip_fill_type;
        self.clip_type = clip_type;
        self.using_poly_tree = true;
        let result = self.execute_internal().inspect(|&succeeded| {
            if succeeded {
                self.build_result2(polytree);
            }
        });
        self.dispose_all_poly_pts();
        self.execute_locked = false;
        result
    }

    /// `FixHoleLinkage`: make `FirstLeft` of `out_rec` point at the right enclosing record.
    fn fix_hole_linkage(&mut self, out_rec: usize) {
        let fl = self.poly_outs[out_rec].first_left;
        // skip if outermost or already points to the correct FirstLeft
        if fl == NIL
            || (self.poly_outs[out_rec].is_hole != self.poly_outs[fl].is_hole
                && self.poly_outs[fl].pts != NIL)
        {
            return;
        }

        let is_hole = self.poly_outs[out_rec].is_hole;
        let mut orfl = fl;
        while orfl != NIL && (self.poly_outs[orfl].is_hole == is_hole || self.poly_outs[orfl].pts == NIL)
        {
            orfl = self.poly_outs[orfl].first_left;
        }
        self.poly_outs[out_rec].first_left = orfl;
    }

    /// `ExecuteInternal` (join lists are cleared in the C# `finally`).
    fn execute_internal(&mut self) -> Res<bool> {
        let r = self.execute_internal_inner();
        self.joins.clear();
        self.ghost_joins.clear();
        r
    }

    fn execute_internal_inner(&mut self) -> Res<bool> {
        self.reset();
        self.sorted_edges = NIL;
        self.maxima.clear();
        self.maxima_head = NIL;

        let (ok, mut bot_y) = self.pop_scanbeam();
        if !ok {
            return Ok(false);
        }
        self.insert_local_minima_into_ael(bot_y);
        loop {
            let (ok, top_y) = self.pop_scanbeam();
            if !(ok || self.local_minima_pending()) {
                break;
            }
            self.process_horizontals()?;
            self.ghost_joins.clear();
            if !self.process_intersections(top_y) {
                return Ok(false);
            }
            self.process_edges_at_top_of_scanbeam(top_y)?;
            bot_y = top_y;
            self.insert_local_minima_into_ael(bot_y);
        }

        // fix orientations
        for i in 0..self.poly_outs.len() {
            if self.poly_outs[i].pts == NIL || self.poly_outs[i].is_open {
                continue;
            }
            let pts = self.poly_outs[i].pts;
            if (self.poly_outs[i].is_hole ^ self.reverse_solution)
                == (area_ring(&self.out_pts, pts) > 0.0)
            {
                self.reverse_poly_pt_links(pts);
            }
        }

        self.join_common_edges();

        for i in 0..self.poly_outs.len() {
            if self.poly_outs[i].pts == NIL {
                continue;
            } else if self.poly_outs[i].is_open {
                self.fixup_out_polyline(i);
            } else {
                self.fixup_out_polygon(i);
            }
        }

        if self.strictly_simple {
            self.do_simple_polygons();
        }
        Ok(true)
    }

    /// `DisposeAllPolyPts`: drop output records and points.
    fn dispose_all_poly_pts(&mut self) {
        self.poly_outs.clear();
        self.out_pts.clear();
    }

    /// `AddJoin`.
    fn add_join(&mut self, op1: usize, op2: usize, off_pt: IntPoint) {
        self.joins.push(Join { out_pt1: op1, out_pt2: op2, off_pt });
    }

    /// `AddGhostJoin`.
    fn add_ghost_join(&mut self, op: usize, off_pt: IntPoint) {
        self.ghost_joins.push(Join { out_pt1: op, out_pt2: NIL, off_pt });
    }

    /// `InsertLocalMinimaIntoAEL`: start edges of all minima at scanline `bot_y`.
    fn insert_local_minima_into_ael(&mut self, bot_y: CInt) {
        while let Some(lm) = self.pop_local_minima(bot_y) {
            let lb = self.minima[lm].left_bound;
            let rb = self.minima[lm].right_bound;

            let mut op1 = NIL;
            if lb == NIL {
                self.insert_edge_into_ael(rb, NIL);
                self.set_winding_count(rb);
                if self.is_contributing(rb) {
                    op1 = self.add_out_pt(rb, self.edges[rb].bot);
                }
            } else if rb == NIL {
                self.insert_edge_into_ael(lb, NIL);
                self.set_winding_count(lb);
                if self.is_contributing(lb) {
                    op1 = self.add_out_pt(lb, self.edges[lb].bot);
                }
                let ty = self.edges[lb].top.y;
                self.insert_scanbeam(ty);
            } else {
                self.insert_edge_into_ael(lb, NIL);
                self.insert_edge_into_ael(rb, lb);
                self.set_winding_count(lb);
                self.edges[rb].wind_cnt = self.edges[lb].wind_cnt;
                self.edges[rb].wind_cnt2 = self.edges[lb].wind_cnt2;
                if self.is_contributing(lb) {
                    op1 = self.add_local_min_poly(lb, rb, self.edges[lb].bot);
                }
                let ty = self.edges[lb].top.y;
                self.insert_scanbeam(ty);
            }

            if rb != NIL {
                if self.edges[rb].delta.y == 0 {
                    let nl = self.edges[rb].next_in_lml;
                    if nl != NIL {
                        let ty = self.edges[nl].top.y;
                        self.insert_scanbeam(ty);
                    }
                    self.add_edge_to_sel(rb);
                } else {
                    let ty = self.edges[rb].top.y;
                    self.insert_scanbeam(ty);
                }
            }

            if lb == NIL || rb == NIL {
                continue;
            }

            // if output polygons share an edge with a horizontal rb, they'll need joining later
            if op1 != NIL
                && self.edges[rb].delta.y == 0
                && !self.ghost_joins.is_empty()
                && self.edges[rb].wind_delta != 0
            {
                for i in 0..self.ghost_joins.len() {
                    // if the horizontal rb and a 'ghost' horizontal overlap, convert the
                    // ghost join to a real join ready for later
                    let (j_op1, j_off) = (self.ghost_joins[i].out_pt1, self.ghost_joins[i].off_pt);
                    if self.horz_segments_overlap(
                        self.out_pts[j_op1].pt.x,
                        j_off.x,
                        self.edges[rb].bot.x,
                        self.edges[rb].top.x,
                    ) {
                        self.add_join(j_op1, op1, j_off);
                    }
                }
            }

            let lb_prev = self.edges[lb].prev_in_ael;
            if self.edges[lb].out_idx >= 0
                && lb_prev != NIL
                && self.edges[lb_prev].curr.x == self.edges[lb].bot.x
                && self.edges[lb_prev].out_idx >= 0
                && slopes_equal4(
                    self.edges[lb_prev].curr,
                    self.edges[lb_prev].top,
                    self.edges[lb].curr,
                    self.edges[lb].top,
                    self.use_full_range,
                )
                && self.edges[lb].wind_delta != 0
                && self.edges[lb_prev].wind_delta != 0
            {
                let op2 = self.add_out_pt(lb_prev, self.edges[lb].bot);
                self.add_join(op1, op2, self.edges[lb].top);
            }

            if self.edges[lb].next_in_ael != rb {
                let rb_prev = self.edges[rb].prev_in_ael;
                if self.edges[rb].out_idx >= 0
                    && self.edges[rb_prev].out_idx >= 0
                    && slopes_equal4(
                        self.edges[rb_prev].curr,
                        self.edges[rb_prev].top,
                        self.edges[rb].curr,
                        self.edges[rb].top,
                        self.use_full_range,
                    )
                    && self.edges[rb].wind_delta != 0
                    && self.edges[rb_prev].wind_delta != 0
                {
                    let op2 = self.add_out_pt(rb_prev, self.edges[rb].bot);
                    self.add_join(op1, op2, self.edges[rb].top);
                }

                let mut e = self.edges[lb].next_in_ael;
                if e != NIL {
                    while e != rb {
                        // nb: IntersectEdges() assumes param1 will be right of param2 ABOVE
                        // the intersection
                        let pt = self.edges[lb].curr;
                        self.intersect_edges(rb, e, pt); // order important here
                        e = self.edges[e].next_in_ael;
                    }
                }
            }
        }
    }

    /// `InsertEdgeIntoAEL`. `edge` is inserted after `start_edge` (NIL = from the front).
    fn insert_edge_into_ael(&mut self, edge: usize, mut start_edge: usize) {
        if self.active_edges == NIL {
            self.edges[edge].prev_in_ael = NIL;
            self.edges[edge].next_in_ael = NIL;
            self.active_edges = edge;
        } else if start_edge == NIL && self.e2_inserts_before_e1(self.active_edges, edge) {
            self.edges[edge].prev_in_ael = NIL;
            self.edges[edge].next_in_ael = self.active_edges;
            let ae = self.active_edges;
            self.edges[ae].prev_in_ael = edge;
            self.active_edges = edge;
        } else {
            if start_edge == NIL {
                start_edge = self.active_edges;
            }
            while self.edges[start_edge].next_in_ael != NIL
                && !self.e2_inserts_before_e1(self.edges[start_edge].next_in_ael, edge)
            {
                start_edge = self.edges[start_edge].next_in_ael;
            }
            let sn = self.edges[start_edge].next_in_ael;
            self.edges[edge].next_in_ael = sn;
            if sn != NIL {
                self.edges[sn].prev_in_ael = edge;
            }
            self.edges[edge].prev_in_ael = start_edge;
            self.edges[start_edge].next_in_ael = edge;
        }
    }

    /// `E2InsertsBeforeE1`. Returns true when `e2` sorts left of `e1` in the AEL.
    fn e2_inserts_before_e1(&self, e1: usize, e2: usize) -> bool {
        let (a, b) = (&self.edges[e1], &self.edges[e2]);
        if b.curr.x == a.curr.x {
            if b.top.y > a.top.y {
                b.top.x < top_x(a, b.top.y)
            } else {
                a.top.x > top_x(b, a.top.y)
            }
        } else {
            b.curr.x < a.curr.x
        }
    }

    /// `IsEvenOddFillType`.
    fn is_even_odd_fill_type(&self, edge: usize) -> bool {
        if self.edges[edge].poly_typ == PolyType::Subject {
            self.subj_fill_type == PolyFillType::EvenOdd
        } else {
            self.clip_fill_type == PolyFillType::EvenOdd
        }
    }

    /// `IsEvenOddAltFillType`.
    fn is_even_odd_alt_fill_type(&self, edge: usize) -> bool {
        if self.edges[edge].poly_typ == PolyType::Subject {
            self.clip_fill_type == PolyFillType::EvenOdd
        } else {
            self.subj_fill_type == PolyFillType::EvenOdd
        }
    }

    /// `IsContributing`: does `edge` belong to the solution given fill rules and clip type?
    fn is_contributing(&self, edge: usize) -> bool {
        let ed = &self.edges[edge];
        let (pft, pft2) = if ed.poly_typ == PolyType::Subject {
            (self.subj_fill_type, self.clip_fill_type)
        } else {
            (self.clip_fill_type, self.subj_fill_type)
        };

        match pft {
            PolyFillType::EvenOdd => {
                // false if a subj line has been flagged as inside a subj polygon
                if ed.wind_delta == 0 && ed.wind_cnt != 1 {
                    return false;
                }
            }
            PolyFillType::NonZero => {
                if ed.wind_cnt.abs() != 1 {
                    return false;
                }
            }
            PolyFillType::Positive => {
                if ed.wind_cnt != 1 {
                    return false;
                }
            }
            PolyFillType::Negative => {
                if ed.wind_cnt != -1 {
                    return false;
                }
            }
        }

        match self.clip_type {
            ClipType::Intersection => match pft2 {
                PolyFillType::EvenOdd | PolyFillType::NonZero => ed.wind_cnt2 != 0,
                PolyFillType::Positive => ed.wind_cnt2 > 0,
                PolyFillType::Negative => ed.wind_cnt2 < 0,
            },
            ClipType::Union => match pft2 {
                PolyFillType::EvenOdd | PolyFillType::NonZero => ed.wind_cnt2 == 0,
                PolyFillType::Positive => ed.wind_cnt2 <= 0,
                PolyFillType::Negative => ed.wind_cnt2 >= 0,
            },
            ClipType::Difference => {
                if ed.poly_typ == PolyType::Subject {
                    match pft2 {
                        PolyFillType::EvenOdd | PolyFillType::NonZero => ed.wind_cnt2 == 0,
                        PolyFillType::Positive => ed.wind_cnt2 <= 0,
                        PolyFillType::Negative => ed.wind_cnt2 >= 0,
                    }
                } else {
                    match pft2 {
                        PolyFillType::EvenOdd | PolyFillType::NonZero => ed.wind_cnt2 != 0,
                        PolyFillType::Positive => ed.wind_cnt2 > 0,
                        PolyFillType::Negative => ed.wind_cnt2 < 0,
                    }
                }
            }
            ClipType::Xor => {
                if ed.wind_delta == 0 {
                    // XOr always contributing unless open
                    match pft2 {
                        PolyFillType::EvenOdd | PolyFillType::NonZero => ed.wind_cnt2 == 0,
                        PolyFillType::Positive => ed.wind_cnt2 <= 0,
                        PolyFillType::Negative => ed.wind_cnt2 >= 0,
                    }
                } else {
                    true
                }
            }
        }
    }

    /// `SetWindingCount`: compute WindCnt / WindCnt2 for `edge` from its AEL neighbours.
    fn set_winding_count(&mut self, edge: usize) {
        let mut e = self.edges[edge].prev_in_ael;
        // find the edge of the same polytype that immediately precedes 'edge' in AEL
        while e != NIL
            && (self.edges[e].poly_typ != self.edges[edge].poly_typ || self.edges[e].wind_delta == 0)
        {
            e = self.edges[e].prev_in_ael;
        }
        if e == NIL {
            let pft = if self.edges[edge].poly_typ == PolyType::Subject {
                self.subj_fill_type
            } else {
                self.clip_fill_type
            };
            if self.edges[edge].wind_delta == 0 {
                self.edges[edge].wind_cnt = if pft == PolyFillType::Negative { -1 } else { 1 };
            } else {
                self.edges[edge].wind_cnt = self.edges[edge].wind_delta;
            }
            self.edges[edge].wind_cnt2 = 0;
            e = self.active_edges; // get ready to calc WindCnt2
        } else if self.edges[edge].wind_delta == 0 && self.clip_type != ClipType::Union {
            self.edges[edge].wind_cnt = 1;
            self.edges[edge].wind_cnt2 = self.edges[e].wind_cnt2;
            e = self.edges[e].next_in_ael; // get ready to calc WindCnt2
        } else if self.is_even_odd_fill_type(edge) {
            // EvenOdd filling
            if self.edges[edge].wind_delta == 0 {
                // are we inside a subj polygon
                let mut inside = true;
                let mut e2 = self.edges[e].prev_in_ael;
                while e2 != NIL {
                    if self.edges[e2].poly_typ == self.edges[e].poly_typ
                        && self.edges[e2].wind_delta != 0
                    {
                        inside = !inside;
                    }
                    e2 = self.edges[e2].prev_in_ael;
                }
                self.edges[edge].wind_cnt = if inside { 0 } else { 1 };
            } else {
                self.edges[edge].wind_cnt = self.edges[edge].wind_delta;
            }
            self.edges[edge].wind_cnt2 = self.edges[e].wind_cnt2;
            e = self.edges[e].next_in_ael; // get ready to calc WindCnt2
        } else {
            // nonZero, Positive or Negative filling
            let (e_wc, e_wd) = (self.edges[e].wind_cnt, self.edges[e].wind_delta);
            let ewd = self.edges[edge].wind_delta;
            if e_wc * e_wd < 0 {
                // prev edge is 'decreasing' WindCount toward zero: outside the previous polygon
                if e_wc.abs() > 1 {
                    // outside prev poly but still inside another
                    if e_wd * ewd < 0 {
                        self.edges[edge].wind_cnt = e_wc;
                    } else {
                        self.edges[edge].wind_cnt = e_wc + ewd;
                    }
                } else {
                    // now outside all polys of same polytype so set own WC
                    self.edges[edge].wind_cnt = if ewd == 0 { 1 } else { ewd };
                }
            } else {
                // prev edge is 'increasing' WindCount away from zero: inside previous polygon
                if ewd == 0 {
                    self.edges[edge].wind_cnt = if e_wc < 0 { e_wc - 1 } else { e_wc + 1 };
                } else if e_wd * ewd < 0 {
                    // wind direction reversing prev: use same WC
                    self.edges[edge].wind_cnt = e_wc;
                } else {
                    self.edges[edge].wind_cnt = e_wc + ewd;
                }
            }
            self.edges[edge].wind_cnt2 = self.edges[e].wind_cnt2;
            e = self.edges[e].next_in_ael; // get ready to calc WindCnt2
        }

        // update WindCnt2
        if self.is_even_odd_alt_fill_type(edge) {
            // EvenOdd filling
            while e != edge {
                if self.edges[e].wind_delta != 0 {
                    self.edges[edge].wind_cnt2 = if self.edges[edge].wind_cnt2 == 0 { 1 } else { 0 };
                }
                e = self.edges[e].next_in_ael;
            }
        } else {
            // nonZero, Positive or Negative filling
            while e != edge {
                self.edges[edge].wind_cnt2 += self.edges[e].wind_delta;
                e = self.edges[e].next_in_ael;
            }
        }
    }

    /// `AddEdgeToSEL`: push `edge` to the front of the SEL.
    fn add_edge_to_sel(&mut self, edge: usize) {
        // SEL pointers build transient lists of horizontal edges; additions go to the front
        if self.sorted_edges == NIL {
            self.sorted_edges = edge;
            self.edges[edge].prev_in_sel = NIL;
            self.edges[edge].next_in_sel = NIL;
        } else {
            self.edges[edge].next_in_sel = self.sorted_edges;
            self.edges[edge].prev_in_sel = NIL;
            let se = self.sorted_edges;
            self.edges[se].prev_in_sel = edge;
            self.sorted_edges = edge;
        }
    }

    /// `PopEdgeFromSEL`. Returns the front edge of the SEL, if any.
    fn pop_edge_from_sel(&mut self) -> Option<usize> {
        let e = self.sorted_edges;
        if e == NIL {
            return None;
        }
        self.sorted_edges = self.edges[e].next_in_sel;
        if self.sorted_edges != NIL {
            let se = self.sorted_edges;
            self.edges[se].prev_in_sel = NIL;
        }
        self.edges[e].next_in_sel = NIL;
        self.edges[e].prev_in_sel = NIL;
        Some(e)
    }

    /// `CopyAELToSEL`.
    fn copy_ael_to_sel(&mut self) {
        let mut e = self.active_edges;
        self.sorted_edges = e;
        while e != NIL {
            self.edges[e].prev_in_sel = self.edges[e].prev_in_ael;
            self.edges[e].next_in_sel = self.edges[e].next_in_ael;
            e = self.edges[e].next_in_ael;
        }
    }

    /// `SwapPositionsInSEL`: swap two edges in the sorted edge list.
    fn swap_positions_in_sel(&mut self, edge1: usize, edge2: usize) {
        if self.edges[edge1].next_in_sel == NIL && self.edges[edge1].prev_in_sel == NIL {
            return;
        }
        if self.edges[edge2].next_in_sel == NIL && self.edges[edge2].prev_in_sel == NIL {
            return;
        }

        if self.edges[edge1].next_in_sel == edge2 {
            let next = self.edges[edge2].next_in_sel;
            if next != NIL {
                self.edges[next].prev_in_sel = edge1;
            }
            let prev = self.edges[edge1].prev_in_sel;
            if prev != NIL {
                self.edges[prev].next_in_sel = edge2;
            }
            self.edges[edge2].prev_in_sel = prev;
            self.edges[edge2].next_in_sel = edge1;
            self.edges[edge1].prev_in_sel = edge2;
            self.edges[edge1].next_in_sel = next;
        } else if self.edges[edge2].next_in_sel == edge1 {
            let next = self.edges[edge1].next_in_sel;
            if next != NIL {
                self.edges[next].prev_in_sel = edge2;
            }
            let prev = self.edges[edge2].prev_in_sel;
            if prev != NIL {
                self.edges[prev].next_in_sel = edge1;
            }
            self.edges[edge1].prev_in_sel = prev;
            self.edges[edge1].next_in_sel = edge2;
            self.edges[edge2].prev_in_sel = edge1;
            self.edges[edge2].next_in_sel = next;
        } else {
            let next = self.edges[edge1].next_in_sel;
            let prev = self.edges[edge1].prev_in_sel;
            self.edges[edge1].next_in_sel = self.edges[edge2].next_in_sel;
            let n1 = self.edges[edge1].next_in_sel;
            if n1 != NIL {
                self.edges[n1].prev_in_sel = edge1;
            }
            self.edges[edge1].prev_in_sel = self.edges[edge2].prev_in_sel;
            let p1 = self.edges[edge1].prev_in_sel;
            if p1 != NIL {
                self.edges[p1].next_in_sel = edge1;
            }
            self.edges[edge2].next_in_sel = next;
            let n2 = self.edges[edge2].next_in_sel;
            if n2 != NIL {
                self.edges[n2].prev_in_sel = edge2;
            }
            self.edges[edge2].prev_in_sel = prev;
            let p2 = self.edges[edge2].prev_in_sel;
            if p2 != NIL {
                self.edges[p2].next_in_sel = edge2;
            }
        }

        if self.edges[edge1].prev_in_sel == NIL {
            self.sorted_edges = edge1;
        } else if self.edges[edge2].prev_in_sel == NIL {
            self.sorted_edges = edge2;
        }
    }

    /// `AddLocalMaxPoly`: close / merge output polys where `e1` and `e2` meet at `pt`.
    fn add_local_max_poly(&mut self, e1: usize, e2: usize, pt: IntPoint) {
        self.add_out_pt(e1, pt);
        if self.edges[e2].wind_delta == 0 {
            self.add_out_pt(e2, pt);
        }
        if self.edges[e1].out_idx == self.edges[e2].out_idx {
            self.edges[e1].out_idx = UNASSIGNED;
            self.edges[e2].out_idx = UNASSIGNED;
        } else if self.edges[e1].out_idx < self.edges[e2].out_idx {
            self.append_polygon(e1, e2);
        } else {
            self.append_polygon(e2, e1);
        }
    }

    /// `AddLocalMinPoly`: start a new output poly at `pt`. Returns the first `OutPt`.
    fn add_local_min_poly(&mut self, e1: usize, e2: usize, pt: IntPoint) -> usize {
        let result;
        let e;
        let prev_e;
        if self.edges[e2].delta.y == 0 || self.edges[e1].dx > self.edges[e2].dx {
            result = self.add_out_pt(e1, pt);
            self.edges[e2].out_idx = self.edges[e1].out_idx;
            self.edges[e1].side = EdgeSide::Left;
            self.edges[e2].side = EdgeSide::Right;
            e = e1;
            if self.edges[e].prev_in_ael == e2 {
                prev_e = self.edges[e2].prev_in_ael;
            } else {
                prev_e = self.edges[e].prev_in_ael;
            }
        } else {
            result = self.add_out_pt(e2, pt);
            self.edges[e1].out_idx = self.edges[e2].out_idx;
            self.edges[e1].side = EdgeSide::Right;
            self.edges[e2].side = EdgeSide::Left;
            e = e2;
            if self.edges[e].prev_in_ael == e1 {
                prev_e = self.edges[e1].prev_in_ael;
            } else {
                prev_e = self.edges[e].prev_in_ael;
            }
        }

        if prev_e != NIL
            && self.edges[prev_e].out_idx >= 0
            && self.edges[prev_e].top.y < pt.y
            && self.edges[e].top.y < pt.y
        {
            let x_prev = top_x(&self.edges[prev_e], pt.y);
            let x_e = top_x(&self.edges[e], pt.y);
            if x_prev == x_e
                && self.edges[e].wind_delta != 0
                && self.edges[prev_e].wind_delta != 0
                && slopes_equal4(
                    IntPoint::new(x_prev, pt.y),
                    self.edges[prev_e].top,
                    IntPoint::new(x_e, pt.y),
                    self.edges[e].top,
                    self.use_full_range,
                )
            {
                let out_pt = self.add_out_pt(prev_e, pt);
                self.add_join(result, out_pt, self.edges[e].top);
            }
        }
        result
    }

    /// `AddOutPt`: append `pt` to the output poly of edge `e`. Returns the `OutPt`.
    fn add_out_pt(&mut self, e: usize, pt: IntPoint) -> usize {
        if self.edges[e].out_idx < 0 {
            let out_rec = self.create_out_rec();
            self.poly_outs[out_rec].is_open = self.edges[e].wind_delta == 0;
            let idx = self.poly_outs[out_rec].idx;
            let new_op = self.new_out_pt(idx, pt, NIL, NIL);
            self.out_pts[new_op].next = new_op;
            self.out_pts[new_op].prev = new_op;
            self.poly_outs[out_rec].pts = new_op;
            if !self.poly_outs[out_rec].is_open {
                self.set_hole_state(e, out_rec);
            }
            self.edges[e].out_idx = idx;
            new_op
        } else {
            let out_rec = self.edges[e].out_idx as usize;
            // OutRec.Pts is the 'Left-most' point & OutRec.Pts.Prev is the 'Right-most'
            let op = self.poly_outs[out_rec].pts;
            let to_front = self.edges[e].side == EdgeSide::Left;
            if to_front && pt == self.out_pts[op].pt {
                return op;
            } else if !to_front && pt == self.out_pts[self.out_pts[op].prev].pt {
                return self.out_pts[op].prev;
            }

            let idx = self.poly_outs[out_rec].idx;
            let op_prev = self.out_pts[op].prev;
            let new_op = self.new_out_pt(idx, pt, op, op_prev);
            self.out_pts[op_prev].next = new_op;
            self.out_pts[op].prev = new_op;
            if to_front {
                self.poly_outs[out_rec].pts = new_op;
            }
            new_op
        }
    }

    /// `GetLastOutPt`. Returns the last point added for edge `e`.
    fn get_last_out_pt(&self, e: usize) -> usize {
        let pts = self.poly_outs[self.edges[e].out_idx as usize].pts;
        if self.edges[e].side == EdgeSide::Left {
            pts
        } else {
            self.out_pts[pts].prev
        }
    }

    /// `HorzSegmentsOverlap`. Returns true when the two X ranges overlap (open interval).
    fn horz_segments_overlap(&self, seg1a: CInt, seg1b: CInt, seg2a: CInt, seg2b: CInt) -> bool {
        let (seg1a, seg1b) = if seg1a > seg1b { (seg1b, seg1a) } else { (seg1a, seg1b) };
        let (seg2a, seg2b) = if seg2a > seg2b { (seg2b, seg2a) } else { (seg2a, seg2b) };
        seg1a < seg2b && seg2a < seg1b
    }

    /// `SetHoleState`: set `IsHole` / `FirstLeft` of new `out_rec` from the AEL left of `e`.
    fn set_hole_state(&mut self, e: usize, out_rec: usize) {
        let mut e2 = self.edges[e].prev_in_ael;
        let mut e_tmp = NIL;
        while e2 != NIL {
            if self.edges[e2].out_idx >= 0 && self.edges[e2].wind_delta != 0 {
                if e_tmp == NIL {
                    e_tmp = e2;
                } else if self.edges[e_tmp].out_idx == self.edges[e2].out_idx {
                    e_tmp = NIL; // paired
                }
            }
            e2 = self.edges[e2].prev_in_ael;
        }

        if e_tmp == NIL {
            self.poly_outs[out_rec].first_left = NIL;
            self.poly_outs[out_rec].is_hole = false;
        } else {
            let fl = self.edges[e_tmp].out_idx as usize;
            self.poly_outs[out_rec].first_left = fl;
            self.poly_outs[out_rec].is_hole = !self.poly_outs[fl].is_hole;
        }
    }

    /// `GetDx`. Returns slope dx/dy of the segment (horizontal marker if flat).
    fn get_dx(pt1: IntPoint, pt2: IntPoint) -> f64 {
        if pt1.y == pt2.y {
            HORIZONTAL
        } else {
            (pt2.x.wrapping_sub(pt1.x)) as f64 / (pt2.y.wrapping_sub(pt1.y)) as f64
        }
    }

    /// `FirstIsBottomPt`. Returns true when `btm_pt1` is the real bottom point vs `btm_pt2`.
    fn first_is_bottom_pt(&self, btm_pt1: usize, btm_pt2: usize) -> bool {
        let o = &self.out_pts;
        let mut p = o[btm_pt1].prev;
        while o[p].pt == o[btm_pt1].pt && p != btm_pt1 {
            p = o[p].prev;
        }
        let dx1p = Self::get_dx(o[btm_pt1].pt, o[p].pt).abs();
        p = o[btm_pt1].next;
        while o[p].pt == o[btm_pt1].pt && p != btm_pt1 {
            p = o[p].next;
        }
        let dx1n = Self::get_dx(o[btm_pt1].pt, o[p].pt).abs();

        p = o[btm_pt2].prev;
        while o[p].pt == o[btm_pt2].pt && p != btm_pt2 {
            p = o[p].prev;
        }
        let dx2p = Self::get_dx(o[btm_pt2].pt, o[p].pt).abs();
        p = o[btm_pt2].next;
        while o[p].pt == o[btm_pt2].pt && p != btm_pt2 {
            p = o[p].next;
        }
        let dx2n = Self::get_dx(o[btm_pt2].pt, o[p].pt).abs();

        if dx1p.max(dx1n) == dx2p.max(dx2n) && dx1p.min(dx1n) == dx2p.min(dx2n) {
            area_ring(o, btm_pt1) > 0.0 // if otherwise identical use orientation
        } else {
            (dx1p >= dx2p && dx1p >= dx2n) || (dx1n >= dx2p && dx1n >= dx2n)
        }
    }

    /// `GetBottomPt`. `pp` is a ring start. Returns its bottom-most (then left-most) point.
    fn get_bottom_pt(&self, mut pp: usize) -> usize {
        let o = &self.out_pts;
        let mut dups = NIL;
        let mut p = o[pp].next;
        while p != pp {
            if o[p].pt.y > o[pp].pt.y {
                pp = p;
                dups = NIL;
            } else if o[p].pt.y == o[pp].pt.y && o[p].pt.x <= o[pp].pt.x {
                if o[p].pt.x < o[pp].pt.x {
                    dups = NIL;
                    pp = p;
                } else if o[p].next != pp && o[p].prev != pp {
                    dups = p;
                }
            }
            p = o[p].next;
        }
        if dups != NIL {
            // there appears to be at least 2 vertices at bottomPt so ...
            while dups != p {
                if !self.first_is_bottom_pt(p, dups) {
                    pp = dups;
                }
                dups = o[dups].next;
                while o[dups].pt != o[pp].pt {
                    dups = o[dups].next;
                }
            }
        }
        pp
    }

    /// `GetLowermostRec`. Returns whichever record has the lower bottom point.
    fn get_lowermost_rec(&mut self, out_rec1: usize, out_rec2: usize) -> usize {
        // work out which polygon fragment has the correct hole state
        if self.poly_outs[out_rec1].bottom_pt == NIL {
            let b = self.get_bottom_pt(self.poly_outs[out_rec1].pts);
            self.poly_outs[out_rec1].bottom_pt = b;
        }
        if self.poly_outs[out_rec2].bottom_pt == NIL {
            let b = self.get_bottom_pt(self.poly_outs[out_rec2].pts);
            self.poly_outs[out_rec2].bottom_pt = b;
        }
        let b_pt1 = self.poly_outs[out_rec1].bottom_pt;
        let b_pt2 = self.poly_outs[out_rec2].bottom_pt;
        let (p1, p2) = (self.out_pts[b_pt1].pt, self.out_pts[b_pt2].pt);
        if p1.y > p2.y {
            out_rec1
        } else if p1.y < p2.y {
            out_rec2
        } else if p1.x < p2.x {
            out_rec1
        } else if p1.x > p2.x {
            out_rec2
        } else if self.out_pts[b_pt1].next == b_pt1 {
            out_rec2
        } else if self.out_pts[b_pt2].next == b_pt2 {
            out_rec1
        } else if self.first_is_bottom_pt(b_pt1, b_pt2) {
            out_rec1
        } else {
            out_rec2
        }
    }

    /// `OutRec1RightOfOutRec2`. Returns true when `out_rec2` is in the FirstLeft chain of 1.
    fn out_rec1_right_of_out_rec2(&self, out_rec1: usize, out_rec2: usize) -> bool {
        let mut r = out_rec1;
        loop {
            r = self.poly_outs[r].first_left;
            if r == out_rec2 {
                return true;
            }
            if r == NIL {
                break;
            }
        }
        false
    }

    /// `GetOutRec`: follow merged-record redirections from `idx`.
    fn get_out_rec(&self, idx: i32) -> usize {
        let mut outrec = idx as usize;
        while outrec != self.poly_outs[outrec].idx as usize {
            outrec = self.poly_outs[outrec].idx as usize;
        }
        outrec
    }

    /// `AppendPolygon`: join output poly of `e2` onto that of `e1`.
    fn append_polygon(&mut self, e1: usize, e2: usize) {
        let out_rec1 = self.edges[e1].out_idx as usize;
        let out_rec2 = self.edges[e2].out_idx as usize;

        let hole_state_rec = if self.out_rec1_right_of_out_rec2(out_rec1, out_rec2) {
            out_rec2
        } else if self.out_rec1_right_of_out_rec2(out_rec2, out_rec1) {
            out_rec1
        } else {
            self.get_lowermost_rec(out_rec1, out_rec2)
        };

        // get the start and ends of both output polygons and join E2 poly onto E1 poly
        let p1_lft = self.poly_outs[out_rec1].pts;
        let p1_rt = self.out_pts[p1_lft].prev;
        let p2_lft = self.poly_outs[out_rec2].pts;
        let p2_rt = self.out_pts[p2_lft].prev;

        if self.edges[e1].side == EdgeSide::Left {
            if self.edges[e2].side == EdgeSide::Left {
                // z y x a b c
                self.reverse_poly_pt_links(p2_lft);
                self.out_pts[p2_lft].next = p1_lft;
                self.out_pts[p1_lft].prev = p2_lft;
                self.out_pts[p1_rt].next = p2_rt;
                self.out_pts[p2_rt].prev = p1_rt;
                self.poly_outs[out_rec1].pts = p2_rt;
            } else {
                // x y z a b c
                self.out_pts[p2_rt].next = p1_lft;
                self.out_pts[p1_lft].prev = p2_rt;
                self.out_pts[p2_lft].prev = p1_rt;
                self.out_pts[p1_rt].next = p2_lft;
                self.poly_outs[out_rec1].pts = p2_lft;
            }
        } else if self.edges[e2].side == EdgeSide::Right {
            // a b c z y x
            self.reverse_poly_pt_links(p2_lft);
            self.out_pts[p1_rt].next = p2_rt;
            self.out_pts[p2_rt].prev = p1_rt;
            self.out_pts[p2_lft].next = p1_lft;
            self.out_pts[p1_lft].prev = p2_lft;
        } else {
            // a b c x y z
            self.out_pts[p1_rt].next = p2_lft;
            self.out_pts[p2_lft].prev = p1_rt;
            self.out_pts[p1_lft].prev = p2_rt;
            self.out_pts[p2_rt].next = p1_lft;
        }

        self.poly_outs[out_rec1].bottom_pt = NIL;
        if hole_state_rec == out_rec2 {
            if self.poly_outs[out_rec2].first_left != out_rec1 {
                self.poly_outs[out_rec1].first_left = self.poly_outs[out_rec2].first_left;
            }
            self.poly_outs[out_rec1].is_hole = self.poly_outs[out_rec2].is_hole;
        }
        self.poly_outs[out_rec2].pts = NIL;
        self.poly_outs[out_rec2].bottom_pt = NIL;

        self.poly_outs[out_rec2].first_left = out_rec1;

        let ok_idx = self.edges[e1].out_idx;
        let obsolete_idx = self.edges[e2].out_idx;

        self.edges[e1].out_idx = UNASSIGNED; // nb: safe because we only get here via AddLocalMaxPoly
        self.edges[e2].out_idx = UNASSIGNED;

        let mut e = self.active_edges;
        while e != NIL {
            if self.edges[e].out_idx == obsolete_idx {
                self.edges[e].out_idx = ok_idx;
                self.edges[e].side = self.edges[e1].side;
                break;
            }
            e = self.edges[e].next_in_ael;
        }
        self.poly_outs[out_rec2].idx = self.poly_outs[out_rec1].idx;
    }

    /// `ReversePolyPtLinks`: swap Next / Prev of every point in the ring at `pp`.
    fn reverse_poly_pt_links(&mut self, pp: usize) {
        if pp == NIL {
            return;
        }
        let mut pp1 = pp;
        loop {
            let pp2 = self.out_pts[pp1].next;
            self.out_pts[pp1].next = self.out_pts[pp1].prev;
            self.out_pts[pp1].prev = pp2;
            pp1 = pp2;
            if pp1 == pp {
                break;
            }
        }
    }

    /// `SwapSides`.
    fn swap_sides(&mut self, edge1: usize, edge2: usize) {
        let side = self.edges[edge1].side;
        self.edges[edge1].side = self.edges[edge2].side;
        self.edges[edge2].side = side;
    }

    /// `SwapPolyIndexes`.
    fn swap_poly_indexes(&mut self, edge1: usize, edge2: usize) {
        let out_idx = self.edges[edge1].out_idx;
        self.edges[edge1].out_idx = self.edges[edge2].out_idx;
        self.edges[edge2].out_idx = out_idx;
    }

    /// `IntersectEdges`: handle crossing of `e1` and `e2` at `pt`; updates winding and output.
    fn intersect_edges(&mut self, e1: usize, e2: usize, pt: IntPoint) {
        // e1 will be left of e2 BELOW the intersection. Therefore e1 is before e2 in AEL
        // except when e1 is being inserted at the intersection point
        let e1_contributing = self.edges[e1].out_idx >= 0;
        let e2_contributing = self.edges[e2].out_idx >= 0;

        // if either edge is on an OPEN path
        if self.edges[e1].wind_delta == 0 || self.edges[e2].wind_delta == 0 {
            // ignore subject-subject open path intersections UNLESS both are open paths
            // AND both are 'contributing maximas'
            if self.edges[e1].wind_delta == 0 && self.edges[e2].wind_delta == 0 {
                return;
            } else if self.edges[e1].poly_typ == self.edges[e2].poly_typ
                && self.edges[e1].wind_delta != self.edges[e2].wind_delta
                && self.clip_type == ClipType::Union
            {
                // intersecting a subj line with a subj poly
                if self.edges[e1].wind_delta == 0 {
                    if e2_contributing {
                        self.add_out_pt(e1, pt);
                        if e1_contributing {
                            self.edges[e1].out_idx = UNASSIGNED;
                        }
                    }
                } else if e1_contributing {
                    self.add_out_pt(e2, pt);
                    if e2_contributing {
                        self.edges[e2].out_idx = UNASSIGNED;
                    }
                }
            } else if self.edges[e1].poly_typ != self.edges[e2].poly_typ {
                if self.edges[e1].wind_delta == 0
                    && self.edges[e2].wind_cnt.abs() == 1
                    && (self.clip_type != ClipType::Union || self.edges[e2].wind_cnt2 == 0)
                {
                    self.add_out_pt(e1, pt);
                    if e1_contributing {
                        self.edges[e1].out_idx = UNASSIGNED;
                    }
                } else if self.edges[e2].wind_delta == 0
                    && self.edges[e1].wind_cnt.abs() == 1
                    && (self.clip_type != ClipType::Union || self.edges[e1].wind_cnt2 == 0)
                {
                    self.add_out_pt(e2, pt);
                    if e2_contributing {
                        self.edges[e2].out_idx = UNASSIGNED;
                    }
                }
            }
            return;
        }

        // update winding counts; assumes e1 will be to the Right of e2 ABOVE the intersection
        if self.edges[e1].poly_typ == self.edges[e2].poly_typ {
            if self.is_even_odd_fill_type(e1) {
                let old_e1_wind_cnt = self.edges[e1].wind_cnt;
                self.edges[e1].wind_cnt = self.edges[e2].wind_cnt;
                self.edges[e2].wind_cnt = old_e1_wind_cnt;
            } else {
                if self.edges[e1].wind_cnt + self.edges[e2].wind_delta == 0 {
                    self.edges[e1].wind_cnt = -self.edges[e1].wind_cnt;
                } else {
                    self.edges[e1].wind_cnt += self.edges[e2].wind_delta;
                }
                if self.edges[e2].wind_cnt - self.edges[e1].wind_delta == 0 {
                    self.edges[e2].wind_cnt = -self.edges[e2].wind_cnt;
                } else {
                    self.edges[e2].wind_cnt -= self.edges[e1].wind_delta;
                }
            }
        } else {
            if !self.is_even_odd_fill_type(e2) {
                self.edges[e1].wind_cnt2 += self.edges[e2].wind_delta;
            } else {
                self.edges[e1].wind_cnt2 = if self.edges[e1].wind_cnt2 == 0 { 1 } else { 0 };
            }
            if !self.is_even_odd_fill_type(e1) {
                self.edges[e2].wind_cnt2 -= self.edges[e1].wind_delta;
            } else {
                self.edges[e2].wind_cnt2 = if self.edges[e2].wind_cnt2 == 0 { 1 } else { 0 };
            }
        }

        let (e1_fill_type, e1_fill_type2) = if self.edges[e1].poly_typ == PolyType::Subject {
            (self.subj_fill_type, self.clip_fill_type)
        } else {
            (self.clip_fill_type, self.subj_fill_type)
        };
        let (e2_fill_type, e2_fill_type2) = if self.edges[e2].poly_typ == PolyType::Subject {
            (self.subj_fill_type, self.clip_fill_type)
        } else {
            (self.clip_fill_type, self.subj_fill_type)
        };

        let e1_wc = match e1_fill_type {
            PolyFillType::Positive => self.edges[e1].wind_cnt,
            PolyFillType::Negative => -self.edges[e1].wind_cnt,
            _ => self.edges[e1].wind_cnt.abs(),
        };
        let e2_wc = match e2_fill_type {
            PolyFillType::Positive => self.edges[e2].wind_cnt,
            PolyFillType::Negative => -self.edges[e2].wind_cnt,
            _ => self.edges[e2].wind_cnt.abs(),
        };

        if e1_contributing && e2_contributing {
            if (e1_wc != 0 && e1_wc != 1)
                || (e2_wc != 0 && e2_wc != 1)
                || (self.edges[e1].poly_typ != self.edges[e2].poly_typ
                    && self.clip_type != ClipType::Xor)
            {
                self.add_local_max_poly(e1, e2, pt);
            } else {
                self.add_out_pt(e1, pt);
                self.add_out_pt(e2, pt);
                self.swap_sides(e1, e2);
                self.swap_poly_indexes(e1, e2);
            }
        } else if e1_contributing {
            if e2_wc == 0 || e2_wc == 1 {
                self.add_out_pt(e1, pt);
                self.swap_sides(e1, e2);
                self.swap_poly_indexes(e1, e2);
            }
        } else if e2_contributing {
            if e1_wc == 0 || e1_wc == 1 {
                self.add_out_pt(e2, pt);
                self.swap_sides(e1, e2);
                self.swap_poly_indexes(e1, e2);
            }
        } else if (e1_wc == 0 || e1_wc == 1) && (e2_wc == 0 || e2_wc == 1) {
            // neither edge is currently contributing
            let e1_wc2: CInt = match e1_fill_type2 {
                PolyFillType::Positive => self.edges[e1].wind_cnt2 as CInt,
                PolyFillType::Negative => -(self.edges[e1].wind_cnt2 as CInt),
                _ => (self.edges[e1].wind_cnt2 as CInt).abs(),
            };
            let e2_wc2: CInt = match e2_fill_type2 {
                PolyFillType::Positive => self.edges[e2].wind_cnt2 as CInt,
                PolyFillType::Negative => -(self.edges[e2].wind_cnt2 as CInt),
                _ => (self.edges[e2].wind_cnt2 as CInt).abs(),
            };

            if self.edges[e1].poly_typ != self.edges[e2].poly_typ {
                self.add_local_min_poly(e1, e2, pt);
            } else if e1_wc == 1 && e2_wc == 1 {
                match self.clip_type {
                    ClipType::Intersection => {
                        if e1_wc2 > 0 && e2_wc2 > 0 {
                            self.add_local_min_poly(e1, e2, pt);
                        }
                    }
                    ClipType::Union => {
                        if e1_wc2 <= 0 && e2_wc2 <= 0 {
                            self.add_local_min_poly(e1, e2, pt);
                        }
                    }
                    ClipType::Difference => {
                        if (self.edges[e1].poly_typ == PolyType::Clip && e1_wc2 > 0 && e2_wc2 > 0)
                            || (self.edges[e1].poly_typ == PolyType::Subject
                                && e1_wc2 <= 0
                                && e2_wc2 <= 0)
                        {
                            self.add_local_min_poly(e1, e2, pt);
                        }
                    }
                    ClipType::Xor => {
                        self.add_local_min_poly(e1, e2, pt);
                    }
                }
            } else {
                self.swap_sides(e1, e2);
            }
        }
    }

    /// `ProcessHorizontals`: drain the SEL.
    fn process_horizontals(&mut self) -> Res<()> {
        while let Some(horz_edge) = self.pop_edge_from_sel() {
            self.process_horizontal(horz_edge)?;
        }
        Ok(())
    }

    /// `GetHorzDirection`. Returns (direction, left X, right X) of the horizontal edge.
    fn get_horz_direction(&self, horz_edge: usize) -> (Direction, CInt, CInt) {
        let h = &self.edges[horz_edge];
        if h.bot.x < h.top.x {
            (Direction::LeftToRight, h.bot.x, h.top.x)
        } else {
            (Direction::RightToLeft, h.top.x, h.bot.x)
        }
    }

    /// `ProcessHorizontal`: walk a horizontal edge (and consecutive ones) across the AEL.
    fn process_horizontal(&mut self, mut horz_edge: usize) -> Res<()> {
        let is_open = self.edges[horz_edge].wind_delta == 0;

        let (mut dir, mut horz_left, mut horz_right) = self.get_horz_direction(horz_edge);

        let mut e_last_horz = horz_edge;
        let mut e_max_pair = NIL;
        while self.edges[e_last_horz].next_in_lml != NIL
            && self.edges[self.edges[e_last_horz].next_in_lml].delta.y == 0
        {
            e_last_horz = self.edges[e_last_horz].next_in_lml;
        }
        if self.edges[e_last_horz].next_in_lml == NIL {
            e_max_pair = self.get_maxima_pair(e_last_horz);
        }

        let mut curr_max = self.maxima_head;
        if curr_max != NIL {
            // get the first maxima in range (X)
            if dir == Direction::LeftToRight {
                while curr_max != NIL && self.maxima[curr_max].x <= self.edges[horz_edge].bot.x {
                    curr_max = self.maxima[curr_max].next;
                }
                if curr_max != NIL && self.maxima[curr_max].x >= self.edges[e_last_horz].top.x {
                    curr_max = NIL;
                }
            } else {
                while self.maxima[curr_max].next != NIL
                    && self.maxima[self.maxima[curr_max].next].x < self.edges[horz_edge].bot.x
                {
                    curr_max = self.maxima[curr_max].next;
                }
                if self.maxima[curr_max].x <= self.edges[e_last_horz].top.x {
                    curr_max = NIL;
                }
            }
        }

        let mut op1 = NIL;
        loop {
            // loop through consec. horizontal edges
            let is_last_horz = horz_edge == e_last_horz;
            let mut e = self.get_next_in_ael(horz_edge, dir);
            while e != NIL {
                // insert extra coords into horizontal edges (in output polygons) wherever
                // maxima touch these horizontal edges; helps 'simplifying' polygons
                if curr_max != NIL {
                    if dir == Direction::LeftToRight {
                        while curr_max != NIL && self.maxima[curr_max].x < self.edges[e].curr.x {
                            if self.edges[horz_edge].out_idx >= 0 && !is_open {
                                let p = IntPoint::new(
                                    self.maxima[curr_max].x,
                                    self.edges[horz_edge].bot.y,
                                );
                                self.add_out_pt(horz_edge, p);
                            }
                            curr_max = self.maxima[curr_max].next;
                        }
                    } else {
                        while curr_max != NIL && self.maxima[curr_max].x > self.edges[e].curr.x {
                            if self.edges[horz_edge].out_idx >= 0 && !is_open {
                                let p = IntPoint::new(
                                    self.maxima[curr_max].x,
                                    self.edges[horz_edge].bot.y,
                                );
                                self.add_out_pt(horz_edge, p);
                            }
                            curr_max = self.maxima[curr_max].prev;
                        }
                    }
                }

                if (dir == Direction::LeftToRight && self.edges[e].curr.x > horz_right)
                    || (dir == Direction::RightToLeft && self.edges[e].curr.x < horz_left)
                {
                    break;
                }

                // also break at the end of an intermediate horizontal edge
                // nb: smaller Dx's are right of larger Dx's ABOVE the horizontal
                if self.edges[e].curr.x == self.edges[horz_edge].top.x
                    && self.edges[horz_edge].next_in_lml != NIL
                    && self.edges[e].dx < self.edges[self.edges[horz_edge].next_in_lml].dx
                {
                    break;
                }

                if self.edges[horz_edge].out_idx >= 0 && !is_open {
                    // note: may be done multiple times
                    op1 = self.add_out_pt(horz_edge, self.edges[e].curr);
                    let mut e_next_horz = self.sorted_edges;
                    while e_next_horz != NIL {
                        if self.edges[e_next_horz].out_idx >= 0
                            && self.horz_segments_overlap(
                                self.edges[horz_edge].bot.x,
                                self.edges[horz_edge].top.x,
                                self.edges[e_next_horz].bot.x,
                                self.edges[e_next_horz].top.x,
                            )
                        {
                            let op2 = self.get_last_out_pt(e_next_horz);
                            self.add_join(op2, op1, self.edges[e_next_horz].top);
                        }
                        e_next_horz = self.edges[e_next_horz].next_in_sel;
                    }
                    self.add_ghost_join(op1, self.edges[horz_edge].bot);
                }

                // OK, so far we're still in range of the horizontal edge but make sure
                // we're at the last of consec. horizontals when matching with e_max_pair
                if e == e_max_pair && is_last_horz {
                    if self.edges[horz_edge].out_idx >= 0 {
                        let top = self.edges[horz_edge].top;
                        self.add_local_max_poly(horz_edge, e_max_pair, top);
                    }
                    self.delete_from_ael(horz_edge);
                    self.delete_from_ael(e_max_pair);
                    return Ok(());
                }

                let pt = IntPoint::new(self.edges[e].curr.x, self.edges[horz_edge].curr.y);
                if dir == Direction::LeftToRight {
                    self.intersect_edges(horz_edge, e, pt);
                } else {
                    self.intersect_edges(e, horz_edge, pt);
                }
                let e_next = self.get_next_in_ael(e, dir);
                self.swap_positions_in_ael(horz_edge, e);
                e = e_next;
            } // end while e != NIL

            // break out of loop if horz_edge.NextInLML is not also horizontal
            let nl = self.edges[horz_edge].next_in_lml;
            if nl == NIL || self.edges[nl].delta.y != 0 {
                break;
            }

            self.update_edge_into_ael(&mut horz_edge)?;
            if self.edges[horz_edge].out_idx >= 0 {
                self.add_out_pt(horz_edge, self.edges[horz_edge].bot);
            }
            let (d, l, r) = self.get_horz_direction(horz_edge);
            dir = d;
            horz_left = l;
            horz_right = r;
        } // end loop

        if self.edges[horz_edge].out_idx >= 0 && op1 == NIL {
            op1 = self.get_last_out_pt(horz_edge);
            let mut e_next_horz = self.sorted_edges;
            while e_next_horz != NIL {
                if self.edges[e_next_horz].out_idx >= 0
                    && self.horz_segments_overlap(
                        self.edges[horz_edge].bot.x,
                        self.edges[horz_edge].top.x,
                        self.edges[e_next_horz].bot.x,
                        self.edges[e_next_horz].top.x,
                    )
                {
                    let op2 = self.get_last_out_pt(e_next_horz);
                    self.add_join(op2, op1, self.edges[e_next_horz].top);
                }
                e_next_horz = self.edges[e_next_horz].next_in_sel;
            }
            self.add_ghost_join(op1, self.edges[horz_edge].top);
        }

        if self.edges[horz_edge].next_in_lml != NIL {
            if self.edges[horz_edge].out_idx >= 0 {
                op1 = self.add_out_pt(horz_edge, self.edges[horz_edge].top);

                self.update_edge_into_ael(&mut horz_edge)?;
                if self.edges[horz_edge].wind_delta == 0 {
                    return Ok(());
                }
                // nb: horz_edge is no longer horizontal here
                let e_prev = self.edges[horz_edge].prev_in_ael;
                let e_next = self.edges[horz_edge].next_in_ael;
                let hb = self.edges[horz_edge].bot;
                if e_prev != NIL
                    && self.edges[e_prev].curr.x == hb.x
                    && self.edges[e_prev].curr.y == hb.y
                    && self.edges[e_prev].wind_delta != 0
                    && (self.edges[e_prev].out_idx >= 0
                        && self.edges[e_prev].curr.y > self.edges[e_prev].top.y
                        && slopes_equal_edges(
                            &self.edges[horz_edge],
                            &self.edges[e_prev],
                            self.use_full_range,
                        ))
                {
                    let op2 = self.add_out_pt(e_prev, hb);
                    self.add_join(op1, op2, self.edges[horz_edge].top);
                } else if e_next != NIL
                    && self.edges[e_next].curr.x == hb.x
                    && self.edges[e_next].curr.y == hb.y
                    && self.edges[e_next].wind_delta != 0
                    && self.edges[e_next].out_idx >= 0
                    && self.edges[e_next].curr.y > self.edges[e_next].top.y
                    && slopes_equal_edges(
                        &self.edges[horz_edge],
                        &self.edges[e_next],
                        self.use_full_range,
                    )
                {
                    let op2 = self.add_out_pt(e_next, hb);
                    self.add_join(op1, op2, self.edges[horz_edge].top);
                }
            } else {
                self.update_edge_into_ael(&mut horz_edge)?;
            }
        } else {
            if self.edges[horz_edge].out_idx >= 0 {
                self.add_out_pt(horz_edge, self.edges[horz_edge].top);
            }
            self.delete_from_ael(horz_edge);
        }
        Ok(())
    }

    /// `GetNextInAEL`. Returns the AEL neighbour of `e` in direction `dir`.
    fn get_next_in_ael(&self, e: usize, dir: Direction) -> usize {
        if dir == Direction::LeftToRight {
            self.edges[e].next_in_ael
        } else {
            self.edges[e].prev_in_ael
        }
    }

    /// `IsMaxima`.
    fn is_maxima(&self, e: usize, y: CInt) -> bool {
        e != NIL && self.edges[e].top.y == y && self.edges[e].next_in_lml == NIL
    }

    /// `IsIntermediate`.
    fn is_intermediate(&self, e: usize, y: CInt) -> bool {
        self.edges[e].top.y == y && self.edges[e].next_in_lml != NIL
    }

    /// `GetMaximaPair`. Returns the edge ending at the same top point, or NIL.
    fn get_maxima_pair(&self, e: usize) -> usize {
        let (n, p) = (self.nx(e), self.pv(e));
        if self.edges[n].top == self.edges[e].top && self.edges[n].next_in_lml == NIL {
            n
        } else if self.edges[p].top == self.edges[e].top && self.edges[p].next_in_lml == NIL {
            p
        } else {
            NIL
        }
    }

    /// `GetMaximaPairEx`: as above but NIL if the pair isn't in the AEL (unless horizontal).
    fn get_maxima_pair_ex(&self, e: usize) -> usize {
        let result = self.get_maxima_pair(e);
        if result == NIL
            || self.edges[result].out_idx == SKIP
            || (self.edges[result].next_in_ael == self.edges[result].prev_in_ael
                && self.edges[result].delta.y != 0)
        {
            return NIL;
        }
        result
    }

    /// `ProcessIntersections`. `top_y` is the scanbeam top. Returns false on failure.
    fn process_intersections(&mut self, top_y: CInt) -> bool {
        if self.active_edges == NIL {
            return true;
        }
        self.build_intersect_list(top_y);
        if self.intersect_list.is_empty() {
            return true;
        }
        if self.intersect_list.len() == 1 || self.fixup_intersection_order() {
            self.process_intersect_list();
        } else {
            return false;
        }
        self.sorted_edges = NIL;
        true
    }

    /// `BuildIntersectList`: bubble sort the AEL to top of scanbeam, recording crossings.
    fn build_intersect_list(&mut self, top_y: CInt) {
        if self.active_edges == NIL {
            return;
        }

        // prepare for sorting
        let mut e = self.active_edges;
        self.sorted_edges = e;
        while e != NIL {
            self.edges[e].prev_in_sel = self.edges[e].prev_in_ael;
            self.edges[e].next_in_sel = self.edges[e].next_in_ael;
            let x = top_x(&self.edges[e], top_y);
            self.edges[e].curr.x = x;
            e = self.edges[e].next_in_ael;
        }

        // bubblesort
        let mut is_modified = true;
        while is_modified && self.sorted_edges != NIL {
            is_modified = false;
            e = self.sorted_edges;
            while self.edges[e].next_in_sel != NIL {
                let e_next = self.edges[e].next_in_sel;
                if self.edges[e].curr.x > self.edges[e_next].curr.x {
                    let mut pt = intersect_point(&self.edges[e], &self.edges[e_next]);
                    if pt.y < top_y {
                        pt = IntPoint::new(top_x(&self.edges[e], top_y), top_y);
                    }
                    self.intersect_list.push(IntersectNode { edge1: e, edge2: e_next, pt });
                    // mirror List<T> growth for the sort depth limit
                    if self.intersect_list.len() > self.intersect_capacity {
                        self.intersect_capacity =
                            if self.intersect_capacity == 0 { 4 } else { self.intersect_capacity * 2 };
                    }

                    self.swap_positions_in_sel(e, e_next);
                    is_modified = true;
                } else {
                    e = e_next;
                }
            }
            let ep = self.edges[e].prev_in_sel;
            if ep != NIL {
                self.edges[ep].next_in_sel = NIL;
            } else {
                break;
            }
        }
        self.sorted_edges = NIL;
    }

    /// `EdgesAdjacent`. Returns true when the node's edges touch in the SEL.
    fn edges_adjacent(&self, inode: &IntersectNode) -> bool {
        self.edges[inode.edge1].next_in_sel == inode.edge2
            || self.edges[inode.edge1].prev_in_sel == inode.edge2
    }

    /// `FixupIntersectionOrder`: sort by Y and make sure only adjacent edges intersect.
    fn fixup_intersection_order(&mut self) -> bool {
        // pre-condition: intersections are sorted bottom-most first
        let cmp = |a: &IntersectNode, b: &IntersectNode| -> i32 {
            // MyIntersectNodeSort.Compare
            let i = b.pt.y.wrapping_sub(a.pt.y);
            if i > 0 {
                1
            } else if i < 0 {
                -1
            } else {
                0
            }
        };
        list_sort(&mut self.intersect_list, self.intersect_capacity, &cmp);

        self.copy_ael_to_sel();
        let cnt = self.intersect_list.len();
        for i in 0..cnt {
            if !self.edges_adjacent(&self.intersect_list[i]) {
                let mut j = i + 1;
                while j < cnt && !self.edges_adjacent(&self.intersect_list[j]) {
                    j += 1;
                }
                if j == cnt {
                    return false;
                }
                self.intersect_list.swap(i, j);
            }
            let (a, b) = (self.intersect_list[i].edge1, self.intersect_list[i].edge2);
            self.swap_positions_in_sel(a, b);
        }
        true
    }

    /// `ProcessIntersectList`: apply all recorded crossings in order.
    fn process_intersect_list(&mut self) {
        for i in 0..self.intersect_list.len() {
            let n = self.intersect_list[i];
            self.intersect_edges(n.edge1, n.edge2, n.pt);
            self.swap_positions_in_ael(n.edge1, n.edge2);
        }
        self.intersect_list.clear();
    }

    /// `ProcessEdgesAtTopOfScanbeam`. `top_y` is the scanline just reached.
    fn process_edges_at_top_of_scanbeam(&mut self, top_y: CInt) -> Res<()> {
        let mut e = self.active_edges;
        while e != NIL {
            // 1. process maxima, treating them as if they're 'bent' horizontal edges,
            //    but exclude maxima with horizontal edges. nb: e can't be a horizontal.
            let mut is_maxima_edge = self.is_maxima(e, top_y);

            if is_maxima_edge {
                let e_max_pair = self.get_maxima_pair_ex(e);
                is_maxima_edge = e_max_pair == NIL || self.edges[e_max_pair].delta.y != 0;
            }

            if is_maxima_edge {
                if self.strictly_simple {
                    let x = self.edges[e].top.x;
                    self.insert_maxima(x);
                }
                let e_prev = self.edges[e].prev_in_ael;
                self.do_maxima(e)?;
                if e_prev == NIL {
                    e = self.active_edges;
                } else {
                    e = self.edges[e_prev].next_in_ael;
                }
            } else {
                // 2. promote horizontal edges, otherwise update Curr.X and Curr.Y
                if self.is_intermediate(e, top_y)
                    && self.edges[self.edges[e].next_in_lml].delta.y == 0
                {
                    self.update_edge_into_ael(&mut e)?;
                    if self.edges[e].out_idx >= 0 {
                        self.add_out_pt(e, self.edges[e].bot);
                    }
                    self.add_edge_to_sel(e);
                } else {
                    let x = top_x(&self.edges[e], top_y);
                    self.edges[e].curr.x = x;
                    self.edges[e].curr.y = top_y;
                }
                // when StrictlySimple and e is touched by another edge, make sure both
                // edges have a vertex here
                if self.strictly_simple {
                    let e_prev = self.edges[e].prev_in_ael;
                    if self.edges[e].out_idx >= 0
                        && self.edges[e].wind_delta != 0
                        && e_prev != NIL
                        && self.edges[e_prev].out_idx >= 0
                        && self.edges[e_prev].curr.x == self.edges[e].curr.x
                        && self.edges[e_prev].wind_delta != 0
                    {
                        let ip = self.edges[e].curr;
                        let op = self.add_out_pt(e_prev, ip);
                        let op2 = self.add_out_pt(e, ip);
                        self.add_join(op, op2, ip); // StrictlySimple (type-3) join
                    }
                }

                e = self.edges[e].next_in_ael;
            }
        }

        // 3. process horizontals at the top of the scanbeam
        self.process_horizontals()?;
        self.maxima.clear();
        self.maxima_head = NIL;

        // 4. promote intermediate vertices
        e = self.active_edges;
        while e != NIL {
            if self.is_intermediate(e, top_y) {
                let mut op = NIL;
                if self.edges[e].out_idx >= 0 {
                    op = self.add_out_pt(e, self.edges[e].top);
                }
                self.update_edge_into_ael(&mut e)?;

                // if output polygons share an edge, they'll need joining later
                let e_prev = self.edges[e].prev_in_ael;
                let e_next = self.edges[e].next_in_ael;
                let eb = self.edges[e].bot;
                if e_prev != NIL
                    && self.edges[e_prev].curr.x == eb.x
                    && self.edges[e_prev].curr.y == eb.y
                    && op != NIL
                    && self.edges[e_prev].out_idx >= 0
                    && self.edges[e_prev].curr.y > self.edges[e_prev].top.y
                    && slopes_equal4(
                        self.edges[e].curr,
                        self.edges[e].top,
                        self.edges[e_prev].curr,
                        self.edges[e_prev].top,
                        self.use_full_range,
                    )
                    && self.edges[e].wind_delta != 0
                    && self.edges[e_prev].wind_delta != 0
                {
                    let op2 = self.add_out_pt(e_prev, eb);
                    self.add_join(op, op2, self.edges[e].top);
                } else if e_next != NIL
                    && self.edges[e_next].curr.x == eb.x
                    && self.edges[e_next].curr.y == eb.y
                    && op != NIL
                    && self.edges[e_next].out_idx >= 0
                    && self.edges[e_next].curr.y > self.edges[e_next].top.y
                    && slopes_equal4(
                        self.edges[e].curr,
                        self.edges[e].top,
                        self.edges[e_next].curr,
                        self.edges[e_next].top,
                        self.use_full_range,
                    )
                    && self.edges[e].wind_delta != 0
                    && self.edges[e_next].wind_delta != 0
                {
                    let op2 = self.add_out_pt(e_next, eb);
                    self.add_join(op, op2, self.edges[e].top);
                }
            }
            e = self.edges[e].next_in_ael;
        }
        Ok(())
    }

    /// `DoMaxima`: close the poly at the maxima of `e` and its pair.
    fn do_maxima(&mut self, e: usize) -> Res<()> {
        let e_max_pair = self.get_maxima_pair_ex(e);
        if e_max_pair == NIL {
            if self.edges[e].out_idx >= 0 {
                self.add_out_pt(e, self.edges[e].top);
            }
            self.delete_from_ael(e);
            return Ok(());
        }

        let mut e_next = self.edges[e].next_in_ael;
        while e_next != NIL && e_next != e_max_pair {
            let top = self.edges[e].top;
            self.intersect_edges(e, e_next, top);
            self.swap_positions_in_ael(e, e_next);
            e_next = self.edges[e].next_in_ael;
        }

        if self.edges[e].out_idx == UNASSIGNED && self.edges[e_max_pair].out_idx == UNASSIGNED {
            self.delete_from_ael(e);
            self.delete_from_ael(e_max_pair);
        } else if self.edges[e].out_idx >= 0 && self.edges[e_max_pair].out_idx >= 0 {
            if self.edges[e].out_idx >= 0 {
                let top = self.edges[e].top;
                self.add_local_max_poly(e, e_max_pair, top);
            }
            self.delete_from_ael(e);
            self.delete_from_ael(e_max_pair);
        } else if self.edges[e].wind_delta == 0 {
            if self.edges[e].out_idx >= 0 {
                self.add_out_pt(e, self.edges[e].top);
                self.edges[e].out_idx = UNASSIGNED;
            }
            self.delete_from_ael(e);

            if self.edges[e_max_pair].out_idx >= 0 {
                self.add_out_pt(e_max_pair, self.edges[e].top);
                self.edges[e_max_pair].out_idx = UNASSIGNED;
            }
            self.delete_from_ael(e_max_pair);
        } else {
            return Err(ClipperException("DoMaxima error".into()));
        }
        Ok(())
    }

    /// `ReversePaths`: reverse every path in place. `polys` are the paths.
    pub fn reverse_paths(polys: &mut [Path]) {
        for poly in polys.iter_mut() {
            poly.reverse();
        }
    }

    /// `Orientation`. `poly` is a closed path. Returns true when area >= 0.
    pub fn orientation(poly: &[IntPoint]) -> bool {
        Self::area(poly) >= 0.0
    }

    /// `PointCount`: number of points in ring at `pts` (0 for NIL).
    fn point_count(&self, pts: usize) -> usize {
        if pts == NIL {
            return 0;
        }
        let mut result = 0;
        let mut p = pts;
        loop {
            result += 1;
            p = self.out_pts[p].next;
            if p == pts {
                break;
            }
        }
        result
    }

    /// `BuildResult`: output rings into `polyg`.
    fn build_result(&self, polyg: &mut Paths) {
        polyg.clear();
        for out_rec in &self.poly_outs {
            if out_rec.pts == NIL {
                continue;
            }
            let mut p = self.out_pts[out_rec.pts].prev;
            let cnt = self.point_count(p);
            if cnt < 2 {
                continue;
            }
            let mut pg = Path::with_capacity(cnt);
            for _ in 0..cnt {
                pg.push(self.out_pts[p].pt);
                p = self.out_pts[p].prev;
            }
            polyg.push(pg);
        }
    }

    /// `BuildResult2`: output rings into `polytree`.
    fn build_result2(&mut self, polytree: &mut PolyTree) {
        polytree.clear();

        // add each output polygon/contour to polytree
        for i in 0..self.poly_outs.len() {
            let cnt = self.point_count(self.poly_outs[i].pts);
            let is_open = self.poly_outs[i].is_open;
            if (is_open && cnt < 2) || (!is_open && cnt < 3) {
                continue;
            }
            self.fix_hole_linkage(i);
            let mut polygon = Path::with_capacity(cnt);
            let mut op = self.out_pts[self.poly_outs[i].pts].prev;
            for _ in 0..cnt {
                polygon.push(self.out_pts[op].pt);
                op = self.out_pts[op].prev;
            }
            let pn = polytree.new_node(polygon);
            polytree.all_polys.push(pn);
            self.poly_outs[i].poly_node = Some(pn);
        }

        // fixup PolyNode links etc
        for i in 0..self.poly_outs.len() {
            let Some(pn) = self.poly_outs[i].poly_node else {
                continue;
            };
            if self.poly_outs[i].is_open {
                polytree.nodes[pn].is_open = true;
                polytree.add_child(PolyTree::ROOT, pn);
            } else if self.poly_outs[i].first_left != NIL
                && self.poly_outs[self.poly_outs[i].first_left].poly_node.is_some()
            {
                let parent = self.poly_outs[self.poly_outs[i].first_left].poly_node.unwrap_or(0);
                polytree.add_child(parent, pn);
            } else {
                polytree.add_child(PolyTree::ROOT, pn);
            }
        }
    }

    /// `FixupOutPolyline`: remove duplicate points from an open output path.
    fn fixup_out_polyline(&mut self, outrec: usize) {
        let mut pp = self.poly_outs[outrec].pts;
        let mut last_pp = self.out_pts[pp].prev;
        while pp != last_pp {
            pp = self.out_pts[pp].next;
            if self.out_pts[pp].pt == self.out_pts[self.out_pts[pp].prev].pt {
                if pp == last_pp {
                    last_pp = self.out_pts[pp].prev;
                }
                let tmp_pp = self.out_pts[pp].prev;
                let pn = self.out_pts[pp].next;
                self.out_pts[tmp_pp].next = pn;
                self.out_pts[pn].prev = tmp_pp;
                pp = tmp_pp;
            }
        }
        if pp == self.out_pts[pp].prev {
            self.poly_outs[outrec].pts = NIL;
        }
    }

    /// `FixupOutPolygon`: remove duplicate points and collinear middle vertices.
    fn fixup_out_polygon(&mut self, out_rec: usize) {
        let mut last_ok = NIL;
        self.poly_outs[out_rec].bottom_pt = NIL;
        let mut pp = self.poly_outs[out_rec].pts;
        let preserve_col = self.preserve_collinear || self.strictly_simple;
        loop {
            let (ppp, ppn) = (self.out_pts[pp].prev, self.out_pts[pp].next);
            if ppp == pp || ppp == ppn {
                self.poly_outs[out_rec].pts = NIL;
                return;
            }
            // test for duplicate points and collinear edges
            let (pt, pt_prev, pt_next) =
                (self.out_pts[pp].pt, self.out_pts[ppp].pt, self.out_pts[ppn].pt);
            if pt == pt_next
                || pt == pt_prev
                || (slopes_equal3(pt_prev, pt, pt_next, self.use_full_range)
                    && (!preserve_col || !pt2_is_between_pt1_and_pt3(pt_prev, pt, pt_next)))
            {
                last_ok = NIL;
                self.out_pts[ppp].next = ppn;
                self.out_pts[ppn].prev = ppp;
                pp = ppp;
            } else if pp == last_ok {
                break;
            } else {
                if last_ok == NIL {
                    last_ok = pp;
                }
                pp = ppn;
            }
        }
        self.poly_outs[out_rec].pts = pp;
    }

    /// `DupOutPt`: duplicate point `out_pt` after (`insert_after`) or before it. Returns the copy.
    fn dup_out_pt(&mut self, out_pt: usize, insert_after: bool) -> usize {
        let (pt, idx) = (self.out_pts[out_pt].pt, self.out_pts[out_pt].idx);
        if insert_after {
            let next = self.out_pts[out_pt].next;
            let result = self.new_out_pt(idx, pt, next, out_pt);
            self.out_pts[next].prev = result;
            self.out_pts[out_pt].next = result;
            result
        } else {
            let prev = self.out_pts[out_pt].prev;
            let result = self.new_out_pt(idx, pt, out_pt, prev);
            self.out_pts[prev].next = result;
            self.out_pts[out_pt].prev = result;
            result
        }
    }

    /// `GetOverlap`. Returns (overlaps, left, right) of the two ranges.
    fn get_overlap(a1: CInt, a2: CInt, b1: CInt, b2: CInt) -> (bool, CInt, CInt) {
        let (left, right);
        if a1 < a2 {
            if b1 < b2 {
                left = a1.max(b1);
                right = a2.min(b2);
            } else {
                left = a1.max(b2);
                right = a2.min(b1);
            }
        } else if b1 < b2 {
            left = a2.max(b1);
            right = a1.min(b2);
        } else {
            left = a2.max(b2);
            right = a1.min(b1);
        }
        (left < right, left, right)
    }

    /// `JoinHorz`: splice two overlapping horizontal runs at `pt`. Returns false if directions match.
    fn join_horz(
        &mut self,
        mut op1: usize,
        mut op1b: usize,
        mut op2: usize,
        mut op2b: usize,
        pt: IntPoint,
        discard_left: bool,
    ) -> bool {
        let dir1 = if self.out_pts[op1].pt.x > self.out_pts[op1b].pt.x {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let dir2 = if self.out_pts[op2].pt.x > self.out_pts[op2b].pt.x {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        if dir1 == dir2 {
            return false;
        }

        // When discard_left, we want op1b on the Left of op1, otherwise on the Right (likewise
        // op2 / op2b). So while inserting op1b / op2b: when discard_left make sure we're AT or
        // RIGHT of pt before adding op1b, otherwise AT or LEFT of pt.
        if dir1 == Direction::LeftToRight {
            while self.out_pts[self.out_pts[op1].next].pt.x <= pt.x
                && self.out_pts[self.out_pts[op1].next].pt.x >= self.out_pts[op1].pt.x
                && self.out_pts[self.out_pts[op1].next].pt.y == pt.y
            {
                op1 = self.out_pts[op1].next;
            }
            if discard_left && self.out_pts[op1].pt.x != pt.x {
                op1 = self.out_pts[op1].next;
            }
            op1b = self.dup_out_pt(op1, !discard_left);
            if self.out_pts[op1b].pt != pt {
                op1 = op1b;
                self.out_pts[op1].pt = pt;
                op1b = self.dup_out_pt(op1, !discard_left);
            }
        } else {
            while self.out_pts[self.out_pts[op1].next].pt.x >= pt.x
                && self.out_pts[self.out_pts[op1].next].pt.x <= self.out_pts[op1].pt.x
                && self.out_pts[self.out_pts[op1].next].pt.y == pt.y
            {
                op1 = self.out_pts[op1].next;
            }
            if !discard_left && self.out_pts[op1].pt.x != pt.x {
                op1 = self.out_pts[op1].next;
            }
            op1b = self.dup_out_pt(op1, discard_left);
            if self.out_pts[op1b].pt != pt {
                op1 = op1b;
                self.out_pts[op1].pt = pt;
                op1b = self.dup_out_pt(op1, discard_left);
            }
        }

        if dir2 == Direction::LeftToRight {
            while self.out_pts[self.out_pts[op2].next].pt.x <= pt.x
                && self.out_pts[self.out_pts[op2].next].pt.x >= self.out_pts[op2].pt.x
                && self.out_pts[self.out_pts[op2].next].pt.y == pt.y
            {
                op2 = self.out_pts[op2].next;
            }
            if discard_left && self.out_pts[op2].pt.x != pt.x {
                op2 = self.out_pts[op2].next;
            }
            op2b = self.dup_out_pt(op2, !discard_left);
            if self.out_pts[op2b].pt != pt {
                op2 = op2b;
                self.out_pts[op2].pt = pt;
                op2b = self.dup_out_pt(op2, !discard_left);
            }
        } else {
            while self.out_pts[self.out_pts[op2].next].pt.x >= pt.x
                && self.out_pts[self.out_pts[op2].next].pt.x <= self.out_pts[op2].pt.x
                && self.out_pts[self.out_pts[op2].next].pt.y == pt.y
            {
                op2 = self.out_pts[op2].next;
            }
            if !discard_left && self.out_pts[op2].pt.x != pt.x {
                op2 = self.out_pts[op2].next;
            }
            op2b = self.dup_out_pt(op2, discard_left);
            if self.out_pts[op2b].pt != pt {
                op2 = op2b;
                self.out_pts[op2].pt = pt;
                op2b = self.dup_out_pt(op2, discard_left);
            }
        }

        if (dir1 == Direction::LeftToRight) == discard_left {
            self.out_pts[op1].prev = op2;
            self.out_pts[op2].next = op1;
            self.out_pts[op1b].next = op2b;
            self.out_pts[op2b].prev = op1b;
        } else {
            self.out_pts[op1].next = op2;
            self.out_pts[op2].prev = op1;
            self.out_pts[op1b].prev = op2b;
            self.out_pts[op2b].next = op1b;
        }
        true
    }

    /// `JoinPoints`: perform join `j` (index in `joins`). `out_rec1` / `out_rec2` are the
    /// owning records. Returns true when the rings were spliced.
    fn join_points(&mut self, j: usize, out_rec1: usize, out_rec2: usize) -> bool {
        let jo1 = self.joins[j].out_pt1;
        let jo2 = self.joins[j].out_pt2;
        let off_pt = self.joins[j].off_pt;
        let mut op1 = jo1;
        let mut op2 = jo2;
        let mut op1b;
        let mut op2b;

        // 3 kinds of joins for output polygons:
        // 1. Horizontal joins where OutPt1 & OutPt2 are vertices anywhere along (horizontal)
        //    collinear edges (& OffPt is on the same horizontal).
        // 2. Non-horizontal joins where OutPt1 & OutPt2 are at the same location at the Bottom
        //    of the overlapping segment (& OffPt is above).
        // 3. StrictlySimple joins where edges touch but are not collinear and where OutPt1,
        //    OutPt2 & OffPt all share the same point.
        let is_horizontal = self.out_pts[jo1].pt.y == off_pt.y;

        if is_horizontal && off_pt == self.out_pts[jo1].pt && off_pt == self.out_pts[jo2].pt {
            // Strictly Simple join
            if out_rec1 != out_rec2 {
                return false;
            }
            op1b = self.out_pts[jo1].next;
            while op1b != op1 && self.out_pts[op1b].pt == off_pt {
                op1b = self.out_pts[op1b].next;
            }
            let reverse1 = self.out_pts[op1b].pt.y > off_pt.y;
            op2b = self.out_pts[jo2].next;
            while op2b != op2 && self.out_pts[op2b].pt == off_pt {
                op2b = self.out_pts[op2b].next;
            }
            let reverse2 = self.out_pts[op2b].pt.y > off_pt.y;
            if reverse1 == reverse2 {
                return false;
            }
            if reverse1 {
                op1b = self.dup_out_pt(op1, false);
                op2b = self.dup_out_pt(op2, true);
                self.out_pts[op1].prev = op2;
                self.out_pts[op2].next = op1;
                self.out_pts[op1b].next = op2b;
                self.out_pts[op2b].prev = op1b;
                self.joins[j].out_pt1 = op1;
                self.joins[j].out_pt2 = op1b;
                true
            } else {
                op1b = self.dup_out_pt(op1, true);
                op2b = self.dup_out_pt(op2, false);
                self.out_pts[op1].next = op2;
                self.out_pts[op2].prev = op1;
                self.out_pts[op1b].prev = op2b;
                self.out_pts[op2b].next = op1b;
                self.joins[j].out_pt1 = op1;
                self.joins[j].out_pt2 = op1b;
                true
            }
        } else if is_horizontal {
            // treat horizontal joins differently since we're not yet sure where the overlapping
            // is. OutPt1.Pt & OutPt2.Pt may be anywhere along the horizontal edge.
            op1b = op1;
            while self.out_pts[self.out_pts[op1].prev].pt.y == self.out_pts[op1].pt.y
                && self.out_pts[op1].prev != op1b
                && self.out_pts[op1].prev != op2
            {
                op1 = self.out_pts[op1].prev;
            }
            while self.out_pts[self.out_pts[op1b].next].pt.y == self.out_pts[op1b].pt.y
                && self.out_pts[op1b].next != op1
                && self.out_pts[op1b].next != op2
            {
                op1b = self.out_pts[op1b].next;
            }
            if self.out_pts[op1b].next == op1 || self.out_pts[op1b].next == op2 {
                return false; // a flat 'polygon'
            }

            op2b = op2;
            while self.out_pts[self.out_pts[op2].prev].pt.y == self.out_pts[op2].pt.y
                && self.out_pts[op2].prev != op2b
                && self.out_pts[op2].prev != op1b
            {
                op2 = self.out_pts[op2].prev;
            }
            while self.out_pts[self.out_pts[op2b].next].pt.y == self.out_pts[op2b].pt.y
                && self.out_pts[op2b].next != op2
                && self.out_pts[op2b].next != op1
            {
                op2b = self.out_pts[op2b].next;
            }
            if self.out_pts[op2b].next == op2 || self.out_pts[op2b].next == op1 {
                return false; // a flat 'polygon'
            }

            // op1 -> op1b & op2 -> op2b are the extremites of the horizontal edges
            let (ok, left, right) = Self::get_overlap(
                self.out_pts[op1].pt.x,
                self.out_pts[op1b].pt.x,
                self.out_pts[op2].pt.x,
                self.out_pts[op2b].pt.x,
            );
            if !ok {
                return false;
            }

            // DiscardLeftSide: when overlapping edges are joined, a spike is created which
            // needs cleanup; but don't catch op1 / op2 on the discard side
            let (p1, p1b, p2, p2b) = (
                self.out_pts[op1].pt,
                self.out_pts[op1b].pt,
                self.out_pts[op2].pt,
                self.out_pts[op2b].pt,
            );
            let pt;
            let discard_left_side;
            if p1.x >= left && p1.x <= right {
                pt = p1;
                discard_left_side = p1.x > p1b.x;
            } else if p2.x >= left && p2.x <= right {
                pt = p2;
                discard_left_side = p2.x > p2b.x;
            } else if p1b.x >= left && p1b.x <= right {
                pt = p1b;
                discard_left_side = p1b.x > p1.x;
            } else {
                pt = p2b;
                discard_left_side = p2b.x > p2.x;
            }
            self.joins[j].out_pt1 = op1;
            self.joins[j].out_pt2 = op2;
            self.join_horz(op1, op1b, op2, op2b, pt, discard_left_side)
        } else {
            // nb: for non-horizontal joins
            //   1. Jr.OutPt1.Pt.Y == Jr.OutPt2.Pt.Y
            //   2. Jr.OutPt1.Pt > Jr.OffPt.Y

            // make sure the polygons are correctly oriented
            op1b = self.out_pts[op1].next;
            while self.out_pts[op1b].pt == self.out_pts[op1].pt && op1b != op1 {
                op1b = self.out_pts[op1b].next;
            }
            let reverse1 = self.out_pts[op1b].pt.y > self.out_pts[op1].pt.y
                || !slopes_equal3(
                    self.out_pts[op1].pt,
                    self.out_pts[op1b].pt,
                    off_pt,
                    self.use_full_range,
                );
            if reverse1 {
                op1b = self.out_pts[op1].prev;
                while self.out_pts[op1b].pt == self.out_pts[op1].pt && op1b != op1 {
                    op1b = self.out_pts[op1b].prev;
                }
                if self.out_pts[op1b].pt.y > self.out_pts[op1].pt.y
                    || !slopes_equal3(
                        self.out_pts[op1].pt,
                        self.out_pts[op1b].pt,
                        off_pt,
                        self.use_full_range,
                    )
                {
                    return false;
                }
            }
            op2b = self.out_pts[op2].next;
            while self.out_pts[op2b].pt == self.out_pts[op2].pt && op2b != op2 {
                op2b = self.out_pts[op2b].next;
            }
            let reverse2 = self.out_pts[op2b].pt.y > self.out_pts[op2].pt.y
                || !slopes_equal3(
                    self.out_pts[op2].pt,
                    self.out_pts[op2b].pt,
                    off_pt,
                    self.use_full_range,
                );
            if reverse2 {
                op2b = self.out_pts[op2].prev;
                while self.out_pts[op2b].pt == self.out_pts[op2].pt && op2b != op2 {
                    op2b = self.out_pts[op2b].prev;
                }
                if self.out_pts[op2b].pt.y > self.out_pts[op2].pt.y
                    || !slopes_equal3(
                        self.out_pts[op2].pt,
                        self.out_pts[op2b].pt,
                        off_pt,
                        self.use_full_range,
                    )
                {
                    return false;
                }
            }

            if op1b == op1
                || op2b == op2
                || op1b == op2b
                || (out_rec1 == out_rec2 && reverse1 == reverse2)
            {
                return false;
            }

            if reverse1 {
                op1b = self.dup_out_pt(op1, false);
                op2b = self.dup_out_pt(op2, true);
                self.out_pts[op1].prev = op2;
                self.out_pts[op2].next = op1;
                self.out_pts[op1b].next = op2b;
                self.out_pts[op2b].prev = op1b;
                self.joins[j].out_pt1 = op1;
                self.joins[j].out_pt2 = op1b;
                true
            } else {
                op1b = self.dup_out_pt(op1, true);
                op2b = self.dup_out_pt(op2, false);
                self.out_pts[op1].next = op2;
                self.out_pts[op2].prev = op1;
                self.out_pts[op1b].prev = op2b;
                self.out_pts[op2b].next = op1b;
                self.joins[j].out_pt1 = op1;
                self.joins[j].out_pt2 = op1b;
                true
            }
        }
    }

    /// `PointInPolygon(IntPoint, Path)`. `pt` is tested against closed `path`.
    /// Returns 0 outside, +1 inside, -1 on the boundary.
    pub fn point_in_polygon(pt: IntPoint, path: &[IntPoint]) -> i32 {
        // See "The Point in Polygon Problem for Arbitrary Polygons" by Hormann & Agathos
        let mut result = 0;
        let cnt = path.len();
        if cnt < 3 {
            return 0;
        }
        let mut ip = path[0];
        for i in 1..=cnt {
            let ip_next = if i == cnt { path[0] } else { path[i] };
            if ip_next.y == pt.y
                && (ip_next.x == pt.x || (ip.y == pt.y && ((ip_next.x > pt.x) == (ip.x < pt.x))))
            {
                return -1;
            }
            if (ip.y < pt.y) != (ip_next.y < pt.y) {
                if ip.x >= pt.x {
                    if ip_next.x > pt.x {
                        result = 1 - result;
                    } else {
                        let d = (ip.x.wrapping_sub(pt.x)) as f64 * (ip_next.y.wrapping_sub(pt.y)) as f64
                            - (ip_next.x.wrapping_sub(pt.x)) as f64
                                * (ip.y.wrapping_sub(pt.y)) as f64;
                        if d == 0.0 {
                            return -1;
                        } else if (d > 0.0) == (ip_next.y > ip.y) {
                            result = 1 - result;
                        }
                    }
                } else if ip_next.x > pt.x {
                    let d = (ip.x.wrapping_sub(pt.x)) as f64 * (ip_next.y.wrapping_sub(pt.y)) as f64
                        - (ip_next.x.wrapping_sub(pt.x)) as f64 * (ip.y.wrapping_sub(pt.y)) as f64;
                    if d == 0.0 {
                        return -1;
                    } else if (d > 0.0) == (ip_next.y > ip.y) {
                        result = 1 - result;
                    }
                }
            }
            ip = ip_next;
        }
        result
    }

    /// `ParseFirstLeft`: skip disposed records up the FirstLeft chain.
    fn parse_first_left(&self, mut first_left: usize) -> usize {
        while first_left != NIL && self.poly_outs[first_left].pts == NIL {
            first_left = self.poly_outs[first_left].first_left;
        }
        first_left
    }

    /// `FixupFirstLefts1`: re-parent records of `old` that sit inside `new`.
    fn fixup_first_lefts1(&mut self, old_out_rec: usize, new_out_rec: usize) {
        for i in 0..self.poly_outs.len() {
            let first_left = self.parse_first_left(self.poly_outs[i].first_left);
            if self.poly_outs[i].pts != NIL
                && first_left == old_out_rec
                && poly2_contains_poly1(
                    &self.out_pts,
                    self.poly_outs[i].pts,
                    self.poly_outs[new_out_rec].pts,
                )
            {
                self.poly_outs[i].first_left = new_out_rec;
            }
        }
    }

    /// `FixupFirstLefts2`: a poly split such that `inner` is now inside `outer`.
    fn fixup_first_lefts2(&mut self, inner_out_rec: usize, outer_out_rec: usize) {
        // check every polygon that's also contained by outer's FirstLeft container
        // (including nil) to see if they've become inner to the new inner polygon
        let orfl = self.poly_outs[outer_out_rec].first_left;
        for i in 0..self.poly_outs.len() {
            if self.poly_outs[i].pts == NIL || i == outer_out_rec || i == inner_out_rec {
                continue;
            }
            let first_left = self.parse_first_left(self.poly_outs[i].first_left);
            if first_left != orfl && first_left != inner_out_rec && first_left != outer_out_rec {
                continue;
            }
            let pts = self.poly_outs[i].pts;
            if poly2_contains_poly1(&self.out_pts, pts, self.poly_outs[inner_out_rec].pts) {
                self.poly_outs[i].first_left = inner_out_rec;
            } else if poly2_contains_poly1(&self.out_pts, pts, self.poly_outs[outer_out_rec].pts) {
                self.poly_outs[i].first_left = outer_out_rec;
            } else if self.poly_outs[i].first_left == inner_out_rec
                || self.poly_outs[i].first_left == outer_out_rec
            {
                self.poly_outs[i].first_left = orfl;
            }
        }
    }

    /// `FixupFirstLefts3`: as `FixupFirstLefts1` without the containment test.
    fn fixup_first_lefts3(&mut self, old_out_rec: usize, new_out_rec: usize) {
        for i in 0..self.poly_outs.len() {
            let first_left = self.parse_first_left(self.poly_outs[i].first_left);
            if self.poly_outs[i].pts != NIL && first_left == old_out_rec {
                self.poly_outs[i].first_left = new_out_rec;
            }
        }
    }

    /// `JoinCommonEdges`: run all recorded joins.
    fn join_common_edges(&mut self) {
        for i in 0..self.joins.len() {
            let out_rec1 = self.get_out_rec(self.out_pts[self.joins[i].out_pt1].idx);
            let mut out_rec2 = self.get_out_rec(self.out_pts[self.joins[i].out_pt2].idx);

            if self.poly_outs[out_rec1].pts == NIL || self.poly_outs[out_rec2].pts == NIL {
                continue;
            }
            if self.poly_outs[out_rec1].is_open || self.poly_outs[out_rec2].is_open {
                continue;
            }

            // get the polygon fragment with the correct hole state (FirstLeft)
            // before calling JoinPoints()
            let hole_state_rec = if out_rec1 == out_rec2 {
                out_rec1
            } else if self.out_rec1_right_of_out_rec2(out_rec1, out_rec2) {
                out_rec2
            } else if self.out_rec1_right_of_out_rec2(out_rec2, out_rec1) {
                out_rec1
            } else {
                self.get_lowermost_rec(out_rec1, out_rec2)
            };

            if !self.join_points(i, out_rec1, out_rec2) {
                continue;
            }

            if out_rec1 == out_rec2 {
                // instead of joining two polygons, we've just created a new one by
                // splitting one polygon into two
                self.poly_outs[out_rec1].pts = self.joins[i].out_pt1;
                self.poly_outs[out_rec1].bottom_pt = NIL;
                out_rec2 = self.create_out_rec();
                self.poly_outs[out_rec2].pts = self.joins[i].out_pt2;

                // update all out_rec2.Pts Idx's
                self.update_out_pt_idxs(out_rec2);

                let (pts1, pts2) = (self.poly_outs[out_rec1].pts, self.poly_outs[out_rec2].pts);
                if poly2_contains_poly1(&self.out_pts, pts2, pts1) {
                    // out_rec1 contains out_rec2
                    self.poly_outs[out_rec2].is_hole = !self.poly_outs[out_rec1].is_hole;
                    self.poly_outs[out_rec2].first_left = out_rec1;

                    if self.using_poly_tree {
                        self.fixup_first_lefts2(out_rec2, out_rec1);
                    }

                    if (self.poly_outs[out_rec2].is_hole ^ self.reverse_solution)
                        == (area_ring(&self.out_pts, self.poly_outs[out_rec2].pts) > 0.0)
                    {
                        self.reverse_poly_pt_links(self.poly_outs[out_rec2].pts);
                    }
                } else if poly2_contains_poly1(&self.out_pts, pts1, pts2) {
                    // out_rec2 contains out_rec1
                    self.poly_outs[out_rec2].is_hole = self.poly_outs[out_rec1].is_hole;
                    self.poly_outs[out_rec1].is_hole = !self.poly_outs[out_rec2].is_hole;
                    self.poly_outs[out_rec2].first_left = self.poly_outs[out_rec1].first_left;
                    self.poly_outs[out_rec1].first_left = out_rec2;

                    if self.using_poly_tree {
                        self.fixup_first_lefts2(out_rec1, out_rec2);
                    }

                    if (self.poly_outs[out_rec1].is_hole ^ self.reverse_solution)
                        == (area_ring(&self.out_pts, self.poly_outs[out_rec1].pts) > 0.0)
                    {
                        self.reverse_poly_pt_links(self.poly_outs[out_rec1].pts);
                    }
                } else {
                    // the 2 polygons are completely separate
                    self.poly_outs[out_rec2].is_hole = self.poly_outs[out_rec1].is_hole;
                    self.poly_outs[out_rec2].first_left = self.poly_outs[out_rec1].first_left;

                    // fixup FirstLeft pointers that may need reassigning to out_rec2
                    if self.using_poly_tree {
                        self.fixup_first_lefts1(out_rec1, out_rec2);
                    }
                }
            } else {
                // joined 2 polygons together
                self.poly_outs[out_rec2].pts = NIL;
                self.poly_outs[out_rec2].bottom_pt = NIL;
                self.poly_outs[out_rec2].idx = self.poly_outs[out_rec1].idx;

                self.poly_outs[out_rec1].is_hole = self.poly_outs[hole_state_rec].is_hole;
                if hole_state_rec == out_rec2 {
                    self.poly_outs[out_rec1].first_left = self.poly_outs[out_rec2].first_left;
                }
                self.poly_outs[out_rec2].first_left = out_rec1;

                // fixup FirstLeft pointers that may need reassigning to out_rec1
                if self.using_poly_tree {
                    self.fixup_first_lefts3(out_rec2, out_rec1);
                }
            }
        }
    }

    /// `UpdateOutPtIdxs`: stamp every point of the ring with the record's Idx.
    fn update_out_pt_idxs(&mut self, outrec: usize) {
        let start = self.poly_outs[outrec].pts;
        let idx = self.poly_outs[outrec].idx;
        let mut op = start;
        loop {
            self.out_pts[op].idx = idx;
            op = self.out_pts[op].prev;
            if op == start {
                break;
            }
        }
    }

    /// `DoSimplePolygons`: split output rings at repeated vertices (StrictlySimple).
    fn do_simple_polygons(&mut self) {
        let mut i = 0;
        while i < self.poly_outs.len() {
            let outrec = i;
            i += 1;
            let mut op = self.poly_outs[outrec].pts;
            if op == NIL || self.poly_outs[outrec].is_open {
                continue;
            }
            loop {
                // for each Pt in Polygon until duplicate found do ...
                let mut op2 = self.out_pts[op].next;
                while op2 != self.poly_outs[outrec].pts {
                    if self.out_pts[op].pt == self.out_pts[op2].pt
                        && self.out_pts[op2].next != op
                        && self.out_pts[op2].prev != op
                    {
                        // split the polygon into two
                        let op3 = self.out_pts[op].prev;
                        let op4 = self.out_pts[op2].prev;
                        self.out_pts[op].prev = op4;
                        self.out_pts[op4].next = op;
                        self.out_pts[op2].prev = op3;
                        self.out_pts[op3].next = op2;

                        self.poly_outs[outrec].pts = op;
                        let outrec2 = self.create_out_rec();
                        self.poly_outs[outrec2].pts = op2;
                        self.update_out_pt_idxs(outrec2);
                        let (pts1, pts2) =
                            (self.poly_outs[outrec].pts, self.poly_outs[outrec2].pts);
                        if poly2_contains_poly1(&self.out_pts, pts2, pts1) {
                            // outrec2 is contained by outrec
                            self.poly_outs[outrec2].is_hole = !self.poly_outs[outrec].is_hole;
                            self.poly_outs[outrec2].first_left = outrec;
                            if self.using_poly_tree {
                                self.fixup_first_lefts2(outrec2, outrec);
                            }
                        } else if poly2_contains_poly1(&self.out_pts, pts1, pts2) {
                            // outrec is contained by outrec2
                            self.poly_outs[outrec2].is_hole = self.poly_outs[outrec].is_hole;
                            self.poly_outs[outrec].is_hole = !self.poly_outs[outrec2].is_hole;
                            self.poly_outs[outrec2].first_left = self.poly_outs[outrec].first_left;
                            self.poly_outs[outrec].first_left = outrec2;
                            if self.using_poly_tree {
                                self.fixup_first_lefts2(outrec, outrec2);
                            }
                        } else {
                            // the 2 polygons are separate
                            self.poly_outs[outrec2].is_hole = self.poly_outs[outrec].is_hole;
                            self.poly_outs[outrec2].first_left = self.poly_outs[outrec].first_left;
                            if self.using_poly_tree {
                                self.fixup_first_lefts1(outrec, outrec2);
                            }
                        }
                        op2 = op; // get ready for the next iteration
                    }
                    op2 = self.out_pts[op2].next;
                }
                op = self.out_pts[op].next;
                if op == self.poly_outs[outrec].pts {
                    break;
                }
            }
        }
    }

    /// `Area(Path)`. `poly` is a closed path. Returns signed area (0 for < 3 points).
    pub fn area(poly: &[IntPoint]) -> f64 {
        let cnt = poly.len();
        if cnt < 3 {
            return 0.0;
        }
        let mut a = 0.0;
        let mut j = cnt - 1;
        for i in 0..cnt {
            a += (poly[j].x as f64 + poly[i].x as f64) * (poly[j].y as f64 - poly[i].y as f64);
            j = i;
        }
        -a * 0.5
    }

    // ---- SimplifyPolygon functions: convert self-intersecting polygons into simple polygons ----

    /// `SimplifyPolygon`. `poly` is the polygon, `fill_type` its fill rule.
    /// Returns the simple polygons.
    pub fn simplify_polygon(poly: &[IntPoint], fill_type: PolyFillType) -> Res<Paths> {
        let mut result = Paths::new();
        let mut c = Clipper::new(0);
        c.strictly_simple = true;
        c.add_path(poly, PolyType::Subject, true)?;
        c.execute_fill(ClipType::Union, &mut result, fill_type, fill_type)?;
        Ok(result)
    }

    /// `SimplifyPolygons`. `polys` are the polygons, `fill_type` their fill rule.
    /// Returns the simple polygons.
    pub fn simplify_polygons(polys: &[Path], fill_type: PolyFillType) -> Res<Paths> {
        let mut result = Paths::new();
        let mut c = Clipper::new(0);
        c.strictly_simple = true;
        c.add_paths(polys, PolyType::Subject, true)?;
        c.execute_fill(ClipType::Union, &mut result, fill_type, fill_type)?;
        Ok(result)
    }

    /// `DistanceSqrd`. Returns squared distance between two points.
    #[allow(dead_code)]
    fn distance_sqrd(pt1: IntPoint, pt2: IntPoint) -> f64 {
        let dx = pt1.x as f64 - pt2.x as f64;
        let dy = pt1.y as f64 - pt2.y as f64;
        dx * dx + dy * dy
    }

    /// `DistanceFromLineSqrd`. Returns squared perpendicular distance of `pt` to line ln1-ln2.
    fn distance_from_line_sqrd(pt: IntPoint, ln1: IntPoint, ln2: IntPoint) -> f64 {
        // The equation of a line in general form (Ax + By + C = 0) given 2 points ...
        // A = (y1 - y2); B = (x2 - x1); C = (y2 - y1)x1 - (x2 - x1)y1
        // perpendicular distance of point (x3,y3) = (Ax3 + By3 + C)/Sqrt(A^2 + B^2)
        let a = (ln1.y.wrapping_sub(ln2.y)) as f64;
        let b = (ln2.x.wrapping_sub(ln1.x)) as f64;
        let mut c = a * ln1.x as f64 + b * ln1.y as f64;
        c = a * pt.x as f64 + b * pt.y as f64 - c;
        (c * c) / (a * a + b * b)
    }

    /// `SlopesNearCollinear`. Returns true when the geometrically middle point is within
    /// `dist_sqrd` (squared) of the line through the other two.
    fn slopes_near_collinear(pt1: IntPoint, pt2: IntPoint, pt3: IntPoint, dist_sqrd: f64) -> bool {
        // more accurate when the point GEOMETRICALLY between the other 2 is tested;
        // with 'spikes', either pt1 or pt3 is geometrically between the other pts
        if pt1.x.wrapping_sub(pt2.x).wrapping_abs() > pt1.y.wrapping_sub(pt2.y).wrapping_abs() {
            if (pt1.x > pt2.x) == (pt1.x < pt3.x) {
                Self::distance_from_line_sqrd(pt1, pt2, pt3) < dist_sqrd
            } else if (pt2.x > pt1.x) == (pt2.x < pt3.x) {
                Self::distance_from_line_sqrd(pt2, pt1, pt3) < dist_sqrd
            } else {
                Self::distance_from_line_sqrd(pt3, pt1, pt2) < dist_sqrd
            }
        } else if (pt1.y > pt2.y) == (pt1.y < pt3.y) {
            Self::distance_from_line_sqrd(pt1, pt2, pt3) < dist_sqrd
        } else if (pt2.y > pt1.y) == (pt2.y < pt3.y) {
            Self::distance_from_line_sqrd(pt2, pt1, pt3) < dist_sqrd
        } else {
            Self::distance_from_line_sqrd(pt3, pt1, pt2) < dist_sqrd
        }
    }

    /// `PointsAreClose`. Returns true when squared distance <= `dist_sqrd`.
    fn points_are_close(pt1: IntPoint, pt2: IntPoint, dist_sqrd: f64) -> bool {
        let dx = pt1.x as f64 - pt2.x as f64;
        let dy = pt1.y as f64 - pt2.y as f64;
        (dx * dx) + (dy * dy) <= dist_sqrd
    }

    /// `ExcludeOp`: unlink `op` from its ring in `pts`. Returns the previous point.
    fn exclude_op(pts: &mut [OutPt], op: usize) -> usize {
        let result = pts[op].prev;
        let next = pts[op].next;
        pts[result].next = next;
        pts[next].prev = result;
        pts[result].idx = 0;
        result
    }

    /// `CleanPolygon`. `path` is the polygon, `distance` the proximity (units) below which
    /// vertices are stripped (C# default 1.415). Returns the cleaned polygon.
    pub fn clean_polygon(path: &[IntPoint], distance: f64) -> Path {
        // Default ~= sqrt(2) so when adjacent vertices or semi-adjacent vertices have
        // both x & y coords within 1 unit, then the second vertex will be stripped.
        let mut cnt = path.len() as i64;

        if cnt == 0 {
            return Path::new();
        }

        let n = path.len();
        let mut out_pts: Vec<OutPt> = (0..n)
            .map(|i| OutPt { idx: 0, pt: path[i], next: (i + 1) % n, prev: NIL })
            .collect();
        for i in 0..n {
            let n = out_pts[i].next;
            out_pts[n].prev = i;
        }

        let dist_sqrd = distance * distance;
        let mut op = 0usize;
        while out_pts[op].idx == 0 && out_pts[op].next != out_pts[op].prev {
            let (p, pp, pn) = (out_pts[op].pt, out_pts[out_pts[op].prev].pt, out_pts[out_pts[op].next].pt);
            if Self::points_are_close(p, pp, dist_sqrd) {
                op = Self::exclude_op(&mut out_pts, op);
                cnt -= 1;
            } else if Self::points_are_close(pp, pn, dist_sqrd) {
                let n = out_pts[op].next;
                Self::exclude_op(&mut out_pts, n);
                op = Self::exclude_op(&mut out_pts, op);
                cnt -= 2;
            } else if Self::slopes_near_collinear(pp, p, pn, dist_sqrd) {
                op = Self::exclude_op(&mut out_pts, op);
                cnt -= 1;
            } else {
                out_pts[op].idx = 1;
                op = out_pts[op].next;
            }
        }

        if cnt < 3 {
            cnt = 0;
        }
        let mut result = Path::with_capacity(cnt as usize);
        for _ in 0..cnt {
            result.push(out_pts[op].pt);
            op = out_pts[op].next;
        }
        result
    }

    /// `CleanPolygons`. `polys` are cleaned one by one with `distance` (C# default 1.415).
    /// Returns the cleaned polygons.
    pub fn clean_polygons(polys: &[Path], distance: f64) -> Paths {
        polys.iter().map(|p| Self::clean_polygon(p, distance)).collect()
    }

    /// `Minkowski`: quads swept by `pattern` along `path`. `is_sum` picks sum vs diff,
    /// `is_closed` closes the path. Returns the quads.
    fn minkowski(pattern: &[IntPoint], path: &[IntPoint], is_sum: bool, is_closed: bool) -> Paths {
        let delta = usize::from(is_closed);
        let poly_cnt = pattern.len();
        let path_cnt = path.len();
        let mut result = Paths::with_capacity(path_cnt);
        for pp in path {
            let mut p = Path::with_capacity(poly_cnt);
            for ip in pattern {
                if is_sum {
                    p.push(IntPoint::new(pp.x + ip.x, pp.y + ip.y));
                } else {
                    p.push(IntPoint::new(pp.x - ip.x, pp.y - ip.y));
                }
            }
            result.push(p);
        }

        let mut quads = Paths::new();
        if path_cnt + delta == 0 {
            return quads;
        }
        for i in 0..(path_cnt + delta - 1) {
            for j in 0..poly_cnt {
                let mut quad = Path::with_capacity(4);
                quad.push(result[i % path_cnt][j % poly_cnt]);
                quad.push(result[(i + 1) % path_cnt][j % poly_cnt]);
                quad.push(result[(i + 1) % path_cnt][(j + 1) % poly_cnt]);
                quad.push(result[i % path_cnt][(j + 1) % poly_cnt]);
                if !Self::orientation(&quad) {
                    quad.reverse();
                }
                quads.push(quad);
            }
        }
        quads
    }

    /// `MinkowskiSum(Path pattern, Path path, bool)`. Returns the union of the swept quads.
    pub fn minkowski_sum(pattern: &[IntPoint], path: &[IntPoint], path_is_closed: bool) -> Res<Paths> {
        let mut paths = Self::minkowski(pattern, path, true, path_is_closed);
        let mut c = Clipper::new(0);
        c.add_paths(&paths, PolyType::Subject, true)?;
        let mut out = Paths::new();
        c.execute_fill(ClipType::Union, &mut out, PolyFillType::NonZero, PolyFillType::NonZero)?;
        std::mem::swap(&mut paths, &mut out);
        Ok(paths)
    }

    /// `TranslatePath`. Returns `path` shifted by `delta`.
    fn translate_path(path: &[IntPoint], delta: IntPoint) -> Path {
        path.iter().map(|p| IntPoint::new(p.x + delta.x, p.y + delta.y)).collect()
    }

    /// `MinkowskiSum(Path pattern, Paths paths, bool)`. Returns the union over all paths.
    pub fn minkowski_sum_paths(
        pattern: &[IntPoint],
        paths: &[Path],
        path_is_closed: bool,
    ) -> Res<Paths> {
        let mut solution = Paths::new();
        let mut c = Clipper::new(0);
        for p in paths {
            let tmp = Self::minkowski(pattern, p, true, path_is_closed);
            c.add_paths(&tmp, PolyType::Subject, true)?;
            if path_is_closed {
                let path = Self::translate_path(p, pattern[0]);
                c.add_path(&path, PolyType::Clip, true)?;
            }
        }
        c.execute_fill(ClipType::Union, &mut solution, PolyFillType::NonZero, PolyFillType::NonZero)?;
        Ok(solution)
    }

    /// `MinkowskiDiff`. Returns the union of quads of `poly2` swept by `poly1` (difference).
    pub fn minkowski_diff(poly1: &[IntPoint], poly2: &[IntPoint]) -> Res<Paths> {
        let paths = Self::minkowski(poly1, poly2, false, true);
        let mut c = Clipper::new(0);
        c.add_paths(&paths, PolyType::Subject, true)?;
        let mut out = Paths::new();
        c.execute_fill(ClipType::Union, &mut out, PolyFillType::NonZero, PolyFillType::NonZero)?;
        Ok(out)
    }

    /// `PolyTreeToPaths`. Returns every contour of the tree, depth first.
    pub fn poly_tree_to_paths(polytree: &PolyTree) -> Paths {
        let mut result = Paths::new();
        Self::add_poly_node_to_paths(polytree, PolyTree::ROOT, NodeType::Any, &mut result);
        result
    }

    /// `AddPolyNodeToPaths`. Collects `polynode` and descendants matching `nt` into `paths`.
    pub(crate) fn add_poly_node_to_paths(
        polytree: &PolyTree,
        polynode: NodeId,
        nt: NodeType,
        paths: &mut Paths,
    ) {
        let node = polytree.node(polynode);
        let mut matches = true;
        match nt {
            NodeType::Open => return,
            NodeType::Closed => matches = !node.is_open,
            NodeType::Any => {}
        }

        if !node.polygon.is_empty() && matches {
            paths.push(node.polygon.clone());
        }
        for &pn in &node.childs {
            Self::add_poly_node_to_paths(polytree, pn, nt, paths);
        }
    }

    /// `OpenPathsFromPolyTree`. Returns the open paths (top level nodes flagged open).
    pub fn open_paths_from_poly_tree(polytree: &PolyTree) -> Paths {
        let mut result = Paths::new();
        for &c in polytree.childs() {
            if polytree.node(c).is_open {
                result.push(polytree.node(c).polygon.clone());
            }
        }
        result
    }

    /// `ClosedPathsFromPolyTree`. Returns the closed contours of the tree, depth first.
    pub fn closed_paths_from_poly_tree(polytree: &PolyTree) -> Paths {
        let mut result = Paths::new();
        Self::add_poly_node_to_paths(polytree, PolyTree::ROOT, NodeType::Closed, &mut result);
        result
    }
}

/// `Clipper.NodeType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeType {
    Any,
    #[allow(dead_code)] // C# ntOpen: declared, never passed
    Open,
    Closed,
}
