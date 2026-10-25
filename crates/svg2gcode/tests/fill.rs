//! Hatch fill in layered conversion: rect, cross, open path, circle, transform, passes, colour filter.

use lg_svg2gcode::{ColorFilter, GCodeFromSvg, HatchFill, LayerParams, SvgConvOptions};
use std::sync::Mutex;

/// The converter shares process-wide modal state; serialise tests that convert.
static LOCK: Mutex<()> = Mutex::new(());

const RECT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100"><g id="a"><rect x="10" y="20" width="20" height="10"/></g></svg>"#;

/// Layered conversion of one layer. `svg`: text. `fill`: fill setting. `edit`: tweak params. Returns layer section.
fn conv(svg: &str, fill: Option<HatchFill>, edit: impl Fn(&mut LayerParams)) -> String {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let o = SvgConvOptions::default();
    let mut lp = LayerParams::from_options(&o, ColorFilter::All);
    lp.fill = fill;
    edit(&mut lp);
    let (g, r) = GCodeFromSvg::new().convert_from_text_layered(svg, &o, true, &[lp]).expect("conv");
    g[r[0].clone().unwrap()].to_string()
}

/// Value after axis letter in a G-code line. `l`: line. `c`: letter. Returns number.
fn val(l: &str, c: char) -> Option<f64> {
    let i = l.find(c)? + 1;
    let e = l[i..].find(|ch: char| !(ch.is_ascii_digit() || ch == '.' || ch == '-')).map_or(l.len(), |e| e + i);
    l[i..e].parse().ok()
}

/// Parse G0/G1 moves with modal X/Y. `sec`: G-code text. Returns (is_g1, x, y) per move.
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

/// Fill segments: moves after the outline moves of `base`, as (G0 start, G1 end) pairs. `sec`: layer text. `base`: same layer without fill.
fn pairs(sec: &str, base: &str) -> Vec<Pair> {
    let m = moves(sec);
    let nb = moves(base).len();
    m[nb..].chunks(2).filter(|c| c.len() == 2 && !c[0].0 && c[1].0).map(|c| ((c[0].1, c[0].2), (c[1].1, c[1].2))).collect()
}

/// Fill segment: (G0 start, G1 end).
type Pair = ((f64, f64), (f64, f64));

fn f(d: f64, a: f64, cross: bool) -> Option<HatchFill> {
    Some(HatchFill { distance_mm: d, angle_deg: a, cross, per_shape: false })
}

#[test]
fn rect_fill_rows_inside() {
    let base = conv(RECT, None, |_| {});
    let sec = conv(RECT, f(1.0, 0.0, false), |_| {});
    assert!(sec.len() > base.len());
    let p = pairs(&sec, &base);
    assert_eq!(p.len(), 10, "{sec}");
    for (a, b) in &p {
        assert!((a.1 - b.1).abs() < 1e-3);
        for q in [a, b] {
            assert!(q.0 >= 10.0 - 1e-3 && q.0 <= 30.0 + 1e-3 && q.1 >= 70.0 - 1e-3 && q.1 <= 80.0 + 1e-3, "{q:?}");
        }
    }
}

#[test]
fn cross_adds_vertical() {
    let one = conv(RECT, f(1.0, 0.0, false), |_| {});
    let two = conv(RECT, f(1.0, 0.0, true), |_| {});
    let base = conv(RECT, None, |_| {});
    let p = pairs(&two, &base);
    assert_eq!(p.len(), 10 + 20);
    assert!(p[10..].iter().all(|(a, b)| (a.0 - b.0).abs() < 1e-3));
    assert!(two.len() > one.len());
}

#[test]
fn fill_none_identical() {
    assert_eq!(conv(RECT, None, |_| {}), conv(RECT, None, |l| l.fill = None));
}

#[test]
fn open_polyline_no_fill() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100"><g><polyline fill="none" points="10,10 30,10 30,30 10,30"/></g></svg>"#;
    assert_eq!(conv(svg, None, |_| {}), conv(svg, f(1.0, 0.0, true), |_| {}));
}

#[test]
fn circle_rows_within_radius() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100"><g><circle cx="50" cy="50" r="10"/></g></svg>"#;
    let base = conv(svg, None, |_| {});
    let p = pairs(&conv(svg, f(1.0, 0.0, false), |_| {}), &base);
    assert!(p.len() >= 18, "{}", p.len());
    for (a, b) in &p {
        for q in [a, b] {
            assert!(((q.0 - 50.0).powi(2) + (q.1 - 50.0).powi(2)).sqrt() <= 10.0 + 0.02, "{q:?}");
        }
    }
}

#[test]
fn transformed_group_inside_bounds() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100"><g transform="translate(20,10) scale(2)"><rect x="5" y="5" width="10" height="5"/></g></svg>"#;
    let base = conv(svg, None, |_| {});
    let p = pairs(&conv(svg, f(1.0, 30.0, false), |_| {}), &base);
    assert!(!p.is_empty());
    // x 30..50, y flipped: 100-30..100-20 = 70..80
    for (a, b) in &p {
        for q in [a, b] {
            assert!(q.0 >= 30.0 - 1e-3 && q.0 <= 50.0 + 1e-3 && q.1 >= 70.0 - 1e-3 && q.1 <= 80.0 + 1e-3, "{q:?}");
        }
    }
}

#[test]
fn passes_twice_repeats_fill() {
    let one = conv(RECT, f(1.0, 0.0, false), |_| {});
    let base2 = conv(RECT, None, |l| l.passes = 2);
    let two = conv(RECT, f(1.0, 0.0, false), |l| l.passes = 2);
    let n = |s: &str| s.lines().filter(|l| l.starts_with("G1")).count();
    let n_base = n(&conv(RECT, None, |_| {}));
    let fills1 = n(&one) - n_base;
    assert_eq!(n(&two) - n(&base2), 2 * fills1);
}

#[test]
fn color_filter_excludes_shape() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100"><g><rect style="stroke:#00ff00;fill:none" x="10" y="20" width="20" height="10"/></g></svg>"##;
    let red = |l: &mut LayerParams| l.filter = ColorFilter::Red;
    assert_eq!(conv(svg, None, red), conv(svg, f(1.0, 0.0, false), red));
}

#[test]
fn per_shape_same_segments_other_order() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100"><g><rect x="10" y="20" width="10" height="5"/><rect x="40" y="20" width="10" height="5"/></g></svg>"#;
    let base = conv(svg, None, |_| {});
    let mk = |per_shape| pairs(&conv(svg, Some(HatchFill { distance_mm: 1.0, angle_deg: 0.0, cross: false, per_shape }), |_| {}), &base);
    let (on, off) = (mk(true), mk(false));
    assert_eq!(on.len(), 10);
    assert_ne!(on, off);
    // unordered endpoint pairs, rounded to 1e-3
    let norm = |v: &[Pair]| {
        let r = |p: (f64, f64)| ((p.0 * 1e3).round() as i64, (p.1 * 1e3).round() as i64);
        let mut o: Vec<_> = v.iter().map(|(a, b)| (r(*a).min(r(*b)), r(*a).max(r(*b)))).collect();
        o.sort();
        o
    };
    assert_eq!(norm(&on), norm(&off));
    // per shape: left shape (x < 30) first, contiguous
    assert!(on[..5].iter().all(|(a, _)| a.0 < 30.0) && on[5..].iter().all(|(a, _)| a.0 > 30.0));
    // row mode alternates shapes
    assert!(off[0].0.0 < 30.0 && off[1].0.0 > 30.0);
}
