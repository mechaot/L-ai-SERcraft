// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! gcode_analyze + state_builder goldens.

mod common;
use common::*;
use lg_dotnet::{fmt_decimal as d, fmt_double};
use lg_gcode::{GrblCommand, GrblConfST, GrblFile, StatePositionBuilder, configuration};

fn programs() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(golden_root().join("gcode_analyze"))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".nc"))
        .collect();
    v.sort();
    v
}

#[test]
fn gcode_analyze_matches_oracle() {
    let _g = sandbox();
    let root = golden_root().join("gcode_analyze");
    let confs = parse_confs(root.join("confs.txt"));
    let mut files = 0;
    for prog in programs() {
        let stem = prog.trim_end_matches(".nc");
        for cn in ["default", "v08", "v09", "v11a", "v11b"] {
            let exp_path = root.join(format!("{stem}.{cn}.txt"));
            let Ok(expected) = std::fs::read_to_string(&exp_path) else { continue };
            reset_settings();
            apply_conf(&confs, cn);
            lg_base::settings::set("Support Hardware PWM", true);
            let mut f = GrblFile::new();
            let ok = f.load_file(root.join(&prog).to_str().unwrap(), false).is_ok();
            let mut out = String::new();
            out.push_str(&format!("program={prog}\nconf={cn}\nload_ok={ok}\ncount={}\n", f.count()));
            out.push_str(&format!("estimated_ticks={}\n", f.estimated_time().ticks()));
            out.push_str(&format!("drawing_range={}\nmoving_range={}\n", range_str(&f.range().drawing_range), range_str(&f.range().moving_range)));
            let s = &f.range().spindle_range;
            out.push_str(&format!("spindle_range=smin={} smax={} valid={}\n", d(s.s.min), d(s.s.max), s.valid_range()));
            out.push_str(&format!("quadrant={}\n---\n", f.quadrant()));
            for (i, c) in f.iter().enumerate() {
                out.push_str(&format!("{i}\t{}\t{}\n", c.time_offset().ticks(), esc(c.command())));
            }
            assert_eq!(out, expected, "{prog} {cn}");
            files += 1;
        }
    }
    eprintln!("gcode_analyze files ok: {files}");
    assert!(files > 50);
}

#[test]
fn state_builder_matches_oracle() {
    let _g = sandbox();
    let root = golden_root().join("state_builder");
    let src = golden_root().join("gcode_analyze");
    let mut files = 0;
    for prog in programs() {
        let stem = prog.trim_end_matches(".nc");
        for pwm in [true, false] {
            let exp_path = root.join(format!("{stem}.pwm{}.txt", if pwm { 1 } else { 0 }));
            let Ok(expected) = std::fs::read_to_string(&exp_path) else { continue };
            reset_settings();
            lg_base::settings::set("Support Hardware PWM", pwm);
            let text = std::fs::read_to_string(src.join(&prog)).unwrap();
            let mut list: Vec<GrblCommand> = Vec::new();
            for raw in text.split('\n') {
                let line = raw.replace('\r', "");
                let line = line.trim();
                if !line.is_empty() {
                    let c = GrblCommand::new(line);
                    if !c.is_empty() {
                        list.push(c);
                    }
                }
            }
            let mut out = String::new();
            out.push_str(&format!("program={prog}\nsupport_pwm={pwm}\nconf=default\ncount={}\n---\n", list.len()));
            let mut spb = StatePositionBuilder::new();
            let conf: GrblConfST = configuration::configuration();
            let mut prev_gen = 0u64;
            for (i, cmd) in list.iter_mut().enumerate() {
                let delay = spb.analyze_command(cmd, true, Some(&conf)).unwrap();
                let mut f: Vec<String> = vec![
                    format!("i={i}"),
                    format!("cmd={}", esc(cmd.command())),
                    format!("delay={}", delay.ticks()),
                    format!("X={}/{}", d(spb.x().number()), d(spb.x().previous())),
                    format!("Y={}/{}", d(spb.y().number()), d(spb.y().previous())),
                    format!("Z={}/{}", d(spb.z().number()), d(spb.z().previous())),
                    format!("F={}", d(spb.f().number())),
                    format!("S={}", d(spb.s().number())),
                    format!("burn={}", spb.laser_burning()),
                    format!("G0G1={}", spb.g0g1()),
                    format!("G2G3={}", spb.g2g3()),
                    format!("true_move={}", spb.true_movement()),
                    format!("motion={}", &*spb.motion_mode),
                    format!("abs={}", spb.abs()),
                    format!("m3m4={}", spb.m3m4()),
                    format!("wco={},{},{}", d(spb.wco_x()), d(spb.wco_y()), d(spb.wco_z())),
                    format!("settled={}", esc(GrblCommand::from_elements(spb.get_settled_modals().iter()).command())),
                ];
                match &spb.last_arc_helper_result {
                    None => f.push("arc=none".into()),
                    Some(a) => {
                        f.push(format!("arc={}", if spb.arc_generation == prev_gen { "stale" } else { "fresh" }));
                        let bits = |v: f64| format!("{:016x}", v.to_bits());
                        f.push(format!("arc.cw={}", a.cw));
                        f.push(format!(
                            "arc.bits={}",
                            [a.center_x, a.center_y, a.ray, a.start_angle, a.end_angle, a.angular_width, a.b_box.x, a.b_box.y, a.b_box.width, a.b_box.height]
                                .iter()
                                .map(|v| bits(*v))
                                .collect::<Vec<_>>()
                                .join(",")
                        ));
                        let _ = fmt_double;
                    }
                }
                prev_gen = spb.arc_generation;
                out.push_str(&f.join("\t"));
                out.push('\n');
            }
            // compare ignoring the textual (non-bit) arc fields
            let strip = |s: &str| -> String {
                s.lines()
                    .map(|l| {
                        l.split('\t')
                            .filter(|f| !(f.starts_with("arc.") && !f.starts_with("arc.bits") && !f.starts_with("arc.cw")))
                            .collect::<Vec<_>>()
                            .join("\t")
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let (a, b) = (strip(&out), strip(&expected));
            if a != b {
                for (x, y) in a.lines().zip(b.lines()) {
                    if x != y {
                        panic!("{prog} pwm={pwm}\n got: {x}\n exp: {y}");
                    }
                }
                panic!("{prog}: line count differs");
            }
            files += 1;
        }
    }
    eprintln!("state_builder files ok: {files}");
    assert!(files > 25);
}
