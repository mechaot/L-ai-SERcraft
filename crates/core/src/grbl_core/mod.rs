// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `Core/GrblCore.cs` (+ `MarlinCore.cs`, `SmoothieCore.cs`, `VigoCore.cs`): the GRBL streaming engine.
//!
//! `GrblCore` is used as `Arc<GrblCore>`. State guarded by `lock(this)` in C# lives in one `Mutex<CoreState>`
//! (see `state.rs` for the locking rules). The C# subclasses are the `Firmware` value fixed at construction;
//! every C# `virtual` member is a `match` inside the shared method.
//!
//! UI inversion (PLAN 2.1): dialogs are not opened here. `run_program` returns [`RunDecision`], the resume
//! dialog continues with [`GrblCore::continue_program`] / [`GrblCore::run_program_from_start_user`];
//! `open_file` returns an [`OpenFileOutcome`]; the safety countdown, `HKDisconnect` confirmation and the
//! laser selector are UI-side.

pub mod expr;
pub mod jog;
pub mod state;
pub mod testing;
pub mod time_projection;
pub mod types;

pub use expr::{ExprVars, FileBounds};
pub use jog::ContinuousJog;
pub use time_projection::{Clock, TimeProjection};
pub use types::*;

use crate::com::{create_wrapper, ComConfig, ComWrapper, WrapperType};
use crate::usage_stats::UsageCounters;
use crate::util::get_timespan_setting;
use jog::JogTargets;
use lg_base::logger;
use lg_base::tools::cronometro::PeriodicEventTimer;
use lg_base::tools::hires_timer;
use lg_base::tools::retained_setting::RetainedSetting;
use lg_base::tools::thread_class::{ThreadHandle, ThreadObject};
use lg_dotnet::{decimal_to_f32, Decimal, TimeSpan};
use lg_gcode::{configuration, Firmware, GPoint, GrblCommand, GrblConfParam, GrblConfST, GrblFile, GrblMessage, GrblRow, GrblVersionInfo};
use state::CoreState;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, Weak};
use std::time::Duration;

/// `GrblCore.ImageExtensions`.
pub const IMAGE_EXTENSIONS: [&str; 5] = [".jpg", ".jpeg", ".bmp", ".png", ".gif"];
/// `GrblCore.GCodeExtensions`.
pub const GCODE_EXTENSIONS: [&str; 5] = [".nc", ".cnc", ".tap", ".gcode", ".ngc"];
/// `GrblCore.ProjectFileExtensions`.
pub const PROJECT_FILE_EXTENSIONS: [&str; 1] = [".lps"];

/// Parts of the core that live outside the big lock.
pub(crate) struct Shared {
    pub firmware: Firmware,
    pub events: crossbeam_channel::Sender<CoreEvent>,
    pub file: Arc<RwLock<GrblFile>>,
    pub com: RwLock<Arc<dyn ComWrapper>>,
    pub tp_cell: Arc<Mutex<TimeProjection>>,
    pub tx: Mutex<ThreadObject>,
    pub rx: Mutex<ThreadObject>,
    pub tx_h: ThreadHandle,
    pub rx_h: ThreadHandle,
    pub jog: Mutex<JogTargets>,
    pub fix_ch340: AtomicBool,
    pub ch340_goodread: AtomicU32,
    pub ch340_exception: AtomicU32,
    pub me: Weak<GrblCore>,
    pub clock: Clock,
}

impl Shared {
    /// Lock the time projection (leaf lock).
    pub(crate) fn tp(&self) -> MutexGuard<'_, TimeProjection> {
        self.tp_cell.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// View options persisted through `RetainedSetting` (C# `ShowLaserOffMovements` ...).
pub struct ViewOptions {
    pub show_laser_off_movements: RetainedSetting<bool>,
    pub show_executed_commands: RetainedSetting<bool>,
    pub show_performance_diagnostic: RetainedSetting<bool>,
    pub show_bounding_box: RetainedSetting<bool>,
    pub cross_cursor: RetainedSetting<bool>,
    pub preview_line_size: RetainedSetting<f32>,
    pub auto_size_on_drawing: RetainedSetting<bool>,
}

impl ViewOptions {
    fn new() -> Self {
        Self {
            show_laser_off_movements: RetainedSetting::new("ShowLaserOffMovements", true),
            show_executed_commands: RetainedSetting::new("ShowExecutedCommands", true),
            show_performance_diagnostic: RetainedSetting::new("ShowPerformanceDiagnostic", false),
            show_bounding_box: RetainedSetting::new("ShowBoundingBox", true),
            cross_cursor: RetainedSetting::new("CrossCursor", true),
            preview_line_size: RetainedSetting::new("PreviewLineSize", 1.0),
            auto_size_on_drawing: RetainedSetting::new("AutoSizeOnDrawing", true),
        }
    }
}

/// The engine.
pub struct GrblCore {
    sh: Arc<Shared>,
    st: Mutex<CoreState>,
    view: Mutex<ViewOptions>,
}

/// Recover a poisoned wait: panics in worker threads must not kill the core.
fn guard<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl GrblCore {
    /// Create the core (C# ctor of `GrblCore` / `MarlinCore` / `SmoothieCore` / `VigoCore`).
    ///
    /// `firmware`: variant, fixed for the life of the core (setting "Firmware Type" in the C# app).
    /// `events`: sender for [`CoreEvent`]; the UI drains the receiver. The link starts as a serial `UsbSerial`.
    pub fn new(firmware: Firmware, events: crossbeam_channel::Sender<CoreEvent>) -> Arc<GrblCore> {
        Self::with_com(firmware, events, create_wrapper(WrapperType::UsbSerial), hires_timer::total_milliseconds)
    }

    /// Same as `new` with a given link and clock (tests: `FakeCom`, mocked millisecond clock).
    ///
    /// `com`: initial link. `clock`: millisecond time source for job timing, connect timeout and hang detection.
    pub fn with_com(firmware: Firmware, events: crossbeam_channel::Sender<CoreEvent>, com: Arc<dyn ComWrapper>, clock: Clock) -> Arc<GrblCore> {
        if firmware != Firmware::Grbl {
            logger::log_message("Program", &format!("Load {firmware:?} core"));
        }

        // with version 4.5.0 default ThreadingMode change from "UltraFast" to "Fast"
        if !lg_base::settings::is_new_file() && lg_base::settings::prev_version() < (4, 5, 0) {
            let cur: ThreadingMode = lg_base::settings::get("Threading Mode", ThreadingMode::Fast);
            if matches!(cur, ThreadingMode::Insane | ThreadingMode::UltraFast) {
                lg_base::settings::set("Threading Mode", ThreadingMode::Fast);
            }
        }
        let mode: ThreadingMode = lg_base::settings::get("Threading Mode", ThreadingMode::Fast);

        Arc::new_cyclic(|weak: &Weak<GrblCore>| {
            let w_tx = weak.clone();
            let w_tx0 = weak.clone();
            let w_rx = weak.clone();
            let tx = ThreadObject::with_callbacks(
                "Serial TX Thread",
                1,
                Box::new(move || {
                    if let Some(c) = w_tx.upgrade() {
                        c.thread_tx();
                    }
                }),
                Some(Box::new(move || {
                    if let Some(c) = w_tx0.upgrade() {
                        c.start_tx();
                    }
                })),
            );
            let rx = ThreadObject::with_callbacks(
                "Serial RX Thread",
                1,
                Box::new(move || {
                    if let Some(c) = w_rx.upgrade() {
                        c.thread_rx();
                    }
                }),
                None,
            );
            let conf = configuration::configuration();
            let file = Arc::new(RwLock::new(GrblFile::with_range(Decimal::ZERO, Decimal::ZERO, conf.table_width(), conf.table_height())));
            let tp_cell = Arc::new(Mutex::new(TimeProjection::with_clock(clock)));
            // file events: reset the projection and forward to the UI (no state lock: see state.rs)
            {
                let (ev1, tp1) = (events.clone(), tp_cell.clone());
                let (ev2, tp2) = (events.clone(), tp_cell.clone());
                let mut f = file.write().unwrap_or_else(|e| e.into_inner());
                f.add_on_file_loading(Box::new(move |elapsed_ms, filename| {
                    tp1.lock().unwrap_or_else(|e| e.into_inner()).reset(true);
                    let _ = ev1.send(CoreEvent::FileLoading { elapsed_ms, filename: filename.to_string() });
                }));
                f.add_on_file_loaded(Box::new(move |elapsed_ms, filename| {
                    tp2.lock().unwrap_or_else(|e| e.into_inner()).reset(true);
                    let _ = ev2.send(CoreEvent::FileLoaded { elapsed_ms, filename: filename.to_string() });
                }));
            }
            let sh = Arc::new(Shared {
                firmware,
                events,
                file,
                com: RwLock::new(com),
                tp_cell,
                tx_h: tx.handle(),
                rx_h: rx.handle(),
                tx: Mutex::new(tx),
                rx: Mutex::new(rx),
                jog: Mutex::new(JogTargets::default()),
                fix_ch340: AtomicBool::new(false),
                ch340_goodread: AtomicU32::new(0),
                ch340_exception: AtomicU32::new(0),
                me: weak.clone(),
                clock,
            });
            let st = CoreState::new(sh.clone(), mode);
            if let Some(v) = &st.grbl_version {
                // load setting for last known version
                lg_gcode::csvd::load_appropriate(v);
            }
            GrblCore { sh, st: Mutex::new(st), view: Mutex::new(ViewOptions::new()) }
        })
    }

    /// Lock the state.
    fn s(&self) -> MutexGuard<'_, CoreState> {
        guard(&self.st)
    }

    fn com(&self) -> Arc<dyn ComWrapper> {
        self.sh.com.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The firmware variant.
    pub fn firmware(&self) -> Firmware {
        self.sh.firmware
    }

    /// View options (retained settings); lock for the duration of the access.
    pub fn view_options(&self) -> MutexGuard<'_, ViewOptions> {
        guard(&self.view)
    }

    // ------------------------------------------------------------------ threads

    /// TX thread first run (`StartTX`).
    fn start_tx(&self) {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.s().start_tx()));
        if r.is_err() {
            logger::log_message("StartTX", "panic");
        }
    }

    /// `ThreadTX`.
    fn thread_tx(&self) {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.s().thread_tx()));
        if r.is_err() {
            logger::log_message("ThreadTX", "panic");
        }
    }

    /// `ThreadRX`.
    fn thread_rx(&self) {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(line) = self.wait_com_line_or_disconnect() {
                let mut st = self.s();
                st.manage_received_line(&line);
                st.handle_missing_ok();
            }
            let mode = self.s().current_threading_mode();
            let sleep = if self.has_incoming_data() { mode.rx_short() } else { mode.rx_long() };
            self.sh.rx_h.set_sleep_time(sleep);
        }));
        if r.is_err() {
            logger::log_message("ThreadRX", "panic");
        }
    }

    /// `WaitComLineOrDisconnect()` including the CH340 workaround.
    fn wait_com_line_or_disconnect(&self) -> Option<String> {
        let com = self.com();
        if self.sh.fix_ch340.load(Ordering::SeqCst) && (!com.is_open() || !self.has_incoming_data()) {
            return None;
        }
        let mut rv: Option<String> = None;
        loop {
            match com.read_line_blocking() {
                Ok(Some(tmp)) => {
                    // remove line end, then leading/trailing spaces
                    let t = tmp.trim_end_matches(['\r', '\n', ' ']).trim();
                    if !t.is_empty() {
                        rv = Some(t.to_string());
                    }
                    self.sh.ch340_goodread.fetch_add(1, Ordering::SeqCst);
                }
                Ok(None) => {}
                Err(e) if matches!(e.kind(), io::ErrorKind::NotConnected | io::ErrorKind::BrokenPipe | io::ErrorKind::UnexpectedEof | io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted) => {
                    // non-IOException failure in C#: close the connection
                    let _ = self.close_com(false);
                }
                Err(_) => {
                    let n = self.sh.ch340_exception.fetch_add(1, Ordering::SeqCst) + 1;
                    if !self.sh.fix_ch340.load(Ordering::SeqCst) && n > 5 {
                        // self fix for CH340: HasIncomingData reduces the number of exceptions
                        self.sh.fix_ch340.store(true, Ordering::SeqCst);
                        logger::log_message("FixCH340", "Detected CH340 driver: ");
                    }
                    if n.is_multiple_of(10) {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                }
            }
            if rv.is_some() || !com.is_open() || self.sh.rx_h.must_exit() {
                break;
            }
        }
        rv
    }

    /// `HasIncomingData()`.
    fn has_incoming_data(&self) -> bool {
        self.com().has_data()
    }

    // ------------------------------------------------------------------ connection

    /// `Configure(wraptype, params)`: switch link type when different, then configure it.
    ///
    /// `wraptype`: link type. `conf`: its parameters.
    pub fn configure(&self, wraptype: WrapperType, conf: &ComConfig) {
        let cur = self.com();
        if cur.wrapper_type() != wraptype {
            *self.sh.com.write().unwrap_or_else(|e| e.into_inner()) = create_wrapper(wraptype);
        }
        self.com().configure(conf);
    }

    /// Type of the current link.
    pub fn wrapper_type(&self) -> WrapperType {
        self.com().wrapper_type()
    }

    /// `OpenCom()`. Returns Err(message) for the connect error box (state is cleaned up first).
    pub fn open_com(&self) -> Result<(), String> {
        self.sh.fix_ch340.store(false, Ordering::SeqCst);
        self.sh.ch340_exception.store(0, Ordering::SeqCst);
        self.sh.ch340_goodread.store(0, Ordering::SeqCst);
        {
            let mut st = self.s();
            // reset to default buffer size
            st.auto_buffer_size = state::DEFAULT_BUFFER_SIZE;
            st.set_status(MacStatus::Connecting);
            st.connect_start = st.now_ms();
        }
        let com = self.com();
        let r = (|| -> io::Result<()> {
            if !com.is_open() {
                com.open()?;
            }
            let _g = self.s();
            guard(&self.sh.rx).start();
            guard(&self.sh.tx).start();
            Ok(())
        })();
        if let Err(e) = r {
            logger::log_message("OpenCom", &format!("Error: {e}"));
            self.s().set_status(MacStatus::Disconnected);
            com.close(true);
            return Err(e.to_string());
        }
        Ok(())
    }

    /// `CloseCom(user)`.
    ///
    /// `user`: closed by the user (true) or by the core after a failure (false).
    pub fn close_com(&self, user: bool) -> Result<(), String> {
        {
            let mut st = self.s();
            let (issue, inprog) = {
                let tp = self.sh.tp();
                (tp.last_issue(), tp.in_program())
            };
            if issue == DetectedIssue::Unknown && st.machine_status == MacStatus::Run && inprog {
                st.set_issue(if user { DetectedIssue::ManualDisconnect } else { DetectedIssue::UnexpectedDisconnect });
            }
        }
        let com = self.com();
        if com.is_open() {
            com.close(!user);
        }
        self.s().used_buffer = 0;
        self.sh.tp().job_end(true);

        // threads are stopped without the state lock (their loops take it)
        guard(&self.sh.tx).stop();
        guard(&self.sh.rx).stop();

        let mut st = self.s();
        // do not reset the sent list so the log is not blanked
        st.clear_queue(false);
        st.set_status(MacStatus::Disconnected);
        Ok(())
    }

    /// `Exiting()`: close, stop threads, save laser life.
    pub fn exiting(&self) {
        let _ = self.close_com(true);
        guard(&self.sh.rx).stop();
        guard(&self.sh.tx).stop();
        crate::laser_life::save_now();
    }

    /// `HKConnect`.
    pub fn hk_connect(&self) -> Result<(), String> {
        if !self.is_connected() { self.open_com() } else { Ok(()) }
    }

    /// True when `HKDisconnect` must ask the user first (`InProgram`).
    pub fn hk_disconnect_needs_confirmation(&self) -> bool {
        self.in_program()
    }

    /// `HKDisconnect` after the optional confirmation.
    pub fn hk_disconnect(&self) {
        if self.is_connected() {
            let _ = self.close_com(true);
        }
    }

    /// `IsConnected`.
    pub fn is_connected(&self) -> bool {
        self.s().is_connected()
    }

    // ------------------------------------------------------------------ immediate commands

    /// `SendImmediate(b, mute)`: write one real-time byte.
    pub fn send_immediate(&self, b: u8, mute: bool) {
        self.s().send_immediate(b, mute);
    }

    /// `CycleStartResume(auto)`.
    pub fn cycle_start_resume(&self, auto: bool) {
        self.s().cycle_start_resume(auto);
    }

    /// `FeedHold(cooling)`.
    pub fn feed_hold(&self, cooling: bool) {
        self.s().feed_hold(cooling);
    }

    /// `SafetyDoor()`.
    pub fn safety_door(&self) {
        self.s().send_immediate(64, false);
    }

    /// `GrblReset()` (manual reset button).
    pub fn grbl_reset(&self) {
        let mut st = self.s();
        if st.can_reset_grbl() {
            let (issue, inprog) = {
                let tp = self.sh.tp();
                (tp.last_issue(), tp.in_program())
            };
            if issue == DetectedIssue::Unknown && st.machine_status == MacStatus::Run && inprog {
                st.set_issue(DetectedIssue::ManualReset);
            }
            st.internal_reset(true);
        }
    }

    /// `ManageOverrides()`.
    pub fn manage_overrides(&self) {
        self.s().manage_overrides();
    }

    /// `HotKeyOverride(action)`.
    pub fn hot_key_override(&self, action: OverrideAction) {
        self.s().hot_key_override(action);
    }

    /// `UnlockFromBufferStuck(auto)`.
    pub fn unlock_from_buffer_stuck(&self, auto: bool) {
        self.s().unlock_from_buffer_stuck(auto);
    }

    /// `IsBufferStuck()`.
    pub fn is_buffer_stuck(&self) -> bool {
        self.s().is_buffer_stuck()
    }

    /// `EnqueueCommand(cmd)`.
    pub fn enqueue_command(&self, cmd: &GrblCommand) {
        self.s().enqueue_command(cmd);
    }

    /// `SendHomingCommand()` (virtual; enqueues `$H`).
    pub fn send_homing_command(&self) {
        self.s().enqueue_text("$H");
    }

    /// `SendUnlockCommand()` (virtual; Marlin and Smoothie do nothing).
    pub fn send_unlock_command(&self) {
        if !matches!(self.sh.firmware, Firmware::Marlin | Firmware::Smoothie) {
            self.s().enqueue_text("$X");
        }
    }

    /// `GrblHoming()`.
    pub fn grbl_homing(&self) {
        let mut st = self.s();
        if st.is_connected() && matches!(st.machine_status, MacStatus::Idle | MacStatus::Alarm) && st.configuration().homing_enabled() {
            st.enqueue_text("$H");
        }
    }

    /// `GrblUnlock()`.
    pub fn grbl_unlock(&self) {
        if self.can_unlock() {
            self.s().enqueue_text("$X");
        }
    }

    /// `SetNewZero()`.
    pub fn set_new_zero(&self) {
        if self.can_do_zeroing() {
            self.s().enqueue_text("G92 X0 Y0 Z0");
        }
    }

    // ------------------------------------------------------------------ getters

    /// `MachineStatus`.
    pub fn machine_status(&self) -> MacStatus {
        self.s().machine_status
    }
    /// `InProgram`.
    pub fn in_program(&self) -> bool {
        self.sh.tp().in_program()
    }
    /// `ProgramTarget`.
    pub fn program_target(&self) -> i32 {
        self.sh.tp().target()
    }
    /// `ProgramSent`.
    pub fn program_sent(&self) -> i32 {
        self.sh.tp().sent()
    }
    /// `ProgramExecuted`.
    pub fn program_executed(&self) -> i32 {
        self.sh.tp().executed()
    }
    /// `ProgramTime`.
    pub fn program_time(&self) -> TimeSpan {
        self.sh.tp().total_job_time()
    }
    /// `ProgramGlobalTime`.
    pub fn program_global_time(&self) -> TimeSpan {
        self.sh.tp().total_global_job_time()
    }
    /// `ProjectedTime`.
    pub fn projected_time(&self) -> TimeSpan {
        self.sh.tp().projected_target()
    }
    /// `MachinePosition`.
    pub fn machine_position(&self) -> GPoint {
        self.s().mpos
    }
    /// `WorkPosition` (MPos - WCO).
    pub fn work_position(&self) -> GPoint {
        let st = self.s();
        st.mpos - st.wco
    }
    /// `WorkingOffset`.
    pub fn working_offset(&self) -> GPoint {
        self.s().wco
    }
    /// `Executed`: number of rows in the sent list.
    pub fn executed(&self) -> usize {
        self.s().sent.len()
    }
    /// `SentCommand(index, count)`: rows of the sent list (clones share their result with the queue).
    pub fn sent_command(&self, index: i64, count: i64) -> Vec<SentRow> {
        let st = self.s();
        let len = st.sent.len() as i64;
        let index = index.min(len - 1);
        let count = count.min(len - index);
        if index >= 0 && count > 0 { st.sent[index as usize..(index + count) as usize].to_vec() } else { Vec::new() }
    }
    /// `UsedBuffer`.
    pub fn used_buffer(&self) -> i32 {
        self.s().used_buffer
    }
    /// `FreeBuffer`.
    pub fn free_buffer(&self) -> i32 {
        self.s().free_buffer()
    }
    /// `BufferSize`.
    pub fn buffer_size(&self) -> i32 {
        self.s().buffer_size()
    }
    /// `GrblBlock`.
    pub fn grbl_block(&self) -> i32 {
        self.s().grbl_blocks
    }
    /// `GrblBuffer`.
    pub fn grbl_buffer(&self) -> i32 {
        self.s().grbl_buffer
    }
    /// `OverrideG1`.
    pub fn override_g1(&self) -> i32 {
        self.s().cur_ov_linear
    }
    /// `OverrideG0`.
    pub fn override_g0(&self) -> i32 {
        self.s().cur_ov_rapids
    }
    /// `OverrideS`.
    pub fn override_s(&self) -> i32 {
        self.s().cur_ov_power
    }
    /// `TOverrideG1`.
    pub fn t_override_g1(&self) -> i32 {
        self.s().tar_ov_linear
    }
    /// `TOverrideG1` setter.
    pub fn set_t_override_g1(&self, v: i32) {
        self.s().tar_ov_linear = v;
    }
    /// `TOverrideG0`.
    pub fn t_override_g0(&self) -> i32 {
        self.s().tar_ov_rapids
    }
    /// `TOverrideG0` setter.
    pub fn set_t_override_g0(&self, v: i32) {
        self.s().tar_ov_rapids = v;
    }
    /// `TOverrideS`.
    pub fn t_override_s(&self) -> i32 {
        self.s().tar_ov_power
    }
    /// `TOverrideS` setter.
    pub fn set_t_override_s(&self, v: i32) {
        self.s().tar_ov_power = v;
    }
    /// `CurrentF`.
    pub fn current_f(&self) -> f32 {
        self.s().cur_f
    }
    /// `CurrentS`.
    pub fn current_s(&self) -> f32 {
        self.s().cur_s
    }
    /// `LoopCount`.
    pub fn loop_count(&self) -> Decimal {
        self.s().loop_count
    }
    /// `LoopCount` setter (raises `LoopCountChanged`).
    pub fn set_loop_count(&self, v: Decimal) {
        self.s().set_loop_count(v);
    }
    /// `DetectedIP`.
    pub fn detected_ip(&self) -> Option<String> {
        self.s().detected_ip.clone()
    }
    /// `FailedConnectionCount`.
    pub fn failed_connection_count(&self) -> i32 {
        self.s().failed_connection
    }
    /// `GrblVersion` (None until a welcome message was seen once).
    pub fn grbl_version(&self) -> Option<GrblVersionInfo> {
        self.s().grbl_version.clone()
    }
    /// `GrblVersion` setter (loads the matching CSV dictionaries).
    pub fn set_grbl_version(&self, v: GrblVersionInfo) {
        self.s().set_grbl_version(v);
    }
    /// `IsOrturBoard`.
    pub fn is_ortur_board(&self) -> bool {
        self.s().grbl_version.as_ref().is_some_and(|v| v.is_ortur())
    }
    /// `IsLongerBoard`.
    pub fn is_longer_board(&self) -> bool {
        self.s().grbl_version.as_ref().is_some_and(|v| v.is_longer())
    }
    /// `GrblCore.Configuration` (static getter).
    pub fn configuration() -> GrblConfST {
        configuration::configuration()
    }
    /// `GrblCore.Configuration` (static setter; empty configs are ignored).
    pub fn set_configuration(c: &GrblConfST) {
        configuration::set_configuration(c);
    }
    /// `ValidateConfig(parid, value)`: error text or None.
    pub fn validate_config(&self, parid: i32) -> Option<&'static str> {
        configuration::configuration().validate_config(parid)
    }
    /// `UsageCounters` copy.
    pub fn usage_counters(&self) -> UsageCounters {
        self.s().usage_counters
    }
    /// Modify `UsageCounters` (e.g. `|c| c.dithering += 1`).
    pub fn update_usage_counters(&self, f: impl FnOnce(&mut UsageCounters)) {
        f(&mut self.s().usage_counters);
    }
    /// `JogSpeed`.
    pub fn jog_speed(&self) -> i32 {
        self.s().jog_speed
    }
    /// `JogSpeed` setter.
    pub fn set_jog_speed(&self, v: i32) {
        self.s().jog_speed = v;
    }
    /// `JogStep`.
    pub fn jog_step(&self) -> Decimal {
        self.s().jog_step
    }
    /// `JogStep` setter.
    pub fn set_jog_step(&self, v: Decimal) {
        self.s().jog_step = v;
    }
    /// `SuspendHK`.
    pub fn suspend_hk(&self) -> bool {
        self.s().suspend_hk
    }
    /// `SuspendHK` setter.
    pub fn set_suspend_hk(&self, v: bool) {
        self.s().suspend_hk = v;
    }
    /// `ContinuosJogEnabled`.
    pub fn continuous_jog_enabled(&self) -> bool {
        lg_base::settings::get("Enable Continuous Jog", false)
    }
    /// `CurrentStreamingMode`.
    pub fn current_streaming_mode(&self) -> StreamingMode {
        self.s().current_streaming_mode()
    }
    /// `UIShowGrblConfig`.
    pub fn ui_show_grbl_config(&self) -> bool {
        !matches!(self.sh.firmware, Firmware::Marlin | Firmware::Smoothie)
    }
    /// `UIShowUnlockButtons`.
    pub fn ui_show_unlock_buttons(&self) -> bool {
        self.ui_show_grbl_config()
    }
    /// `QueueEmpty`.
    pub fn queue_empty(&self) -> bool {
        self.s().queue_empty()
    }

    // ------------------------------------------------------------------ Support*/Can*

    /// `SupportRTO`.
    pub fn support_rto(&self) -> bool {
        self.s().version_at_least_11()
    }
    /// `SupportTrueJogging` (virtual).
    pub fn support_true_jogging(&self) -> bool {
        self.s().support_true_jogging()
    }
    /// `SupportCSV`.
    pub fn support_csv(&self) -> bool {
        self.s().support_csv()
    }
    /// `SupportOverride`.
    pub fn support_override(&self) -> bool {
        self.s().version_at_least_11()
    }
    /// `SupportLaserMode`.
    pub fn support_laser_mode(&self) -> bool {
        self.s().support_laser_mode()
    }
    /// `SupportAutoCooling`.
    pub fn support_auto_cooling(&self) -> bool {
        self.s().support_auto_cooling()
    }
    /// `AutoCooling`.
    pub fn auto_cooling(&self) -> bool {
        self.s().auto_cooling()
    }
    /// `AutoCoolingOn`.
    pub fn auto_cooling_on(&self) -> TimeSpan {
        get_timespan_setting("AutoCooling TOn", TimeSpan::from_minutes(10.0))
    }
    /// `AutoCoolingOff`.
    pub fn auto_cooling_off(&self) -> TimeSpan {
        get_timespan_setting("AutoCooling TOff", TimeSpan::from_minutes(1.0))
    }
    /// `HasProgram`.
    pub fn has_program(&self) -> bool {
        self.s().has_program()
    }
    /// `CanReOpenFile`.
    pub fn can_reopen_file(&self) -> bool {
        let last: Option<String> = lg_base::settings::get("Core.LastOpenFile", None);
        self.can_load_new_file() && last.is_some_and(|f| Path::new(&f).exists())
    }
    /// `CanLoadNewFile`.
    pub fn can_load_new_file(&self) -> bool {
        !self.in_program() && !self.sh.file.try_read().map(|f| f.check_in_use()).unwrap_or(true)
    }
    /// `CanSendFile`.
    pub fn can_send_file(&self) -> bool {
        self.s().can_send_file()
    }
    /// `CanAbortProgram`.
    pub fn can_abort_program(&self) -> bool {
        self.s().can_abort_program()
    }
    /// `CanImportExport`.
    pub fn can_import_export(&self) -> bool {
        let st = self.s();
        st.is_connected() && st.machine_status == MacStatus::Idle
    }
    /// `CanResetGrbl`.
    pub fn can_reset_grbl(&self) -> bool {
        self.s().can_reset_grbl()
    }
    /// `CanSendManualCommand`.
    pub fn can_send_manual_command(&self) -> bool {
        let st = self.s();
        st.is_connected() && st.machine_status != MacStatus::Disconnected && !st.in_program()
    }
    /// `CanDoHoming`.
    pub fn can_do_homing(&self) -> bool {
        let st = self.s();
        st.is_connected() && matches!(st.machine_status, MacStatus::Idle | MacStatus::Alarm) && st.configuration().homing_enabled()
    }
    /// `CanDoZeroing`.
    pub fn can_do_zeroing(&self) -> bool {
        let st = self.s();
        st.is_connected() && st.machine_status == MacStatus::Idle && (st.mpos - st.wco) != GPoint::ZERO
    }
    /// `CanUnlock`.
    pub fn can_unlock(&self) -> bool {
        let st = self.s();
        st.is_connected() && matches!(st.machine_status, MacStatus::Idle | MacStatus::Alarm)
    }
    /// `CanFeedHold`.
    pub fn can_feed_hold(&self) -> bool {
        self.s().can_feed_hold()
    }
    /// `CanResumeHold`.
    pub fn can_resume_hold(&self) -> bool {
        self.s().can_resume_hold()
    }
    /// `CanReadWriteConfig`.
    pub fn can_read_write_config(&self) -> bool {
        self.s().can_read_write_config()
    }
    /// `JogEnabled`.
    pub fn jog_enabled(&self) -> bool {
        self.s().jog_enabled()
    }

    // ------------------------------------------------------------------ jogging

    /// `LimitToBound(target)`: crop to the machine area when soft limits are on.
    pub fn limit_to_bound(&self, target: PointF) -> PointF {
        let conf = configuration::configuration();
        if conf.soft_limit() {
            let wco = self.s().wco;
            let w = decimal_to_f32(conf.table_width());
            let h = decimal_to_f32(conf.table_height());
            PointF::new((target.x.max(-wco.x)).min(w - wco.x), (target.y.max(-wco.y)).min(h - wco.y))
        } else {
            target
        }
    }

    /// `JogToPosition(target, fast)` (double click).
    pub fn jog_to_position_fast(&self, target: PointF, fast: bool) {
        let speed = if fast { 100000.0 } else { self.jog_speed() as f32 };
        self.jog_to_position(target, speed);
    }

    /// `JogToPosition(target, speed)`.
    pub fn jog_to_position(&self, target: PointF, speed: f32) {
        let target = self.limit_to_bound(target);
        if !self.jog_enabled() {
            return;
        }
        let mut st = self.s();
        if !st.support_true_jogging() {
            for l in jog::jog_v09_position(target, speed) {
                st.enqueue_text(&l);
            }
        } else if !self.continuous_jog_enabled() {
            st.enqueue_text(&jog::jog_v11_position(target, speed));
        } else {
            st.continuous_jog_request(ContinuousJog { direction: JogDirection::Position, speed, target });
        }
    }

    /// `ContinuousJogToPosition(target, speed)`.
    pub fn continuous_jog_to_position(&self, target: PointF, speed: f32) {
        let target = self.limit_to_bound(target);
        if !self.jog_enabled() || !self.support_true_jogging() {
            return;
        }
        self.s().continuous_jog_request(ContinuousJog { direction: JogDirection::Position, speed, target });
    }

    /// `JogToDirection(dir, fast)` with the configured step.
    pub fn jog_to_direction_fast(&self, dir: JogDirection, fast: bool) -> Result<(), String> {
        let step = self.jog_step();
        self.jog_to_direction_step_fast(dir, fast, step)
    }

    /// `JogToDirection(dir, fast, step)`.
    pub fn jog_to_direction_step_fast(&self, dir: JogDirection, fast: bool, step: Decimal) -> Result<(), String> {
        let speed = if fast { 100000.0 } else { self.jog_speed() as f32 };
        self.jog_to_direction(dir, speed, step)
    }

    /// `JogToDirection(dir, speed, step)`. Err for Abort/Position (C# `ArgumentException`).
    pub fn jog_to_direction(&self, dir: JogDirection, speed: f32, step: Decimal) -> Result<(), String> {
        if matches!(dir, JogDirection::Abort | JogDirection::Position) {
            return Err("Invalid option".into());
        }
        if !self.jog_enabled() {
            return Ok(());
        }
        let continuous = self.continuous_jog_enabled();
        let mut st = self.s();
        if !st.support_true_jogging() {
            for l in jog::jog_v09_direction(dir, step, speed) {
                st.enqueue_text(&l);
            }
        } else if !continuous || dir == JogDirection::Zdown || dir == JogDirection::Zup {
            st.enqueue_text(&jog::jog_v11_direction(dir, step, speed));
        } else {
            st.continuous_jog_request(ContinuousJog { direction: dir, speed, target: PointF::default() });
        }
        Ok(())
    }

    /// `ContinuousJogToDirection(dir, speed)`.
    pub fn continuous_jog_to_direction(&self, dir: JogDirection, speed: f32) -> Result<(), String> {
        if matches!(dir, JogDirection::Abort | JogDirection::Position) {
            return Err("Invalid option".into());
        }
        if !self.jog_enabled() || !self.support_true_jogging() {
            return Ok(());
        }
        guard(&self.sh.jog).to_direction(dir, speed)
    }

    /// `JogAbort()` (button up).
    pub fn jog_abort(&self) {
        if self.support_true_jogging() && self.continuous_jog_enabled() {
            guard(&self.sh.jog).abort();
        }
    }

    /// `ContinuousJogAbort()`.
    pub fn continuous_jog_abort(&self) {
        if self.support_true_jogging() {
            guard(&self.sh.jog).abort();
        }
    }

    /// `RiseJogStateChange(jog)`.
    pub fn rise_jog_state_change(&self, jog: bool) {
        let _ = self.sh.events.send(CoreEvent::JogStateChanged(jog));
    }

    // ------------------------------------------------------------------ files and programs

    /// `LoadedFile`.
    pub fn loaded_file(&self) -> Arc<RwLock<GrblFile>> {
        self.sh.file.clone()
    }

    /// `OpenFile(filename, append)` dispatch; dialogs are UI work.
    ///
    /// `filename`: the path (the UI runs the file dialog when the user did not pick one). `append`: keep the
    /// old program. Sets `Core.LastOpenFile`, counts usage; G-code starts loading on a worker thread.
    pub fn open_file(&self, filename: &Path, append: bool) -> OpenFileOutcome {
        if !self.can_load_new_file() {
            return OpenFileOutcome::Busy;
        }
        logger::log_message("OpenFile", &format!("Open {}", filename.display()));
        let name = filename.to_string_lossy().to_string();
        lg_base::settings::set("Core.LastOpenFile", Some(name.clone()));
        let ext = filename.extension().map(|e| format!(".{}", e.to_string_lossy().to_lowercase())).unwrap_or_default();

        if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            self.update_usage_counters(|c| c.raster_file += 1);
            OpenFileOutcome::Raster(filename.to_path_buf())
        } else if ext == ".svg" {
            self.update_usage_counters(|c| c.svg_file += 1);
            OpenFileOutcome::Svg(filename.to_path_buf())
        } else if GCODE_EXTENSIONS.contains(&ext.as_str()) {
            let n = name.clone();
            match GrblFile::spawn_load(&self.sh.file, move |f| f.load_file(&n, append)) {
                Some(_) => {
                    self.update_usage_counters(|c| c.gcode_file += 1);
                    OpenFileOutcome::GCodeLoading
                }
                None => OpenFileOutcome::Busy,
            }
        } else if PROJECT_FILE_EXTENSIONS.contains(&ext.as_str()) {
            OpenFileOutcome::Project(filename.to_path_buf())
        } else {
            OpenFileOutcome::Unsupported
        }
    }

    /// `ReOpenFile()`.
    pub fn reopen_file(&self) -> Option<OpenFileOutcome> {
        if !self.can_reopen_file() {
            return None;
        }
        let last: Option<String> = lg_base::settings::get("Core.LastOpenFile", None);
        last.map(|f| self.open_file(&PathBuf::from(f), false))
    }

    /// `SaveProgram(...)` after the UI picked the target.
    ///
    /// `path`: output. `header`/`footer`/`between`: custom code blocks. `cycles`: repetitions.
    /// `lf`: force `\n` line ends. Does nothing without a program.
    pub fn save_program(&self, path: &Path, header: bool, footer: bool, between: bool, cycles: i32, lf: bool) -> io::Result<()> {
        if !self.has_program() {
            return Ok(());
        }
        // snapshot first: no core lock while the file lock is held
        let mut vars = self.s().expr_vars();
        let file = self.sh.file.read().unwrap_or_else(|e| e.into_inner());
        vars.bounds = FileBounds::from_file(&file);
        file.save_gcode(path, header, footer, between, cycles, lf, &|s| expr::evaluate_expression(&vars, s))
    }

    /// `EvaluateExpression(str)`.
    pub fn evaluate_expression(&self, s: &str) -> String {
        let vars = self.s().expr_vars();
        expr::evaluate_expression(&vars, s)
    }

    /// `ExecuteCustomCode(code)`: evaluate each line, send immediates, enqueue the rest.
    pub fn execute_custom_code(&self, code: &str) {
        self.s().execute_custom_code(code);
    }

    /// True when `run_program` would ask to resume (an earlier job was interrupted).
    pub fn run_would_ask_resume(&self) -> bool {
        let tp = self.sh.tp();
        !(tp.executed() == 0 || tp.executed() == tp.target())
    }

    /// `RunProgram(parent)`. The safety countdown is UI-side and must have passed.
    pub fn run_program(&self) -> RunDecision {
        let mut st = self.s();
        if !st.can_send_file() {
            return RunDecision::NotAllowed;
        }
        st.doing_send = true;
        let (executed, target) = {
            let tp = self.sh.tp();
            (tp.executed(), tp.target())
        };
        let r = if executed == 0 || executed == target {
            // never started or correctly finished
            st.run_program_from_start_inner(false, true, false);
            RunDecision::Started
        } else {
            // UserWantToContinue arguments
            let (last_issue, last_wco, sent) = {
                let tp = self.sh.tp();
                (tp.last_issue(), tp.last_known_wco(), tp.sent())
            };
            let suggest_setwco = st.wco == GPoint::ZERO && last_wco != GPoint::ZERO;
            let suggest_homing = st.mpos == GPoint::ZERO && last_issue != DetectedIssue::ManualAbort && last_issue != DetectedIssue::ManualReset;
            RunDecision::AskResume(ResumeInfo {
                executed,
                sent,
                target,
                last_issue,
                homing_enabled: st.configuration().homing_enabled(),
                suggest_homing,
                suggest_setwco,
                last_known_wco: last_wco,
            })
        };
        st.doing_send = false;
        r
    }

    /// Resume dialog choice "from start": `RunProgramFromStart(homing)`.
    pub fn run_program_from_start_user(&self, homing: bool) {
        self.s().run_program_from_start_inner(homing, false, false);
    }

    /// `RunProgramFromPosition` first half: data for the "run from position" dialog (None if not allowed).
    pub fn run_from_position_info(&self) -> Option<RunFromPositionInfo> {
        let st = self.s();
        if !st.can_send_file() {
            return None;
        }
        let line_count = self.sh.file.try_read().map(|f| f.count()).unwrap_or(0);
        Some(RunFromPositionInfo { line_count, homing_enabled: st.configuration().homing_enabled() })
    }

    /// `ContinueProgramFromKnown(position, homing, setwco)`.
    ///
    /// `position`: first line (0-based) to run. `homing`: push `$H`. `setwco`: restore the work offset.
    pub fn continue_program(&self, position: i32, homing: bool, setwco: bool) -> Result<(), CoreError> {
        self.s().continue_program_from_known(position, homing, setwco).map_err(CoreError::Other)
    }

    /// `AbortProgram()`.
    pub fn abort_program(&self) {
        let mut st = self.s();
        st.abort_program();
    }

    /// `AutoSizeDrawing()`.
    pub fn auto_size_drawing(&self) {
        let _ = self.sh.events.send(CoreEvent::AutoSizeDrawing);
    }
    /// `ZoomInDrawing()`.
    pub fn zoom_in_drawing(&self) {
        let _ = self.sh.events.send(CoreEvent::ZoomInDrawing);
    }
    /// `ZoomOutDrawing()`.
    pub fn zoom_out_drawing(&self) {
        let _ = self.sh.events.send(CoreEvent::ZoomOutDrawing);
    }

    // ------------------------------------------------------------------ configuration

    /// `RefreshMachineInfo()` (virtual: Marlin does nothing).
    pub fn refresh_machine_info(&self) -> Result<(), CoreError> {
        if self.sh.firmware == Firmware::Marlin || !lg_base::settings::get("Query MachineInfo ($I) at connect", true) {
            return Ok(());
        }
        let cmd = GrblCommand::new("$I");
        {
            let mut st = self.s();
            st.begin_alt();
            st.queue_ptr().push_back(cmd.clone());
        }
        let r = (|| -> Result<(), CoreError> {
            self.wait_command(&cmd)?;
            if cmd.status() == lg_gcode::CommandStatus::ResponseGood {
                // wait for all parameters: +1 is the $I command
                self.wait_rows(2 + 1);
                let rows: Vec<String> = self.s().sent_ptr().iter().map(|r| r.get_decoded_message()).collect();
                for rline in rows {
                    if rline.starts_with("[OPT:") && rline.ends_with(']') {
                        self.manage_opt_message(&rline);
                    }
                    // [VER:..] is only logged by C# (no action)
                }
            }
            Ok(())
        })();
        self.s().end_alt();
        r
    }

    /// `ManageOptMessage`.
    fn manage_opt_message(&self, rline: &str) {
        let body = crate::util::substring(rline, 5, rline.chars().count().saturating_sub(6));
        let txbuffer = body.as_deref().map(|b| b.split(',').map(str::to_string).collect::<Vec<_>>()).and_then(|a| a.get(2).and_then(|t| lg_dotnet::parse_int(t)));
        match txbuffer {
            // more than default buffer size
            Some(tx) if tx > state::DEFAULT_BUFFER_SIZE => self.s().enlarge_buffer(tx, true),
            Some(_) => {}
            None => logger::log_message("OptionsMessage", &format!("Ex on [{rline}] message")),
        }
    }

    /// Wait until `cmd` was sent and answered; Err(Timeout) after 10 s.
    fn wait_command(&self, cmd: &GrblCommand) -> Result<(), CoreError> {
        use lg_gcode::CommandStatus::*;
        let mut timeout = PeriodicEventTimer::new(TimeSpan::from_seconds(10.0), true);
        while matches!(cmd.status(), Queued | WaitingResponse) {
            if timeout.expired() {
                return Err(CoreError::Timeout("No response received from grbl!".into()));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }

    /// Wait for `target` rows in the active sent list; stop when 500 ms silent or 5 s total.
    fn wait_rows(&self, target: usize) {
        let t_start = hires_timer::total_milliseconds();
        let mut t_last = t_start;
        let mut counter = self.s().sent_ptr().len();
        loop {
            let len = self.s().sent_ptr().len();
            let now = hires_timer::total_milliseconds();
            if !(len < target && now - t_last < 500 && now - t_start < 5000) {
                break;
            }
            if len != counter {
                t_last = hires_timer::total_milliseconds();
                counter = len;
            } else {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    /// `RefreshConfig(cause)` (virtual: Marlin does nothing). Reads `$$` into the stored configuration.
    pub fn refresh_config(&self, cause: RefreshCause) -> Result<(), CoreError> {
        if self.sh.firmware == Firmware::Marlin {
            return Ok(());
        }
        if !self.can_read_write_config() {
            let st = self.s();
            logger::log_message(
                "Refresh Config",
                &format!("Cannot refresh config now [{}] [Connected:{} Program:{} Status:{}]", cause, st.is_connected(), self.in_program(), st.machine_status),
            );
            return Ok(());
        }
        logger::log_message("Refresh Config", &format!("Refreshing grbl configuration [{cause}]"));
        let mut conf = GrblConfST::with_version(self.grbl_version());
        let cmd = GrblCommand::new("$$");
        {
            let mut st = self.s();
            st.begin_alt();
            st.queue_ptr().push_back(cmd.clone());
        }
        let r = (|| -> Result<(), CoreError> {
            self.wait_command(&cmd)?;
            if cmd.status() == lg_gcode::CommandStatus::ResponseGood {
                self.wait_rows(conf.expected_count() as usize + 1);
                let rows: Vec<String> = self
                    .s()
                    .sent_ptr()
                    .iter()
                    .filter_map(|r| if let SentRow::Message(m) = r { Some(m.get_native_message().to_string()) } else { None })
                    .collect();
                for m in rows {
                    conf.add_or_update(&m);
                }
                // accept configuration
                configuration::set_configuration(&conf);
                let c = configuration::configuration();
                if c.count() < c.expected_count() as usize {
                    logger::log_message("Refresh Config", &format!("Wrong number of config param found! (Expected: {} Found: {})", c.expected_count(), c.count()));
                } else {
                    logger::log_message("Refresh Config", &format!("Configuration successfully received! (Expected: {} Found: {})", c.expected_count(), c.count()));
                }
            }
            Ok(())
        })();
        if let Err(e) = &r {
            logger::log_message("Refresh Config", &e.to_string());
        }
        self.s().end_alt();
        r
    }

    /// Wait (polling) until the active queue is empty and nothing is pending, or the link closed.
    fn wait_queue_drained(&self) {
        loop {
            {
                let mut st = self.s();
                if !(self.com().is_open() && (!st.queue_ptr().is_empty() || st.has_pending_commands())) {
                    break;
                }
            }
            // C# spins; sleep 1 ms instead
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// `WriteConfig(config)`: send `$n=v` lines and check every answer.
    pub fn write_config(&self, config: &[GrblConfParam]) -> Result<(), CoreError> {
        if !self.can_read_write_config() {
            return Ok(());
        }
        {
            let mut st = self.s();
            st.begin_alt();
            for p in config {
                st.queue_ptr().push_back(GrblCommand::with_options(&format!("${}={}", p.number, p.value), 0, true));
            }
        }
        self.wait_queue_drained();
        let errors: Vec<SentRow> = self
            .s()
            .sent_ptr()
            .iter()
            .filter(|r| matches!(r, SentRow::Command(c) if c.status() != lg_gcode::CommandStatus::ResponseGood))
            .cloned()
            .collect();
        let r = if errors.is_empty() {
            Ok(())
        } else {
            let mut msg = String::new();
            for r in &errors {
                msg += &format!("{} {}\n", r.get_decoded_message(), r.get_result(true, false).unwrap_or_default());
            }
            let e = CoreError::WriteConfig { message: msg.trim().to_string(), errors: errors.iter().map(|r| r.get_decoded_message()).collect() };
            logger::log_message("Write Config", &e.to_string());
            Err(e)
        };
        self.s().end_alt();
        r
    }

    /// `WriteWiFiConfig(ssid, password)` for Ortur and Longer boards.
    pub fn write_wifi_config(&self, ssid: &str, password: &str) {
        let (ortur, longer) = {
            let mut st = self.s();
            st.detected_ip = None;
            (st.grbl_version.as_ref().is_some_and(|v| v.is_ortur()), st.grbl_version.as_ref().is_some_and(|v| v.is_longer()))
        };
        if ortur {
            if self.can_read_write_config() {
                {
                    let mut st = self.s();
                    st.queue_ptr().push_back(GrblCommand::with_options(&format!("${}={}", 74, ssid), 0, true));
                    st.queue_ptr().push_back(GrblCommand::with_options(&format!("${}={}", 75, password), 0, true));
                    st.queue_ptr().push_back(GrblCommand::new("$WRS"));
                }
                self.wait_queue_drained();
            }
        } else if longer {
            let mut st = self.s();
            st.queue_ptr().push_back(GrblCommand::with_options("$radio/mode=sta", 0, true));
            st.queue_ptr().push_back(GrblCommand::with_options(&format!("$sta/ssid={ssid}"), 0, true));
            st.queue_ptr().push_back(GrblCommand::with_options(&format!("$sta/password={password}"), 0, true));
            st.queue_ptr().push_back(GrblCommand::with_options("$wifi/begin", 0, true));
        }
    }

    /// Append a message row to the sent list (UI/diagnostics).
    pub fn add_message(&self, m: GrblMessage) {
        self.s().push_message(m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_and_defaults() {
        // no real config dir
        crate::test_support::init_env();
        let (tx, rx) = crossbeam_channel::unbounded();
        let core = GrblCore::new(Firmware::Grbl, tx);
        assert_eq!(core.machine_status(), MacStatus::Disconnected);
        assert!(!core.is_connected());
        assert_eq!(core.buffer_size(), 127);
        assert_eq!(core.loop_count(), Decimal::ONE);
        assert!(core.ui_show_grbl_config());
        assert_eq!(core.wrapper_type(), WrapperType::UsbSerial);
        core.configure(WrapperType::Telnet, &ComConfig::Address("x".into()));
        assert_eq!(core.wrapper_type(), WrapperType::Telnet);
        assert!(rx.try_recv().is_err());
        let m = GrblCore::new(Firmware::Marlin, crossbeam_channel::unbounded().0);
        assert!(!m.ui_show_grbl_config());
        assert!(!m.support_true_jogging());
        assert_eq!(m.current_streaming_mode(), StreamingMode::Synchronous);
        let v = GrblCore::new(Firmware::VigoWork, crossbeam_channel::unbounded().0);
        assert_eq!(v.buffer_size(), 127);
    }
}
