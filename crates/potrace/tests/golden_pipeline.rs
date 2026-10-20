//! Whole-pipeline pieces of `tests/golden/potrace` (full `LoadImagePotrace` G-code, Mono oracle).
//!
//! GrblFile-level steps (path optimisation, command analysis) live in `lg-gcode`, so here:
//! * border part (everything after `F<borderSpeed>`) is compared exactly (non-fast cases);
//! * vector fill paths are compared as a multiset of point lists, order/direction independent
//!   (C# `ParallelOptimizePaths` reorders and reverses them).

use image::imageops::flip_vertical;
use lg_potrace::{
    Curve, Direction, FillConf, PotraceParams, build_filling, export_to_gcode, potrace_trace,
};
use std::collections::HashMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/potrace")
}

fn dir_of(s: &str) -> Direction {
    use Direction::*;
    match s {
        "None" => None,
        "Horizontal" => Horizontal,
        "Vertical" => Vertical,
        "Diagonal" => Diagonal,
        "NewHorizontal" => NewHorizontal,
        "NewVertical" => NewVertical,
        "NewDiagonal" => NewDiagonal,
        "NewReverseDiagonal" => NewReverseDiagonal,
        "NewGrid" => NewGrid,
        "NewDiagonalGrid" => NewDiagonalGrid,
        "NewCross" => NewCross,
        "NewDiagonalCross" => NewDiagonalCross,
        "NewSquares" => NewSquares,
        "NewZigZag" => NewZigZag,
        "NewHilbert" => NewHilbert,
        "NewInsetFilling" => NewInsetFilling,
        _ => panic!("dir {s}"),
    }
}

/// Split G-code into paths of point strings keyed by command text, normalised for direction.
fn paths_of(lines: &[String], skip: &str) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    for l in lines {
        if l.starts_with(&format!("{skip} X")) {
            out.push(vec![l.split_once(' ').unwrap().1.to_string()]);
        } else if l.starts_with("G1 ") || l.starts_with("G2 ") || l.starts_with("G3 ") {
            out.last_mut().unwrap().push(l.clone());
        }
    }
    out
}

/// Plain point list (X/Y of every move) of a path.
fn norm(p: &[String]) -> Vec<String> {
    let pts: Vec<String> = p
        .iter()
        .map(|l| {
            let w: Vec<&str> = l.split(' ').collect();
            w.iter().filter(|t| t.starts_with('X') || t.starts_with('Y')).cloned().collect::<Vec<_>>().join(" ")
        })
        .collect();
    let mut rev = pts.clone();
    rev.reverse();
    if rev < pts { rev } else { pts }
}

#[test]
fn potrace_pipeline_parts() {
    let index = std::fs::read_to_string(root().join("index.txt")).unwrap();
    let mut n = 0;
    let mut n_fill = 0;
    let mut bad = Vec::new();
    for id in index.lines().filter(|l| !l.is_empty()) {
        let text = std::fs::read_to_string(root().join("cases").join(format!("{id}.txt"))).unwrap();
        let (head, body) = text.split_once("---\n").unwrap();
        let kv: HashMap<&str, &str> = head.lines().filter_map(|l| l.split_once('=')).collect();
        if kv["exception"] != "none" {
            continue;
        }
        let b = |k: &str| kv[k] == "true";
        let params = PotraceParams {
            turdsize: if b("UseSpotRemoval") { kv["SpotRemoval"].parse().unwrap() } else { 2 },
            alphamax: if b("UseSmoothing") { kv["Smoothing"].parse().unwrap() } else { 0.0 },
            opttolerance: if b("UseOptimize") { kv["Optimize"].parse().unwrap() } else { 0.2 },
            curveoptimizing: b("UseOptimize"),
            ..PotraceParams::default()
        };
        let img = image::open(root().join("inputs").join(kv["image"])).unwrap().to_rgba8();
        let img = flip_vertical(&img);
        let (w, h) = (img.width() as f64, img.height() as f64);
        let res: f64 = kv["res"].parse().unwrap();
        let fres: f64 = kv["fres"].parse().unwrap();
        let ox: f32 = kv["oX"].parse().unwrap();
        let oy: f32 = kv["oY"].parse().unwrap();
        let pwm = b("setting.Support Hardware PWM");
        let skip = if b("setting.Disable G0 fast skip") { "G1" } else { "G0" };
        let (l_on, l_off) = if pwm {
            (format!("S{}", kv["maxPower"]), "S0".to_string())
        } else {
            (kv["lOn"].to_string(), kv["lOff"].to_string())
        };
        let dir = dir_of(kv["dir"]);
        let mut plist: Vec<Vec<Curve>> = potrace_trace(&img, &params);
        let exp: Vec<String> = body.lines().map(String::from).collect();
        let size = (img.width(), img.height());

        // vector fill
        if dir >= Direction::NewHorizontal {
            let conf = FillConf { dir, res, fres };
            let fl = build_filling(&plist, w, h, &conf).unwrap().unwrap();
            let got = export_to_gcode(&fl, ox, oy, res, &l_on, &l_off, size, skip);
            let mark = exp.iter().position(|l| *l == format!("F{}", kv["markSpeed"])).unwrap();
            let border = exp.iter().rposition(|l| *l == format!("F{}", kv["borderSpeed"])).unwrap();
            let mut a: Vec<_> = paths_of(&got, skip).iter().map(|p| norm(p)).collect();
            let mut e: Vec<_> = paths_of(&exp[mark..border], skip).iter().map(|p| norm(p)).collect();
            a.sort();
            e.sort();
            n_fill += 1;
            if a != e {
                bad.push(format!("fill {id}"));
            }
        }

        // border (only when not path-optimised)
        if !b("useOptimizeFast") {
            plist.reverse();
            let got = export_to_gcode(&plist, ox, oy, res, &l_on, &l_off, size, skip);
            let border = exp.iter().rposition(|l| *l == format!("F{}", kv["borderSpeed"])).unwrap();
            let tail: Vec<String> = exp[border + 1..].to_vec();
            // C# adds trailing laser off line when supportPWM
            let mut got = got;
            if pwm {
                got.push(kv["lOff"].to_string());
            }
            if got != tail {
                bad.push(format!("border {id}"));
            }
        }
        n += 1;
    }
    eprintln!("cases {n}, fill cases {n_fill}, mismatches {}", bad.len());
    assert!(n >= 250);
    assert!(bad.is_empty(), "{bad:?}");
}
