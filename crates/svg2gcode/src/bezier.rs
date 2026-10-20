//! Port of `SvgConverter/BezierTools.cs` (adaptive cubic bezier flattening).

use crate::matrix::Point;

/// `SplitCurveAtT`: De Casteljau split.
///
/// `cp`: control points. `t`: split parameter. Returns (left, right) curves.
fn split_curve_at_t(cp: &[Point], t: f64) -> (Vec<Point>, Vec<Point>) {
    let degree = cp.len() - 1;
    let mut v = vec![vec![Point::default(); degree + 1]; degree + 1];
    // copy control points
    v[0][..=degree].copy_from_slice(cp);
    // triangle computation
    for i in 1..=degree {
        for j in 0..=(degree - i) {
            v[i][j] = Point {
                x: (1.0 - t) * v[i - 1][j].x + t * v[i - 1][j + 1].x,
                y: (1.0 - t) * v[i - 1][j].y + t * v[i - 1][j + 1].y,
            };
        }
    }
    let mut a = vec![Point::default(); degree + 1];
    let mut b = vec![Point::default(); degree + 1];
    for i in 0..=degree {
        a[i] = v[i][0];
        b[i] = v[degree - i][i];
    }
    (a, b)
}

/// `CalculateTriangleArea`.
///
/// `a`,`b`,`c`: triangle corners. Returns area.
fn triangle_area(a: Point, b: Point, c: Point) -> f64 {
    (a.x * b.y + b.x * c.y + c.x * a.y - a.y * b.x - b.y * c.x - c.y * a.x).abs() / 2.0
}

/// `FlattenSegmentTo`: recursive flatten of one cubic.
///
/// `seg`: 4 control points. `error`: flatness limit. `sub`: depth. `max_sub`: max depth.
/// Returns the polyline points (first and last included).
fn flatten_segment_to(seg: &[Point], error: f64, sub: i32, max_sub: i32) -> Vec<Point> {
    // base case: flat or deep enough
    if sub >= max_sub
        || triangle_area(seg[0], seg[1], seg[2]).sqrt() < error && triangle_area(seg[1], seg[2], seg[3]).sqrt() < error
    {
        return seg.to_vec();
    }
    let (l, r) = split_curve_at_t(seg, 0.5);
    let mut out = flatten_segment_to(&l[..4], error, sub + 1, max_sub);
    out.extend(flatten_segment_to(&r[..4], error, sub + 1, max_sub).into_iter().skip(1));
    out
}

/// `BezierTools.FlattenTo(points, error, maxSubdivisions)`.
///
/// `points`: contiguous cubic segments (1 + 3n points; must be non-empty). `error`: linear error
/// (C# default 0.01). `max_subdivisions`: recursion cap (C# default 20). Returns line points.
pub fn flatten_to(points: &[Point], error: f64, max_subdivisions: i32) -> Vec<Point> {
    let mut result = vec![points[0]];
    let mut i = 0;
    while i + 3 <= points.len() {
        // C# Skip(i).Take(4): may hold fewer than 4 points on the tail; only full ones reach here in practice
        let end = (i + 4).min(points.len());
        let seg = &points[i..end];
        if seg.len() == 4 {
            result.extend(flatten_segment_to(seg, error, 0, max_subdivisions).into_iter().skip(1));
        }
        i += 3;
    }
    result
}
