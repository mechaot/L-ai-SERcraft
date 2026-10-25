// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `CsPotrace/PotraceClipper.cs`: vector filling of a traced shape with Clipper.

use crate::biarc_free::Hilbert;
use crate::gdi_path::GraphicsPath;
use crate::trace::{Curve, CurveKind, DPoint};
use lg_clipper::{
    ClipType, Clipper, ClipperException, ClipperOffset, EndType, IntPoint, JoinType, Path, Paths, PolyFillType,
    PolyTree, PolyType,
};

const RESOLUTION: f64 = 1000.0;

/// `RasterConverter.ImageProcessor.Direction` (variant order identical to C#).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Direction {
    #[default]
    None,
    Horizontal,
    Vertical,
    Diagonal,
    NewHorizontal,
    NewVertical,
    NewDiagonal,
    NewReverseDiagonal,
    NewGrid,
    NewDiagonalGrid,
    NewCross,
    NewDiagonalCross,
    NewSquares,
    NewZigZag,
    NewHilbert,
    NewInsetFilling,
}

/// The `L2LConf` fields `BuildFilling` reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillConf {
    /// `dir`: filling pattern.
    pub dir: Direction,
    /// `res`: pixels per mm of the trace.
    pub res: f64,
    /// `fres`: fill lines per mm.
    pub fres: f64,
}

type ClipRes<T> = Result<T, ClipperException>;

/// `BuildFilling`: fill pattern curves clipped to the shape.
///
/// `plist` traced curves, `w`/`h` bitmap size in pixels, `c` fill configuration. Returns
/// Ok(None) when Clipper reports failure (C# null), Err on Clipper exceptions.
pub fn build_filling(plist: &[Vec<Curve>], w: f64, h: f64, c: &FillConf) -> ClipRes<Option<Vec<Vec<Curve>>>> {
    if c.dir == Direction::NewInsetFilling {
        build_inset_filling(plist, c).map(Some)
    } else {
        build_grid_filling(plist, w, h, c)
    }
}

/// `BuildGridFilling`: intersect open grid lines with the shape (even-odd).
fn build_grid_filling(plist: &[Vec<Curve>], w: f64, h: f64, cnf: &FillConf) -> ClipRes<Option<Vec<Vec<Curve>>>> {
    let mut c = Clipper::new(0);
    add_grid_subject(&mut c, w, h, cnf)?;
    add_grid_clip(plist, &mut c)?;

    let mut solution = PolyTree::new();
    let succeeded = c.execute_tree_fill(ClipType::Intersection, &mut solution, PolyFillType::EvenOdd, PolyFillType::EvenOdd)?;
    if !succeeded {
        return Ok(None);
    }
    let ll = Clipper::open_paths_from_poly_tree(&solution);
    Ok(Some(to_potrace_list(&ll, false)))
}

/// `BuildInsetFilling`: repeated inward offsets of the shape.
fn build_inset_filling(plist: &[Vec<Curve>], cnf: &FillConf) -> ClipRes<Vec<Vec<Curve>>> {
    let spacing = cnf.res / cnf.fres;
    let mut flist = Vec::new();
    let mut c = ClipperOffset::new(2.0, 0.25);
    add_offset_subject(plist, &mut c);

    let mut i = 1;
    loop {
        let mut solution = Paths::new();
        c.execute(&mut solution, -spacing * i as f64 * RESOLUTION)?;

        if !solution.is_empty() {
            flist.extend(to_potrace_list(&solution, true));
        } else {
            break;
        }
        i += 1;
    }
    Ok(flist)
}

/// Line curve `a`->`b` as the C# `new Curve(Line, a, a, b, b)`.
fn line_curve(a: DPoint, b: DPoint) -> Curve {
    Curve::new(CurveKind::Line, a, a, b, b)
}

/// `ToPotraceList`: clipper paths back to line curves. `close` adds the closing segment.
fn to_potrace_list(ll: &[Path], close: bool) -> Vec<Vec<Curve>> {
    let pt = |p: &IntPoint| DPoint::new(p.x as f64 / RESOLUTION, p.y as f64 / RESOLUTION);
    let mut flist = Vec::new();
    for pg in ll {
        if pg.len() == 2 {
            flist.push(vec![line_curve(pt(&pg[0]), pt(&pg[1]))]);
        } else {
            let mut potcurve = Vec::new();
            for i in 0..pg.len().saturating_sub(1) {
                potcurve.push(line_curve(pt(&pg[i]), pt(&pg[i + 1])));
            }
            if close {
                potcurve.push(line_curve(pt(&pg[pg.len() - 1]), pt(&pg[0])));
            }
            flist.push(potcurve);
        }
    }
    flist
}

/// Build the flattened GDI+ path of one curve list (shared by the two AddXxx functions).
fn curves_to_flat_points(lc: &[Curve]) -> Vec<IntPoint> {
    let mut current = GraphicsPath::new();
    for c in lc {
        let a = (c.a.x as f32, c.a.y as f32);
        let b = (c.b.x as f32, c.b.y as f32);
        if c.kind == CurveKind::Line {
            current.add_line(a, b);
        } else {
            current.add_bezier(
                a,
                (c.control_point_a.x as f32, c.control_point_a.y as f32),
                (c.control_point_b.x as f32, c.control_point_b.y as f32),
                b,
            );
        }
    }
    current.flatten();
    current.path_points().iter().map(|p| IntPoint::from_f64(p.0 as f64 * RESOLUTION, p.1 as f64 * RESOLUTION)).collect()
}

/// `AddGridClip`: add every traced path as closed clip polygon.
fn add_grid_clip(plist: &[Vec<Curve>], c: &mut Clipper) -> ClipRes<()> {
    for lc in plist {
        c.add_path(&curves_to_flat_points(lc), PolyType::Clip, true)?;
    }
    Ok(())
}

/// `AddOffsetSubject`: add every traced path as closed polygon to the offsetter.
fn add_offset_subject(plist: &[Vec<Curve>], c: &mut ClipperOffset) {
    for lc in plist {
        c.add_path(&curves_to_flat_points(lc), JoinType::Round, EndType::ClosedPolygon);
    }
}

/// `AddGridSubject`: add the open fill pattern lines for `cnf.dir` over a `w` x `h` area.
fn add_grid_subject(c: &mut Clipper, w: f64, h: f64, cnf: &FillConf) -> ClipRes<()> {
    let dir = cnf.dir;

    let step = cnf.res / cnf.fres;
    // step for diagonal (1.414)
    let dstep = step * 2.0f64.sqrt();
    let rdstep = step * (1.0 / 2.0f64.sqrt());

    let mut paths: Paths = Vec::new();

    if dir == Direction::NewHorizontal || dir == Direction::NewGrid {
        let mut y = 1;
        while (y as f64) < (h / step) {
            add_segment(&mut paths, 0.0, y as f64 * step, w, y as f64 * step, y % 2 == 1);
            y += 1;
        }
    }
    if dir == Direction::NewVertical || dir == Direction::NewGrid {
        let mut x = 1;
        while (x as f64) < (w / step) {
            add_segment(&mut paths, x as f64 * step, 0.0, x as f64 * step, h, x % 2 == 0);
            x += 1;
        }
    }
    if dir == Direction::NewDiagonal || dir == Direction::NewDiagonalGrid {
        let mut i = 0;
        while (i as f64) < (w + h) / dstep {
            add_segment(&mut paths, 0.0, i as f64 * dstep, i as f64 * dstep, 0.0, i % 2 == 0);
            i += 1;
        }
    }
    if dir == Direction::NewReverseDiagonal || dir == Direction::NewDiagonalGrid {
        let mut i = 0;
        while (i as f64) < (w + h) / dstep {
            add_segment(&mut paths, 0.0, h - (i as f64 * dstep), i as f64 * dstep, h, i % 2 == 1);
            i += 1;
        }
    }

    if dir == Direction::NewCross {
        // cross len
        let cl = step / 3.0;
        let mut y = 1;
        while (y as f64) < (h / step) {
            let mut x = 1;
            while (x as f64) < (w / step) {
                let (xs, ys) = (x as f64 * step, y as f64 * step);
                // horizontal bars
                add_segment(&mut paths, xs - cl, ys, xs + cl, ys, y % 2 == 1);
                // vertical bars
                add_segment(&mut paths, xs, ys - cl, xs, ys + cl, x % 2 == 0);
                x += 1;
            }
            y += 1;
        }
    }

    if dir == Direction::NewDiagonalCross {
        let cl = rdstep / 3.0;
        let mut y = 0;
        while (y as f64) < (h / step) + 1.0 {
            let mut x = 0;
            while (x as f64) < (w / step) + 1.0 {
                let (xs, ys) = (x as f64 * step, y as f64 * step);
                // upward bar
                add_segment(&mut paths, xs - cl, ys - cl, xs + cl, ys + cl, x % 2 == 1);
                // downward bar
                add_segment(&mut paths, xs + cl, ys - cl, xs - cl, ys + cl, x % 2 == 1);
                x += 1;
            }
            y += 1;
        }
    }
    if dir == Direction::NewSquares {
        let cl = step / 3.0;
        let mut y = 0;
        while (y as f64) < (h / step) + 1.0 {
            let mut x = 0;
            while (x as f64) < (w / step) + 1.0 {
                let (xs, ys) = (x as f64 * step, y as f64 * step);
                let mut list = Vec::new();
                add_path_point(&mut list, xs - cl, ys);
                add_path_point(&mut list, xs, ys + cl);
                add_path_point(&mut list, xs + cl, ys);
                add_path_point(&mut list, xs, ys - cl);
                add_path_point(&mut list, xs - cl, ys);
                paths.push(list);
                x += 1;
            }
            y += 1;
        }
    }
    if dir == Direction::NewZigZag {
        // halfstep
        let hs = dstep / 2.0;

        let mut i = 1;
        while (i as f64) < (w + h) / dstep {
            let mut list = Vec::new();

            if i % 2 == 1 {
                let mut my = 0.0;
                let mut mx = (i as f64 * dstep) - hs;

                add_path_point(&mut list, mx, my);
                while mx > 0.0 {
                    my += hs;
                    add_path_point(&mut list, mx, my);
                    mx -= hs;
                    add_path_point(&mut list, mx, my);
                }
            } else {
                let mut mx = 0.0;
                let mut my = (i as f64 * dstep) - hs;

                // upstream: the +/-1 tricks clipper into keeping my direction
                add_path_point(&mut list, mx - 1.0, my + 1.0);
                add_path_point(&mut list, mx, my);
                while my > 0.0 {
                    mx += hs;
                    add_path_point(&mut list, mx, my);
                    my -= hs;
                    add_path_point(&mut list, mx, my);
                }
            }

            paths.push(list);
            i += 1;
        }
    }
    if dir == Direction::NewHilbert {
        let n = 6;
        let ts = (2.0f64.powi(n) * step) as f32;
        // element of n recursions on step, size 2^n * step
        let texel = Hilbert::execute(n, step as f32);

        let mut y = 0;
        while (y as f64) < (h / ts as f64) {
            let mut x = 0;
            while (x as f64) < (w / ts as f64) {
                let mut list = Vec::new();
                for t in &texel {
                    add_path_point(&mut list, (t.0 + (x as f32 * ts)) as f64, (t.1 + (y as f32 * ts)) as f64);
                }
                paths.push(list);
                x += 1;
            }
            y += 1;
        }
    }
    c.add_paths(&paths, PolyType::Subject, false)?;
    Ok(())
}

/// `AddSegment`: two point open path, reversed when `invert`.
fn add_segment(target: &mut Paths, x1: f64, y1: f64, x2: f64, y2: f64, invert: bool) {
    let p1 = IntPoint::from_f64(x1 * RESOLUTION, y1 * RESOLUTION);
    let p2 = IntPoint::from_f64(x2 * RESOLUTION, y2 * RESOLUTION);
    if !invert {
        target.push(vec![p1, p2]);
    } else {
        target.push(vec![p2, p1]);
    }
}

/// `AddPathPoint`: push scaled point.
fn add_path_point(target: &mut Vec<IntPoint>, x: f64, y: f64) {
    target.push(IntPoint::from_f64(x * RESOLUTION, y * RESOLUTION));
}
