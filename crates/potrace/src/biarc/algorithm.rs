// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) László Domoszlai (Bezier2Biarc)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `BezierToBiarc/Algorithm.cs`.

use super::arc::BiArc;
use super::complex::Complex;
use super::cubic_bezier::CubicBezier;
use super::line::Line;
use super::vector2::Vector2;

/// `IsRealInflexionPoint`: real and strictly inside (0,1). `t` is the root.
fn is_real_inflexion_point(t: Complex) -> bool {
    t.imaginary == 0.0 && t.real > 0.0 && t.real < 1.0
}

/// `ApproxCubicBezier`: approximate a bezier by biarcs.
///
/// `bezier` is the curve, `sampling_step` the error sampling step (arc length / step = sample
/// count), `tolerance` the accepted max deviation. Returns None (C# null) if too many points
/// would have to be processed.
pub fn approx_cubic_bezier(bezier: CubicBezier, sampling_step: f32, tolerance: f32) -> Option<Vec<BiArc>> {
    let mut biarcs = Vec::new();
    let mut curves: Vec<CubicBezier> = vec![bezier];

    // split at inflexion points first
    let mut to_split = curves.pop().expect("one curve");
    let (it1, it2) = to_split.inflexion_points();
    let i1 = is_real_inflexion_point(it1);
    let i2 = is_real_inflexion_point(it2);

    if i1 && !i2 {
        let (a, b) = to_split.split(it1.real as f32);
        curves.push(b);
        curves.push(a);
    } else if !i1 && i2 {
        let (a, b) = to_split.split(it2.real as f32);
        curves.push(b);
        curves.push(a);
    } else if i1 && i2 {
        let mut t1 = it1.real as f32;
        let mut t2 = it2.real as f32;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        let (s1a, s1b) = to_split.split(t1);
        t2 *= 1.0 - t1;
        to_split = s1b;
        let (s2a, s2b) = to_split.split(t2);
        curves.push(s2b);
        curves.push(s2a);
        curves.push(s1a);
    } else {
        curves.push(to_split);
    }

    // approximate until no curves left
    while let Some(bezier) = curves.pop() {
        // V: intersection of tangent lines
        let t1 = Line::from_points(bezier.p1, bezier.c1);
        let t2 = Line::from_points(bezier.p2, bezier.c2);
        let v = t1.intersection(t2);

        // G: incenter of triangle (P1, V, P2)
        let d_p2v = Vector2::distance(bezier.p2, v);
        let d_p1v = Vector2::distance(bezier.p1, v);
        let d_p1p2 = Vector2::distance(bezier.p1, bezier.p2);
        let g = (d_p2v * bezier.p1 + d_p1v * bezier.p2 + d_p1p2 * v) / (d_p2v + d_p1v + d_p1p2);

        let biarc = BiArc::new(bezier.p1, bezier.p1 - bezier.c1, bezier.p2, bezier.p2 - bezier.c2, g);

        let mut max_distance = 0.0f32;
        let mut max_distance_at = 0.0f32;

        let nr_points_to_check = biarc.length() / sampling_step;
        let parameter_step = 1.0f32 / nr_points_to_check;

        if nr_points_to_check > 1000.0 {
            return None; // too many points to process
        }

        let mut i: i32 = 0;
        while (i as f32) <= nr_points_to_check {
            let t = parameter_step * i as f32;
            let u1 = biarc.point_at(t);
            let u2 = bezier.point_at(t);
            let distance = (u1 - u2).length();

            if distance > max_distance {
                max_distance = distance;
                max_distance_at = t;
            }
            i += 1;
        }

        if max_distance > tolerance {
            // split at max deviation and retry both halves
            let (a, b) = bezier.split(max_distance_at);
            curves.push(b);
            curves.push(a);
        } else {
            biarcs.push(biarc);
        }
    }

    Some(biarcs)
}
