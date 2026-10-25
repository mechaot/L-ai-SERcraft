// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Golden test: full SVG conversions vs the original (Mono) in tests/golden/svg/ (WP-H).

use lg_svg2gcode::{ColorFilter, Firmware, GCodeFromSvg, PointF, SvgConvOptions};
use std::path::PathBuf;

/// Split on CR/LF and drop empties (as `LoadImportedSVG` does).
///
/// `s`: text. Returns the non-empty lines.
fn lines(s: &str) -> Vec<&str> {
    s.split(['\r', '\n']).filter(|l| !l.is_empty()).collect()
}

/// Settings of one harness variant (mirrors tools/golden/harness/SvgGold.cs).
///
/// `name`: variant name from index.tsv. Returns (options, scale?, max size, offset, filter, importMM).
fn variant(name: &str) -> (SvgConvOptions, bool, f32, PointF, ColorFilter, bool) {
    let mut o = SvgConvOptions::default();
    let (mut scale, mut max, mut off, mut filter, mut mm) = (false, 100.0f32, PointF::default(), ColorFilter::All, true);
    match name {
        "default" => {}
        "pwm0" => o.support_hardware_pwm = false,
        "pwm0_v11a_m4" => {
            o.support_hardware_pwm = false;
            o.max_pwm = 255.into();
            o.laser_on = "M4".into();
        }
        "pwm0_v11a" => {
            o.support_hardware_pwm = false;
            o.max_pwm = 255.into();
        }
        "powermax1000" => o.power_max = 1000,
        "smoothie" => o.firmware = Firmware::Smoothie,
        "smoothie_pm100" => {
            o.firmware = Firmware::Smoothie;
            o.power_max = 100;
        }
        "noskip" => o.disable_g0_fast_skip = true,
        "speed3000" => o.border_speed = 3000,
        "scale50_off" => {
            scale = true;
            max = 50.0;
            off = PointF { x: 5.0, y: -3.0 };
        }
        "offset_only" => off = PointF { x: 12.5, y: -3.25 },
        "importMM_false" => mm = false,
        "filterRed" => filter = ColorFilter::Red,
        "filterGreen" => filter = ColorFilter::Green,
        "filterBlue" => filter = ColorFilter::Blue,
        "filterBlack" => filter = ColorFilter::Black,
        other => panic!("unknown variant {other}"),
    }
    (o, scale, max, off, filter, mm)
}

#[test]
fn svg_matches_original() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/svg");
    let index = std::fs::read_to_string(root.join("cases/index.tsv")).expect("index");
    let (mut total, mut bad) = (0, 0);
    for row in index.lines().skip(1) {
        let f: Vec<&str> = row.split('\t').collect();
        let (id, svg, legacy, var) = (f[0], f[1], f[2] == "true", f[3]);
        let text = std::fs::read_to_string(root.join("inputs").join(svg)).expect("svg");
        let (opts, scale, max, off, filter, mm) = variant(var);
        let mut c = GCodeFromSvg::new();
        c.use_legacy_bezier = legacy;
        c.svg_scale_apply = scale;
        if scale {
            c.svg_max_size = max;
        }
        c.user_offset = off;
        let got = c.convert_from_text(&text, &opts, mm, filter);
        let exp = std::fs::read_to_string(root.join("cases").join(format!("{id}.gcode"))).expect("case");
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
                    Ok(g) => lines(g).iter().take(12).map(|s| s.to_string()).collect::<Vec<_>>().join(" | "),
                    Err(e) => e.to_string(),
                };
                eprintln!("MISMATCH {id}\n got {g}\n exp {}", lines(&exp).iter().take(12).copied().collect::<Vec<_>>().join(" | "));
            }
        }
    }
    eprintln!("svg golden: {} / {} ok", total - bad, total);
    assert_eq!(bad, 0, "{bad} of {total} mismatched");
}
