// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Golden test: `Expression` vs original C# (Mono) output in tests/golden/expr/cases.tsv.

use lg_expr::{ExprError, Expression};
use std::path::PathBuf;

/// Undo the generator's escaping (`\\`, `\t`, `\n`, `\r`).
///
/// * `s` - escaped text.
///
/// Returns the original string.
fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Render a result like the generator: `hexbits/R` or `NaN` or `ERR:Kind`.
///
/// * `r` - evaluation result.
///
/// Returns the hex bits part only (the "R" tail is not compared), or error text.
fn show_d(r: Result<f64, ExprError>) -> String {
    match r {
        Ok(v) if v.is_nan() => "NaN".into(),
        Ok(v) => format!("{:016x}", v.to_bits()),
        Err(e) => format!("ERR:{}", e.kind()),
    }
}

#[test]
fn expr_matches_original() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/expr/cases.tsv");
    let data = std::fs::read_to_string(path).expect("golden file");
    let vars = "left=-12.5;right=100;top=47.25;bottom=0.1;width=112.5;height=47.15;jogstep=10;jogspeed=1000;WCO.X=1.5;WCO.Y=-2;MPos.X=3;$30=1000;$32=0;x=2;y=-3;z=0;t=1;e=7;a1=0.5;true_=1";
    let (mut total, mut bad) = (0, 0);
    for line in data.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split('\t').collect();
        let expr = unescape(f[2]);
        let vs: usize = f[1].parse().unwrap();
        let mut got = vec![];
        // ctor
        let e = Expression::new(&expr);
        got.push(match &e {
            Ok(_) => "OK".to_string(),
            Err(e) => format!("ERR:{}", e.kind()),
        });
        // expand
        got.push(match lg_expr::expand(&expr) {
            Ok(s) => s,
            Err(e) => format!("ERR:{}", e.kind()),
        });
        let (mut post, mut d, mut b) = ("-".to_string(), "-".to_string(), "-".to_string());
        if let Ok(mut e) = e {
            post = e.post_fix().to_string();
            if vs == 1 {
                for kv in vars.split(';') {
                    let (k, v) = kv.split_once('=').unwrap();
                    e.add_set_variable(k, v.parse().unwrap());
                }
            }
            d = show_d(e.evaluate_d());
            b = match e.evaluate_b() {
                Ok(true) => "True".into(),
                Ok(false) => "False".into(),
                Err(e) => format!("ERR:{}", e.kind()),
            };
        }
        got.push(post);
        got.push(d);
        got.push(b);
        // expected columns: ctor, expand, postfix, evalD (strip "/R" tail), evalB
        let exp_d = f[6].split('/').next().unwrap().to_string();
        let exp = [f[3].to_string(), unescape(f[4]), unescape(f[5]), exp_d, f[7].to_string()];
        total += 1;
        if got[..] != exp[..] {
            bad += 1;
            if bad <= 15 {
                eprintln!("MISMATCH vs={vs} expr={expr:?}\n got {got:?}\n exp {exp:?}");
            }
        }
    }
    eprintln!("expr golden: {total} cases, {bad} mismatches");
    assert!(total > 500);
    assert_eq!(bad, 0);
}
