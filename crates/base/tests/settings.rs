// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Settings store tests (one process, one test fn: the store is global).

use lg_base::settings::*;
use std::time::{Duration, Instant};

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::path::PathBuf::from(std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into())).join(format!("lg_settings_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize, Default, Clone, Copy)]
enum Mode {
    #[default]
    A,
    B,
}

#[test]
fn settings_all() {
    let d = tmp("main");
    let f = d.join("LaserGRBL.Settings.json");
    init_at(f.clone());
    assert!(is_new_file());
    assert_eq!(prev_version(), (0, 0, 0));

    // typed get/set with defaults
    assert_eq!(get("x.int", 5i32), 5);
    set("x.int", 7i32);
    assert_eq!(get("x.int", 5i32), 7);
    // type mismatch -> default
    assert!(!get("x.int", false));
    assert_eq!(get("x.int", String::from("d")), "d");
    set("x.mode", Mode::B);
    assert_eq!(get("x.mode", Mode::A), Mode::B);
    set("x.arr", vec![1, 2, 3]);
    assert_eq!(get("x.arr", Vec::<i32>::new()), vec![1, 2, 3]);
    assert!(exists("x.int"));
    assert_eq!(get_and_delete("x.int", 0i32), 7);
    assert!(!exists("x.int"));

    // debounce: nothing written immediately, written after ~2 s
    let t0 = Instant::now();
    assert!(!f.exists());
    while !f.exists() && t0.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(50));
    }
    let el = t0.elapsed();
    assert!(f.exists(), "no save");
    assert!(el >= Duration::from_millis(1500), "saved too early {el:?}");
    let txt = std::fs::read_to_string(&f).unwrap();
    assert!(txt.contains("\"x.mode\": \"B\""), "{txt}");
    assert!(txt.contains("Current LaserGRBL Version"));

    // exiting flushes now
    set("y", 1i32);
    exiting();
    assert!(std::fs::read_to_string(&f).unwrap().contains("\"y\": 1"));

    // reload: not new, prev version = current
    init_at(f.clone());
    assert!(!is_new_file());
    assert_eq!(prev_version(), CURRENT_VERSION);
    assert_eq!(get("y", 0i32), 1);
    assert_eq!(get("x.mode", Mode::A), Mode::B);

    // unchanged value does not trigger save, force does
    exiting();
    let m1 = std::fs::metadata(&f).unwrap().modified().unwrap();
    std::thread::sleep(Duration::from_millis(30));
    set("y", 1i32);
    exiting();
    assert_eq!(std::fs::metadata(&f).unwrap().modified().unwrap(), m1);
    set_force_save("y", 1i32);
    exiting();
    assert!(std::fs::metadata(&f).unwrap().modified().unwrap() > m1);

    // RetainedSetting
    {
        use std::sync::{atomic::{AtomicI32, Ordering}, Arc};
        let hits = Arc::new(AtomicI32::new(0));
        let h2 = hits.clone();
        let mut r = lg_base::tools::retained_setting::RetainedSetting::new("ret.val", 5i32);
        r.set_on_change(Box::new(move |_| {
            h2.fetch_add(1, Ordering::SeqCst);
        }));
        assert_eq!(*r.value(), 5);
        assert_eq!(get("ret.val", 0i32), 5);
        r.set(5);
        assert_eq!(hits.load(Ordering::SeqCst), 0);
        r.set(9);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert_eq!(get("ret.val", 0i32), 9);
    }

    // graphic modes
    assert_eq!(configured_graphic_mode(), GraphicMode::Auto);
    set_configured_graphic_mode(GraphicMode::Fbo);
    assert_eq!(get("ConfiguredGraphicMode", 0i32), 3);
    assert_eq!(requested_graphic_mode(), GraphicMode::Fbo);
    set_forced_graphic_mode(GraphicMode::Gdi);
    assert_eq!(requested_graphic_mode(), GraphicMode::Gdi);
    set_current_graphic_mode(GraphicMode::Dib);
    assert_eq!(current_graphic_mode(), GraphicMode::Dib);
    set_forced_graphic_mode(GraphicMode::Auto);

    // import of a Windows .bin when JSON is missing
    let d2 = tmp("import");
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/base/settings_sample.bin"),
        d2.join("LaserGRBL.Settings.bin"),
    )
    .unwrap();
    init_at(d2.join("LaserGRBL.Settings.json"));
    assert!(!is_new_file());
    assert!(get("t.bool", false));
    assert_eq!(get("t.int", 0i32), -42);
    assert_eq!(get("t.string", String::new()), "hello \u{e4}\u{20ac}");
    assert_eq!(get("t.enum", 0i32), 3);
    exiting();
}
