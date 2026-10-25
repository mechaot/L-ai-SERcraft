// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Engine tests against a recording fake link (deterministic replies) and against the in-process emulator.
//! Expected command strings were derived by reading `Core/GrblCore.cs`.

use crossbeam_channel::Receiver;
use lasergrbl_core::com::{ComWrapper, EmulatorCom, FakeCom};
use lasergrbl_core::emulator::Grblv11Emulator;
use lasergrbl_core::*;
use lg_dotnet::Decimal;
use lg_gcode::Firmware;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

type Responder = lasergrbl_core::com::fake::Responder;

static SERIAL: Mutex<()> = Mutex::new(());
static FAKE_NOW: AtomicI64 = AtomicI64::new(1_000_000);

fn fake_clock() -> i64 {
    FAKE_NOW.load(Ordering::SeqCst)
}

/// Serialise tests (global settings) and give them a clean settings store.
fn setup(name: &str) -> MutexGuard<'static, ()> {
    let g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("lg_core_it_{}_{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    lg_base::paths::set_data_path_override(dir.clone());
    lg_base::settings::init_at(dir.join("settings.json"));
    for s in ["Success", "Warning", "Fatal", "Connect", "Disconnect"] {
        lg_base::settings::set(&format!("Sound.{s}.Enabled"), false);
    }
    lg_base::settings::set("Threading Mode", ThreadingMode::Insane);
    lg_base::settings::set("GCode.CustomHeader", "G90".to_string());
    lg_base::settings::set("GCode.CustomFooter", "M5".to_string());
    lg_base::settings::set("GCode.CustomPasses", "G4 P0".to_string());
    lasergrbl_core::laser_life::reload_from(dir.join("llc.json"));
    g
}

fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    while Instant::now() < end {
        if cond() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timeout waiting for: {what}");
}

fn write_nc(name: &str, body: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("lg_core_it_{}_{}.nc", std::process::id(), name));
    std::fs::write(&p, body).unwrap();
    p
}

fn load_program(core: &Arc<GrblCore>, name: &str, body: &str) {
    let p = write_nc(name, body);
    core.loaded_file().write().unwrap().load_file(p.to_str().unwrap(), false).unwrap();
}

/// Answer `ok` to every G-code line (like a idle Grbl that always accepts).
fn ok_responder() -> Responder {
    Box::new(|l| if l.trim().is_empty() { vec![] } else { vec!["ok".to_string()] })
}

struct Rig {
    core: Arc<GrblCore>,
    fake: Arc<FakeCom>,
    rx: Receiver<CoreEvent>,
    auto_ok: bool,
}

fn rig(fw: Firmware, responder: Option<Responder>) -> Rig {
    let auto_ok = responder.is_none();
    let fake = match responder {
        Some(r) => FakeCom::with_responder(r),
        None => FakeCom::new(),
    };
    let (tx, rx) = crossbeam_channel::unbounded();
    let core = GrblCore::with_com(fw, tx, fake.clone(), lg_base::tools::hires_timer::total_milliseconds);
    Rig { core, fake, rx, auto_ok }
}

/// Connect with a Grbl 1.1f welcome and one idle status report; the machine answers config queries empty.
fn connect_idle(r: &Rig) {
    r.core.open_com().unwrap();
    assert_eq!(r.core.machine_status(), MacStatus::Connecting);
    wait_until("reset byte written", || r.fake.written_bytes().contains(&24));
    r.fake.inject("Grbl 1.1f ['$' for help]");
    r.fake.inject("<Idle|MPos:0.000,0.000,0.000|FS:0,0>");
    wait_until("idle", || r.core.machine_status() == MacStatus::Idle);
    // let the on-connect `$$` / `$I` refresh finish first (it runs 500 ms after the connect), so it
    // cannot interleave with what the test sends
    wait_until("$$ sent", || r.fake.written_lines().contains(&"$$".to_string()));
    if r.auto_ok {
        r.fake.inject("ok");
    }
    wait_until("$I sent", || r.fake.written_lines().contains(&"$I".to_string()));
    if r.auto_ok {
        r.fake.inject("ok");
    }
    // both waits end after 500 ms of silence at the latest
    std::thread::sleep(Duration::from_millis(700));
}

/// `connect_idle` and forget what was written so far.
fn connect_idle_clean(r: &Rig) {
    connect_idle(r);
    r.fake.clear_written();
}

fn events(r: &Rig) -> Vec<CoreEvent> {
    r.rx.try_iter().collect()
}

#[test]
fn connect_welcome_config_and_info() {
    let _g = setup("connect");
    // $$ and $I are answered like a Grbl 1.1 board
    let r = rig(
        Firmware::Grbl,
        Some(Box::new(|l| match l.trim() {
            "$$" => {
                let mut v: Vec<String> = vec!["$0=10".into(), "$30=255".into(), "$32=1".into(), "$130=300.000".into(), "$131=200.000".into()];
                v.push("ok".into());
                v
            }
            "$I" => vec!["[VER:1.1f.20170801:]".into(), "[OPT:V,15,254]".into(), "ok".into()],
            _ => vec!["ok".into()],
        })),
    );
    connect_idle(&r);
    // first bytes: soft reset on connect (default true), then a position query
    let bytes = r.fake.written_bytes();
    assert_eq!(bytes[0], 24);
    wait_until("config refreshed", || GrblCore::configuration().count() == 5);
    assert_eq!(GrblCore::configuration().max_pwm(), Decimal::from(255));
    let v = r.core.grbl_version().unwrap();
    assert_eq!((v.major(), v.minor(), v.build()), (1, 1, 'f'));
    assert!(r.core.support_true_jogging() && r.core.support_csv() && r.core.support_override());
    // $I told us the real buffer size
    wait_until("buffer enlarged", || r.core.buffer_size() == 254);
    let lines = r.fake.written_lines();
    assert!(lines.contains(&"$$".to_string()) && lines.contains(&"$I".to_string()), "{lines:?}");
    assert!(events(&r).contains(&CoreEvent::MachineStatusChanged));
    r.core.close_com(true).unwrap();
    assert_eq!(r.core.machine_status(), MacStatus::Disconnected);
}

#[test]
fn buffered_streaming_accounts_bytes() {
    let _g = setup("buffered");
    let r = rig(Firmware::Grbl, None);
    connect_idle_clean(&r);
    // 40 lines of 10 serial bytes ("G1X100.00\n") against a 127 byte buffer
    let body: String = (0..40).map(|i| format!("G1 X{}.00\n", 100 + i % 3)).collect();
    load_program(&r.core, "buffered", &body);
    r.fake.clear_written();
    assert_eq!(r.core.run_program(), RunDecision::Started);
    // header "G90" (4 bytes) + 12 lines = 124 <= 127 ; next line would be 134
    wait_until("buffer full", || r.core.used_buffer() == 124);
    std::thread::sleep(Duration::from_millis(100));
    let lines = r.fake.written_lines();
    assert_eq!(lines.len(), 13, "{lines:?}");
    assert_eq!(lines[0], "G90");
    assert_eq!(r.core.used_buffer(), 124);
    assert_eq!(r.core.free_buffer(), 3);
    // one ok frees 4 bytes ("G90\n"): the 10 byte line still does not fit, a second ok makes room
    r.fake.inject("ok");
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(r.fake.written_lines().len(), 13);
    r.fake.inject("ok");
    wait_until("one more line", || r.fake.written_lines().len() == 14);
    assert!(r.core.used_buffer() <= 127);
    assert!(r.core.in_program());
    r.core.close_com(true).unwrap();
}

#[test]
fn full_program_with_header_footer() {
    let _g = setup("program");
    let r = rig(Firmware::Grbl, Some(ok_responder()));
    connect_idle_clean(&r);
    load_program(&r.core, "program", "G0 X10\nG1 X20 F1000\nM5\n");
    r.fake.clear_written();
    assert_eq!(r.core.run_program(), RunDecision::Started);
    wait_until("footer written", || r.fake.written_lines().contains(&"M5".to_string()) && r.fake.written_lines().len() >= 5);
    wait_until("job ended", || !r.core.in_program());
    assert_eq!(r.fake.written_lines(), vec!["G90", "G0X10", "G1X20F1000", "M5", "M5"]);
    assert_eq!(r.core.program_target(), 4);
    assert_eq!(r.core.program_executed(), 4);
    wait_until("buffer empty", || r.core.used_buffer() == 0);
    let ev = events(&r);
    assert!(ev.contains(&CoreEvent::ProgramEnded));
    // sent list: reset message rows, welcome, commands with good results
    let rows = r.core.sent_command(0, 1000);
    let cmds: Vec<_> = rows.iter().filter_map(|x| if let SentRow::Command(c) = x { Some(c.clone()) } else { None }).collect();
    assert!(cmds.iter().all(|c| c.status() == lg_gcode::CommandStatus::ResponseGood));
    assert_eq!(cmds.len(), 5);
    r.core.close_com(true).unwrap();
}

#[test]
fn synchronous_streaming_one_at_a_time() {
    let _g = setup("sync");
    lg_base::settings::set("Streaming Mode", StreamingMode::Synchronous);
    let r = rig(Firmware::Grbl, None);
    connect_idle_clean(&r);
    load_program(&r.core, "sync", "G1 X1\nG1 X2\n");
    r.fake.clear_written();
    r.core.run_program();
    wait_until("first", || r.fake.written_lines().len() == 1);
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(r.fake.written_lines(), vec!["G90"]);
    r.fake.inject("ok");
    wait_until("second", || r.fake.written_lines().len() == 2);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(r.fake.written_lines().len(), 2);
    r.fake.inject("ok");
    wait_until("third", || r.fake.written_lines().len() == 3);
    assert_eq!(r.fake.written_lines(), vec!["G90", "G1X1", "G1X2"]);
    r.core.close_com(true).unwrap();
}

#[test]
fn repeat_on_error_resends_three_times() {
    let _g = setup("repeat");
    lg_base::settings::set("Streaming Mode", StreamingMode::RepeatOnError);
    lg_base::settings::set("GCode.CustomHeader", String::new());
    lg_base::settings::set("GCode.CustomFooter", String::new());
    let r = rig(Firmware::Grbl, Some(Box::new(|l| vec![if l.contains("X5") { "error:3" } else { "ok" }.to_string()])));
    connect_idle_clean(&r);
    // NOTE(upstream): a retry of the LAST line is sent after the job already ended, so use two lines
    load_program(&r.core, "repeat", "G1 X5\nG1 X6\n");
    r.fake.clear_written();
    r.core.run_program();
    wait_until("1+3 sends and next", || r.fake.written_lines().len() == 5);
    wait_until("job ended", || !r.core.in_program());
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(r.fake.written_lines(), vec!["G1X5", "G1X5", "G1X5", "G1X5", "G1X6"]);
    r.core.close_com(true).unwrap();
}

#[test]
fn hold_resume_and_status_mapping() {
    let _g = setup("hold");
    let r = rig(Firmware::Grbl, None);
    connect_idle_clean(&r);
    assert!(!r.core.can_feed_hold());
    r.fake.inject("<Run|MPos:1.000,2.000,0.000|WCO:0.500,0.500,0.000>");
    wait_until("run", || r.core.machine_status() == MacStatus::Run);
    assert_eq!(r.core.work_position().x, 0.5);
    r.fake.clear_written();
    r.core.feed_hold(false);
    assert_eq!(r.fake.written_bytes(), vec![33]);
    // user hold stays Hold, a hold we did not ask for is AutoHold
    r.fake.inject("<Hold:0|MPos:1.000,2.000,0.000>");
    wait_until("hold", || r.core.machine_status() == MacStatus::Hold);
    r.core.cycle_start_resume(false);
    assert_eq!(r.fake.written_bytes(), vec![33, 126]);
    r.fake.inject("<Run|MPos:1.000,2.000,0.000>");
    wait_until("run again", || r.core.machine_status() == MacStatus::Run);
    r.fake.inject("<Hold:0|MPos:1.000,2.000,0.000>");
    wait_until("autohold", || r.core.machine_status() == MacStatus::AutoHold);
    r.core.safety_door();
    assert_eq!(*r.fake.written_bytes().last().unwrap(), 64);
    r.core.close_com(true).unwrap();
}

#[test]
fn abort_reset_and_resume_dialog_data() {
    let _g = setup("abort");
    let r = rig(Firmware::Grbl, None);
    connect_idle_clean(&r);
    load_program(&r.core, "abort", "G1 X1\nG1 X2\nG1 X3\n");
    assert!(!r.core.run_would_ask_resume());
    r.core.run_program();
    wait_until("sent", || r.core.program_sent() >= 1);
    r.fake.inject("ok");
    wait_until("executed", || r.core.program_executed() >= 1);
    r.fake.clear_written();
    r.fake.inject("<Run|MPos:1.000,0.000,0.000>");
    wait_until("run", || r.core.machine_status() == MacStatus::Run);
    assert!(r.core.can_abort_program());
    r.core.abort_program();
    assert!(!r.core.in_program());
    wait_until("M5 sent", || r.fake.written_lines().contains(&"M5".to_string()));
    // not finished -> asks to resume; abort by the user is not reported to the UI
    r.fake.inject("<Idle|MPos:1.000,0.000,0.000>");
    wait_until("idle", || r.core.machine_status() == MacStatus::Idle);
    assert!(!events(&r).iter().any(|e| matches!(e, CoreEvent::IssueDetected(_))));
    // pending M5 must be answered or CanSendFile stays false (queue empty but pending irrelevant)
    r.fake.inject("ok");
    r.fake.inject("ok");
    r.fake.inject("ok");
    std::thread::sleep(Duration::from_millis(100));
    match r.core.run_program() {
        RunDecision::AskResume(i) => {
            assert_eq!(i.last_issue, DetectedIssue::ManualAbort);
            assert!(!i.suggest_homing);
            assert!(i.target > i.executed);
        }
        other => panic!("{other:?}"),
    }
    // manual reset: soft reset byte and overrides back to 100
    r.core.set_t_override_g1(120);
    r.fake.clear_written();
    r.core.grbl_reset();
    assert_eq!(r.fake.written_bytes(), vec![24]);
    assert_eq!(r.core.t_override_g1(), 100);
    r.core.close_com(true).unwrap();
}

#[test]
fn jog_commands_v11() {
    let _g = setup("jog");
    let r = rig(Firmware::Grbl, None);
    connect_idle_clean(&r);
    r.core.set_jog_speed(1000);
    r.core.set_jog_step(lg_dotnet::parse_decimal("10").unwrap());
    r.fake.clear_written();
    r.core.jog_to_direction_fast(JogDirection::NE, false).unwrap();
    wait_until("jog sent", || !r.fake.written_lines().is_empty());
    assert_eq!(r.fake.written_lines(), vec!["$J=G91X10.0Y10.0F1000"]);
    r.fake.inject("ok");
    r.core.jog_to_position(PointF::new(12.5, 7.0), 3000.0);
    wait_until("pos sent", || r.fake.written_lines().len() == 2);
    assert_eq!(r.fake.written_lines()[1], "$J=G90X12.50Y7.00F3000");
    r.fake.inject("ok");
    r.core.jog_to_direction_step_fast(JogDirection::Zdown, true, lg_dotnet::parse_decimal("0.5").unwrap()).unwrap();
    wait_until("z sent", || r.fake.written_lines().len() == 3);
    assert_eq!(r.fake.written_lines()[2], "$J=G91Z-0.5F100000");
    assert!(r.core.jog_to_direction(JogDirection::Abort, 1.0, Decimal::ONE).is_err());
    // continuous jog: new target aborts the previous one with 0x85
    lg_base::settings::set("Enable Continuous Jog", true);
    r.fake.inject("ok");
    r.fake.clear_written();
    r.core.jog_to_direction(JogDirection::E, 2000.0, Decimal::ONE).unwrap();
    wait_until("cj1", || r.fake.written_lines().len() == 1);
    r.fake.inject("ok");
    r.core.jog_to_direction(JogDirection::N, 2000.0, Decimal::ONE).unwrap();
    wait_until("cj2", || r.fake.written_lines().len() == 2);
    assert!(r.fake.written_bytes().contains(&0x85));
    assert_eq!(r.fake.written_lines()[1], "$J=G53Y200.0F2000");
    r.core.close_com(true).unwrap();
}

#[test]
fn overrides_steps() {
    let _g = setup("overrides");
    let r = rig(Firmware::Grbl, None);
    connect_idle_clean(&r);
    r.fake.clear_written();
    r.core.set_t_override_g1(120);
    r.fake.inject("<Run|MPos:0.000,0.000,0.000|Ov:100,100,100>");
    wait_until("+10", || r.fake.written_bytes().contains(&145));
    r.core.set_t_override_g1(105);
    r.fake.inject("<Run|MPos:0.000,0.000,0.000|Ov:100,100,100>");
    wait_until("+1", || r.fake.written_bytes().contains(&147));
    r.core.set_t_override_g1(100);
    r.core.set_t_override_s(100);
    r.core.set_t_override_g0(25);
    r.fake.inject("<Run|MPos:0.000,0.000,0.000|Ov:130,90,100>");
    wait_until("reset linear", || r.fake.written_bytes().contains(&144));
    assert!(r.fake.written_bytes().contains(&151));
    assert_eq!((r.core.override_g1(), r.core.override_g0(), r.core.override_s()), (130, 90, 100));
    assert!(events(&r).contains(&CoreEvent::OverrideChanged));
    r.core.hot_key_override(OverrideAction::PowerUp);
    assert_eq!(r.core.t_override_s(), 101);
    r.core.hot_key_override(OverrideAction::RapidDown);
    assert_eq!(r.core.t_override_g0(), 25);
    r.core.close_com(true).unwrap();
}

#[test]
fn hang_detection_with_mocked_clock() {
    let _g = setup("hang");
    FAKE_NOW.store(1_000_000, Ordering::SeqCst);
    let fake = FakeCom::new();
    let (tx, rx) = crossbeam_channel::unbounded();
    let core = GrblCore::with_com(Firmware::Grbl, tx, fake.clone(), fake_clock);
    let r = Rig { core, fake, rx, auto_ok: true };
    connect_idle_clean(&r);
    load_program(&r.core, "hang", "G1 X1\n");
    r.core.run_program();
    r.fake.inject("<Run|MPos:0.000,0.000,0.000>");
    wait_until("run", || r.core.machine_status() == MacStatus::Run);
    std::thread::sleep(Duration::from_millis(100));
    assert!(!events(&r).iter().any(|e| matches!(e, CoreEvent::IssueDetected(_))));
    // 3 s without status: below max(10 * period, 5 s)
    FAKE_NOW.fetch_add(3_000, Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(100));
    assert!(!events(&r).iter().any(|e| matches!(e, CoreEvent::IssueDetected(_))));
    FAKE_NOW.fetch_add(3_000, Ordering::SeqCst);
    wait_until("issue", || events(&r).contains(&CoreEvent::IssueDetected(DetectedIssue::StopResponding)));
    r.core.close_com(true).unwrap();
}

#[test]
fn unexpected_disconnect_and_reset_are_issues() {
    let _g = setup("issues");
    let r = rig(Firmware::Grbl, None);
    connect_idle_clean(&r);
    load_program(&r.core, "issues", "G1 X1\nG1 X2\n");
    r.core.run_program();
    r.fake.inject("<Run|MPos:0.000,0.000,0.000>");
    wait_until("run", || r.core.machine_status() == MacStatus::Run);
    // board restarts by itself: a welcome message while running
    r.fake.inject("Grbl 1.1f ['$' for help]");
    wait_until("reset issue", || events(&r).contains(&CoreEvent::IssueDetected(DetectedIssue::UnexpectedReset)));
    assert!(!r.core.in_program());
    // alarm during a job
    load_program(&r.core, "issues2", "G1 X1\n");
    r.fake.inject("<Idle|MPos:0.000,0.000,0.000>");
    wait_until("idle", || r.core.machine_status() == MacStatus::Idle);
    r.core.run_program();
    r.fake.inject("ALARM:1");
    wait_until("alarm", || events(&r).contains(&CoreEvent::IssueDetected(DetectedIssue::MachineAlarm)));
    // link failure
    r.fake.inject("<Run|MPos:0.000,0.000,0.000>");
    wait_until("run2", || r.core.machine_status() == MacStatus::Run);
    r.fake.inject_error();
    wait_until("disconnected", || r.core.machine_status() == MacStatus::Disconnected);
    r.core.close_com(true).unwrap();
}

#[test]
fn loop_count_and_passes() {
    let _g = setup("loop");
    let r = rig(Firmware::Grbl, Some(ok_responder()));
    connect_idle_clean(&r);
    load_program(&r.core, "loop", "G1 X5\n");
    r.core.set_loop_count(Decimal::from(2));
    r.fake.clear_written();
    r.core.run_program();
    wait_until("done", || r.fake.written_lines().len() >= 5 && !r.core.in_program());
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(r.fake.written_lines(), vec!["G90", "G1X5", "G4P0", "G1X5", "M5"]);
    let ev = events(&r);
    assert_eq!(ev.iter().filter(|e| **e == CoreEvent::ProgramEnded).count(), 2);
    assert!(ev.contains(&CoreEvent::LoopCountChanged(Decimal::ONE)));
    assert_eq!(r.core.loop_count(), Decimal::ONE);
    r.core.close_com(true).unwrap();
}

#[test]
fn resume_from_position_commands() {
    let _g = setup("resume");
    lg_base::settings::set("GCode.CustomFooter", String::new());
    let r = rig(Firmware::Grbl, None);
    connect_idle_clean(&r);
    load_program(&r.core, "resume", "G90\nG0 X10 Y10\nM3 S500\nG1 X20 F1000\nG1 X30\nM5\n");
    r.fake.inject("<Idle|MPos:5.000,6.000,0.000>");
    wait_until("pos", || r.core.machine_position().x == 5.0);
    r.fake.clear_written();
    r.core.continue_program(3, true, true).unwrap();
    // let everything out one by one
    for i in 0..9 {
        wait_until("line", || r.fake.written_lines().len() > i || i >= 7);
        r.fake.inject("ok");
        if r.fake.written_lines().len() >= 7 {
            break;
        }
    }
    wait_until("all", || r.fake.written_lines().len() >= 7);
    // $H first, G92 uses the machine position minus the last known WCO (homing: from zero), absolute mode,
    // fast move with F and S of the analysed part, settled modals, then the rest of the file
    assert_eq!(
        r.fake.written_lines()[..7],
        ["$H", "G92X0Y0Z0", "G90", "M5G0X10Y10Z0F0S500", "G90M3", "G1X20F1000", "G1X30"][..]
    );
    assert_eq!(r.core.program_target(), 6);
    r.core.close_com(true).unwrap();
}

#[test]
fn marlin_smoothie_vigo_variants() {
    let _g = setup("variants");
    // Marlin: M114 position query, status from position line, synchronous
    let m = rig(Firmware::Marlin, None);
    m.core.open_com().unwrap();
    wait_until("M114", || m.fake.written_lines().contains(&"M114".to_string()));
    assert!(m.fake.written_bytes().is_empty(), "marlin sends no soft reset byte");
    m.fake.inject("X:10.00 Y:2.00 Z:0.00 E:0.00 Count X:1600 Y:0 Z:0");
    wait_until("idle", || m.core.machine_status() == MacStatus::Idle);
    assert_eq!(m.core.machine_position().x, 10.0);
    assert!(m.core.refresh_config(RefreshCause::OnRead).is_ok());
    m.core.close_com(true).unwrap();

    // Smoothie: no soft reset, newline then '?'
    let s = rig(Firmware::Smoothie, None);
    s.core.open_com().unwrap();
    wait_until("smoothie init", || s.fake.written_bytes().len() >= 2);
    assert_eq!(s.fake.written_bytes()[..2], [10, 63]);
    s.core.close_com(true).unwrap();

    // Vigo: 0x88 status query
    let v = rig(Firmware::VigoWork, None);
    v.core.open_com().unwrap();
    wait_until("vigo query", || v.fake.written_bytes().contains(&0x88));
    v.core.close_com(true).unwrap();
}

#[test]
fn custom_code_expressions_and_immediates() {
    let _g = setup("custom");
    let r = rig(Firmware::Grbl, None);
    connect_idle_clean(&r);
    load_program(&r.core, "custom", "M3 S500\nG1 X10 Y20\nG1 X30 Y50\n");
    assert_eq!(r.core.evaluate_expression("G0 X[left] Y[top]"), "G0 X0.000 Y50.000");
    assert_eq!(r.core.evaluate_expression("S[$30]"), "S1000.000");
    r.fake.clear_written();
    r.core.execute_custom_code("G91\n?\n0x85\nG0 X[width]");
    wait_until("lines", || r.fake.written_lines().len() == 2);
    assert_eq!(r.fake.written_lines(), vec!["G91", "G0X30.000"]);
    assert_eq!(r.fake.written_bytes(), vec![63, 0x85]);
    let out = std::env::temp_dir().join(format!("lg_core_it_{}_saved.nc", std::process::id()));
    r.core.save_program(&out, true, true, false, 1, true).unwrap();
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "G90\nM3 S500\nG1 X10 Y20\nG1 X30 Y50\nM5\n");
    r.core.close_com(true).unwrap();
}

#[test]
fn open_file_dispatch() {
    let _g = setup("open");
    let r = rig(Firmware::Grbl, None);
    let p = write_nc("open", "G1 X1\n");
    assert_eq!(r.core.open_file(&p, false), OpenFileOutcome::GCodeLoading);
    wait_until("loaded", || r.core.has_program());
    let ev: Vec<_> = r.rx.try_iter().collect();
    assert!(ev.iter().any(|e| matches!(e, CoreEvent::FileLoaded { .. })), "{ev:?}");
    assert_eq!(r.core.usage_counters().gcode_file, 1);
    assert!(r.core.can_reopen_file());
    let last: Option<String> = lg_base::settings::get("Core.LastOpenFile", None);
    assert_eq!(last.as_deref(), p.to_str());
    assert_eq!(r.core.open_file(std::path::Path::new("/x/a.PNG"), false), OpenFileOutcome::Raster("/x/a.PNG".into()));
    assert_eq!(r.core.open_file(std::path::Path::new("/x/a.svg"), false), OpenFileOutcome::Svg("/x/a.svg".into()));
    assert_eq!(r.core.open_file(std::path::Path::new("/x/a.lps"), false), OpenFileOutcome::Project("/x/a.lps".into()));
    assert_eq!(r.core.open_file(std::path::Path::new("/x/a.xyz"), false), OpenFileOutcome::Unsupported);
    let c = r.core.usage_counters();
    assert_eq!((c.raster_file, c.svg_file), (1, 1));
}

#[test]
fn full_core_against_emulator() {
    let _g = setup("emulator");
    let cfg = std::env::temp_dir().join(format!("lg_core_it_{}_emu.json", std::process::id()));
    let _ = std::fs::remove_file(&cfg);
    let com: Arc<dyn ComWrapper> = Arc::new(EmulatorCom::with_emulator(|sink| Grblv11Emulator::with_config_file(sink, cfg.clone())));
    let (tx, rx) = crossbeam_channel::unbounded();
    let core = GrblCore::with_com(Firmware::Grbl, tx, com, lg_base::tools::hires_timer::total_milliseconds);
    core.open_com().unwrap();
    wait_until("idle", || core.machine_status() == MacStatus::Idle);
    let v = core.grbl_version().unwrap();
    assert_eq!((v.major(), v.minor(), v.build()), (1, 1, '#'));
    // $$ refresh (34 params) and $I buffer info (OPT says 1024 > 127)
    wait_until("config", || GrblCore::configuration().count() == 34);
    wait_until("info", || core.buffer_size() == 1024);
    // small program
    load_program(&core, "emu", "G0 X1 Y1\nG1 X2 Y2 F6000\nG1 X0 Y0\n");
    assert_eq!(core.run_program(), RunDecision::Started);
    wait_until("program done", || !core.in_program() && core.program_executed() == core.program_target() && core.program_target() > 0);
    wait_until("buffer empty", || core.used_buffer() == 0);
    assert!(rx.try_iter().any(|e| e == CoreEvent::ProgramEnded));
    // write config round trip
    wait_until("idle again", || core.machine_status() == MacStatus::Idle);
    core.write_config(&[lg_gcode::GrblConfParam::new(100, "300")]).unwrap();
    assert_eq!(GrblCore::configuration().get(100), Some("300"));
    let err = core.write_config(&[lg_gcode::GrblConfParam::new(999, "1")]).unwrap_err();
    assert!(matches!(err, CoreError::WriteConfig { .. }));
    // hold/resume through the emulator
    core.close_com(true).unwrap();
    assert_eq!(core.machine_status(), MacStatus::Disconnected);
}
