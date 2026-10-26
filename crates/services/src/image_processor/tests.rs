// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Unit tests of the image processor (preview + generator wiring).

use super::*;
use crate::test_support::settings_lock;
use crossbeam_channel::{unbounded, Receiver};
use image::Rgba;
use lg_gcode::L2LConf;
use lg_raster::{gray_scale, resize_image, threshold, whitenize};
use std::time::Duration;

const T: Duration = Duration::from_secs(20);

fn color_image(w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| Rgba([(x * 255 / w) as u8, (y * 255 / h) as u8, ((x + y) * 3 % 256) as u8, 255]))
}

fn make(img: RgbaImage) -> (ImageProcessor, Receiver<ImageProcessorEvent>, Arc<RwLock<GrblFile>>) {
    let (tx, rx) = unbounded();
    let file = Arc::new(RwLock::new(GrblFile::new()));
    let mut p = ImageProcessor::from_image(file.clone(), "test.png", img, 96, (100, 100), false, tx);
    p.suspend();
    p.set_red(100);
    p.set_green(100);
    p.set_blue(100);
    p.set_brightness(100);
    p.set_contrast(100);
    p.set_white_clip(5);
    p.set_threshold(50);
    p.set_quality(Decimal::from(3));
    p.set_filling_quality(Decimal::from(3));
    p.gcode = GcodeOptions { target_size: (10.0, 7.5), target_offset: (1.0, 2.0), laser_on: "M3".into(), laser_off: "M5".into(), border_speed: 800, mark_speed: 1000, min_power: 0, max_power: 255 };
    (p, rx, file)
}

fn next_preview(rx: &Receiver<ImageProcessorEvent>) -> RgbaImage {
    loop {
        match rx.recv_timeout(T).expect("no event") {
            ImageProcessorEvent::PreviewReady(i) => return i,
            ImageProcessorEvent::GenerationComplete(r) => panic!("unexpected completion {r:?}"),
            ImageProcessorEvent::PreviewBegin => {}
        }
    }
}

fn completion(rx: &Receiver<ImageProcessorEvent>) -> Result<(), String> {
    loop {
        if let ImageProcessorEvent::GenerationComplete(r) = rx.recv_timeout(T).expect("no completion") {
            return r;
        }
    }
}

fn program(f: &Arc<RwLock<GrblFile>>) -> Vec<String> {
    f.read().unwrap().commands().iter().map(|c| c.to_string()).collect()
}

#[test]
fn fit_and_gray_helpers() {
    assert_eq!(calculate_resize_to_fit((200, 100), (100, 100)), (100, 50));
    assert_eq!(calculate_resize_to_fit((100, 200), (100, 100)), (50, 100));
    // banker's rounding: 5 * 0.5 = 2.5 -> 2
    assert_eq!(calculate_resize_to_fit((5, 5), (2, 3)), (2, 2));
    assert!(test_gray_scale(&RgbaImage::from_pixel(30, 30, Rgba([100, 105, 95, 255]))));
    assert!(!test_gray_scale(&RgbaImage::from_pixel(30, 30, Rgba([255, 0, 0, 255]))));
    assert_eq!(job::get_vector_quality(100.0, false), 10.0);
    assert_eq!(job::get_vector_quality(1.0, true), 255.0);
    assert_eq!(job::get_vector_quality(1e12, true), 4.0);
    let q = job::get_vector_quality(10000.0, true);
    assert!((q - 255.0 / 10.0).abs() < 1e-9);
}

#[test]
fn suspend_resume_and_change_detection() {
    let (mut p, rx, _) = make(color_image(40, 30));
    // suspended: nothing happens
    p.set_red(50);
    assert!(rx.try_recv().is_err());
    p.resume();
    assert!(matches!(rx.recv_timeout(T).unwrap(), ImageProcessorEvent::PreviewBegin));
    let img = next_preview(&rx);
    // 40x30 into a 100x100 box
    assert_eq!((img.width(), img.height()), (100, 75));
    // same value: no refresh
    p.set_red(50);
    assert!(p.wait_worker(T));
    assert!(rx.try_recv().is_err());
    // quiet properties never refresh
    p.set_quality(Decimal::from(9));
    assert!(rx.try_recv().is_err());
    p.set_red(60);
    assert!(matches!(rx.recv_timeout(T).unwrap(), ImageProcessorEvent::PreviewBegin));
    p.dispose();
}

#[test]
fn preview_every_tool() {
    let (mut p, rx, _) = make(color_image(40, 30));
    p.resume();
    let _ = next_preview(&rx);
    for tool in [Tool::Line2Line, Tool::Dithering, Tool::NoProcessing, Tool::Vectorize] {
        p.set_selected_tool(tool);
        p.set_line_preview(true);
        p.set_line_direction(Direction::Horizontal);
        let img = next_preview(&rx);
        assert_eq!((img.width(), img.height()), (100, 75), "{tool:?}");
    }
    // demo shows the white point
    p.set_demo(true);
    let demo = next_preview(&rx);
    assert_eq!(demo.width(), 100);
    p.dispose();
}

#[test]
fn line_preview_draws_pattern() {
    let (mut p, rx, _) = make(RgbaImage::from_pixel(40, 30, Rgba([0, 0, 0, 255])));
    p.set_selected_tool(Tool::Line2Line);
    p.set_line_preview(true);
    p.set_line_direction(Direction::Horizontal);
    p.resume();
    let img = next_preview(&rx);
    // black picture: even rows lightened (alpha 200), odd rows black
    assert_eq!(img.get_pixel(5, 0)[0], 200);
    assert_eq!(img.get_pixel(5, 1)[0], 0);
    p.dispose();
}

#[test]
fn vector_preview_has_red_borders() {
    let mut img = RgbaImage::from_pixel(60, 60, Rgba([255, 255, 255, 255]));
    for y in 15..45 {
        for x in 15..45 {
            img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
        }
    }
    let (mut p, rx, _) = make(img);
    p.set_box_for_test((60, 60));
    p.set_selected_tool(Tool::Vectorize);
    p.resume();
    let out = next_preview(&rx);
    let red = out.pixels().filter(|px| px[0] > 200 && px[0] as i32 > px[1] as i32 + 60 && px[0] as i32 > px[2] as i32 + 60).count();
    assert!(red > 50, "red pixels {red}");
    p.dispose();
}

#[cfg(unix)]
fn fake_autotrace(body: &str) -> std::path::PathBuf {
    let d = lg_base::paths::test_dir("services_ip");
    let s = d.join(format!("fake-{}", body.len()));
    crate::test_support::write_script(&s, &format!("#!/bin/sh\n{body}\n"));
    s
}

#[cfg(unix)]
#[test]
fn centerline_preview_and_error() {
    let _g = settings_lock();
    let ok = fake_autotrace("echo '<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"75\"><path d=\"M 5 5 L 90 60\" stroke=\"black\" fill=\"none\"/></svg>'");
    lg_base::settings::set(crate::autotrace::SETTING_PATH, ok.display().to_string());
    let (mut p, rx, _) = make(color_image(40, 30));
    p.set_selected_tool(Tool::Centerline);
    p.resume();
    let out = next_preview(&rx);
    assert!(out.pixels().any(|px| px[0] > 200 && px[1] < 100 && px[2] < 100), "no red line");
    // white veil lifts the black parts
    assert!(out.pixels().all(|px| px[0] >= 170 || px[0] > px[1]));
    // failing autotrace: still a (veiled) preview
    lg_base::settings::set(crate::autotrace::SETTING_PATH, "/nonexistent/autotrace".to_string());
    p.set_red(90);
    let out = next_preview(&rx);
    assert!(out.pixels().all(|px| px[0] >= 170));
    p.dispose();
    lg_base::settings::set(crate::autotrace::SETTING_PATH, String::new());
}

/// Expected main bitmap for the default test options.
fn expected_bitmap(src: &RgbaImage, size: (u32, u32)) -> RgbaImage {
    let r = resize_image(src, size.0, size.1, false, InterpolationMode::HighQualityBicubic);
    let gs = gray_scale(&r, 1.0, 1.0, 1.0, -0.0, 1.0, lg_raster::Formula::SimpleAverage);
    let w = whitenize(&gs, 5, false);
    threshold(&w, 0.5, false)
}

#[test]
fn generate_line2line_equals_direct_gcode() {
    let _g = settings_lock();
    lg_base::settings::set("Support Hardware PWM", true);
    lg_base::settings::set("Disable G0 fast skip", false);
    let src = color_image(40, 30);
    let (mut p, rx, file) = make(src.clone());
    p.set_selected_tool(Tool::Line2Line);
    p.set_line_direction(Direction::Horizontal);
    p.resume();
    p.generate_gcode();
    completion(&rx).unwrap();
    let got = program(&file);
    assert!(got.len() > 10);

    // direct call with the same bitmap / conf
    let mut bmp = expected_bitmap(&src, (30, 22));
    let conf = L2LConf {
        res: 3.0,
        fres: 3.0,
        o_x: 1.0,
        o_y: 2.0,
        mark_speed: 1000,
        border_speed: 800,
        min_power: 0,
        max_power: 255,
        l_on: "M3".into(),
        l_off: "M5".into(),
        dir: Direction::Horizontal,
        pwm: true,
        ..L2LConf::default()
    };
    let mut direct = GrblFile::new();
    direct.load_image_l2l(&mut bmp, "test.png", &conf, false).unwrap();
    let want: Vec<String> = direct.commands().iter().map(|c| c.to_string()).collect();
    assert_eq!(got, want);
}

#[test]
fn generate_vectorize_equals_direct_gcode() {
    let _g = settings_lock();
    lg_base::settings::set("Support Hardware PWM", true);
    let mut src = RgbaImage::from_pixel(50, 50, Rgba([255, 255, 255, 255]));
    for y in 10..40 {
        for x in 12..38 {
            src.put_pixel(x, y, Rgba([0, 0, 0, 255]));
        }
    }
    let (mut p, rx, file) = make(src.clone());
    p.gcode.target_size = (20.0, 20.0);
    p.set_selected_tool(Tool::Vectorize);
    p.set_filling_direction(Direction::None);
    p.resume();
    p.generate_gcode();
    completion(&rx).unwrap();
    let got = program(&file);
    assert!(got.iter().any(|l| l.starts_with("F800")));

    // vectorize: res = min(maxRes, 10) -> 200 px
    let mut bmp = expected_bitmap(&src, (200, 200));
    let mut conf = L2LConf { res: 10.0, fres: 3.0, o_x: 1.0, o_y: 2.0, mark_speed: 1000, border_speed: 800, max_power: 255, l_on: "M3".into(), l_off: "M5".into(), dir: Direction::None, pwm: true, ..L2LConf::default() };
    let mut direct = GrblFile::new();
    direct.load_image_potrace(&mut bmp, "test.png", false, 0, false, Decimal::ZERO, false, Decimal::ZERO, false, &mut conf, false).unwrap();
    let want: Vec<String> = direct.commands().iter().map(|c| c.to_string()).collect();
    assert_eq!(got, want);
}

#[test]
fn generate_no_processing_uses_file_dpi() {
    let _g = settings_lock();
    lg_base::settings::set("Support Hardware PWM", true);
    let src = color_image(20, 10);
    let (mut p, rx, file) = make(src.clone());
    p.set_selected_tool(Tool::NoProcessing);
    p.resume();
    p.generate_gcode();
    completion(&rx).unwrap();
    let got = program(&file);
    let mut bmp = gray_scale(&src, 0.0, 0.0, 0.0, 0.0, 1.0, lg_raster::Formula::SimpleAverage);
    let r = 96.0 / 25.4;
    let conf = L2LConf { res: r, fres: r, o_x: 1.0, o_y: 2.0, mark_speed: 1000, border_speed: 800, max_power: 255, l_on: "M3".into(), l_off: "M5".into(), dir: Direction::Horizontal, pwm: true, ..L2LConf::default() };
    let mut direct = GrblFile::new();
    direct.load_image_l2l(&mut bmp, "test.png", &conf, false).unwrap();
    assert_eq!(got, direct.commands().iter().map(|c| c.to_string()).collect::<Vec<_>>());
}

#[test]
fn generate_zero_quality_reports_error() {
    let _g = settings_lock();
    let (mut p, rx, file) = make(color_image(20, 10));
    p.set_quality(Decimal::ZERO);
    p.set_selected_tool(Tool::Line2Line);
    p.resume();
    p.generate_gcode();
    assert_eq!(completion(&rx).unwrap_err(), "Target size too big for processing!");
    assert!(program(&file).is_empty());
}

#[test]
fn image_edit_operations() {
    let (mut p, _rx, _) = make(color_image(40, 30));
    assert_eq!(p.true_original().dimensions(), (40, 30));
    p.rotate_cw();
    assert_eq!(p.true_original().dimensions(), (30, 40));
    p.rotate_ccw();
    p.flip_h();
    p.flip_v();
    assert_eq!(p.true_original().dimensions(), (40, 30));
    // crop half of the preview (100x75) -> 20x15 pixels
    assert!(p.crop_image(Rect { x: 0, y: 0, width: 50, height: 37 }, (100, 75)));
    assert_eq!(p.true_original().dimensions(), (20, 14));
    assert!(!p.crop_image(Rect { x: 0, y: 0, width: 0, height: 5 }, (100, 75)));
    p.revert();
    assert_eq!(p.true_original().dimensions(), (40, 30));
    let before = p.true_original().get_pixel(0, 0)[0];
    p.invert();
    assert_eq!(p.true_original().get_pixel(0, 0)[0], 255 - before);
    assert!((p.width_to_height(40.0) - 30.0).abs() < 1e-4);
    assert!((p.height_to_widht(30.0) - 40.0).abs() < 1e-4);
    assert_eq!(p.file_dpi(), 96);
}

#[test]
fn auto_trim_removes_white_border() {
    let mut img = RgbaImage::from_pixel(40, 40, Rgba([255, 255, 255, 255]));
    for y in 10..30 {
        for x in 8..32 {
            img.put_pixel(x, y, Rgba([(x * 8) as u8, (y * 8) as u8, 0, 255]));
        }
    }
    let (mut p, _rx, _) = make(img);
    p.auto_trim();
    // borders 8 / 8 / 10 / 10 are homogeneous white; inner rows vary
    assert_eq!(p.true_original().dimensions(), (24, 20));
}

#[test]
fn rapid_refresh_aborts_old_previews() {
    let (mut p, rx, _) = make(color_image(120, 90));
    p.resume();
    for i in 0..8 {
        p.set_red(10 + i);
    }
    // the last preview must arrive; earlier ones may be dropped
    let mut last = None;
    let end = std::time::Instant::now() + T;
    while std::time::Instant::now() < end {
        match rx.recv_timeout(Duration::from_millis(300)) {
            Ok(ImageProcessorEvent::PreviewReady(i)) => last = Some(i),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    assert!(last.is_some());
    p.dispose();
}
