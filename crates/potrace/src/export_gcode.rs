// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `CsPotrace/CsPotraceExportGCODE.cs`: curve lists to G-code lines (arcs via biarcs).
//!
//! The C# file reads no `Settings` keys, so no options struct is needed. The `debug` preview
//! drawing (always off in C#) is not ported.

use crate::biarc::{Arc, CubicBezier, Vector2, approx_cubic_bezier};
use crate::trace::{Curve, CurveKind};
use lg_dotnet::fmt_double_custom;

/// `Export2GCode`: curve lists to G-code.
///
/// `list` curve lists, `o_x`/`o_y` offsets in mm (scaled by `scale` here), `scale` pixels per mm,
/// `l_on`/`l_off` laser on/off commands, `_original_image_size` only used by the C# debug
/// preview (unused), `skipcmd` the fast-move command ("G0"/"G1"). Returns the G-code lines.
#[allow(clippy::too_many_arguments)]
pub fn export_to_gcode(
    list: &[Vec<Curve>],
    o_x: f32,
    o_y: f32,
    scale: f64,
    l_on: &str,
    l_off: &str,
    _original_image_size: (u32, u32),
    skipcmd: &str,
) -> Vec<String> {
    let mut rv = Vec::new();
    for curves in list {
        rv.extend(get_path_gc(curves, l_on, l_off, o_x as f64 * scale, o_y as f64 * scale, scale, skipcmd));
    }
    rv
}

/// `GetPathGC`: one path. Args as in [`export_to_gcode`] with offsets already scaled.
fn get_path_gc(curves: &[Curve], l_on: &str, l_off: &str, o_x: f64, o_y: f64, scale: f64, skipcmd: &str) -> Vec<String> {
    let mut rv = Vec::new();
    on_path_begin(curves, l_on, o_x, o_y, scale, &mut rv, skipcmd);
    for c in curves {
        on_path_segment(c, o_x, o_y, scale, &mut rv);
    }
    on_path_end(curves, l_off, &mut rv);
    rv
}

/// `OnPathSegment`: line as G1, bezier as biarc arcs (fallback G1 to the end point).
fn on_path_segment(curve: &Curve, o_x: f64, o_y: f64, scale: f64, rv: &mut Vec<String>) {
    // problem?
    if curve.linear_length().is_nan() {
        return;
    }

    let g1_end = |rv: &mut Vec<String>| {
        rv.push(format!(
            "G1 X{} Y{}",
            formatnumber(curve.b.x + o_x, scale),
            formatnumber(curve.b.y + o_y, scale)
        ));
    };

    if curve.kind == CurveKind::Line {
        g1_end(rv);
    }

    if curve.kind == CurveKind::Bezier {
        let cb = CubicBezier::new(
            Vector2::new(curve.a.x as f32, curve.a.y as f32),
            Vector2::new(curve.control_point_a.x as f32, curve.control_point_a.y as f32),
            Vector2::new(curve.control_point_b.x as f32, curve.control_point_b.y as f32),
            Vector2::new(curve.b.x as f32, curve.b.y as f32),
        );

        match approx_cubic_bezier(cb, 5.0, 2.0) {
            Some(bal) => {
                for ba in bal {
                    if !ba.a1.length().is_nan() && !ba.a1.linear_length().is_nan() {
                        rv.push(get_arc_gc(&ba.a1, o_x, o_y, scale));
                    }
                    if !ba.a2.length().is_nan() && !ba.a2.linear_length().is_nan() {
                        rv.push(get_arc_gc(&ba.a2, o_x, o_y, scale));
                    }
                }
            }
            // too many points: same as exception
            None => g1_end(rv),
        }
    }
}

/// `OnPathBegin`: fast move to start, laser on.
fn on_path_begin(curves: &[Curve], l_on: &str, o_x: f64, o_y: f64, scale: f64, rv: &mut Vec<String>, skipcmd: &str) {
    if !curves.is_empty() {
        rv.push(format!(
            "{} X{} Y{}",
            skipcmd,
            formatnumber(curves[0].a.x + o_x, scale),
            formatnumber(curves[0].a.y + o_y, scale)
        ));
        rv.push(l_on.to_string());
    }
}

/// `OnPathEnd`: laser off.
fn on_path_end(curves: &[Curve], l_off: &str, rv: &mut Vec<String>) {
    if !curves.is_empty() {
        rv.push(l_off.to_string());
    }
}

/// `GetArcGC`: G2/G3 for an arc, G1 for degenerate ones.
fn get_arc_gc(arc: &Arc, o_x: f64, o_y: f64, scale: f64) -> String {
    if !caso_limite(arc, scale) {
        format!(
            "G{} X{} Y{} I{} J{}",
            if !arc.is_clockwise() { 2 } else { 3 },
            formatnumber(arc.p2.x as f64 + o_x, scale),
            formatnumber(arc.p2.y as f64 + o_y, scale),
            formatnumber((arc.c.x - arc.p1.x) as f64, scale),
            formatnumber((arc.c.y - arc.p1.y) as f64, scale)
        )
    } else {
        // approximate with a line
        format!(
            "G1 X{} Y{}",
            formatnumber(arc.p2.x as f64 + o_x, scale),
            formatnumber(arc.p2.y as f64 + o_y, scale)
        )
    }
}

/// `CasoLimite`: arc too small / flat to emit as arc.
fn caso_limite(arc: &Arc, scale: f64) -> bool {
    if arc.r == 0.0 || arc.linear_length() == 0.0 {
        return true;
    }
    if (arc.linear_length() as f64 / scale) <= 0.5 {
        return true;
    }
    arc.r / arc.linear_length() > 1000.0
}

/// `formatnumber`: `number / scale` as "0.###" invariant, "0" for NaN.
fn formatnumber(number: f64, scale: f64) -> String {
    let num = number / scale;
    if !num.is_nan() { fmt_double_custom(num, "0.###") } else { "0".to_string() }
}
