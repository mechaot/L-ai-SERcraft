//! Golden tests: compare against tables produced by Mono (tools/golden/dotnet/run.sh).

use lg_dotnet::*;
use std::path::PathBuf;

/// Read a golden TSV into rows of fields.
///
/// `name`: file name inside tests/golden/dotnet.
fn rows(name: &str) -> Vec<Vec<String>> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/dotnet").join(name);
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    text.lines().map(|l| l.split('\t').map(str::to_string).collect()).collect()
}

/// Undo the generator's escaping (\\ and \uXXXX).
///
/// `s`: escaped text.
fn unesc(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut o = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '\\' {
            if c[i + 1] == 'u' {
                let h: String = c[i + 2..i + 6].iter().collect();
                o.push(char::from_u32(u32::from_str_radix(&h, 16).unwrap()).unwrap());
                i += 6;
            } else {
                o.push(c[i + 1]);
                i += 2;
            }
        } else {
            o.push(c[i]);
            i += 1;
        }
    }
    o
}

/// Collects mismatches and fails with a summary.
#[derive(Default)]
struct Diffs {
    total: usize,
    bad: Vec<String>,
}

impl Diffs {
    /// Record one comparison.
    ///
    /// `ctx`: description. `got`/`want`: values.
    fn check(&mut self, ctx: impl FnOnce() -> String, got: &str, want: &str) {
        self.total += 1;
        if got != want {
            self.bad.push(format!("{} => got {:?} want {:?}", ctx(), got, want));
        }
    }

    /// Panic if anything differed.
    ///
    /// `what`: table name.
    fn finish(self, what: &str) {
        if !self.bad.is_empty() {
            let n = self.bad.len();
            let shown: Vec<_> = self.bad.iter().take(25).cloned().collect();
            panic!("{what}: {n}/{} mismatches\n{}", self.total, shown.join("\n"));
        }
        println!("{what}: {} rows ok", self.total);
    }
}

/// Format selector: "<none>" means plain ToString().
fn fm(f: &str) -> String {
    if f == "<none>" { String::new() } else { unesc(f) }
}

#[test]
fn golden_double_format() {
    let mut d = Diffs::default();
    for r in rows("double.tsv") {
        let v = f64::from_bits(u64::from_str_radix(&r[0], 16).unwrap());
        let f = fm(&r[1]);
        if r[2] == "EXC" {
            continue;
        }
        let got = fmt_double_custom(v, &f);
        d.check(|| format!("{:016x}({v:e}) {:?}", v.to_bits(), f), &got, &unesc(&r[2]));
    }
    d.finish("double.tsv");
}

#[test]
fn golden_float_format() {
    let mut d = Diffs::default();
    for r in rows("float.tsv") {
        let v = f32::from_bits(u32::from_str_radix(&r[0], 16).unwrap());
        let f = fm(&r[1]);
        if r[2] == "EXC" {
            continue;
        }
        let got = fmt_float_custom(v, &f);
        d.check(|| format!("{:08x}({v:e}) {:?}", v.to_bits(), f), &got, &unesc(&r[2]));
    }
    d.finish("float.tsv");
}

#[test]
fn golden_decimal_format() {
    let mut d = Diffs::default();
    for r in rows("decimal.tsv") {
        let v = parse_decimal(&r[0]).unwrap();
        let f = fm(&r[1]);
        if r[2] == "EXC" {
            continue;
        }
        let got = fmt_decimal_custom(v, &f);
        d.check(|| format!("{} {:?}", r[0], f), &got, &unesc(&r[2]));
    }
    d.finish("decimal.tsv");
}

#[test]
fn golden_decimal_ops() {
    let mut d = Diffs::default();
    for r in rows("decimal_ops.tsv") {
        let op = r[0].as_str();
        let a = parse_decimal(&r[1]).unwrap();
        if op == "round" || op == "rounda" {
            let n: u32 = r[2].parse().unwrap();
            let got = fmt_decimal(if op == "round" { round_decimal(a, n) } else { round_decimal_away(a, n) });
            d.check(|| format!("{op} {} {n}", r[1]), &got, &r[3]);
            continue;
        }
        let b = parse_decimal(&r[2]).unwrap();
        let got = match op {
            "+" => dec_add(a, b),
            "-" => dec_sub(a, b),
            "*" => dec_mul(a, b),
            "/" => dec_div(a, b),
            _ => dec_rem(a, b),
        };
        let got = got.map(fmt_decimal).unwrap_or_else(|| "EXC".into());
        d.check(|| format!("{} {op} {}", r[1], r[2]), &got, &r[3]);
    }
    d.finish("decimal_ops.tsv");
}

#[test]
fn golden_int_format() {
    let mut d = Diffs::default();
    for r in rows("int.tsv") {
        let f = fm(&r[2]);
        if r[3] == "EXC" {
            continue;
        }
        let got = match r[0].as_str() {
            "i32" => fmt_i32(r[1].parse().unwrap(), &f),
            "i64" => fmt_i64(r[1].parse().unwrap(), &f),
            "u32" => fmt_u32(r[1].parse().unwrap(), &f),
            "u8" => fmt_u32(r[1].parse().unwrap(), &f),
            _ => unreachable!(),
        };
        d.check(|| format!("{} {} {:?}", r[0], r[1], f), &got, &unesc(&r[3]));
    }
    d.finish("int.tsv");
}

#[test]
fn golden_parse() {
    let mut d = Diffs::default();
    for r in rows("parse.tsv") {
        let s = unesc(&r[1]);
        let want = r[2].as_str();
        // Mono rounds this exact-below-half denormal input up; correct rounding gives 0. Not a .NET Fx claim.
        // Mono also truncates long digit strings (>17 digits) for double/float: skip those.
        if (s == "2.4703282292062327e-324" || s.starts_with("1234567890123456789012345678.")) && r[0] != "decimal" {
            continue;
        }
        let got = match r[0].as_str() {
            "double" | "double_default" => parse_double(&s).map(|v| format!("{:016x}", v.to_bits())),
            "float" => parse_float(&s).map(|v| format!("{:08x}", v.to_bits())),
            "int" => parse_int(&s).map(|v| v.to_string()),
            "decimal" => parse_decimal(&s).map(fmt_decimal),
            _ => unreachable!(),
        }
        .unwrap_or_else(|| "FAIL".into());
        if want == "EXC" {
            continue;
        }
        d.check(|| format!("{} {:?}", r[0], s), &got, want);
    }
    d.finish("parse.tsv");
}

#[test]
fn golden_casts() {
    let mut d = Diffs::default();
    for r in rows("cast.tsv") {
        let k = r[0].as_str();
        let want = r[2].as_str();
        if want == "EXC" {
            continue;
        }
        let bits = |s: &str| f64::from_bits(u64::from_str_radix(s, 16).unwrap());
        let bits32 = |s: &str| f32::from_bits(u32::from_str_radix(s, 16).unwrap());
        let got = match k {
            "d2i" => f64_to_i32(bits(&r[1])).to_string(),
            "d2l" => f64_to_i64(bits(&r[1])).to_string(),
            "f2i" => f32_to_i32(bits32(&r[1])).to_string(),
            "f2l" => f32_to_i64(bits32(&r[1])).to_string(),
            "d2dec" => fmt_decimal(try_f64_to_decimal(bits(&r[1])).unwrap_or_default()),
            "f2dec" => fmt_decimal(try_f32_to_decimal(bits32(&r[1])).unwrap_or_default()),
            "round" => format!("{:016x}", round_half_even(bits(&r[1])).to_bits()),
            "dec2d" => format!("{:016x}", decimal_to_f64(parse_decimal(&r[1]).unwrap()).to_bits()),
            "dec2f" => format!("{:08x}", decimal_to_f32(parse_decimal(&r[1]).unwrap()).to_bits()),
            _ => unreachable!(),
        };
        d.check(|| format!("{k} {}", r[1]), &got, want);
    }
    d.finish("cast.tsv");
}

#[test]
fn golden_timespan() {
    let mut d = Diffs::default();
    for r in rows("timespan.tsv") {
        let k = r[0].as_str();
        if r[2] == "EXC" {
            continue;
        }
        match k {
            "props" => {
                let t = TimeSpan::from_ticks(r[1].parse().unwrap());
                let got = format!(
                    "{} {} {} {} {} {:016x} {:016x} {:016x} {:016x} {:016x}",
                    t.days(),
                    t.hours(),
                    t.minutes(),
                    t.seconds(),
                    t.milliseconds(),
                    t.total_days().to_bits(),
                    t.total_hours().to_bits(),
                    t.total_minutes().to_bits(),
                    t.total_seconds().to_bits(),
                    t.total_milliseconds().to_bits()
                );
                d.check(|| format!("props {}", r[1]), &got, &r[2]);
            }
            "str" => {
                let t = TimeSpan::from_ticks(r[1].parse().unwrap());
                d.check(|| format!("str {}", r[1]), &t.to_string(), &r[2]);
            }
            _ => {
                let v = f64::from_bits(u64::from_str_radix(&r[1], 16).unwrap());
                let t = match k {
                    "from_days" => TimeSpan::from_days(v),
                    "from_hours" => TimeSpan::from_hours(v),
                    "from_minutes" => TimeSpan::from_minutes(v),
                    "from_seconds" => TimeSpan::from_seconds(v),
                    _ => TimeSpan::from_milliseconds(v),
                };
                d.check(|| format!("{k} {v:e}"), &t.ticks().to_string(), &r[2]);
            }
        }
    }
    d.finish("timespan.tsv");
}

#[test]
fn golden_chars() {
    // rebuild per-class sets from ranges, compare against all BMP code units
    let mut sets: std::collections::HashMap<String, Vec<(u32, u32)>> = Default::default();
    for r in rows("chars.tsv") {
        sets.entry(r[0].clone())
            .or_default()
            .push((u32::from_str_radix(&r[1], 16).unwrap(), u32::from_str_radix(&r[2], 16).unwrap()));
    }
    let mut d = Diffs::default();
    for (name, f) in [
        ("letter", is_letter as fn(char) -> bool),
        ("digit", is_digit),
        ("number", is_number),
        ("white", is_white_space),
    ] {
        let rs = &sets[name];
        for c in 0u32..0x10000 {
            let want = rs.iter().any(|&(a, b)| c >= a && c <= b);
            if let Some(ch) = char::from_u32(c) {
                d.check(|| format!("{name} U+{c:04X}"), &f(ch).to_string(), &want.to_string());
            }
        }
    }
    // non-BMP is false
    assert!(!is_letter('\u{10000}') && !is_digit('\u{1D7CE}'));
    d.finish("chars.tsv");
}

#[test]
fn golden_split() {
    let mut d = Diffs::default();
    for r in rows("split.tsv") {
        let s = unesc(&r[0]);
        let sep = char::from_u32(r[1].parse().unwrap()).unwrap();
        let parts = split_keep_empty(&s, sep);
        let got = format!("{}\t{}", parts.len(), parts.join("|"));
        let want = format!("{}\t{}", r[2], unesc(r.get(3).map(String::as_str).unwrap_or("")));
        d.check(|| format!("{s:?} {sep:?}"), &got, &want);
    }
    d.finish("split.tsv");
}
