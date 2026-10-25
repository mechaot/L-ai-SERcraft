// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Extra full conversions vs Mono (tools/golden/svg2gcode -> tests/golden/svg_unit/convert/).

use lg_svg2gcode::{ColorFilter, Firmware, GCodeFromSvg, PointF, SvgConvOptions};
use std::path::PathBuf;

/// Split on CR/LF, drop empties. `s`: text. Returns lines.
fn lines(s: &str) -> Vec<&str> {
    s.split(['\r', '\n']).filter(|l| !l.is_empty()).collect()
}

#[test]
fn convert_matches_original() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/svg_unit");
    let index = std::fs::read_to_string(root.join("convert/index.tsv")).expect("index");
    let (mut total, mut bad) = (0, 0);
    // same order as the harness: static `gcode` state (lastf) carries over between conversions
    for row in index.lines().skip(1) {
        let f: Vec<&str> = row.split('\t').collect();
        let (id, svg, legacy, var) = (f[0], f[1], f[2] == "true", f[3]);
        // Mono's Matrix.Transform always runs the general formula, so NaN x leaks into y there; the WPF
        // type-flag path (ported) does not. Skip the non-finite input, it is a known Mono/WPF difference.
        if svg == "units_nonfinite.svg" {
            continue;
        }
        let text = std::fs::read_to_string(root.join("inputs").join(svg)).expect("svg");
        let mut o = SvgConvOptions::default();
        let mut c = GCodeFromSvg::new();
        let mut filter = ColorFilter::All;
        match var {
            "default" => {}
            "mm_false" => c.svg_convert_to_mm = false,
            "scale50_off" => {
                c.svg_scale_apply = true;
                c.svg_max_size = 50.0;
                c.user_offset = PointF { x: 5.0, y: -3.0 };
            }
            "scale_mm_false" => {
                c.svg_scale_apply = true;
                c.svg_max_size = 75.0;
                c.user_offset = PointF { x: -2.5, y: 1.25 };
                c.svg_convert_to_mm = false;
            }
            "pwm0" => o.support_hardware_pwm = false,
            "smoothie" => o.firmware = Firmware::Smoothie,
            "noskip_speed" => {
                o.disable_g0_fast_skip = true;
                o.border_speed = 2500;
                o.power_max = 640;
            }
            "filterRed" => filter = ColorFilter::Red,
            "filterGreen" => filter = ColorFilter::Green,
            "filterBlue" => filter = ColorFilter::Blue,
            "filterBlack" => filter = ColorFilter::Black,
            v => panic!("variant {v}"),
        }
        c.use_legacy_bezier = legacy;
        let got = c.convert_from_text(&text, &o, true, filter);
        let exp = std::fs::read_to_string(root.join("convert").join(format!("{id}.gcode"))).expect("case");
        total += 1;
        let ok = match (&got, exp.strip_prefix("EXCEPTION: ")) {
            (Err(e), Some(k)) => e.kind == k.trim(),
            (Ok(g), None) => lines(g) == lines(&exp),
            _ => false,
        };
        if !ok {
            bad += 1;
            if bad <= 8 {
                let g = match &got {
                    Ok(g) => lines(g).iter().take(14).map(|s| s.to_string()).collect::<Vec<_>>().join(" | "),
                    Err(e) => e.to_string(),
                };
                eprintln!("MISMATCH {id}\n got {g}\n exp {}", lines(&exp).iter().take(14).copied().collect::<Vec<_>>().join(" | ")); if let Ok(g) = &got { let (a, b) = (lines(g), lines(&exp)); for (i, (x, y)) in a.iter().zip(&b).enumerate() { if x != y { eprintln!("  first diff line {i}: got {x:?} exp {y:?}"); break; } } eprintln!("  len got {} exp {}", a.len(), b.len()); }
            }
        }
    }
    eprintln!("svg_unit convert: {} / {} ok", total - bad, total);
    assert_eq!(bad, 0);
}
