// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Logger + ComLogger output tests.

use lg_base::logger;

#[test]
fn logger_files() {
    let d = lg_base::paths::test_dir("logger");
    lg_base::paths::set_data_path_override(d.clone());

    // pre-existing log with 1500 lines gets rotated to the last 1000
    let old: String = (0..1500).map(|i| format!("old{i}\r\n")).collect();
    std::fs::write(logger::log_file_path(), old).unwrap();

    logger::start();
    logger::log_message("Test", "line one\nline two");
    logger::log_exception("Ctx", &"boom");
    logger::stop();
    let t = std::fs::read_to_string(logger::log_file_path()).unwrap();
    let lines: Vec<&str> = t.split("\r\n").collect();
    assert_eq!(lines[0], "old500");
    // version from Cargo, no hard-coded number
    let start = format!("\tProgram     \t------- L-ai-SERcraft v{} [", env!("CARGO_PKG_VERSION"));
    assert!(t.contains(&start), "{t}");
    assert!(t.contains("\tTest        \tline one\r\n"));
    assert!(t.contains("\tTest        \tline two\r\n"));
    assert!(t.contains("\tCtx         \tboom\r\n"));
    assert!(t.contains("---------------- PROGRAM STOP -----------------\r\n\r\n"));
    // timestamp shape dd/MM/yyyy HH:mm:ss.fff
    let l = lines.iter().find(|l| l.contains("line one")).unwrap();
    let ts = l.split('\t').next().unwrap();
    assert_eq!(ts.len(), 23, "{ts}");
    assert_eq!(&ts[2..3], "/");
    assert_eq!(&ts[19..20], ".");

    // ComLogger
    let cf = d.join("com.txt");
    assert!(!logger::com::enabled());
    logger::com::log("tx", b"ignored");
    let m = logger::com::start_log(cf.clone());
    assert!(m.starts_with("Recording session started @ "));
    assert!(logger::com::enabled());
    logger::com::log("tx", &[0x0a]);
    logger::com::log("rx", &[1, 0xab, 0xff]);
    logger::com::log_str("com", "a\r\nb");
    let stop = logger::com::stop_log().unwrap();
    assert!(stop.starts_with("Recording session stopped @ "));
    assert!(!logger::com::enabled());
    let t = std::fs::read_to_string(&cf).unwrap();
    let rows: Vec<Vec<&str>> = t.split("\r\n").filter(|l| !l.is_empty()).map(|l| l.split('\t').collect()).collect();
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0][0], "00000");
    assert_eq!(rows[0][2], "log");
    assert_eq!(rows[1][0], "00001");
    assert_eq!(rows[1][1].len(), 8);
    assert_eq!((rows[1][2], rows[1][3]), ("tx", "[0A]"));
    assert_eq!(rows[2][3], "[01, AB, FF]");
    assert_eq!(rows[3][3], "a\\r\\nb");
    assert_eq!(rows[4][0], "00004");
    // set_enabled uses default file
    logger::com::set_enabled(true);
    assert!(logger::com::enabled());
    logger::com::set_enabled(false);
    assert!(logger::com::default_file().exists());
}
