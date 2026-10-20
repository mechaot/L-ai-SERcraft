//! Golden tests against the original LaserGRBL run under Mono + libgdiplus.
//! Data: `tests/golden/raster/{input,output,manifest.txt}` (see `tools/golden/raster/run.sh`).
//!
//! * Pure integer ops (dither, whitenize, rotate/flip, fill, outline, invert, opaque threshold,
//!   opaque grayscale in `Libgdiplus` mode): exact equality.
//! * Windows-semantics grayscale (`Gdi` mode, round): max 1 level off the truncating oracle
//!   when no coefficient is negative.
//! * Alpha-blended results and resize: tolerance metrics (libgdiplus goes through cairo).

use image::RgbaImage;
use lg_raster::*;
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/raster")
}

fn load(dir: &str, name: &str) -> RgbaImage {
    image::open(golden_dir().join(dir).join(format!("{name}.png"))).unwrap().to_rgba8()
}

/// Returns (max abs channel diff, mean abs channel diff); dims must match.
fn diff(a: &RgbaImage, b: &RgbaImage) -> (u32, f64) {
    assert_eq!(a.dimensions(), b.dimensions());
    let mut max = 0u32;
    let mut sum = 0u64;
    for (p, q) in a.pixels().zip(b.pixels()) {
        for c in 0..4 {
            let d = (p[c] as i32 - q[c] as i32).unsigned_abs();
            max = max.max(d);
            sum += d as u64;
        }
    }
    (max, sum as f64 / (a.width() * a.height() * 4) as f64)
}

/// Alpha must match exactly; RGB only compared where alpha >= 128 (cairo premultiplied
/// storage in libgdiplus loses precision at low alpha). Returns (max rgb diff, mean).
fn diff_visible(a: &RgbaImage, b: &RgbaImage) -> (u32, f64) {
    assert_eq!(a.dimensions(), b.dimensions());
    let mut max = 0u32;
    let mut sum = 0u64;
    for (p, q) in a.pixels().zip(b.pixels()) {
        for c in 0..4 {
            if c == 3 {
                assert_eq!(p[3], q[3], "alpha mismatch");
                continue;
            }
            if p[3] < 128 {
                continue;
            }
            let d = (p[c] as i32 - q[c] as i32).unsigned_abs();
            max = max.max(d);
            sum += d as u64;
        }
    }
    (max, sum as f64 / (a.width() * a.height() * 4) as f64)
}

fn exact(name: &str, got: &RgbaImage, want: &RgbaImage) {
    assert_eq!(got.dimensions(), want.dimensions(), "{name}: size");
    for (i, (p, q)) in got.pixels().zip(want.pixels()).enumerate() {
        assert_eq!(p, q, "{name}: pixel {} ({},{})", i, i as u32 % got.width(), i as u32 / got.width());
    }
}

fn parse_mode(s: &str) -> InterpolationMode {
    match s {
        "NearestNeighbor" => InterpolationMode::NearestNeighbor,
        "Bilinear" => InterpolationMode::Bilinear,
        "Bicubic" => InterpolationMode::Bicubic,
        "HighQualityBilinear" => InterpolationMode::HighQualityBilinear,
        "HighQualityBicubic" => InterpolationMode::HighQualityBicubic,
        "Default" => InterpolationMode::Default,
        "Low" => InterpolationMode::Low,
        "High" => InterpolationMode::High,
        _ => panic!("mode {s}"),
    }
}

fn parse_dither(i: i32) -> DitheringMode {
    use DitheringMode::*;
    [Atkinson, FloydSteinberg, Burks, Jarvis, Random, Sierra2, Sierra3, SierraLight, Stucki][i as usize]
}

fn parse_formula(i: i32) -> Formula {
    [Formula::SimpleAverage, Formula::WeightAverage, Formula::OpticalCorrect, Formula::Custom][i as usize]
}

fn parse_rf(i: i32) -> RotateFlipType {
    use RotateFlipType::*;
    // .NET numbering
    [RotateNoneFlipNone, Rotate90FlipNone, Rotate180FlipNone, Rotate270FlipNone, RotateNoneFlipX, Rotate90FlipX, Rotate180FlipX, Rotate270FlipX]
        [i as usize]
}

#[test]
fn golden_all() {
    let manifest = std::fs::read_to_string(golden_dir().join("manifest.txt")).unwrap();
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    let mut worst_resize = std::collections::BTreeMap::<String, f64>::new();
    for line in manifest.lines() {
        let t: Vec<&str> = line.split(' ').collect();
        let (name, op, inp) = (t[0], t[1], t[2]);
        let src = load("input", inp);
        let want = load("output", name);
        let opaque = src.pixels().all(|p| p[3] == 255);
        *counts.entry(op.to_string()).or_default() += 1;
        match op {
            "gs" => {
                let f = parse_formula(t[3].parse().unwrap());
                let a: Vec<f32> = t[4..9].iter().map(|s| s.parse().unwrap()).collect();
                let lib = gray_scale_mode(&src, a[0], a[1], a[2], a[3], a[4], f, ColorMatrixMode::Libgdiplus);
                if opaque {
                    exact(name, &lib, &want);
                    // Windows rounding: truncation oracle differs by at most 1 when nothing is negative
                    if a[3] >= 0.0 {
                        let gdi = gray_scale(&src, a[0], a[1], a[2], a[3], a[4], f);
                        let (mx, _) = diff(&gdi, &want);
                        assert!(mx <= 1, "{name}: gdi vs oracle max {mx}");
                    }
                } else {
                    let (mx, _) = diff_visible(&lib, &want);
                    assert!(mx <= 3, "{name}: alpha libgdiplus mode max {mx}");
                }
            }
            "whiten" => {
                let got = whitenize(&src, t[3].parse().unwrap(), t[4] == "1");
                exact(name, &got, &want);
            }
            "thr" => {
                let got = threshold(&src, t[3].parse().unwrap(), t[4] == "1");
                if opaque {
                    exact(name, &got, &want);
                } else {
                    // cairo compositing on white rounds differently than we do
                    let (mx, _) = diff(&got, &want);
                    let bound = 2;
                    assert!(mx <= bound, "{name}: max {mx}");
                }
            }
            "inv" => {
                let got = inverting_image(&src);
                if opaque {
                    exact(name, &got, &want);
                } else {
                    let (mx, _) = diff_visible(&got, &want);
                    assert!(mx <= 3, "{name}: max {mx}");
                }
            }
            "dither" => {
                let m = parse_dither(t[3].parse().unwrap());
                let got = dither_image_with_seed(&src, m, t[4].parse().unwrap());
                exact(name, &got, &want);
            }
            "fill" => {
                let argb = u32::from_str_radix(t[5], 16).unwrap() as i32;
                let got = fill(&src, Point::new(t[3].parse().unwrap(), t[4].parse().unwrap()), Color::from_argb_i32(argb), t[6].parse().unwrap()).unwrap();
                exact(name, &got, &want);
            }
            "outline" => {
                let got = outliner(&src, Point::new(t[3].parse().unwrap(), t[4].parse().unwrap())).unwrap();
                exact(name, &got, &want);
            }
            "rotflip" => {
                let got = rotate_flip(&src, parse_rf(t[3].parse().unwrap()));
                exact(name, &got, &want);
            }
            "resize" => {
                let mode = parse_mode(t[3]);
                let got = resize_image(&src, t[4].parse().unwrap(), t[5].parse().unwrap(), false, mode);
                let (_, mean) = diff(&got, &want);
                let e = worst_resize.entry(t[3].to_string()).or_default();
                *e = e.max(mean);
                println!("RESIZE {name} mean abs diff {mean:.2}");
            }
            _ => panic!("unknown op {op}"),
        }
    }
    println!("{counts:?}");
    for (m, v) in &worst_resize {
        // libgdiplus uses cairo filters: loose for others, tight for the mode the app uses
        let bound = if m == "HighQualityBicubic" || m == "High" { 3.0 } else { 25.0 };
        assert!(*v < bound, "resize {m}: mean diff {v}");
    }
}
