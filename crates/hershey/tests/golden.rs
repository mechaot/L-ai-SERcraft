//! Golden test: `create_string` vs original C# (Mono) output in tests/golden/hershey/cases.txt.

use std::path::PathBuf;

/// Undo the generator's escaping (`\\`, `\t`, `\n`, `\uXXXX` UTF-16 units).
///
/// * `s` - escaped text.
///
/// Returns the original string.
fn unescape(s: &str) -> String {
    let mut units: Vec<u16> = Vec::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            let mut b = [0u16; 2];
            units.extend_from_slice(c.encode_utf16(&mut b));
            continue;
        }
        match it.next() {
            Some('\\') => units.push(b'\\' as u16),
            Some('t') => units.push(9),
            Some('n') => units.push(10),
            Some('u') => {
                let h: String = (0..4).filter_map(|_| it.next()).collect();
                units.push(u16::from_str_radix(&h, 16).expect("hex"));
            }
            other => panic!("bad escape {other:?}"),
        }
    }
    String::from_utf16(&units).expect("utf16")
}

#[test]
fn hershey_matches_original() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/hershey/cases.txt");
    let data = std::fs::read_to_string(path).expect("golden file");
    let mut lines = data.lines();
    let (mut total, mut bad) = (0, 0);
    while let Some(h) = lines.next() {
        let f: Vec<&str> = h.split('\t').collect();
        assert_eq!(f[0], "CASE", "bad header {h}");
        let text = unescape(f[1]);
        let (cx, cy): (f64, f64) = (f[2].parse().unwrap(), f[3].parse().unwrap());
        let (fs, sp): (i32, i32) = (f[4].parse().unwrap(), f[5].parse().unwrap());
        let horiz = f[6] == "H";
        let n: usize = f[8].parse().unwrap();
        let expect: Vec<&str> = (0..n).map(|_| lines.next().unwrap()).collect();
        // GrblCommand ctor: Trim + ToUpper
        let got: Vec<String> = lg_hershey::create_string(&text, cx, cy, fs, sp, horiz, f[7])
            .iter()
            .map(|l| l.trim().to_uppercase())
            .collect();
        total += 1;
        if got != expect {
            bad += 1;
            if bad <= 5 {
                eprintln!("MISMATCH {h}\n got {:?}\n exp {:?}", &got[..got.len().min(6)], &expect[..expect.len().min(6)]);
            }
        }
    }
    eprintln!("hershey golden: {total} cases, {bad} mismatches");
    assert!(total > 1000);
    assert_eq!(bad, 0);
}
