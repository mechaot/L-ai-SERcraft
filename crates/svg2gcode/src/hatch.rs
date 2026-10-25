//! Hatch fill geometry: parallel meander lines inside closed polygons (even-odd), arc flattening.

use crate::matrix::Point;

/// Fill settings of a layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HatchFill {
    /// Line distance in output units (mm).
    pub distance_mm: f64,
    /// Line angle in degrees, 0 = along +X, positive = counterclockwise.
    pub angle_deg: f64,
    /// Add a second hatch rotated +90 degrees.
    pub cross: bool,
}

/// Sin/cos of degrees with exact values for multiples of 90. `deg`: angle. Returns (sin, cos).
fn sin_cos(deg: f64) -> (f64, f64) {
    let (s, c) = deg.to_radians().sin_cos();
    // snap float noise so axis-aligned hatches stay exact
    let snap = |v: f64| if v.abs() < 1e-15 { 0.0 } else { v };
    (snap(s), snap(c))
}

/// Hatch segments for closed polygons (even-odd over ALL polygons given).
///
/// `polys`: polygons, implicitly closed, NaN points and polygons < 3 points ignored.
/// `distance`: line spacing, <= 0 or non-finite gives empty. `angle_deg`: line angle (0 = +X, ccw).
/// Returns segments in burn order (meander), each (start, end) in input coordinates.
pub fn hatch_segments(polys: &[Vec<Point>], distance: f64, angle_deg: f64) -> Vec<(Point, Point)> {
    if !distance.is_finite() || distance <= 0.0 || !angle_deg.is_finite() {
        return Vec::new();
    }
    let (s, c) = sin_cos(angle_deg);
    // edges in rotated frame, lower end first: (ylo, yhi, x at ylo, dx/dy)
    let mut edges: Vec<(f64, f64, f64, f64)> = Vec::new();
    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
    for poly in polys {
        let pts: Vec<(f64, f64)> = poly
            .iter()
            .filter(|p| p.x.is_finite() && p.y.is_finite())
            .map(|p| (p.x * c + p.y * s, -p.x * s + p.y * c))
            .collect();
        if pts.len() < 3 {
            continue;
        }
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            ymin = ymin.min(a.1);
            ymax = ymax.max(a.1);
            // horizontal edges never cross
            if a.1 == b.1 {
                continue;
            }
            let (lo, hi) = if a.1 < b.1 { (a, b) } else { (b, a) };
            edges.push((lo.1, hi.1, lo.0, (hi.0 - lo.0) / (hi.1 - lo.1)));
        }
    }
    if edges.is_empty() {
        return Vec::new();
    }
    edges.sort_by(|a, b| a.0.total_cmp(&b.0));
    let k0 = (ymin / distance - 0.5).ceil() as i64;
    let k1 = (ymax / distance - 0.5).floor() as i64;
    let mut out = Vec::new();
    let mut active: Vec<usize> = Vec::new();
    let mut next = 0usize;
    let mut xs: Vec<f64> = Vec::new();
    let mut row = 0usize;
    for k in k0..=k1 {
        let y = (k as f64 + 0.5) * distance;
        // add edges that started, drop edges that ended (half-open: lo <= y < hi)
        while next < edges.len() && edges[next].0 <= y {
            active.push(next);
            next += 1;
        }
        active.retain(|&e| edges[e].1 > y);
        xs.clear();
        xs.extend(active.iter().map(|&e| edges[e].2 + (y - edges[e].0) * edges[e].3));
        xs.sort_by(f64::total_cmp);
        let mut iv: Vec<(f64, f64)> = xs.as_chunks::<2>().0.iter().map(|p| (p[0], p[1])).filter(|p| p.1 - p.0 >= 1e-6).collect();
        if iv.is_empty() {
            continue;
        }
        // odd rows run right to left
        if row % 2 == 1 {
            iv.reverse();
            for p in iv.iter_mut() {
                *p = (p.1, p.0);
            }
        }
        row += 1;
        // rotate back
        let back = |x: f64| Point::new(x * c - y * s, x * s + y * c);
        out.extend(iv.into_iter().map(|(a, b)| (back(a), back(b))));
    }
    out
}

/// Hatch for `fill.angle_deg`, plus (if `fill.cross`) for angle+90 appended after the first set.
///
/// `polys`: closed polygons (see [`hatch_segments`]). `fill`: settings. Returns segments in burn order.
pub fn fill_segments(polys: &[Vec<Point>], fill: &HatchFill) -> Vec<(Point, Point)> {
    let mut v = hatch_segments(polys, fill.distance_mm, fill.angle_deg);
    if fill.cross {
        v.extend(hatch_segments(polys, fill.distance_mm, fill.angle_deg + 90.0));
    }
    v
}

/// Points of a G2/G3-style arc for recording.
///
/// `start`: arc start. `end`: arc end (equal to start within 1e-9 = full circle). `center`: arc center.
/// `ccw`: direction. Chord error <= 0.01 mm, at least 8 segments per full circle.
/// Returns the points after `start` up to and including `end`.
pub fn flatten_arc(start: Point, end: Point, center: Point, ccw: bool) -> Vec<Point> {
    use std::f64::consts::TAU;
    let (dx0, dy0) = (start.x - center.x, start.y - center.y);
    let (dx1, dy1) = (end.x - center.x, end.y - center.y);
    let (r0, r1) = (dx0.hypot(dy0), dx1.hypot(dy1));
    let (a0, a1) = (dy0.atan2(dx0), dy1.atan2(dx1));
    let full = (start.x - end.x).abs() < 1e-9 && (start.y - end.y).abs() < 1e-9;
    // signed sweep in (0, 2pi] along the direction
    let mut sweep = if ccw { a1 - a0 } else { a0 - a1 };
    if full {
        sweep = TAU;
    } else {
        while sweep <= 0.0 {
            sweep += TAU;
        }
        while sweep > TAU {
            sweep -= TAU;
        }
    }
    // step from chord error: r(1 - cos(t/2)) <= 0.01
    let r = r0.max(r1);
    let step = if r > 0.01 { 2.0 * (1.0 - 0.01 / r).acos() } else { TAU };
    let n = ((sweep / step).ceil().max((8.0 * sweep / TAU).ceil()).max(1.0)) as usize;
    let mut pts = Vec::with_capacity(n);
    for i in 1..n {
        let t = i as f64 / n as f64;
        let a = if ccw { a0 + sweep * t } else { a0 - sweep * t };
        let rr = r0 + (r1 - r0) * t;
        pts.push(Point::new(center.x + rr * a.cos(), center.y + rr * a.sin()));
    }
    // exact end
    pts.push(end);
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Axis-aligned square polygon. `x`,`y`: corner. `s`: side. Returns points.
    fn sq(x: f64, y: f64, s: f64) -> Vec<Point> {
        vec![Point::new(x, y), Point::new(x + s, y), Point::new(x + s, y + s), Point::new(x, y + s)]
    }

    /// Circle polygon. `r`: radius. `n`: vertices. Returns points around the origin.
    fn circle(r: f64, n: usize) -> Vec<Point> {
        (0..n).map(|i| {
            let a = i as f64 / n as f64 * std::f64::consts::TAU;
            Point::new(r * a.cos(), r * a.sin())
        }).collect()
    }

    #[test]
    fn square_rows_meander() {
        let s = hatch_segments(&[sq(0.0, 0.0, 10.0)], 1.0, 0.0);
        assert_eq!(s.len(), 10);
        for (i, (a, b)) in s.iter().enumerate() {
            assert!((a.y - (i as f64 + 0.5)).abs() < 1e-9 && (b.y - a.y).abs() < 1e-9);
            assert!(((b.x - a.x).abs() - 10.0).abs() < 1e-9);
            // even rows ltr, odd rtl
            assert_eq!(b.x > a.x, i % 2 == 0);
        }
    }

    #[test]
    fn angle_90_vertical() {
        let s = hatch_segments(&[sq(0.0, 0.0, 10.0)], 1.0, 90.0);
        assert_eq!(s.len(), 10);
        for (a, b) in &s {
            assert!((a.x - b.x).abs() < 1e-9);
            assert!(((b.y - a.y).abs() - 10.0).abs() < 1e-9);
        }
    }

    #[test]
    fn hole_splits_rows() {
        let s = hatch_segments(&[sq(0.0, 0.0, 10.0), sq(4.0, 4.0, 2.0)], 1.0, 0.0);
        // rows y=4.5 and 5.5 cross the hole
        let at = |y: f64| s.iter().filter(|(a, _)| (a.y - y).abs() < 1e-9).count();
        assert_eq!(at(4.5), 2);
        assert_eq!(at(5.5), 2);
        assert_eq!(at(0.5), 1);
        assert_eq!(s.len(), 12);
    }

    #[test]
    fn overlap_not_filled() {
        let s = hatch_segments(&[sq(0.0, 0.0, 10.0), sq(5.0, 0.0, 10.0)], 1.0, 0.0);
        // row 0.5: [0,5] and [10,15]
        let row: Vec<_> = s.iter().filter(|(a, _)| (a.y - 0.5).abs() < 1e-9).collect();
        assert_eq!(row.len(), 2);
        let mut lens: Vec<f64> = row.iter().map(|(a, b)| (b.x - a.x).abs()).collect();
        lens.sort_by(f64::total_cmp);
        assert!((lens[0] - 5.0).abs() < 1e-9 && (lens[1] - 5.0).abs() < 1e-9);
    }

    #[test]
    fn angle_45_inside() {
        let s = hatch_segments(&[sq(0.0, 0.0, 10.0)], 0.5, 45.0);
        assert!(!s.is_empty());
        for (a, b) in &s {
            for p in [a, b] {
                assert!(p.x > -1e-6 && p.x < 10.0 + 1e-6 && p.y > -1e-6 && p.y < 10.0 + 1e-6);
            }
            // direction is 45 degrees
            assert!(((b.x - a.x).abs() - (b.y - a.y).abs()).abs() < 1e-6);
        }
    }

    #[test]
    fn cross_adds_second_set() {
        let p = [sq(0.0, 0.0, 10.0)];
        let f = HatchFill { distance_mm: 1.0, angle_deg: 0.0, cross: true };
        let n = fill_segments(&p, &f).len();
        assert_eq!(n, hatch_segments(&p, 1.0, 0.0).len() + hatch_segments(&p, 1.0, 90.0).len());
        assert_eq!(fill_segments(&p, &HatchFill { cross: false, ..f }).len(), 10);
    }

    #[test]
    fn arc_full_circle_and_quadrants() {
        let c = Point::new(1.0, 2.0);
        let st = Point::new(6.0, 2.0);
        let pts = flatten_arc(st, st, c, true);
        assert!(pts.len() >= 8);
        for p in &pts {
            assert!((((p.x - c.x).powi(2) + (p.y - c.y).powi(2)).sqrt() - 5.0).abs() < 1e-9);
        }
        assert_eq!(*pts.last().unwrap(), st);
        // quarter (5,0) -> (0,5): ccw passes first quadrant, cw goes the long way
        let o = Point::new(0.0, 0.0);
        let q = flatten_arc(Point::new(5.0, 0.0), Point::new(0.0, 5.0), o, true);
        assert!(q.iter().all(|p| p.x > -1e-9 && p.y > -1e-9));
        let w = flatten_arc(Point::new(5.0, 0.0), Point::new(0.0, 5.0), o, false);
        assert!(w.iter().any(|p| p.y < -1.0));
        assert_eq!(*w.last().unwrap(), Point::new(0.0, 5.0));
        // cw quarter ends in lower-right to lower-left
        let cw = flatten_arc(Point::new(5.0, 0.0), Point::new(0.0, -5.0), o, false);
        assert!(cw.iter().all(|p| p.x > -1e-9 && p.y < 1e-9));
    }

    #[test]
    fn zero_distance_empty() {
        assert!(hatch_segments(&[sq(0.0, 0.0, 10.0)], 0.0, 0.0).is_empty());
        assert!(hatch_segments(&[sq(0.0, 0.0, 10.0)], f64::NAN, 0.0).is_empty());
    }

    #[test]
    fn perf_dense_circle() {
        let t = std::time::Instant::now();
        let s = hatch_segments(&[circle(10.0, 2000)], 0.01, 0.0);
        assert!(t.elapsed().as_secs_f64() < 1.0);
        assert!((1900..=2100).contains(&s.len()), "{}", s.len());
    }
}
