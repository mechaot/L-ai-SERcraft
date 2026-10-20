//! Compare Utils.HumanReadableDateDiff / TimeSpanToString with Mono goldens.

use chrono::NaiveDateTime;
use lg_base::tools::utils::*;
use lg_dotnet::TimeSpan;

fn golden(name: &str) -> String {
    std::fs::read_to_string(format!("{}/../../tests/golden/timespan_fmt/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

/// Parse `yyyy-MM-ddTHH:mm:ss.fffffff`.
fn dt(s: &str) -> NaiveDateTime {
    let (a, f) = s.split_once('.').unwrap();
    let base = NaiveDateTime::parse_from_str(a, "%Y-%m-%dT%H:%M:%S").unwrap();
    base + chrono::Duration::nanoseconds(f.parse::<i64>().unwrap() * 100)
}

#[test]
fn hrdd_golden() {
    let mut n = 0;
    for l in golden("hrdd.tsv").lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let c: Vec<&str> = l.split('\t').collect();
        let p = TimePrecision::from_i32(c[2].parse().unwrap()).unwrap();
        let f = TimePrecision::from_i32(c[3].parse().unwrap()).unwrap();
        let got = human_readable_date_diff(dt(c[0]), dt(c[1]), p, f, c[4], c[5] == "true");
        assert_eq!(got, c[6], "{l}");
        n += 1;
    }
    assert!(n > 3000);
}

#[test]
fn timespan_golden() {
    let limit = 28i64 * 864_000_000_000;
    let mut n = 0;
    for l in golden("timespan_to_string.tsv").lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let c: Vec<&str> = l.split('\t').collect();
        let t: i64 = c[0].parse().unwrap();
        if c[5].starts_with("EXCEPTION") || (t != i64::MAX && t != i64::MIN && t.abs() >= limit) {
            continue;
        }
        let p = TimePrecision::from_i32(c[1].parse().unwrap()).unwrap();
        let f = TimePrecision::from_i32(c[2].parse().unwrap()).unwrap();
        let got = time_span_to_string(TimeSpan::from_ticks(t), p, f, c[3], c[4] == "true");
        assert_eq!(got, c[5], "{l}");
        n += 1;
    }
    assert!(n > 3000);
}
