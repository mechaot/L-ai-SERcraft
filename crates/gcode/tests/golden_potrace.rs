// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! potrace golden: `LoadImagePotrace` (vector fills; raster fills are smoke-tested only because
//! GDI+ resampling is not reproducible).

mod common;
use common::*;
use lg_dotnet::parse_decimal;
use lg_gcode::{Direction, Firmware, GrblFile, L2LConf};
use std::collections::HashMap;

fn dir_from(n: &str) -> Direction {
    match n {
        "None" => Direction::None,
        "Horizontal" => Direction::Horizontal,
        "Vertical" => Direction::Vertical,
        "Diagonal" => Direction::Diagonal,
        "NewHorizontal" => Direction::NewHorizontal,
        "NewVertical" => Direction::NewVertical,
        "NewDiagonal" => Direction::NewDiagonal,
        "NewReverseDiagonal" => Direction::NewReverseDiagonal,
        "NewGrid" => Direction::NewGrid,
        "NewDiagonalGrid" => Direction::NewDiagonalGrid,
        "NewCross" => Direction::NewCross,
        "NewDiagonalCross" => Direction::NewDiagonalCross,
        "NewSquares" => Direction::NewSquares,
        "NewZigZag" => Direction::NewZigZag,
        "NewHilbert" => Direction::NewHilbert,
        "NewInsetFilling" => Direction::NewInsetFilling,
        other => panic!("dir {other}"),
    }
}

/// Runs all cases of a set; returns (ok, mismatching ids).
fn run_set(area: &str, exact: bool) -> (usize, Vec<String>) {
    let root = golden_root().join(area);
    let confs = parse_confs(root.join("confs.txt"));
    let index = std::fs::read_to_string(root.join("index.txt")).unwrap();
    let mut ok = 0;
    let mut bad = Vec::new();
    for id in index.lines().filter(|l| !l.is_empty()) {
        let expected = std::fs::read_to_string(root.join("cases").join(format!("{id}.txt"))).unwrap();
        let (head, _) = expected.split_once("---\n").unwrap();
        let kv: HashMap<&str, &str> = head.lines().filter_map(|l| l.split_once('=')).collect();
        let b = |k: &str| kv[k] == "true";
        reset_settings();
        apply_conf(&confs, kv["grbl_conf"]);
        lg_base::settings::set("Support Hardware PWM", b("setting.Support Hardware PWM"));
        lg_base::settings::set("Disable G0 fast skip", b("setting.Disable G0 fast skip"));

        let mut bmp = image::open(root.join("inputs").join(kv["image"])).unwrap().to_rgba8();
        let mut c = L2LConf {
            res: kv["res"].parse().unwrap(),
            fres: kv["fres"].parse().unwrap(),
            o_x: kv["oX"].parse().unwrap(),
            o_y: kv["oY"].parse().unwrap(),
            mark_speed: kv["markSpeed"].parse().unwrap(),
            border_speed: kv["borderSpeed"].parse().unwrap(),
            min_power: kv["minPower"].parse().unwrap(),
            max_power: kv["maxPower"].parse().unwrap(),
            l_on: kv["lOn"].to_string(),
            l_off: kv["lOff"].to_string(),
            dir: dir_from(kv["dir"]),
            pwm: b("c.pwm"),
            firmware_type: Firmware::Grbl,
            ..Default::default()
        };
        let mut f = GrblFile::new();
        let r = f.load_image_potrace(
            &mut bmp,
            "test",
            b("UseSpotRemoval"),
            kv["SpotRemoval"].parse().unwrap(),
            b("UseSmoothing"),
            parse_decimal(kv["Smoothing"]).unwrap(),
            b("UseOptimize"),
            parse_decimal(kv["Optimize"]).unwrap(),
            b("useOptimizeFast"),
            &mut c,
            false,
        );
        assert!(r.is_ok(), "{id}: {r:?}");
        if !exact {
            ok += 1;
            continue;
        }
        let mut out = String::new();
        for l in head.lines() {
            let k = l.split_once('=').unwrap().0;
            match k {
                "exception" => out.push_str("exception=none\n"),
                "count" => out.push_str(&format!("count={}\n", f.count())),
                "estimated_ticks" => out.push_str(&format!("estimated_ticks={}\n", f.estimated_time().ticks())),
                "drawing_range" => out.push_str(&format!("drawing_range={}\n", range_str(&f.range().drawing_range))),
                "moving_range" => out.push_str(&format!("moving_range={}\n", range_str(&f.range().moving_range))),
                "quadrant" => out.push_str(&format!("quadrant={}\n", f.quadrant())),
                _ => {
                    out.push_str(l);
                    out.push('\n');
                }
            }
        }
        out.push_str("---\n");
        for cmd in f.iter() {
            out.push_str(cmd.command());
            out.push('\n');
        }
        if out == expected {
            ok += 1;
        } else {
            let first = out.lines().zip(expected.lines()).find(|(a, b)| a != b).map(|(a, b)| format!("got [{a}] exp [{b}]")).unwrap_or_else(|| "length".into());
            bad.push(format!("{id}: {first}"));
        }
    }
    (ok, bad)
}

#[test]
fn potrace_matches_oracle() {
    let _g = sandbox();
    let (ok, bad) = run_set("potrace", true);
    eprintln!("potrace cases ok: {ok}, mismatch: {}", bad.len());
    for b in bad.iter().take(10) {
        eprintln!("{b}");
    }
    assert!(bad.is_empty(), "{} potrace cases differ", bad.len());
}

#[test]
fn potrace_rasterfill_smoke() {
    let _g = sandbox();
    let (ok, _) = run_set("potrace_rasterfill", false);
    eprintln!("potrace_rasterfill smoke ok: {ok}");
    assert!(ok > 0);
}
