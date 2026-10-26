// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Andreas Pösch

//! Backend of the raster-image dock: brightness, histogram, layout and line-to-line G-code with a brightness to power map.
//!
//! Not part of the C# app (intentional deviation): replaces the modal raster dialog for plain L2L
//! engraving. Pure functions, no UI and no settings reads; the app passes every value in [`RasterParams`].

use crate::image_processor::file_dpi;
use image::{Rgba, RgbaImage};
use lg_dotnet::f64_to_i32;
use lg_gcode::{Direction, Firmware, GResult, GcodeError, GrblFile, L2LConf, PowerMap};
use lg_raster::{resize_image, InterpolationMode};
use std::path::Path;

/// Memory limit in pixels, same rule as `do_true_work_inner` (bigger on 64 bit).
const MAX_PIXELS: f64 = if cfg!(target_pointer_width = "64") { 22000.0 * 22000.0 } else { 6000.0 * 7000.0 };

/// Everything one generation run needs (the user edits these in the dock).
#[derive(Debug, Clone, PartialEq)]
pub struct RasterParams {
    /// Lower-left corner in mm (job offset).
    pub x0: f32,
    pub y0: f32,
    /// Upper-right corner in mm (size = `(x1 - x0, y1 - y0)`).
    pub x1: f32,
    pub y1: f32,
    /// Wanted lines per mm (capped by the memory limit, see [`raster_layout`]).
    pub quality: f64,
    /// Raster filling only: Horizontal, Vertical or Diagonal.
    pub direction: Direction,
    /// Brightness to power map.
    pub map: PowerMap,
    /// Engraving feed in mm/min.
    pub mark_speed: i32,
    /// S-MIN / S-MAX (laser power range).
    pub min_power: i32,
    pub max_power: i32,
    /// Laser on / off words, e.g. "M3" / "M5".
    pub laser_on: String,
    pub laser_off: String,
    /// Hardware PWM: true = S value per pixel, false = on/off only.
    pub pwm: bool,
    pub firmware: Firmware,
}

/// Load and decode an image file, like `ImageProcessor::new`.
///
/// `path`: image file (jpg, png, bmp, gif ...). Returns the RGBA pixels and the file DPI
/// (96 when the file has none), or the read / decode error text.
pub fn load_image(path: &Path) -> Result<(RgbaImage, i32), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?.to_rgba8();
    Ok((img, file_dpi(&bytes)))
}

/// The one brightness definition (histogram and generation): Rec.601 luma over white.
///
/// `p`: RGBA pixel (straight alpha). Returns 0 (black) ..= 255 (white); transparent = 255.
pub fn brightness(p: Rgba<u8>) -> u8 {
    let [r, g, b, a] = p.0.map(u32::from);
    // Rec.601 luma, integer math, rounded
    let luma = (299 * r + 587 * g + 114 * b + 500) / 1000;
    // composite on white
    ((luma * a + 255 * (255 - a) + 127) / 255) as u8
}

/// Gray version of an image (`[g, g, g, 255]` per pixel, g from [`brightness`]).
///
/// `img`: source image. Returns an opaque gray image of the same size.
pub fn gray_image(img: &RgbaImage) -> RgbaImage {
    RgbaImage::from_fn(img.width(), img.height(), |x, y| {
        let g = brightness(*img.get_pixel(x, y));
        Rgba([g, g, g, 255])
    })
}

/// Brightness histogram of a gray image.
///
/// `gray`: image from [`gray_image`] (channel 0 is read). Returns pixel counts for 0..=255.
pub fn histogram(gray: &RgbaImage) -> [u32; 256] {
    let mut h = [0u32; 256];
    for p in gray.pixels() {
        h[p[0] as usize] += 1;
    }
    h
}

/// Resolution and bitmap size for the target rectangle.
///
/// `p`: parameters (corners and quality are read). Returns `(lines per mm, (pixels w, pixels h))`,
/// each pixel count at least 1, resolution capped by the memory limit. Err with a message when
/// `x1 <= x0`, `y1 <= y0`, a value is not finite or `quality <= 0`.
pub fn raster_layout(p: &RasterParams) -> Result<(f64, (i32, i32)), String> {
    if ![p.x0, p.y0, p.x1, p.y1].iter().all(|v| v.is_finite()) || !p.quality.is_finite() {
        return Err("Position and quality must be finite numbers".into());
    }
    if p.x1 <= p.x0 || p.y1 <= p.y0 {
        return Err("Upper right corner must be above and right of the lower left corner".into());
    }
    if p.quality <= 0.0 {
        return Err("Quality must be greater than 0".into());
    }
    // size in f32 like the dialog (TargetSize), area widened to f64
    let (w, h) = (p.x1 - p.x0, p.y1 - p.y0);
    let max_res = (MAX_PIXELS / (w * h) as f64).sqrt();
    let res = max_res.min(p.quality);
    if res.is_nan() || res <= 0.0 {
        return Err("Target size too big for processing!".into());
    }
    let pw = f64_to_i32(w as f64 * res).max(1);
    let ph = f64_to_i32(h as f64 * res).max(1);
    Ok((res, (pw, ph)))
}

/// Generate the line-to-line program for one image into `file`.
///
/// Runs without the busy check: call it inside `GrblFile::spawn_load`, which owns that check.
/// `file`: program to fill. `gray`: image from [`gray_image`]. `name`: file name for the load events.
/// `p`: parameters. `append`: keep the old commands. Returns Ok(()) when loaded; Err on a bad
/// layout (see [`raster_layout`]) or a generator error.
pub fn generate(file: &mut GrblFile, gray: &RgbaImage, name: &str, p: &RasterParams, append: bool) -> GResult<()> {
    let (res, (w, h)) = raster_layout(p).map_err(GcodeError::Other)?;
    // scale to target pixels, alpha untouched (gray is opaque)
    let mut bmp = resize_image(gray, w as u32, h as u32, false, InterpolationMode::HighQualityBicubic);
    let conf = L2LConf {
        res,
        fres: res,
        o_x: p.x0,
        o_y: p.y0,
        mark_speed: p.mark_speed,
        border_speed: p.mark_speed,
        min_power: p.min_power,
        max_power: p.max_power,
        l_on: p.laser_on.clone(),
        l_off: p.laser_off.clone(),
        dir: p.direction,
        pwm: p.pwm,
        firmware_type: p.firmware,
        power_map: Some(p.map),
        ..L2LConf::default()
    };
    file.load_image_l2l_body(&mut bmp, name, &conf, append)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::settings_lock;
    use std::sync::{Arc, RwLock};

    /// Params for a `w` x `h` mm target at `q` lines/mm, lower left `(1, 2)`, pwm 0..1000.
    fn params(w: f32, h: f32, q: f64, map: PowerMap) -> RasterParams {
        RasterParams {
            x0: 1.0,
            y0: 2.0,
            x1: 1.0 + w,
            y1: 2.0 + h,
            quality: q,
            direction: Direction::Horizontal,
            map,
            mark_speed: 1000,
            min_power: 0,
            max_power: 1000,
            laser_on: "M3".into(),
            laser_off: "M5".into(),
            pwm: true,
            firmware: Firmware::Grbl,
        }
    }

    /// Opaque gray image, column `x` has gray value `cols[x]`.
    fn columns(cols: &[u8], h: u32) -> RgbaImage {
        RgbaImage::from_fn(cols.len() as u32, h, |x, _| Rgba([cols[x as usize], cols[x as usize], cols[x as usize], 255]))
    }

    /// One parsed program line: X, Y, S words and the raw text.
    struct Line {
        x: Option<f64>,
        y: Option<f64>,
        s: Option<f64>,
        raw: String,
    }

    /// Parse every command of the program into [`Line`]s.
    fn lines(f: &GrblFile) -> Vec<Line> {
        f.commands()
            .iter()
            .map(|c| {
                let raw = c.to_string();
                let word = |k: char| raw.split_whitespace().find_map(|t| t.strip_prefix(k).and_then(|v| v.parse::<f64>().ok()));
                Line { x: word('X'), y: word('Y'), s: word('S'), raw }
            })
            .collect()
    }

    /// Generate into a fresh file with the test settings.
    fn run(gray: &RgbaImage, p: &RasterParams) -> GrblFile {
        lg_base::settings::set("Unidirectional Engraving", false);
        lg_base::settings::set("Disable G0 fast skip", false);
        let mut f = GrblFile::new();
        generate(&mut f, gray, "test.png", p, false).unwrap();
        f
    }

    /// Distinct S values above 0, sorted.
    fn burn_levels(f: &GrblFile) -> Vec<i64> {
        let mut v: Vec<i64> = lines(f).iter().filter_map(|l| l.s).filter(|s| *s > 0.0).map(|s| s as i64).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    #[test]
    fn brightness_and_alpha() {
        assert_eq!(brightness(Rgba([0, 0, 0, 255])), 0);
        assert_eq!(brightness(Rgba([255, 255, 255, 255])), 255);
        // Rec.601: (299 * 255 + 500) / 1000
        assert_eq!(brightness(Rgba([255, 0, 0, 255])), 76);
        assert_eq!(brightness(Rgba([0, 255, 0, 255])), 150);
        assert_eq!(brightness(Rgba([0, 0, 255, 255])), 29);
        // transparent = white, any color
        assert_eq!(brightness(Rgba([0, 0, 0, 0])), 255);
        assert_eq!(brightness(Rgba([10, 200, 30, 0])), 255);
        // half transparent black: (255 * 127 + 127) / 255
        assert_eq!(brightness(Rgba([0, 0, 0, 128])), 127);
        // gray 100 at alpha 128: (100 * 128 + 255 * 127 + 127) / 255
        assert_eq!(brightness(Rgba([100, 100, 100, 128])), 177);
    }

    #[test]
    fn gray_image_is_opaque_gray() {
        let src = RgbaImage::from_fn(3, 2, |x, y| Rgba([(x * 100) as u8, (y * 100) as u8, 50, 200]));
        let g = gray_image(&src);
        assert_eq!(g.dimensions(), (3, 2));
        for (x, y, p) in g.enumerate_pixels() {
            assert_eq!(p[0], p[1]);
            assert_eq!(p[1], p[2]);
            assert_eq!(p[3], 255);
            assert_eq!(p[0], brightness(*src.get_pixel(x, y)));
        }
    }

    #[test]
    fn histogram_sums_to_pixels() {
        let g = gray_image(&columns(&[0, 0, 128, 255, 255, 255], 5));
        let h = histogram(&g);
        assert_eq!(h.iter().sum::<u32>(), 30);
        assert_eq!((h[0], h[128], h[255]), (10, 5, 15));
    }

    #[test]
    fn layout_errors() {
        let m = PowerMap { start: 0, stop: 254 };
        let ok = params(10.0, 5.0, 3.0, m);
        assert!(raster_layout(&ok).is_ok());
        let mut p = ok.clone();
        p.x1 = p.x0;
        assert!(raster_layout(&p).is_err());
        let mut p = ok.clone();
        p.y1 = p.y0 - 1.0;
        assert!(raster_layout(&p).is_err());
        let mut p = ok.clone();
        p.quality = 0.0;
        assert!(raster_layout(&p).is_err());
        let mut p = ok.clone();
        p.quality = -1.0;
        assert!(raster_layout(&p).is_err());
        let mut p = ok.clone();
        p.x1 = f32::NAN;
        assert!(raster_layout(&p).is_err());
        let mut p = ok;
        p.quality = f64::NAN;
        assert!(raster_layout(&p).is_err());
    }

    #[test]
    fn layout_pixel_counts() {
        let m = PowerMap { start: 0, stop: 254 };
        let (res, px) = raster_layout(&params(10.0, 5.0, 3.0, m)).unwrap();
        assert_eq!(res, 3.0);
        assert_eq!(px, (30, 15));
        // tiny target: at least one pixel each way
        let (_, px) = raster_layout(&params(0.01, 0.01, 0.1, m)).unwrap();
        assert_eq!(px, (1, 1));
        // huge target: resolution capped, bitmap stays below the memory limit
        let (res, (w, h)) = raster_layout(&params(100_000.0, 100_000.0, 20.0, m)).unwrap();
        assert!(res < 20.0);
        assert!((w as f64) * (h as f64) <= MAX_PIXELS);
    }

    #[test]
    fn generate_places_job_in_rectangle() {
        let _g = settings_lock();
        let m = PowerMap { start: 0, stop: 254 };
        // 8 x 4 px source, target 4 x 2 mm at 2 lines/mm = same size
        let img = gray_image(&columns(&[0, 0, 127, 127, 200, 200, 255, 255], 4));
        let p = params(4.0, 2.0, 2.0, m);
        let f = run(&img, &p);
        let l = lines(&f);
        assert!(l.len() > 5);
        assert_eq!(l[0].raw, "G0 X1 Y2 F1000");
        for c in &l {
            if let Some(x) = c.x {
                assert!((1.0 - 1e-3..=5.0 + 1e-3).contains(&x), "x {x} in {}", c.raw);
            }
            if let Some(y) = c.y {
                assert!((2.0 - 1e-3..=4.0 + 1e-3).contains(&y), "y {y} in {}", c.raw);
            }
        }
        // no white-free row: trailing white is skipped, so X ends at the last burn (4 mm)
        let max_x = l.iter().filter_map(|c| c.x).fold(f64::MIN, f64::max);
        assert!((max_x - 4.0).abs() < 1e-3, "max x {max_x}");
        // all-burn image: the lines reach the right edge, last line is one pixel (0.5 mm) below y1
        let dark = gray_image(&columns(&[0; 8], 4));
        let l = lines(&run(&dark, &p));
        let max_x = l.iter().filter_map(|c| c.x).fold(f64::MIN, f64::max);
        let max_y = l.iter().filter_map(|c| c.y).fold(f64::MIN, f64::max);
        assert!((max_x - 5.0).abs() < 1e-3, "max x {max_x}");
        assert!((max_y - 3.5).abs() < 1e-3, "max y {max_y}");
    }

    #[test]
    fn generate_rescales_to_target_size() {
        let _g = settings_lock();
        let m = PowerMap { start: 0, stop: 254 };
        // 16 x 8 px source squeezed to 6 x 3 mm at 1 line/mm
        let img = gray_image(&columns(&[0; 16], 8));
        let f = run(&img, &params(6.0, 3.0, 1.0, m));
        for c in lines(&f) {
            if let Some(x) = c.x {
                assert!((1.0 - 1e-3..=7.0 + 1e-3).contains(&x), "x {x}");
            }
            if let Some(y) = c.y {
                assert!((2.0 - 1e-3..=5.0 + 1e-3).contains(&y), "y {y}");
            }
        }
    }

    #[test]
    fn normal_map_levels() {
        let _g = settings_lock();
        let m = PowerMap { start: 0, stop: 254 };
        let img = gray_image(&columns(&[0, 0, 127, 127, 200, 200, 255, 255], 4));
        let f = run(&img, &params(4.0, 2.0, 2.0, m));
        // black = S-MAX, mid gray 500, light gray 213 (1000 - 200 * 1000 / 254), white never burns
        assert_eq!(burn_levels(&f), vec![213, 500, 1000]);
        assert_eq!(m.power(200, 0, 1000), 213);
    }

    #[test]
    fn inverted_map_levels() {
        let _g = settings_lock();
        let m = PowerMap { start: 254, stop: 0 };
        let img = gray_image(&columns(&[0, 0, 127, 127, 200, 200, 255, 255], 4));
        let mut p = params(4.0, 2.0, 2.0, m);
        p.min_power = 100;
        let f = run(&img, &p);
        // black = stop = S-MIN 100, mid gray = 550, light gray = 809, white (255) outside = no burn
        assert_eq!(burn_levels(&f), vec![100, 550, 809]);
        assert!(!lines(&f).iter().any(|l| l.s == Some(1000.0)));
    }

    #[test]
    fn out_of_range_makes_no_burn() {
        let _g = settings_lock();
        // everything outside [100, 150]: black and white only
        let img = gray_image(&columns(&[0, 0, 255, 255, 0, 255, 90, 160], 4));
        let f = run(&img, &params(4.0, 2.0, 2.0, PowerMap { start: 100, stop: 150 }));
        assert!(burn_levels(&f).is_empty());
        assert!(!lines(&f).iter().any(|l| l.raw.starts_with("G1")));
        // all-white image with the default map never burns either
        let white = gray_image(&columns(&[255; 8], 4));
        let f = run(&white, &params(4.0, 2.0, 2.0, PowerMap { start: 0, stop: 254 }));
        assert!(burn_levels(&f).is_empty());
    }

    #[test]
    fn equal_map_burns_one_brightness() {
        let _g = settings_lock();
        let img = gray_image(&columns(&[0, 0, 100, 100, 101, 101, 255, 255], 4));
        let f = run(&img, &params(4.0, 2.0, 2.0, PowerMap { start: 100, stop: 100 }));
        assert_eq!(burn_levels(&f), vec![1000]);
        // burning moves stay in the gray-100 columns (x 2..3 mm + 1 offset = 2..3)
        for l in lines(&f).iter().filter(|l| l.s == Some(1000.0)) {
            if let Some(x) = l.x {
                assert!((2.0 - 1e-3..=3.0 + 1e-3).contains(&x), "x {x} in {}", l.raw);
            }
        }
    }

    #[test]
    fn no_pwm_burns_only_in_range() {
        let _g = settings_lock();
        let img = gray_image(&columns(&[0, 0, 0, 0, 255, 255, 255, 255], 4));
        let mut p = params(4.0, 2.0, 2.0, PowerMap { start: 0, stop: 0 });
        p.pwm = false;
        let f = run(&img, &p);
        let l = lines(&f);
        assert!(l.iter().all(|c| c.s.is_none() || c.raw.starts_with("M5")));
        // burning (G1) moves only over the black left half, x 1..3
        let g1: Vec<&Line> = l.iter().filter(|c| c.raw.starts_with("G1")).collect();
        assert!(!g1.is_empty());
        for c in &g1 {
            if let Some(x) = c.x {
                assert!(x <= 3.0 + 1e-3, "x {x} in {}", c.raw);
            }
        }
        // swap the map: white half burns, black does not
        p.map = PowerMap { start: 255, stop: 255 };
        let f = run(&img, &p);
        let l = lines(&f);
        let burn_x: Vec<f64> = l.iter().filter(|c| c.raw.starts_with("G1")).filter_map(|c| c.x).collect();
        assert!(!burn_x.is_empty());
        assert!(burn_x.iter().all(|x| *x >= 3.0 - 1e-3));
    }

    #[test]
    fn generate_via_spawn_load_and_append() {
        let _g = settings_lock();
        lg_base::settings::set("Unidirectional Engraving", false);
        lg_base::settings::set("Disable G0 fast skip", false);
        let img = Arc::new(gray_image(&columns(&[0, 0, 127, 127, 200, 200, 255, 255], 4)));
        let p = params(4.0, 2.0, 2.0, PowerMap { start: 0, stop: 254 });
        let file = Arc::new(RwLock::new(GrblFile::new()));
        let go = |append: bool| {
            let (img, p) = (img.clone(), p.clone());
            GrblFile::spawn_load(&file, move |f| generate(f, &img, "x.png", &p, append)).unwrap().join().unwrap();
        };
        go(false);
        let n = file.read().unwrap().commands().len();
        // the busy check of load_image_l2l would have skipped this silently
        assert!(n > 5);
        go(false);
        assert_eq!(file.read().unwrap().commands().len(), n);
        go(true);
        assert_eq!(file.read().unwrap().commands().len(), 2 * n);
        // bad layout: error is logged by spawn_load, program untouched
        let mut bad = p.clone();
        bad.x1 = bad.x0;
        let r = generate(&mut file.write().unwrap(), &img, "x.png", &bad, false);
        assert!(matches!(r, Err(GcodeError::Other(_))));
        assert_eq!(file.read().unwrap().commands().len(), 2 * n);
    }

    #[test]
    fn load_image_reads_png() {
        let _g = settings_lock();
        let dir = std::env::temp_dir().join(format!("lg_raster_dock_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.png");
        RgbaImage::from_pixel(7, 3, Rgba([1, 2, 3, 255])).save(&path).unwrap();
        let (img, dpi) = load_image(&path).unwrap();
        assert_eq!(img.dimensions(), (7, 3));
        assert_eq!(dpi, 96);
        assert!(load_image(&dir.join("missing.png")).is_err());
        let junk = dir.join("junk.png");
        std::fs::write(&junk, b"not an image").unwrap();
        assert!(load_image(&junk).is_err());
    }
}
