// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Exact comparison of `Tracer::trace` against `tests/golden/potrace_curves` (Mono oracle).

use lg_potrace::{CurveKind, PotraceParams, TurnPolicy, potrace_trace};
use std::collections::HashMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/potrace_curves")
}

/// Parse "x,y" (R format doubles).
fn pt(s: &str) -> (f64, f64) {
    let (x, y) = s.split_once(',').unwrap();
    (x.parse().unwrap(), y.parse().unwrap())
}

#[test]
fn potrace_curves_exact() {
    let index = std::fs::read_to_string(root().join("index.txt")).unwrap();
    let mut n = 0;
    let mut bad = Vec::new();
    for id in index.lines().filter(|l| !l.is_empty()) {
        let text = std::fs::read_to_string(root().join("cases").join(format!("{id}.txt"))).unwrap();
        let (head, body) = text.split_once("---\n").unwrap();
        let kv: HashMap<&str, &str> = head.lines().filter_map(|l| l.split_once('=')).collect();
        let params = PotraceParams {
            turnpolicy: match kv["turnpolicy"] {
                "minority" => TurnPolicy::Minority,
                "majority" => TurnPolicy::Majority,
                "right" => TurnPolicy::Right,
                "black" => TurnPolicy::Black,
                _ => TurnPolicy::White,
            },
            turdsize: kv["turdsize"].parse().unwrap(),
            alphamax: kv["alphamax"].parse().unwrap(),
            curveoptimizing: kv["curveoptimizing"] == "true",
            opttolerance: kv["opttolerance"].parse().unwrap(),
            treshold: kv["Treshold"].parse().unwrap(),
        };
        assert_eq!(kv["exception"], "none", "{id}");
        let img = image::open(root().join("inputs").join(kv["image"])).unwrap().to_rgba8();
        let got = potrace_trace(&img, &params);

        // render in golden text format
        let mut out = String::new();
        for (i, p) in got.iter().enumerate() {
            out.push_str(&format!("path {} {}\n", i, p.len()));
            for c in p {
                let k = if c.kind == CurveKind::Line { "Line" } else { "Bezier" };
                out.push_str(&format!(
                    "{k} {:?},{:?} {:?},{:?} {:?},{:?} {:?},{:?}\n",
                    c.a.x, c.a.y, c.control_point_a.x, c.control_point_a.y, c.control_point_b.x,
                    c.control_point_b.y, c.b.x, c.b.y
                ));
            }
        }
        // compare numerically (formatting of doubles may differ, values must not)
        let ok = compare(&out, body);
        if !ok {
            bad.push(id.to_string());
        }
        assert_eq!(kv["paths"].parse::<usize>().unwrap(), got.len(), "{id}");
        n += 1;
    }
    assert_eq!(n, 68);
    assert!(bad.is_empty(), "{} of {n} mismatched: {bad:?}", bad.len());
}

/// Token-wise compare, numbers by value.
fn compare(a: &str, b: &str) -> bool {
    let la: Vec<&str> = a.lines().collect();
    let lb: Vec<&str> = b.lines().collect();
    if la.len() != lb.len() {
        return false;
    }
    la.iter().zip(lb.iter()).all(|(x, y)| {
        let tx: Vec<&str> = x.split(' ').collect();
        let ty: Vec<&str> = y.split(' ').collect();
        tx.len() == ty.len()
            && tx.iter().zip(ty.iter()).all(|(p, q)| {
                if p == q {
                    return true;
                }
                match (p.split_once(','), q.split_once(',')) {
                    (Some(_), Some(_)) => pt(p) == pt(q),
                    _ => false,
                }
            })
    })
}
