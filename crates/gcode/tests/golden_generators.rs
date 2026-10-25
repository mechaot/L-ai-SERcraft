// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! generators golden: cutting test, greyscale test, shake test.

mod common;
use common::*;
use lg_gcode::GrblFile;
use std::collections::HashMap;

/// Parse `name(k=v, k=v)` into the name and ordered values (quotes stripped).
fn parse_call(call: &str) -> (String, Vec<(String, String)>) {
    let (name, rest) = call.split_once('(').unwrap();
    let rest = rest.strip_suffix(')').unwrap();
    let args = rest
        .split(", ")
        .map(|kv| {
            let (k, v) = kv.split_once('=').unwrap();
            (k.to_string(), v.trim_matches('"').to_string())
        })
        .collect();
    (name.to_string(), args)
}

#[test]
fn generators_match_oracle() {
    let _g = sandbox();
    let root = golden_root().join("generators");
    let confs = parse_confs(root.join("confs.txt"));
    let index = std::fs::read_to_string(root.join("index.txt")).unwrap();
    let mut n = 0;
    for id in index.lines().filter(|l| !l.is_empty()) {
        let expected = std::fs::read_to_string(root.join("cases").join(format!("{id}.txt"))).unwrap();
        let (head, _) = expected.split_once("---\n").unwrap();
        let kv: HashMap<&str, &str> = head.lines().filter_map(|l| l.split_once('=')).collect();
        reset_settings();
        apply_conf(&confs, kv["grbl_conf"]);

        let (name, a) = parse_call(kv["call"]);
        let a: HashMap<String, String> = a.into_iter().collect();
        let i = |k: &str| -> i32 { a[k].parse().unwrap() };
        let mut f = GrblFile::new();
        let r = match name.as_str() {
            "GenerateCuttingTest" => f.generate_cutting_test(i("f_col"), i("f_start"), i("f_end"), i("p_start"), i("p_end"), i("s_fixed"), i("f_text"), i("s_text"), &a["title"], &a["ton"]),
            "GenerateGreyscaleTest" => f.generate_greyscale_test(
                i("f_row"), i("s_col"), i("f_start"), i("f_end"), i("s_start"), i("s_end"), i("x_size"), i("y_size"),
                a["resolution"].parse().unwrap(), i("f_grid"), i("s_grid"), &a["title"], i("f_text"), i("s_text"), &a["ton"],
            ),
            "GenerateShakeTest" => f.generate_shake_test(&a["axis"], i("flimit"), i("axislen"), i("cpower"), i("cspeed")),
            other => panic!("{other}"),
        };
        assert!(r.is_ok(), "{id}");
        let mut out = String::new();
        out.push_str(&format!("call={}\ngrbl_conf={}\ngrbl_conf.MaxRateX={}\nexception=none\n", kv["call"], kv["grbl_conf"], kv["grbl_conf.MaxRateX"]));
        out.push_str(&format!("count={}\nestimated_ticks={}\n", f.count(), f.estimated_time().ticks()));
        out.push_str(&format!("drawing_range={}\nmoving_range={}\nquadrant={}\n---\n", range_str(&f.range().drawing_range), range_str(&f.range().moving_range), f.quadrant()));
        for (i, cmd) in f.iter().enumerate() {
            out.push_str(&format!("{i}\t{}\t{}\n", cmd.time_offset().ticks(), cmd.command()));
        }
        if out != expected {
            for (x, y) in out.lines().zip(expected.lines()) {
                if x != y {
                    panic!("{id}\n got: {x}\n exp: {y}");
                }
            }
            panic!("{id}: length differs");
        }
        n += 1;
    }
    eprintln!("generator cases ok: {n}");
    assert_eq!(n, 18);
}
