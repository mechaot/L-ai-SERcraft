// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Shared helpers for the golden tests: settings sandbox, conf parsing, escaping.
#![allow(dead_code)]

use lg_gcode::{configuration, GrblConfST, GrblVersionInfo};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, Once};

pub fn golden_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

static LOCK: Mutex<()> = Mutex::new(());
static INIT: Once = Once::new();

/// Serialises the tests of one process and points the settings store to a throw-away file.
pub fn sandbox() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    INIT.call_once(|| {
        let dir = std::env::var("TMPDIR").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
        let p = dir.join(format!("lg-gcode-test-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&p);
        lg_base::settings::init_at(p);
    });
    reset_settings();
    g
}

/// Keys the tests touch; deleting them equals the harness `ResetSettings`.
pub fn reset_settings() {
    for k in [
        "Grbl Configuration ST",
        "Grbl Configuration",
        "Support Hardware PWM",
        "Unidirectional Engraving",
        "Disable G0 fast skip",
        "Vector.UseSmartBezier",
        "Firmware Type",
        "GCode.CustomHeader",
        "GCode.CustomPasses",
        "GCode.CustomFooter",
    ] {
        lg_base::settings::delete(k);
    }
}

pub struct ConfDef {
    pub name: String,
    pub version: Option<GrblVersionInfo>,
    pub lines: Vec<String>,
}

/// Parse `confs.txt` of a golden set.
pub fn parse_confs(path: PathBuf) -> Vec<ConfDef> {
    let text = std::fs::read_to_string(path).unwrap();
    let mut out: Vec<ConfDef> = Vec::new();
    for l in text.lines() {
        if let Some(n) = l.strip_prefix("[conf ").and_then(|s| s.strip_suffix(']')) {
            out.push(ConfDef { name: n.to_string(), version: None, lines: vec![] });
        } else if let Some(v) = l.strip_prefix("version=") {
            out.last_mut().unwrap().version = parse_version(v);
        } else if let Some(v) = l.strip_prefix("line=") {
            out.last_mut().unwrap().lines.push(v.to_string());
        }
    }
    out
}

/// "none" | "0.8" | "1.1f".
pub fn parse_version(v: &str) -> Option<GrblVersionInfo> {
    if v == "none" {
        return None;
    }
    let (num, build) = match v.chars().last() {
        Some(c) if c.is_alphabetic() => (&v[..v.len() - 1], c),
        _ => (v, '\0'),
    };
    let mut it = num.split('.');
    let maj: i32 = it.next().unwrap().parse().unwrap();
    let min: i32 = it.next().unwrap().parse().unwrap();
    Some(GrblVersionInfo::with_build(maj, min, build))
}

/// Harness `ApplyConf`.
pub fn apply_conf(confs: &[ConfDef], name: &str) {
    if name == "default" {
        return;
    }
    let c = confs.iter().find(|c| c.name == name).unwrap();
    let mut conf = GrblConfST::with_version(c.version.clone());
    for l in &c.lines {
        conf.add_or_update(l);
    }
    lg_base::settings::set("Grbl Configuration ST", &conf);
    assert!(configuration::configuration().count() > 0);
}

/// Harness `H.Esc`.
pub fn esc(s: &str) -> String {
    use std::fmt::Write;
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => write!(o, "\\x{:02X}", c as u32).unwrap(),
            c => o.push(c),
        }
    }
    o
}

/// Inverse of [`esc`].
pub fn unesc(s: &str) -> String {
    let mut o = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            o.push(c);
            continue;
        }
        match it.next() {
            Some('\\') => o.push('\\'),
            Some('n') => o.push('\n'),
            Some('r') => o.push('\r'),
            Some('t') => o.push('\t'),
            Some('x') => {
                let h: String = it.by_ref().take(2).collect();
                o.push(char::from_u32(u32::from_str_radix(&h, 16).unwrap()).unwrap());
            }
            other => panic!("bad escape {other:?}"),
        }
    }
    o
}

/// Harness `RangeStr`.
pub fn range_str(r: &lg_gcode::XYRange) -> String {
    use lg_dotnet::fmt_decimal as d;
    format!("xmin={} xmax={} ymin={} ymax={} valid={}", d(r.x.min), d(r.x.max), d(r.y.min), d(r.y.max), r.valid_range())
}
