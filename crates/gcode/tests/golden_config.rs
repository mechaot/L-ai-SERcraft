// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! config golden: `GrblConfST` parsing + derived properties, `GrblVersionInfo` comparisons.

mod common;
use common::*;
use lg_dotnet::fmt_decimal as d;
use lg_gcode::{GrblConfST, GrblVersionInfo};

fn versions() -> Vec<(&'static str, Option<GrblVersionInfo>)> {
    vec![
        ("none", None),
        ("0.8", Some(GrblVersionInfo::new(0, 8))),
        ("0.9", Some(GrblVersionInfo::new(0, 9))),
        ("0.9j", Some(GrblVersionInfo::with_build(0, 9, 'j'))),
        ("1.0", Some(GrblVersionInfo::new(1, 0))),
        ("1.1", Some(GrblVersionInfo::new(1, 1))),
        ("1.1f", Some(GrblVersionInfo::with_build(1, 1, 'f'))),
        ("1.2", Some(GrblVersionInfo::new(1, 2))),
        ("ortur1.1", Some(GrblVersionInfo::with_vendor(1, 1, 'f', Some("Ortur Laser Master 2"), Some("170"), false))),
    ]
}

fn esc_opt(s: Option<String>) -> String {
    s.map(|s| esc(&s)).unwrap_or_else(|| "<null>".into())
}

fn data(c: &GrblConfST) -> String {
    let kv: Vec<String> = c.iter().map(|(k, v)| format!("{k}={}", esc(v))).collect();
    kv.join("|")
}

#[test]
fn config_matches_oracle() {
    let _g = sandbox();
    let expected = std::fs::read_to_string(golden_root().join("config/cases.txt")).unwrap();
    let vers = versions();
    let mut bad = 0;
    let mut n = 0;
    for block in expected.split("\n\n").filter(|b| !b.trim().is_empty()) {
        let lines: Vec<String> = block.lines().filter_map(|l| l.strip_prefix("line.")).map(|l| unesc(l.split_once('=').unwrap().1)).collect();
        let mut out = String::new();
        out.push_str(block.lines().next().unwrap());
        out.push('\n');
        for (i, l) in lines.iter().enumerate() {
            out.push_str(&format!("line.{i}={}\n", esc(l)));
        }
        for (i, l) in lines.iter().enumerate() {
            out.push_str(&format!("IsSetConf.{i}={}\n", GrblConfST::is_set_conf(l)));
        }
        for (vn, v) in &vers {
            let mut c = GrblConfST::with_version(v.clone());
            for l in &lines {
                c.add_or_update(l);
            }
            let p = format!("v[{vn}].");
            out.push_str(&format!("{p}Count={}\n{p}data={}\n", c.count(), if c.count() == 0 { "<empty>".to_string() } else { data(&c) }));
            out.push_str(&format!("{p}ExpectedCount={}\n", c.expected_count()));
            out.push_str(&format!("{p}HomingEnabled={}\n", c.homing_enabled()));
            out.push_str(&format!("{p}MaxRateX={}\n{p}MaxRateY={}\n", d(c.max_rate_x()), d(c.max_rate_y())));
            out.push_str(&format!("{p}LaserMode={}\n", c.laser_mode()));
            out.push_str(&format!("{p}MinPWM={}\n{p}MaxPWM={}\n", d(c.min_pwm()), d(c.max_pwm())));
            out.push_str(&format!("{p}ResolutionX={}\n{p}ResolutionY={}\n", d(c.resolution_x()), d(c.resolution_y())));
            out.push_str(&format!("{p}TableWidth={}\n{p}TableHeight={}\n", d(c.table_width()), d(c.table_height())));
            out.push_str(&format!("{p}SoftLimit={}\n", c.soft_limit()));
            out.push_str(&format!("{p}AccelerationXY={}\n", d(c.acceleration_xy())));
            out.push_str(&format!("{p}WiFi_SSID={}\n{p}WiFi_Pwd={}\n{p}TelnetPort={}\n", esc_opt(c.wifi_ssid()), esc_opt(c.wifi_pwd()), esc_opt(c.telnet_port())));
            out.push_str(&format!("{p}ValidateConfig(33)={}\n", esc_opt(c.validate_config(33).map(str::to_string))));
        }
        for l in &lines {
            let mut c = GrblConfST::with_version(Some(GrblVersionInfo::new(1, 1)));
            c.add_or_update("$110=111");
            c.add_or_update("$30=222");
            let r = c.set_value_if_key_exist(l);
            out.push_str(&format!("SetValueIfKeyExist({})={} data={}\n", esc(l), r, data(&c)));
        }
        n += 1;
        if out.trim_end() != block.trim_end() {
            bad += 1;
            if bad <= 3 {
                for (a, b) in out.lines().zip(block.lines()) {
                    if a != b {
                        eprintln!("case {n}: got [{a}] expected [{b}]");
                        break;
                    }
                }
            }
        }
    }
    assert_eq!(bad, 0, "{bad}/{n} config cases differ");
    eprintln!("config cases ok: {n}");
}

#[test]
fn version_info_matches_oracle() {
    let _g = sandbox();
    let expected = std::fs::read_to_string(golden_root().join("config/versions.txt")).unwrap();
    let vers = versions();
    let mut out = String::new();
    for (vn, v) in &vers {
        out.push_str(&format!("[version {vn}]\n"));
        let Some(v) = v else {
            out.push_str("null\n\n");
            continue;
        };
        out.push_str(&format!(
            "ToString={v}\nMajor={}\nMinor={}\nIsOrtur={}\nIsLonger={}\nIsHAL={}\nOrturFWVersionNumber={}\n",
            v.major(),
            v.minor(),
            v.is_ortur(),
            v.is_longer(),
            v.is_hal(),
            v.ortur_fw_version_number()
        ));
        for (vn2, w) in &vers {
            let Some(w) = w else { continue };
            out.push_str(&format!("cmp({vn2})={} ge={} eq={}\n", v.compare_to(w), v >= w, v == w));
        }
        out.push('\n');
    }
    assert_eq!(out.trim_end(), expected.trim_end());
}
