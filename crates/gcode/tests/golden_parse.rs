//! gcode_parse golden: byte compare of the dump produced from `GrblCommand`.

use lg_gcode::{BuildFailure, GrblCommand};
use std::fmt::Write;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

/// Harness `H.Esc`.
fn esc(s: &str) -> String {
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

fn b(v: bool) -> &'static str {
    if v { "true" } else { "false" }
}

fn dump(out: &mut String, p: &str, mut c: GrblCommand) {
    let kv = |out: &mut String, k: &str, v: &str| writeln!(out, "{p}{k}={v}").unwrap();
    kv(out, "pre.Command", &esc(c.command()));
    kv(out, "pre.SerialData", &esc(&c.serial_data()));
    kv(out, "pre.IsEmpty", b(c.is_empty()));
    kv(out, "pre.IsGrblCommand", b(c.is_grbl_command()));
    kv(out, "pre.IsWriteEEPROM", b(c.is_write_eeprom()));
    kv(out, "pre.JustBuilt", b(c.just_built()));
    let fail = c.build_helper_report();
    let name = match fail {
        None => "none",
        Some(BuildFailure::DuplicateLetter) => "System.ArgumentException",
        Some(BuildFailure::Format) => "System.FormatException",
        Some(BuildFailure::Overflow) => "System.OverflowException",
    };
    kv(out, "build.swallowed_exception", name);
    kv(out, "post.Command", &esc(c.command()));
    kv(out, "post.SerialData", &esc(&c.serial_data()));
    kv(out, "post.IsEmpty", b(c.is_empty()));
    kv(out, "post.IsGrblCommand", b(c.is_grbl_command()));
    kv(out, "post.IsWriteEEPROM", b(c.is_write_eeprom()));
    kv(out, "post.JustBuilt", b(c.just_built()));
    let els: Vec<String> = c.elements().iter().map(|e| format!("{}:{}", e.command(), lg_dotnet::fmt_decimal(e.number()))).collect();
    kv(out, "elements", &if els.is_empty() { "<none>".to_string() } else { esc(&els.join("|")) });
    kv(out, "ToString", &esc(&c.to_string()));
    kv(out, "IsMovement", b(c.is_movement()));
    kv(out, "IsLinearMovement", b(c.is_linear_movement()));
    kv(out, "IsArcMovement", b(c.is_arc_movement()));
    kv(out, "IsPause", b(c.is_pause()));
    kv(out, "IsLaserON", b(c.is_laser_on()));
    kv(out, "IsLaserOFF", b(c.is_laser_off()));
    kv(out, "IsSetWCO", b(c.is_set_wco()));
    kv(out, "IsAbsoluteCoord", b(c.is_absolute_coord()));
    kv(out, "IsRelativeCoord", b(c.is_relative_coord()));
    kv(out, "IsM3", b(c.is_m3()));
    kv(out, "IsM4", b(c.is_m4()));
    kv(out, "IsM5", b(c.is_m5()));
    kv(out, "IsCW(false)", b(c.is_cw(false)));
    kv(out, "IsCW(true)", b(c.is_cw(true)));
}

#[test]
fn gcode_parse_matches_oracle() {
    let corpus = std::fs::read_to_string(root().join("gcode_parse/corpus.txt")).unwrap();
    let expected = std::fs::read_to_string(root().join("gcode_parse/expected.txt")).unwrap();
    let mut lines: Vec<&str> = corpus.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        writeln!(out, "[case {i}]").unwrap();
        writeln!(out, "in={}", esc(line)).unwrap();
        dump(&mut out, "", GrblCommand::new(line));
        writeln!(out, "preserve.Command={}", esc(GrblCommand::with_options(line, 0, true).command())).unwrap();
        if line.chars().count() >= 3 && line.chars().take(3).collect::<String>().to_uppercase() == "$J=" {
            writeln!(out, "jog=yes").unwrap();
            dump(&mut out, "jog.", GrblCommand::new_jog(line).unwrap());
        }
        out.push('\n');
    }
    // per-case compare for a readable diff
    let eb: Vec<&str> = expected.split("\n\n").collect();
    let ob: Vec<&str> = out.split("\n\n").collect();
    let mut bad = 0;
    for (i, (e, o)) in eb.iter().zip(ob.iter()).enumerate() {
        if e.trim_end() != o.trim_end() {
            bad += 1;
            if bad <= 5 {
                eprintln!("--- case block {i}\nexpected:\n{e}\ngot:\n{o}");
            }
        }
    }
    assert_eq!(eb.len(), ob.len(), "case count");
    assert_eq!(bad, 0, "{bad} of {} cases differ", eb.len());
}
