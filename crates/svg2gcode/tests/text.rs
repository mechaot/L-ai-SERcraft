// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Andreas Pösch

//! `<text>` to `<path>` pre-pass.

use lg_svg2gcode::{ColorFilter, GCodeFromSvg, HatchFill, LayerParams, SvgConvOptions, system_fontdb, text_to_paths};
use std::collections::BTreeMap;
use std::sync::Mutex;

/// The converter shares process-wide modal state; serialise tests that convert.
static LOCK: Mutex<()> = Mutex::new(());

/// Skip check. Returns true when no system fonts exist (test should return).
fn no_fonts() -> bool {
    let none = system_fontdb().is_empty();
    if none {
        eprintln!("skip: no system fonts");
    }
    none
}

/// Value of `d="..."` in `s`. `s`: svg text. Returns path data.
fn d_of(s: &str) -> &str {
    let i = s.find(" d=\"").expect("d attr") + 4;
    &s[i..i + s[i..].find('"').unwrap()]
}

#[test]
fn no_text_unchanged() {
    let s = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="5" height="5"/></svg>"#;
    assert_eq!(text_to_paths(s), s);
}

#[test]
fn text_becomes_path() {
    if no_fonts() {
        return;
    }
    let s = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><text x="10" y="50" font-family="sans-serif" font-size="20" fill="red" transform="translate(5,0)">Hi</text></svg>"#;
    let o = text_to_paths(s);
    assert!(!o.contains("<text"), "{o}");
    assert!(o.contains("<path"), "{o}");
    assert!(o.contains(r#"transform="translate(5,0)""#));
    assert!(o.contains(r#"fill="red""#));
    let d = d_of(&o);
    assert!(!d.is_empty());
    // letters are commands, rest are x y pairs
    let nums: Vec<f32> = d
        .replace(|c: char| c.is_ascii_alphabetic(), " ")
        .split_whitespace()
        .map(|t| t.parse().unwrap())
        .collect();
    assert!(nums.len() >= 4 && nums.len().is_multiple_of(2));
    for p in nums.chunks(2) {
        assert!((9.0..60.0).contains(&p[0]), "x {}", p[0]);
        assert!((30.0..56.0).contains(&p[1]), "y {}", p[1]);
    }
}

#[test]
fn prefixed_with_id() {
    if no_fonts() {
        return;
    }
    let s = r#"<svg:svg xmlns:svg="http://www.w3.org/2000/svg" width="100" height="100"><svg:text id="t1" x="10" y="50" font-size="20">Hi</svg:text></svg:svg>"#;
    let o = text_to_paths(s);
    assert!(o.contains(r#"<svg:path id="t1""#), "{o}");
    assert!(!o.contains("text"), "{o}");
}

#[test]
fn blank_removed_defs_kept() {
    if no_fonts() {
        return;
    }
    let s = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><text x="1" y="9">   </text><defs><text id="x" y="9">Hi</text></defs></svg>"#;
    let o = text_to_paths(s);
    assert!(!o.contains("   </text>"), "{o}");
    assert!(o.contains(r#"<defs><text id="x" y="9">Hi</text></defs>"#), "{o}");
}

/// Convert the golden text/image SVG. `on`: text_to_path flag. Returns G-code.
fn conv(on: bool) -> String {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/golden/svg/inputs/shape_text_image.svg");
    let svg = std::fs::read_to_string(p).expect("read");
    let mut c = GCodeFromSvg::new();
    c.text_to_path = on;
    c.convert_from_text(&svg, &SvgConvOptions::default(), true, ColorFilter::All).expect("conv")
}

#[test]
fn end_to_end() {
    if no_fonts() {
        return;
    }
    let on = conv(true);
    assert!(!on.contains("Convert Object to Path first"));
    assert!(on.contains("G1"));
    assert!(conv(false).contains("Convert Object to Path first"));
}

/// One glyph as 40-unit text in a 100 mm layer. `t`: text. Returns SVG.
fn glyph_svg(t: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100"><g id="l"><text x="10" y="60" font-family="sans-serif" font-size="40">{t}</text></g></svg>"#)
}

#[test]
fn glyphs_closed_contours() {
    if no_fonts() {
        return;
    }
    // contours per glyph: dot + stem, holes
    for (t, n) in [("i", 2), ("j", 2), ("l", 1), ("o", 2), ("O", 2), ("8", 3), ("B", 3), ("%", 5), ("ij", 4)] {
        let o = text_to_paths(&glyph_svg(t));
        let d = d_of(&o);
        let subs: Vec<&str> = d.split('M').map(str::trim).filter(|x| !x.is_empty()).collect();
        assert_eq!(subs.len(), n, "{t}: {d}");
        assert!(subs.iter().all(|x| x.ends_with('Z')), "{t}: open subpath in {d}");
    }
}

/// Value after axis letter in a G-code line. `l`: line. `c`: letter. Returns number.
fn val(l: &str, c: char) -> Option<f64> {
    let i = l.find(c)? + 1;
    let e = l[i..].find(|ch: char| !(ch.is_ascii_digit() || ch == '.' || ch == '-')).map_or(l.len(), |e| e + i);
    l[i..e].parse().ok()
}

/// G0/G1 moves with modal X/Y. `sec`: G-code. Returns (is_g1, x, y) per move.
fn moves(sec: &str) -> Vec<(bool, f64, f64)> {
    let (mut x, mut y) = (0.0, 0.0);
    let mut g1 = false;
    let mut out = Vec::new();
    for l in sec.lines() {
        if l.starts_with("G0") || l.starts_with("G1") {
            g1 = l.starts_with("G1");
        } else if !(l.starts_with('X') || l.starts_with('Y')) {
            continue;
        }
        x = val(l, 'X').unwrap_or(x);
        y = val(l, 'Y').unwrap_or(y);
        out.push((g1, x, y));
    }
    out
}

/// Burn runs: G0 start then G1 points. `m`: moves. Returns point lists (start included).
fn runs(m: &[(bool, f64, f64)]) -> Vec<Vec<(f64, f64)>> {
    let mut out: Vec<Vec<(f64, f64)>> = Vec::new();
    for &(g1, x, y) in m {
        match (g1, out.last_mut()) {
            (true, Some(r)) => r.push((x, y)),
            _ => out.push(vec![(x, y)]),
        }
    }
    out.retain(|r| r.len() > 1);
    out
}

/// Layered conversion of one layer. `svg`: text. `fill`: hatch. Returns layer G-code.
fn conv_layer(svg: &str, fill: Option<HatchFill>) -> String {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let o = SvgConvOptions::default();
    let mut lp = LayerParams::from_options(&o, ColorFilter::All);
    lp.fill = fill;
    let mut c = GCodeFromSvg::new();
    c.text_to_path = true;
    let (g, r) = c.convert_from_text_layered(svg, &o, true, &[lp]).expect("conv");
    g[r[0].clone().unwrap()].to_string()
}

#[test]
fn gcode_runs_closed() {
    if no_fonts() {
        return;
    }
    let g = conv_layer(&glyph_svg("Bij8%"), None);
    let rs = runs(&moves(&g));
    // B3 i2 j2 8:3 %5
    assert_eq!(rs.len(), 15, "{g}");
    for r in &rs {
        let (a, b) = (r[0], r[r.len() - 1]);
        assert!((a.0 - b.0).abs() < 2e-3 && (a.1 - b.1).abs() < 2e-3, "open run {a:?} -> {b:?}");
    }
}

#[test]
fn fill_hatches_glyph_not_hole() {
    if no_fonts() {
        return;
    }
    let svg = glyph_svg("O");
    let base = conv_layer(&svg, None);
    let hatch = HatchFill { distance_mm: 0.5, angle_deg: 0.0, cross: false, per_shape: false };
    let filled = conv_layer(&svg, Some(hatch));
    let (mb, mf) = (moves(&base), moves(&filled));
    assert!(mf.len() > mb.len() + 20, "no fill moves");
    // glyph bbox from outline
    let ys: Vec<f64> = mb.iter().map(|m| m.2).collect();
    let (y0, y1) = (ys.iter().cloned().fold(f64::MAX, f64::min), ys.iter().cloned().fold(f64::MIN, f64::max));
    // fill segments (G0 start, G1 end) after outline, grouped per row
    let mut rows: BTreeMap<i64, usize> = BTreeMap::new();
    for c in mf[mb.len()..].chunks(2) {
        if c.len() == 2 && !c[0].0 && c[1].0 {
            assert!((c[0].2 - c[1].2).abs() < 1e-6, "angle 0 hatch must be horizontal");
            *rows.entry((c[0].2 * 1000.0).round() as i64).or_default() += 1;
        }
    }
    // middle rows cross the hole: left and right stroke only
    let mid = (y0 + y1) / 2.0;
    let band = (y1 - y0) * 0.15;
    let middle: Vec<usize> = rows.iter().filter(|(y, _)| ((**y as f64 / 1000.0) - mid).abs() < band).map(|(_, n)| *n).collect();
    assert!(middle.len() >= 5, "rows {rows:?}");
    assert!(middle.iter().all(|&n| n == 2), "hole hatched: {middle:?}");
}
