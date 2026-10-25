// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Andreas Pösch

//! Layered conversion tests: layer listing, regression vs plain conversion, per-layer values.

use lg_svg2gcode::{ColorFilter, GCodeFromSvg, LayerParams, SvgConvOptions, list_layers_from_text};
use std::path::PathBuf;
use std::sync::Mutex;

/// The converter shares process-wide modal state; serialise tests that convert.
static LOCK: Mutex<()> = Mutex::new(());

const TWO_LAYERS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="100mm" height="100mm" viewBox="0 0 100 100">
<rect x="1" y="1" width="5" height="5"/>
<g id="a" inkscape:label="Engrave" inkscape:groupmode="layer"><rect x="10" y="10" width="20" height="20"/></g>
<g id="b" inkscape:label="Cut"><circle cx="50" cy="50" r="10"/><g><path d="M 60 60 L 80 80"/></g></g>
<g><line x1="0" y1="0" x2="5" y2="5"/></g>
</svg>"#;

/// Layer params from options. `o`: options. Returns three default layers + root (4 total).
fn defaults(o: &SvgConvOptions, n: usize) -> Vec<LayerParams> {
    vec![LayerParams::from_options(o, ColorFilter::All); n]
}

/// Layered conversion. `text`: svg. `o`: options. `l`: layers. Returns (G-code, ranges).
fn layered(text: &str, o: &SvgConvOptions, l: &[LayerParams]) -> (String, Vec<Option<std::ops::Range<usize>>>) {
    GCodeFromSvg::new().convert_from_text_layered(text, o, true, l).expect("layered")
}

#[test]
fn list_layers_labels_and_keys() {
    let l = list_layers_from_text(TWO_LAYERS).unwrap();
    let keys: Vec<_> = l.iter().map(|x| (x.key.as_str(), x.label.as_str(), x.shapes)).collect();
    assert_eq!(
        keys,
        vec![("#root", "(ungrouped)", 1), ("id:a", "Engrave", 1), ("id:b", "Cut", 2), ("idx:3", "Layer 3", 1)]
    );
    // id fallback, no root pseudo-layer without direct shapes
    let l = list_layers_from_text(r#"<svg xmlns="http://www.w3.org/2000/svg"><g id="x"><rect/></g></svg>"#).unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!((l[0].key.as_str(), l[0].label.as_str()), ("id:x", "x"));
}

#[test]
fn layered_equals_plain_on_golden_svgs() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/svg/inputs");
    let o = SvgConvOptions::default();
    let (mut n, mut bad) = (0, Vec::new());
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let Ok(text) = std::fs::read_to_string(&p) else { continue };
        let Ok(layers) = list_layers_from_text(&text) else { continue };
        if layers.is_empty() {
            continue;
        }
        // warm up shared modal state, then compare from the same start state
        let mut c = GCodeFromSvg::new();
        let Ok(_) = c.convert_from_text(&text, &o, true, ColorFilter::All) else { continue };
        let a = GCodeFromSvg::new().convert_from_text(&text, &o, true, ColorFilter::All);
        let b = GCodeFromSvg::new().convert_from_text_layered(&text, &o, true, &defaults(&o, layers.len()));
        n += 1;
        match (a, b) {
            (Ok(a), Ok((b, _))) if a == b => {}
            _ => bad.push(p.file_name().unwrap().to_string_lossy().into_owned()),
        }
    }
    eprintln!("layered==plain on {n} svgs");
    assert!(n > 0);
    assert!(bad.is_empty(), "mismatch: {bad:?}");
}

#[test]
fn disabled_layer_absent_and_passes_repeat() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let o = SvgConvOptions::default();
    let base = defaults(&o, 4);
    let (full, r_full) = layered(TWO_LAYERS, &o, &base);
    assert!(r_full.iter().all(Option::is_some));
    // disable layer index 2 (the circle group): no arcs left
    let mut l = base.clone();
    l[2].enabled = false;
    let (g, r) = layered(TWO_LAYERS, &o, &l);
    assert!(r[2].is_none());
    assert!(full.contains("G3") && !g.contains("G3"));
    // passes = 2 duplicates the layer's moves
    let mut l = base.clone();
    l[1].passes = 2;
    let (g, r) = layered(TWO_LAYERS, &o, &l);
    let rng = r[1].clone().unwrap();
    let one = r_full[1].clone().unwrap();
    assert!(rng.len() > one.len());
    assert!(g.len() > full.len());
}

#[test]
fn per_layer_feed_and_power_and_mode() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let o = SvgConvOptions::default();
    let mut l = defaults(&o, 4);
    l[1].border_speed = 777;
    l[1].power_max = 111;
    l[2].border_speed = 333;
    l[2].power_max = 222;
    l[2].laser_on = "M4".into();
    let (g, r) = layered(TWO_LAYERS, &o, &l);
    let sec = |i: usize| &g[r[i].clone().unwrap()];
    assert!(sec(1).contains("F777") && sec(1).contains("S111"));
    assert!(sec(2).contains("F333") && sec(2).contains("S222"));
    assert!(!sec(1).contains("M4"));
    // PWM mode switch: M4 S0 once at layer start
    assert_eq!(g.matches("M4 S0").count(), 1);
    assert!(sec(2).starts_with("M4 S0"));
}

#[test]
fn non_pwm_uses_mode_at_pen_down() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let o = SvgConvOptions { support_hardware_pwm: false, ..Default::default() };
    let mut l = defaults(&o, 4);
    l[2].laser_on = "M4".into();
    let (g, r) = layered(TWO_LAYERS, &o, &l);
    let sec = &g[r[2].clone().unwrap()];
    assert!(sec.contains("M4") && !sec.contains("M4 S0"));
    assert!(!g[r[1].clone().unwrap()].contains("M4"));
}
