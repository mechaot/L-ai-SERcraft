// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! l2l golden: `LoadImageL2L` over synthetic bitmaps.

mod common;
use common::*;
use lg_gcode::{Direction, Firmware, GrblFile, L2LConf};
use std::collections::HashMap;

pub fn dir_from(n: &str) -> Direction {
    match n {
        "Horizontal" => Direction::Horizontal,
        "Vertical" => Direction::Vertical,
        "Diagonal" => Direction::Diagonal,
        other => panic!("dir {other}"),
    }
}

#[test]
fn l2l_matches_oracle() {
    let _g = sandbox();
    let root = golden_root().join("l2l");
    let confs = parse_confs(root.join("confs.txt"));
    let index = std::fs::read_to_string(root.join("index.txt")).unwrap();
    let mut n = 0;
    for id in index.lines().filter(|l| !l.is_empty()) {
        let expected = std::fs::read_to_string(root.join("cases").join(format!("{id}.txt"))).unwrap();
        let (head, _body) = expected.split_once("---\n").unwrap();
        let kv: HashMap<&str, &str> = head.lines().filter_map(|l| l.split_once('=')).collect();

        reset_settings();
        apply_conf(&confs, kv["grbl_conf"]);
        lg_base::settings::set("Unidirectional Engraving", kv["setting.Unidirectional Engraving"] == "true");
        lg_base::settings::set("Disable G0 fast skip", kv["setting.Disable G0 fast skip"] == "true");

        let mut bmp = image::open(root.join("inputs").join(kv["image"])).unwrap().to_rgba8();
        let c = L2LConf {
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
            pwm: kv["pwm"] == "true",
            firmware_type: if kv["firmwareType"] == "Smoothie" { Firmware::Smoothie } else { Firmware::Grbl },
            ..Default::default()
        };
        let mut f = GrblFile::new();
        let r = f.load_image_l2l(&mut bmp, "test", &c, false);
        assert!(r.is_ok(), "{id}");
        let mut out = String::new();
        for k in ["image", "width", "height", "dir", "pwm", "firmwareType", "res", "fres", "oX", "oY", "minPower", "maxPower", "markSpeed", "borderSpeed", "lOn", "lOff", "setting.Unidirectional Engraving", "setting.Disable G0 fast skip", "grbl_conf", "grbl_conf.MaxPWM", "grbl_conf.MaxRateX"] {
            out.push_str(&format!("{k}={}\n", kv[k]));
        }
        out.push_str(&format!("exception=none\ncount={}\nestimated_ticks={}\n", f.count(), f.estimated_time().ticks()));
        out.push_str(&format!("drawing_range={}\nmoving_range={}\nquadrant={}\n---\n", range_str(&f.range().drawing_range), range_str(&f.range().moving_range), f.quadrant()));
        for cmd in f.iter() {
            out.push_str(cmd.command());
            out.push('\n');
        }
        if out != expected {
            for (a, b) in out.lines().zip(expected.lines()) {
                if a != b {
                    panic!("{id}\n got: {a}\n exp: {b}");
                }
            }
            panic!("{id}: length differs {} vs {}", out.len(), expected.len());
        }
        n += 1;
    }
    eprintln!("l2l cases ok: {n}");
    assert!(n > 100);
}
