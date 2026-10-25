// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Unit oracle tests vs Mono (tools/golden/svg2gcode -> tests/golden/svg_unit/).

use lg_svg2gcode::{Matrix, Point, Vector};
use std::path::PathBuf;

/// Golden dir. Returns tests/golden/svg_unit.
fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/svg_unit")
}

/// Read a golden/input file. `name`: file name. Returns its lines.
fn read(name: &str) -> Vec<String> {
    std::fs::read_to_string(dir().join(name)).expect(name).lines().map(str::to_string).collect()
}

/// Double as 16 hex digits of its bits; NaN canonical.
fn d(v: f64) -> String {
    if v.is_nan() { "NaN".into() } else { format!("{:016x}", v.to_bits()) }
}

/// Float as 8 hex digits; NaN canonical.
fn f(v: f32) -> String {
    if v.is_nan() { "NaN".into() } else { format!("{:08x}", v.to_bits()) }
}

/// Matrix as the six hex doubles plus identity flag.
fn m(x: &Matrix) -> String {
    format!(
        "{} {} {} {} {} {} {}",
        d(x.m11()), d(x.m12()), d(x.m21()), d(x.m22()), d(x.offset_x()), d(x.offset_y()),
        if x.is_identity() { 1 } else { 0 }
    )
}

/// Parse f64 like C# `double.Parse`. `s`: text.
fn p(s: &str) -> f64 {
    lg_dotnet::parse_double(s).expect("num")
}

/// Build matrix from 6 tokens at offset `o`.
fn mk(t: &[&str], o: usize) -> Matrix {
    Matrix::new(p(t[o]), p(t[o + 1]), p(t[o + 2]), p(t[o + 3]), p(t[o + 4]), p(t[o + 5]))
}

/// Tolerant comparison of two record strings (hex doubles; flags and EX markers must match in shape).
///
/// `a`,`b`: records. Returns true when all doubles agree to ~1e-9 relative.
fn close(a: &str, b: &str, tol: f64) -> bool {
    let (ra, rb): (Vec<&str>, Vec<&str>) = (a.split('|').collect(), b.split('|').collect());
    if ra.len() != rb.len() {
        return false;
    }
    for (x, y) in ra.iter().zip(&rb) {
        let (tx, ty): (Vec<&str>, Vec<&str>) = (x.split(' ').collect(), y.split(' ').collect());
        // Mono and WPF disagree on singular detection and thus EX; require both or neither
        if x.starts_with("EX") || y.starts_with("EX") {
            if x != y {
                return false;
            }
            continue;
        }
        if tx.len() != ty.len() {
            return false;
        }
        for (u, v) in tx.iter().zip(&ty) {
            if u.len() == 16 && v.len() == 16 {
                let (fu, fv) = (f64::from_bits(u64::from_str_radix(u, 16).unwrap()), f64::from_bits(u64::from_str_radix(v, 16).unwrap()));
                if fu.is_finite() != fv.is_finite() || (fu - fv).abs() > tol * (1.0 + fu.abs().max(fv.abs())) {
                    return false;
                }
            } else if u != v && u.len() != 1 {
                // 1-char tokens (identity flag) may differ between Mono and WPF
                return false;
            }
        }
    }
    true
}

#[test]
fn matrix_ops() {
    let (ins, exp) = (read("matrix_ops.txt"), read("matrix.txt"));
    assert_eq!(ins.len(), exp.len());
    let mut bad = 0;
    for (line, want) in ins.iter().zip(&exp) {
        let mut mat = Matrix::default();
        let mut rec: Vec<String> = Vec::new();
        for op in line.split(';') {
            let t: Vec<&str> = op.split(' ').collect();
            match t[0] {
                "new" => mat = mk(&t, 1),
                "ident" => mat.set_identity(),
                "set" => {
                    let v = p(t[2]);
                    match t[1] {
                        "M11" => mat.set_m11(v),
                        "M12" => mat.set_m12(v),
                        "M21" => mat.set_m21(v),
                        "M22" => mat.set_m22(v),
                        "OffsetX" => mat.set_offset_x(v),
                        _ => mat.set_offset_y(v),
                    }
                }
                "scale" => mat.scale(p(t[1]), p(t[2])),
                "scaleat" => mat.scale_at(p(t[1]), p(t[2]), p(t[3]), p(t[4])),
                "rotate" => mat.rotate(p(t[1])),
                "rotateat" => mat.rotate_at(p(t[1]), p(t[2]), p(t[3])),
                "translate" => mat.translate(p(t[1]), p(t[2])),
                "skew" => mat.skew(p(t[1]), p(t[2])),
                "mul" => mat = Matrix::multiply(mat, mk(&t, 1)),
                "mulr" => mat = Matrix::multiply(mk(&t, 1), mat),
                "invert" => {
                    if mat.invert().is_err() {
                        rec.push("EX:System.InvalidOperationException".into());
                        continue;
                    }
                }
                "xf" => {
                    let r = mat.transform(Point::new(p(t[1]), p(t[2])));
                    rec.push(format!("P {} {}", d(r.x), d(r.y)));
                    continue;
                }
                "xv" => {
                    let r = mat.transform_vector(Vector::new(p(t[1]), p(t[2])));
                    rec.push(format!("V {} {}", d(r.x), d(r.y)));
                    continue;
                }
                o => panic!("op {o}"),
            }
            rec.push(m(&mat));
        }
        let got = rec.join("|");
        // Mono's Matrix is not bit-identical to WPF: it always uses the general formulas (so -0.0 appears
        // where WPF's type-flag paths give 0.0, other association in Transform), and Rotate/RotateAt/Skew/
        // ScaleAt/Invert use other formulas (no %360). So: tolerance 1e-13 relative, 1e-9 for those ops.
        let loose = line.split(';').any(|o| ["rotate", "rotateat", "skew", "scaleat", "invert"].contains(&o.split(' ').next().unwrap()));
        let same = close(&got, want, if loose { 1e-9 } else { 1e-13 });
        if !same {
            bad += 1;
            if bad <= 5 {
                eprintln!("MATRIX MISMATCH\n ops {line}\n got {got}\n exp {want}");
            }
        }
    }
    eprintln!("matrix: {} / {} ok", ins.len() - bad, ins.len());
    assert_eq!(bad, 0);
}

#[test]
fn bezier() {
    let (ins, exp) = (read("bezier_in.txt"), read("bezier.txt"));
    let mut bad = 0;
    for (line, want) in ins.iter().zip(&exp) {
        let fl: Vec<&str> = line.split('\t').collect();
        let c: Vec<f64> = fl[2].split(' ').map(p).collect();
        let pts: Vec<Point> = c.chunks(2).map(|x| Point::new(x[0], x[1])).collect();
        let res = if fl[0] == "F" {
            lg_svg2gcode::flatten_to(&pts, p(fl[3]), fl[4].parse().unwrap())
        } else {
            lg_svg2gcode::bezier_approximation_old(&pts, fl[4].parse().unwrap())
        };
        let got = res.iter().map(|q| format!("{} {}", d(q.x), d(q.y))).collect::<Vec<_>>().join("|");
        if &got != want {
            bad += 1;
            if bad <= 5 {
                eprintln!("BEZIER MISMATCH {line}\n got {got}\n exp {want}");
            }
        }
    }
    eprintln!("bezier: {} / {} ok", ins.len() - bad, ins.len());
    assert_eq!(bad, 0);
}

#[test]
fn parsing_helpers() {
    let (ins, exp) = (read("parse_in.txt"), read("parse.txt"));
    let mut bad = 0;
    for (line, want) in ins.iter().zip(&exp) {
        let fl: Vec<&str> = line.split('\t').collect();
        let got = match fl[0] {
            "px" => {
                let ext = if fl.len() > 2 { p(fl[2]) as f32 } else { 1.0 };
                f(lg_svg2gcode::convert_to_pixel(fl[1], ext))
            }
            "tf" => match lg_svg2gcode::parse_transform_string(fl[1]) {
                Ok(x) => m(&x),
                Err(e) => format!("EX:{}", e.kind),
            },
            "tb" => match lg_svg2gcode::get_text_between(fl[1], fl[2]) {
                Ok(s) => format!("S:{s}"),
                Err(e) => format!("EX:{}", e.kind),
            },
            "color" => match lg_svg2gcode::color_from_html(fl[1]) {
                Ok(None) => "EMPTY".to_string(),
                Ok(Some(c)) => format!("{} {} {}", c[0], c[1], c[2]),
                Err(e) => format!("EX:{}", e.kind),
            },
            k => panic!("kind {k}"),
        };
        // tf: Mono's RotateAt/%360 differ from WPF (see matrix test); colour: Mono's html-system-name table is
        // incomplete (ThreeDShadow throws there), Fx/WPF-era table accepts it.
        let same = match fl[0] {
            "tf" => close(&got, want, 1e-9),
            "color" if want.starts_with("EX:") && !got.starts_with("EX:") => {
                ["threedshadow", "threedhighlight", "threeddarkshadow", "threedlightshadow"].contains(&fl[1].to_lowercase().as_str())
            }
            _ => &got == want,
        };
        if !same {
            bad += 1;
            if bad <= 40 {
                eprintln!("PARSE MISMATCH {line:?}\n got {got}\n exp {want}");
            }
        }
    }
    eprintln!("parse: {} / {} ok", ins.len() - bad, ins.len());
    assert_eq!(bad, 0);
}
