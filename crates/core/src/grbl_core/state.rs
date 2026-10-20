//! Mutable state of `GrblCore` (everything the C# code guards with `lock(this)`) and the logic that runs
//! under that lock.
//!
//! Locking rule: the C# `lock` is re-entrant, a Rust `Mutex` is not. Therefore every method here takes
//! `&mut self` (= the already locked state) and never locks `GrblCore::st` again; the public `GrblCore`
//! methods lock once and call into these. Lock order: state -> file (read, never nested) -> time projection ->
//! settings/laser life. The loader thread only takes file(write) -> time projection, so it never waits for
//! the state lock. Nothing here sleeps or waits for the com port while holding the lock (waits live in
//! `GrblCore`, they re-lock between polls).

use super::expr::{self, ExprVars, FileBounds};
use super::jog::{self, ContinuousJog};
use super::types::*;
use super::Shared;
use crate::com::ComWrapper;
use crate::usage_stats::UsageCounters;
use crate::util::{get_timespan_setting, substring, substring_from};
use crate::laser_life;
use lg_base::logger;
use lg_base::logger::com as comlog;
use lg_base::sound::{play_sound, EventId};
use lg_base::tools::cronometro::PeriodicEventTimer;
use lg_base::tools::utils::{time_span_to_string, TimePrecision};
use lg_dotnet::{fmt_float, parse_float, parse_int, Decimal, TimeSpan};
use lg_gcode::{configuration, csvd, Element, Firmware, GPoint, GrblCommand, GrblMessage, GrblVersionInfo, MessageType, StatePositionBuilder};
use regex::Regex;
use std::collections::VecDeque;
use std::sync::{Arc, LazyLock};

/// C# `DEFAULT_BUFFER_SIZE`.
pub(crate) const DEFAULT_BUFFER_SIZE: i32 = 127;

/// `unknownWelcomeRegex`.
static UNKNOWN_WELCOME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?P<fw>[a-zA-Z]+)\s+(?P<maj>\d+)\.(?P<min>\d+)(?P<build>\D)($|\s)").expect("static regex"));

/// Result type of the parsers (Err = the .NET exception text, only logged).
type PResult<T> = Result<T, String>;

/// Alternate queue/sent pair used while config is read/written (C# `mQueuePtr` / `mSentPtr` swap).
#[derive(Default)]
pub(crate) struct Alt {
    pub queue: VecDeque<GrblCommand>,
    pub sent: Vec<SentRow>,
}

/// Vigo specific fields (C# `VigoCore`).
#[derive(Default)]
pub(crate) struct VigoState {
    old_sta: i32,
    injob: bool,
    old_received: i32,
    old_managed: i32,
    old_error: i32,
}

pub(crate) struct CoreState {
    pub sh: Arc<Shared>,
    pub queue: VecDeque<GrblCommand>,
    pub pending: VecDeque<GrblCommand>,
    pub retry: Option<GrblCommand>,
    pub sent: Vec<SentRow>,
    pub alt: Option<Alt>,
    pub welcome_seen: Option<String>,
    pub vendor_info_seen: Option<String>,
    pub vendor_version_seen: Option<String>,
    pub used_buffer: i32,
    pub auto_buffer_size: i32,
    pub mpos: GPoint,
    pub wco: GPoint,
    pub grbl_blocks: i32,
    pub grbl_buffer: i32,
    pub failed_connection: i32,
    pub machine_status: MacStatus,
    pub cur_f: f32,
    pub cur_s: f32,
    pub cur_ov_linear: i32,
    pub cur_ov_rapids: i32,
    pub cur_ov_power: i32,
    pub tar_ov_linear: i32,
    pub tar_ov_rapids: i32,
    pub tar_ov_power: i32,
    pub loop_count: Decimal,
    pub query_timer: PeriodicEventTimer,
    pub connect_start: i64,
    pub last_status_ms: Option<i64>,
    pub last_move_ms: Option<i64>,
    pub detected_ip: Option<String>,
    pub doing_send: bool,
    pub hold_by_user: bool,
    pub hold_by_cooling: bool,
    pub now_cooling: bool,
    pub jog_speed: i32,
    pub jog_step: Decimal,
    pub suspend_hk: bool,
    pub grbl_version: Option<GrblVersionInfo>,
    pub usage_counters: UsageCounters,
    vigo: VigoState,
    /// Test aid (not in C#): every status change in order, only filled after `GrblCore::enable_status_trace`.
    pub status_trace: Option<Vec<MacStatus>>,
}

/// Format one element for a command (`{0}` of an `Element`).
fn el(e: &Element) -> String {
    e.to_string()
}

impl CoreState {
    /// Fresh state (C# field initialisers + the state part of the ctor).
    ///
    /// `sh`: shared parts. `threading_mode`: mode chosen in the ctor (for the query period).
    pub(crate) fn new(sh: Arc<Shared>, threading_mode: ThreadingMode) -> Self {
        let grbl_version: Option<GrblVersionInfo> = lg_base::settings::get("Last GrblVersion known", None);
        Self {
            sh,
            queue: VecDeque::new(),
            pending: VecDeque::new(),
            retry: None,
            sent: Vec::new(),
            alt: None,
            welcome_seen: None,
            vendor_info_seen: None,
            vendor_version_seen: None,
            used_buffer: 0,
            auto_buffer_size: DEFAULT_BUFFER_SIZE,
            mpos: GPoint::ZERO,
            wco: GPoint::ZERO,
            grbl_blocks: -1,
            grbl_buffer: -1,
            failed_connection: 0,
            machine_status: MacStatus::Disconnected,
            cur_f: 0.0,
            cur_s: 0.0,
            cur_ov_linear: 100,
            cur_ov_rapids: 100,
            cur_ov_power: 100,
            tar_ov_linear: 100,
            tar_ov_rapids: 100,
            tar_ov_power: 100,
            loop_count: Decimal::ONE,
            query_timer: PeriodicEventTimer::new(TimeSpan::from_milliseconds(threading_mode.status_query() as f64), false),
            connect_start: 0,
            last_status_ms: None,
            last_move_ms: None,
            detected_ip: None,
            doing_send: false,
            hold_by_user: false,
            hold_by_cooling: false,
            now_cooling: false,
            jog_speed: 0,
            jog_step: Decimal::ZERO,
            suspend_hk: false,
            grbl_version,
            usage_counters: UsageCounters::default(),
            vigo: VigoState::default(),
            status_trace: None,
        }
    }

    // ---------------------------------------------------------------- plumbing

    pub(crate) fn firmware(&self) -> Firmware {
        self.sh.firmware
    }

    pub(crate) fn com(&self) -> Arc<dyn ComWrapper> {
        self.sh.com.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub(crate) fn emit(&self, e: CoreEvent) {
        let _ = self.sh.events.send(e);
    }

    pub(crate) fn now_ms(&self) -> i64 {
        (self.sh.clock)()
    }

    /// `mQueuePtr`.
    pub(crate) fn queue_ptr(&mut self) -> &mut VecDeque<GrblCommand> {
        match &mut self.alt {
            Some(a) => &mut a.queue,
            None => &mut self.queue,
        }
    }

    /// `mSentPtr`.
    pub(crate) fn sent_ptr(&mut self) -> &mut Vec<SentRow> {
        match &mut self.alt {
            Some(a) => &mut a.sent,
            None => &mut self.sent,
        }
    }

    /// `mSentPtr.Add(new GrblMessage(..))`.
    pub(crate) fn push_message(&mut self, m: GrblMessage) {
        self.sent_ptr().push(SentRow::Message(m));
    }

    /// Install a fresh queue/sent pair (RefreshConfig / RefreshMachineInfo / WriteConfig).
    pub(crate) fn begin_alt(&mut self) {
        self.alt = Some(Alt::default());
    }

    /// Restore `mQueue` / `mSent` as the active pair.
    pub(crate) fn end_alt(&mut self) {
        self.alt = None;
    }

    /// `EnqueueCommand`: a clone goes to the active queue.
    pub(crate) fn enqueue_command(&mut self, cmd: &GrblCommand) {
        self.queue_ptr().push_back(cmd.clone());
    }

    /// Enqueue a command built from text.
    pub(crate) fn enqueue_text(&mut self, line: &str) {
        self.enqueue_command(&GrblCommand::new(line));
    }

    // ---------------------------------------------------------------- properties

    pub(crate) fn is_connected(&self) -> bool {
        self.machine_status != MacStatus::Disconnected && self.machine_status != MacStatus::Connecting
    }

    pub(crate) fn in_program(&self) -> bool {
        self.sh.tp().in_program()
    }

    pub(crate) fn configuration(&self) -> lg_gcode::GrblConfST {
        configuration::configuration()
    }

    /// `GrblVersion >= new GrblVersionInfo(1, 1)`.
    pub(crate) fn version_at_least_11(&self) -> bool {
        self.grbl_version.as_ref().is_some_and(|v| *v >= GrblVersionInfo::new(1, 1))
    }

    pub(crate) fn support_csv(&self) -> bool {
        self.version_at_least_11()
    }

    pub(crate) fn support_laser_mode(&self) -> bool {
        self.version_at_least_11()
    }

    /// `SupportTrueJogging` (virtual).
    pub(crate) fn support_true_jogging(&self) -> bool {
        match self.firmware() {
            Firmware::Marlin | Firmware::Smoothie => false,
            _ => self.version_at_least_11(),
        }
    }

    pub(crate) fn support_auto_cooling(&self) -> bool {
        self.version_at_least_11() && self.configuration().laser_mode()
    }

    pub(crate) fn auto_cooling(&self) -> bool {
        lg_base::settings::get("AutoCooling", false) && self.support_auto_cooling()
    }

    /// `BufferSize` (virtual: Vigo is fixed).
    pub(crate) fn buffer_size(&self) -> i32 {
        if self.firmware() == Firmware::VigoWork { 127 } else { self.auto_buffer_size }
    }

    /// `CurrentStreamingMode` (virtual: Marlin/Smoothie are synchronous).
    pub(crate) fn current_streaming_mode(&self) -> StreamingMode {
        match self.firmware() {
            Firmware::Marlin | Firmware::Smoothie => StreamingMode::Synchronous,
            _ => lg_base::settings::get("Streaming Mode", StreamingMode::Buffered),
        }
    }

    /// `CurrentThreadingMode` (NOTE(upstream): default here is UltraFast, the ctor default is Fast).
    pub(crate) fn current_threading_mode(&self) -> ThreadingMode {
        lg_base::settings::get("Threading Mode", ThreadingMode::UltraFast)
    }

    pub(crate) fn has_pending_commands(&self) -> bool {
        !self.pending.is_empty()
    }

    fn in_pause(&self) -> bool {
        self.machine_status != MacStatus::Run && self.machine_status != MacStatus::Idle
    }

    fn is_any_hold_state(s: MacStatus) -> bool {
        s == MacStatus::Hold || s == MacStatus::Cooling || s == MacStatus::AutoHold
    }

    /// Elapsed ms of a "debug" stopwatch (0 when never started, like `ElapsedFromEvent`).
    fn elapsed(&self, since: Option<i64>) -> i64 {
        since.map_or(0, |s| self.now_ms() - s)
    }

    // ---------------------------------------------------------------- status / issue / version

    /// `SetStatus(newStatus)`.
    pub(crate) fn set_status(&mut self, new_status: MacStatus) {
        laser_life::compute_laser_time(self.machine_status);

        if self.machine_status != new_status {
            let old = self.machine_status;
            self.machine_status = new_status;

            logger::log_message("SetStatus", &format!("Machine status [{}]", self.machine_status));
            if old == MacStatus::Connecting && new_status == MacStatus::Disconnected {
                self.failed_connection += 1;
            }
            if old == MacStatus::Connecting && new_status != MacStatus::Disconnected {
                self.failed_connection = 0;
                self.refresh_config_on_connect();
                play_sound(EventId::Connect);
            }
            if new_status == MacStatus::Disconnected {
                play_sound(EventId::Disconnect);
            }
            // leaving any hold state (reset etc.) drops the user/cooling hold flags
            if Self::is_any_hold_state(old) && !Self::is_any_hold_state(new_status) {
                self.hold_by_cooling = false;
                self.hold_by_user = false;
            }

            if let Some(t) = self.status_trace.as_mut() {
                t.push(new_status);
            }
            self.emit(CoreEvent::MachineStatusChanged);

            let in_pause = self.in_pause();
            let mut tp = self.sh.tp();
            if tp.in_program() {
                if in_pause {
                    tp.job_pause();
                } else {
                    tp.job_resume();
                }
            }
            drop(tp);

            if new_status == MacStatus::Disconnected {
                self.set_fs(0.0, 0.0);
            }
        }
    }

    /// `RefreshConfigOnConnect()`: worker thread, 500 ms later refresh config and machine info.
    fn refresh_config_on_connect(&self) {
        let weak = self.sh.me.clone();
        let _ = std::thread::Builder::new().name("RefreshConfigOnConnect".into()).spawn(move || {
            // allow the machine to send all the startup messages before refreshing config and info
            std::thread::sleep(std::time::Duration::from_millis(500));
            let Some(core) = weak.upgrade() else { return };
            if let Err(e) = core.refresh_config(RefreshCause::OnConnect) {
                logger::log_message("Refresh Config", &e.to_string());
            }
            if let Err(e) = core.refresh_machine_info() {
                logger::log_message("Refresh Config", &e.to_string());
            }
        });
    }

    /// `SetIssue(issue)`.
    pub(crate) fn set_issue(&mut self, issue: DetectedIssue) {
        self.sh.tp().job_issue(issue);
        logger::log_message("Issue detector", &format!("{} [{},{},{}]", issue, self.free_buffer(), self.grbl_buffer, self.grbl_blocks));
        // negative numbers are issues caused by the user: not reported
        if issue.value() > 0 {
            self.emit(CoreEvent::IssueDetected(issue));
            play_sound(EventId::Fatal);
        }
    }

    pub(crate) fn free_buffer(&self) -> i32 {
        self.buffer_size() - self.used_buffer
    }

    /// `GrblVersion` setter.
    pub(crate) fn set_grbl_version(&mut self, value: GrblVersionInfo) {
        if self.grbl_version.is_some() {
            logger::log_message("VersionInfo", &format!("Detected Grbl v{value}"));
        }
        if self.grbl_version.is_none() || self.grbl_version.as_ref() != Some(&value) {
            csvd::load_appropriate(&value);
        }
        lg_base::settings::set("Last GrblVersion known", Some(value.clone()));
        self.grbl_version = Some(value);
    }

    // ---------------------------------------------------------------- queue handling

    /// `ClearQueue(bool sent)`.
    pub(crate) fn clear_queue(&mut self, sent: bool) {
        for c in self.queue.iter_mut() {
            c.dispose();
        }
        self.queue.clear();
        for c in self.pending.iter_mut() {
            c.dispose();
        }
        self.pending.clear();
        if sent {
            self.sent.clear();
        }
        self.retry = None;
    }

    // ---------------------------------------------------------------- TX side

    /// `StartTX` (TX thread first run).
    pub(crate) fn start_tx(&mut self) {
        // no soft reset when opening COM port for smoothieware
        if self.firmware() != Firmware::Smoothie {
            let reset = lg_base::settings::get("Reset Grbl On Connect", true);
            self.internal_reset(reset);
        }
        self.initialize_board();
        self.query_timer.start();
    }

    /// `InitializeBoard` (virtual).
    fn initialize_board(&mut self) {
        if self.firmware() == Firmware::Smoothie {
            // send a new line
            self.send_immediate(10, false);
        }
        self.query_position();
    }

    /// `QueryPosition` (virtual).
    pub(crate) fn query_position(&mut self) {
        match self.firmware() {
            Firmware::Marlin => {
                // send "M114\n" to retrieve position
                if self.machine_status != MacStatus::Run {
                    let _ = self.com().write_str("M114\n");
                }
            }
            Firmware::VigoWork => self.send_immediate(0x88, true),
            _ => self.send_immediate(63, true),
        }
    }

    /// One `ThreadTX` iteration (called with the lock held).
    pub(crate) fn thread_tx(&mut self) {
        if self.machine_status == MacStatus::Connecting && self.now_ms() - self.connect_start > 10000 {
            self.on_connect_timeout();
        }

        if !self.sh.tx_h.must_exit() {
            self.handle_continuous_jog();
            if self.can_send() {
                self.send_line();
            }
            self.manage_cooling_cycles();
        }

        if self.query_timer.expired() {
            self.query_position();
        }

        self.detect_hang();

        let mode = self.current_threading_mode();
        let sleep = if self.can_send() { mode.tx_short() } else { mode.tx_long() };
        self.sh.tx_h.set_sleep_time(sleep);
        self.query_timer.set_period(TimeSpan::from_milliseconds(mode.status_query() as f64));
    }

    fn on_connect_timeout(&mut self) {
        let com = self.com();
        if com.is_open() {
            logger::log_message("OpenCom", "Connection timeout!");
            // this causes the disconnection from the RX thread
            com.close(true);
        }
    }

    /// `DetectHang` (virtual: Marlin has its own rule).
    fn detect_hang(&mut self) {
        let (issue, in_program) = {
            let tp = self.sh.tp();
            (tp.last_issue(), tp.in_program())
        };
        if issue != DetectedIssue::Unknown || self.machine_status != MacStatus::Run || !in_program {
            return;
        }
        let period_ms = self.query_timer.period().total_milliseconds() as i64;
        let status_delay = self.elapsed(self.last_status_ms);
        let no_response = if self.firmware() == Firmware::Marlin {
            // Marlin does not answer to immediate commands: use the last activity of the board
            let act = self.elapsed(self.last_move_ms);
            act > 10_000 && act > period_ms * 10 && status_delay > 5_000
        } else {
            status_delay > period_ms * 10 && status_delay > 5_000
        };
        if no_response {
            self.set_issue(DetectedIssue::StopResponding);
        }
    }

    /// `PeekNextCommand` (returns a clone sharing the result with the queued one).
    fn peek_next_command(&self) -> Option<GrblCommand> {
        let queue = match &self.alt {
            Some(a) => &a.queue,
            None => &self.queue,
        };
        let mode = self.current_streaming_mode();
        if self.has_pending_commands() && self.pending.front().is_some_and(|p| p.is_write_eeprom()) {
            // if managing eeprom write act like sync
            None
        } else if mode == StreamingMode::Buffered && !queue.is_empty() {
            queue.front().cloned()
        } else if mode != StreamingMode::Buffered && !self.has_pending_commands() {
            if let Some(r) = &self.retry {
                Some(r.clone())
            } else {
                queue.front().cloned()
            }
        } else {
            None
        }
    }

    /// `HasSpaceInBuffer(command)`.
    fn has_space_in_buffer(&self, command: &GrblCommand) -> bool {
        self.used_buffer + crate::util::utf16_len(&command.serial_data()) <= self.buffer_size()
    }

    /// `CanSend()`.
    pub(crate) fn can_send(&self) -> bool {
        self.peek_next_command().is_some_and(|n| self.has_space_in_buffer(&n))
    }

    /// `BufferIsFull()`.
    #[allow(dead_code)]
    pub(crate) fn buffer_is_full(&self) -> bool {
        self.used_buffer > 0 && self.peek_next_command().is_some_and(|n| !self.has_space_in_buffer(&n))
    }

    /// `RemoveManagedCommand()`.
    fn remove_managed_command(&mut self) {
        if self.retry.is_some() {
            self.retry = None;
        } else {
            self.queue_ptr().pop_front();
        }
    }

    /// `SendLine()`.
    pub(crate) fn send_line(&mut self) {
        let Some(mut tosend) = self.peek_next_command() else { return };
        // the build rewrites the line (comments out, spaces collapsed): that is what goes on the wire
        tosend.build_helper();
        tosend.set_sending();
        let r = (|| -> Result<(), String> {
            tosend.delete_helper();
            self.sent_ptr().push(SentRow::Command(tosend.clone()));
            self.pending.push_back(tosend.clone());
            self.remove_managed_command();

            self.send_to_serial(&tosend).map_err(|e| e.to_string())?;

            let mut tp = self.sh.tp();
            if tp.in_program() {
                tp.job_sent();
            }
            drop(tp);

            self.last_move_ms = Some(self.now_ms());
            Ok(())
        })();
        if let Err(e) = r {
            logger::log_message("SendLine", &format!("Error sending [{}] command: {}", tosend.command(), e));
        }
    }

    /// `SendToSerial(tosend)` (virtual: Vigo answers by itself outside a job).
    fn send_to_serial(&mut self, tosend: &GrblCommand) -> std::io::Result<()> {
        let data = tosend.serial_data();
        if self.firmware() == Firmware::VigoWork {
            if self.vigo.injob {
                self.used_buffer += crate::util::utf16_len(&data);
                self.com().write_str(&data)
            } else {
                let r = self.com().write_str(&data);
                tosend.set_result("ok", false);
                r
            }
        } else {
            self.used_buffer += crate::util::utf16_len(&data);
            self.com().write_str(&data)
        }
    }

    /// `SendImmediate(b, mute)`.
    pub(crate) fn send_immediate(&mut self, b: u8, mute: bool) {
        if !mute {
            logger::log_message("SendImmediate", &format!("Send Immediate Command [0x{b:X}]"));
        }
        let com = self.com();
        if com.is_open()
            && let Err(e) = com.write_byte(b)
        {
            logger::log_exception("SendImmediate", &e);
        }
    }

    /// `SendBoardResetCommand` (virtual).
    fn send_board_reset_command(&mut self) {
        match self.firmware() {
            Firmware::Marlin => {}
            // is it possible to write directly without push into queue??? (C# comment)
            Firmware::Smoothie => {
                let _ = self.com().write_str("reset\r\n");
            }
            _ => self.send_immediate(24, false),
        }
    }

    /// `InternalReset(device)`; the override event is raised here (C# raises it after the lock).
    pub(crate) fn internal_reset(&mut self, device: bool) {
        self.clear_queue(true);
        self.used_buffer = 0;
        self.sh.tp().job_end(true);
        self.cur_ov_linear = 100;
        self.cur_ov_rapids = 100;
        self.cur_ov_power = 100;
        self.tar_ov_linear = 100;
        self.tar_ov_rapids = 100;
        self.tar_ov_power = 100;

        if device {
            self.send_board_reset_command();
        }
        self.emit(CoreEvent::OverrideChanged);
    }

    /// `HandleContinuosJog()`.
    fn handle_continuous_jog(&mut self) {
        if !(self.support_true_jogging() && !self.has_pending_commands()) {
            return;
        }
        let (new_jog, abort_required) = self.sh.jog.lock().unwrap_or_else(|e| e.into_inner()).get_and_clear();
        let Some(j) = new_jog else { return };
        if abort_required {
            // abort previous jog command
            self.send_immediate(0x85, false);
        }
        let conf = self.configuration();
        if let Some(cmd) = jog::continuous_jog_command(&j, conf.table_width(), conf.table_height()) {
            self.enqueue_text(&cmd);
        }
    }

    /// Request a continuous jog; see `ContinuousJog` statics.
    pub(crate) fn continuous_jog_request(&self, j: ContinuousJog) {
        let mut t = self.sh.jog.lock().unwrap_or_else(|e| e.into_inner());
        match j.direction {
            JogDirection::Abort => t.abort(),
            JogDirection::Position => t.to_position(j.target, j.speed),
            d => {
                let _ = t.to_direction(d, j.speed);
            }
        }
    }

    // ---------------------------------------------------------------- cooling

    /// `ManageCoolingCycles()`.
    fn manage_cooling_cycles(&mut self) {
        if self.auto_cooling() && self.in_program() && self.machine_status != MacStatus::Hold && self.machine_status != MacStatus::AutoHold {
            let on = get_timespan_setting("AutoCooling TOn", TimeSpan::from_minutes(10.0));
            let off = get_timespan_setting("AutoCooling TOff", TimeSpan::from_minutes(1.0));
            let global = self.sh.tp().total_global_job_time().ticks();
            // C# would throw DivideByZero for a zero period (caught by ThreadTX); skip here
            if let Some(rem) = global.checked_rem((on + off).ticks()) {
                self.set_now_cooling(rem > on.ticks());
            }
        }
    }

    /// `NowCooling` setter.
    fn set_now_cooling(&mut self, value: bool) {
        if self.now_cooling != value {
            if value {
                self.start_cooling();
            } else {
                self.resume_cooling();
            }
            self.now_cooling = value;
        }
    }

    fn start_cooling(&mut self) {
        if self.support_laser_mode() && self.configuration().laser_mode() {
            self.feed_hold(true);
        }
        // TODO(upstream): emulate pause by pushing laser off and sleep into stream
    }

    fn resume_cooling(&mut self) {
        if self.support_laser_mode() && self.configuration().laser_mode() {
            self.cycle_start_resume(true);
        }
    }

    // ---------------------------------------------------------------- immediate commands

    pub(crate) fn can_resume_hold(&self) -> bool {
        self.is_connected() && (self.machine_status == MacStatus::Door || Self::is_any_hold_state(self.machine_status))
    }

    pub(crate) fn can_feed_hold(&self) -> bool {
        self.is_connected() && self.machine_status == MacStatus::Run
    }

    /// `CycleStartResume(auto)`.
    pub(crate) fn cycle_start_resume(&mut self, _auto: bool) {
        if self.can_resume_hold() {
            self.hold_by_cooling = false;
            self.hold_by_user = false;
            self.send_immediate(126, false);
        }
    }

    /// `FeedHold(cooling)`.
    pub(crate) fn feed_hold(&mut self, cooling: bool) {
        if self.can_feed_hold() {
            self.hold_by_user = !cooling;
            self.hold_by_cooling = cooling;
            self.send_immediate(33, false);
        }
    }

    // ---------------------------------------------------------------- RX side

    /// `HandleMissingOK()`.
    pub(crate) fn handle_missing_ok(&mut self) {
        if self.is_buffer_stuck() && self.machine_say_buffer_free() {
            self.unlock_from_buffer_stuck(true);
        }
    }

    /// `UnlockFromBufferStuck(auto)`.
    pub(crate) fn unlock_from_buffer_stuck(&mut self, auto: bool) {
        if self.is_buffer_stuck() {
            // answer "ok" to every pending command
            let n = self.pending.len();
            self.create_fake_ok(n, auto);
        }
    }

    /// `IsBufferStuck()`.
    pub(crate) fn is_buffer_stuck(&self) -> bool {
        self.machine_status == MacStatus::Run && self.has_pending_commands() && !self.buffer_is_free() && self.machine_not_moving_or_reply()
    }

    fn create_fake_ok(&mut self, count: usize, auto: bool) {
        self.push_message(GrblMessage::with_type("Unlock from buffer stuck!", MessageType::Warning));
        let act = if auto { "auto" } else { "manual" };
        comlog::log_str("com", &format!("Handle Missing OK [{count}] ({act})"));
        logger::log_message("Issue detector", &format!("Handle Missing OK [{count}] ({act})"));
        for _ in 0..count {
            self.manage_command_response("ok");
        }
    }

    fn machine_not_moving_or_reply(&self) -> bool {
        self.elapsed(self.last_move_ms) > 10_000
    }

    fn machine_say_buffer_free(&self) -> bool {
        self.grbl_buffer == self.buffer_size()
    }

    fn buffer_is_free(&self) -> bool {
        self.used_buffer == 0
    }

    /// `ManageReceivedLine(rline)` (virtual in Marlin and Vigo).
    pub(crate) fn manage_received_line(&mut self, rline: &str) {
        match self.firmware() {
            Firmware::Marlin if rline.starts_with("X:") => return self.manage_marlin_real_time_status(rline),
            Firmware::VigoWork if rline.starts_with("<VSta") && rline.ends_with('>') => return self.manage_vigo_status(rline),
            _ => {}
        }
        self.manage_received_line_base(rline);
    }

    fn manage_received_line_base(&mut self, rline: &str) {
        let lower = rline.to_lowercase();
        if lower.starts_with("ok") || lower.starts_with("error") {
            self.manage_command_response(rline);
        } else if rline.starts_with('<') && rline.ends_with('>') {
            self.manage_real_time_status(rline);
        } else if rline.starts_with("GrblHAL ") {
            self.manage_grbl_hal_welcome_message(rline);
        } else if rline.starts_with("Grbl-Vigo:") {
            self.manage_vigo_welcome_message(rline);
        } else if rline.starts_with("Ortur ") || rline.starts_with("Aufero ") {
            self.manage_ortur_model_message(rline);
        } else if rline.starts_with("OLF") {
            self.manage_ortur_firmware_message(rline);
        } else if rline.starts_with("[Machine:") && rline.ends_with(']') {
            self.manage_longer_model_message(rline);
        } else if rline.starts_with("[Software:") && rline.ends_with(']') {
            self.manage_longer_firmware_message(rline);
        } else if rline.starts_with("SimpleLaser ") {
            self.manage_simple_laser_welcome_message(rline);
        } else if rline.starts_with("Grbl ") {
            self.manage_standard_welcome_message(rline);
        } else if self.welcome_seen.is_none() && UNKNOWN_WELCOME.is_match(rline) {
            self.manage_unknown_welcome_message(rline);
        } else if lower.contains("ok") {
            self.manage_broken_ok_message(rline);
        } else if lower.starts_with("alarm:") {
            self.manage_standard_blocking_alarm(rline);
        } else if rline.starts_with("[MSG:Get IP ") && rline.ends_with(']') {
            self.manage_sta_ip_message(rline);
        } else if rline.starts_with("[MSG:Connected with ") && rline.ends_with(']') {
            self.manage_longer_ip_message(rline);
        } else {
            self.manage_generic_message(rline);
        }
    }

    /// `ManageStaIPMessage`: `[MSG:Get IP 192.168.1.182]`.
    fn manage_sta_ip_message(&mut self, rline: &str) {
        match substring(rline, 12, rline.chars().count().saturating_sub(13)) {
            Some(ip) => {
                self.detected_ip = Some(ip);
                self.manage_generic_message(rline);
            }
            None => self.log_parse_error("IP Detector", rline, "ArgumentOutOfRange"),
        }
    }

    /// `ManageLongerIPMessage`: `[MSG:Connected with 192.168.1.182]`.
    fn manage_longer_ip_message(&mut self, rline: &str) {
        match substring(rline, 20, rline.chars().count().saturating_sub(21)) {
            Some(ip) => {
                self.detected_ip = Some(ip);
                self.manage_generic_message(rline);
            }
            None => self.log_parse_error("IP Detector", rline, "ArgumentOutOfRange"),
        }
    }

    fn log_parse_error(&self, ctx: &str, rline: &str, ex: &str) {
        logger::log_message(ctx, &format!("Ex on [{rline}] message"));
        logger::log_exception(ctx, &ex);
    }

    /// `ManageGenericMessage`.
    pub(crate) fn manage_generic_message(&mut self, rline: &str) {
        let decode = self.support_csv();
        self.push_message(GrblMessage::new(rline, decode));
    }

    fn manage_standard_blocking_alarm(&mut self, rline: &str) {
        if self.sh.tp().last_issue() == DetectedIssue::Unknown && self.in_program() {
            self.set_issue(DetectedIssue::MachineAlarm);
        }
        self.manage_generic_message(rline);
    }

    /// Parse the 3 version chars of a welcome line at the given offsets.
    fn welcome_version(rline: &str, p_maj: usize, p_min: usize, p_build: usize) -> PResult<(i32, i32, char)> {
        let maj = parse_int(&substring(rline, p_maj, 1).ok_or("range")?).ok_or("format")?;
        let min = parse_int(&substring(rline, p_min, 1).ok_or("range")?).ok_or("format")?;
        let build = substring(rline, p_build, 1).ok_or("range")?.chars().next().ok_or("range")?;
        Ok((maj, min, build))
    }

    /// `ManageStandardWelcomeMessage`: `Grbl X.Xx ['$' for help]`.
    fn manage_standard_welcome_message(&mut self, rline: &str) {
        let r = (|| -> PResult<()> {
            // welcome already seen - disable unknown firmware detection
            self.welcome_seen = Some(rline.to_string());
            let (maj, min, build) = Self::welcome_version(rline, 5, 7, 8)?;
            self.set_grbl_version(GrblVersionInfo::with_vendor(maj, min, build, self.vendor_info_seen.as_deref(), self.vendor_version_seen.as_deref(), false));
            self.detect_unexpected_reset();
            self.on_startup_message();
            Ok(())
        })();
        if let Err(e) = r {
            self.log_parse_error("VersionInfo", rline, &e);
        }
        self.push_message(GrblMessage::new(rline, false));
    }

    /// `ManageUnknownWelcomeMessage`: `BlaBla X.Xx`.
    fn manage_unknown_welcome_message(&mut self, rline: &str) {
        let r = (|| -> PResult<()> {
            // unknown welcome seen - disable next unknown firmware detection
            self.welcome_seen = Some(rline.to_string());
            let caps = UNKNOWN_WELCOME.captures(rline).ok_or("nomatch")?;
            let maj = parse_int(&caps["maj"]).ok_or("format")?;
            let min = parse_int(&caps["min"]).ok_or("format")?;
            let build = caps["build"].chars().next().ok_or("range")?;
            let fw = caps["fw"].to_string();
            self.vendor_info_seen = Some(fw);

            self.set_grbl_version(GrblVersionInfo::with_vendor(maj, min, build, self.vendor_info_seen.as_deref(), self.vendor_version_seen.as_deref(), false));
            logger::log_message("UnknInfo", &format!("Detected {}", self.vendor_info_seen.as_deref().unwrap_or_default()));

            self.detect_unexpected_reset();
            self.on_startup_message();
            Ok(())
        })();
        if let Err(e) = r {
            self.log_parse_error("VersionInfo", rline, &e);
        }
        self.push_message(GrblMessage::new(rline, false));
    }

    /// `ManageSimpleLaserWelcomeMessage`: `SimpleLaser X.Xx ['$' for help]`.
    fn manage_simple_laser_welcome_message(&mut self, rline: &str) {
        let r = (|| -> PResult<()> {
            self.welcome_seen = Some(rline.to_string());
            let (maj, min, build) = Self::welcome_version(rline, 12, 14, 15)?;
            self.vendor_info_seen = Some("SimpleLaser".to_string());
            self.set_grbl_version(GrblVersionInfo::with_vendor(maj, min, build, self.vendor_info_seen.as_deref(), self.vendor_version_seen.as_deref(), false));
            logger::log_message("SimpleInfo", &format!("Detected {}", self.vendor_info_seen.as_deref().unwrap_or_default()));
            self.detect_unexpected_reset();
            self.on_startup_message();
            Ok(())
        })();
        if let Err(e) = r {
            self.log_parse_error("VersionInfo", rline, &e);
        }
        self.push_message(GrblMessage::new(rline, false));
    }

    /// `ManageGrblHalWelcomeMessage`: `GrblHAL X.Xx ['$' or '$HELP' for help]`.
    fn manage_grbl_hal_welcome_message(&mut self, rline: &str) {
        let r = (|| -> PResult<()> {
            self.welcome_seen = Some(rline.to_string());
            let (maj, min, build) = Self::welcome_version(rline, 8, 10, 11)?;
            self.set_grbl_version(GrblVersionInfo::with_vendor(maj, min, build, self.vendor_info_seen.as_deref(), self.vendor_version_seen.as_deref(), true));
            self.detect_unexpected_reset();
            self.on_startup_message();
            Ok(())
        })();
        if let Err(e) = r {
            self.log_parse_error("VersionInfo", rline, &e);
        }
        self.push_message(GrblMessage::new(rline, false));
    }

    /// `ManageVigoWelcomeMessage`: `Grbl-Vigo:1.1f|Build:G-20170131-V3.0-20200720`.
    fn manage_vigo_welcome_message(&mut self, rline: &str) {
        let r = (|| -> PResult<()> {
            self.welcome_seen = Some(rline.to_string());
            let (maj, min, build) = Self::welcome_version(rline, 10, 12, 13)?;
            self.vendor_info_seen = Some("Grbl-Vigo".to_string());
            self.vendor_version_seen = Some(rline.split(':').nth(2).ok_or("index")?.to_string());
            self.set_grbl_version(GrblVersionInfo::with_vendor(maj, min, build, self.vendor_info_seen.as_deref(), self.vendor_version_seen.as_deref(), false));
            logger::log_message("VigoInfo", &format!("Detected {} {}", self.vendor_info_seen.as_deref().unwrap_or_default(), self.vendor_version_seen.as_deref().unwrap_or_default()));
            self.detect_unexpected_reset();
            self.on_startup_message();
            Ok(())
        })();
        if let Err(e) = r {
            self.log_parse_error("VersionInfo", rline, &e);
        }
        self.push_message(GrblMessage::new(rline, false));
    }

    /// `ManageOrturModelMessage` (also Aufero).
    fn manage_ortur_model_message(&mut self, rline: &str) {
        let v = rline.replace("Ready", "").replace('!', "");
        let v = v.trim().to_string();
        logger::log_message("OrturInfo", &format!("Detected {v}"));
        self.vendor_info_seen = Some(v);
        self.push_message(GrblMessage::new(rline, false));
    }

    /// `ManageLongerModelMessage`: `[Machine:...]`.
    fn manage_longer_model_message(&mut self, rline: &str) {
        let v = rline.replace("[Machine:", "").replace(']', "");
        let v = v.trim().to_string();
        self.vendor_info_seen = Some(v.clone());
        self.set_grbl_version(GrblVersionInfo::with_vendor(1, 1, 'f', Some(&v), self.vendor_version_seen.as_deref(), false));
        logger::log_message("LongerInfo", &format!("Detected {v}"));
        self.push_message(GrblMessage::new(rline, false));
    }

    /// `ManageLongerFirmwareMessage`: `[Software:...]`.
    fn manage_longer_firmware_message(&mut self, rline: &str) {
        let v = rline.replace("[Software:", "").replace(']', "");
        self.vendor_version_seen = Some(v.clone());
        self.set_grbl_version(GrblVersionInfo::with_vendor(1, 1, 'f', self.vendor_info_seen.as_deref(), Some(&v), false));
        logger::log_message("LongerInfo", &format!("Detected FW {v}"));
        self.push_message(GrblMessage::new(rline, false));
    }

    /// `ManageOrturFirmwareMessage`: `OLF ...`.
    fn manage_ortur_firmware_message(&mut self, rline: &str) {
        let v = rline.replace("OLF", "");
        let v = v.trim_matches(|c| c == '.' || c == ' ' || c == ':').to_string();
        logger::log_message("OrturInfo", &format!("Detected OLF {v}"));
        self.vendor_version_seen = Some(v);
        self.push_message(GrblMessage::new(rline, false));
    }

    /// `OnStartupMessage()`: reset everything, works also after a hard unexpected reset.
    fn on_startup_message(&mut self) {
        self.clear_queue(false);
        self.used_buffer = 0;
        self.sh.tp().job_end(true);
        self.cur_ov_linear = 100;
        self.cur_ov_rapids = 100;
        self.cur_ov_power = 100;
        self.tar_ov_linear = 100;
        self.tar_ov_rapids = 100;
        self.tar_ov_power = 100;
        self.emit(CoreEvent::OverrideChanged);
    }

    fn detect_unexpected_reset(&mut self) {
        if self.sh.tp().last_issue() == DetectedIssue::Unknown && self.machine_status == MacStatus::Run && self.in_program() {
            self.set_issue(DetectedIssue::UnexpectedReset);
        }
    }

    /// `StatusReportVersion(rline)`.
    fn status_report_version(&self, rline: &str) -> GrblVersionInfo {
        // if version is known -> return version
        if let Some(v) = &self.grbl_version {
            return v.clone();
        }
        // else guess from rline (the Pin: check is for 1.0c compatibility, issue 317)
        if rline.contains('|') && !rline.contains("Pin:") {
            GrblVersionInfo::new(1, 1)
        } else if rline.contains('|') && rline.contains("Pin:") {
            GrblVersionInfo::with_build(1, 0, 'c')
        } else {
            GrblVersionInfo::new(0, 9)
        }
    }

    /// `ManageRealTimeStatus(rline)`: `<Idle|MPos:...>` (1.1) and `<Idle,MPos:...>` (0.9).
    pub(crate) fn manage_real_time_status(&mut self, rline: &str) {
        self.last_status_ms = Some(self.now_ms());
        // NOTE(upstream): the log shows the stripped line once the strip worked
        let mut shown = rline.to_string();
        let r = (|| -> PResult<()> {
            let stripped = substring(rline, 1, rline.chars().count().saturating_sub(2)).ok_or("range")?;
            shown = stripped.clone();
            let rline = stripped;

            let rversion = self.status_report_version(&rline);
            if rversion >= GrblVersionInfo::new(1, 1) {
                // https://github.com/gnea/grbl/wiki/Grbl-v1.1-Interface#real-time-status-reports
                let arr: Vec<&str> = rline.split('|').collect();
                self.parse_machine_status(arr[0])?;
                for item in arr.iter().skip(1) {
                    if item.starts_with("Ov:") {
                        self.parse_overrides(item)?;
                    } else if item.starts_with("Bf:") {
                        self.parse_bf(item)?;
                    } else if item.starts_with("WPos:") {
                        self.parse_wpos(item)?;
                    } else if item.starts_with("MPos:") {
                        self.parse_mpos(item)?;
                    } else if item.starts_with("WCO:") {
                        self.parse_wco(item)?;
                    } else if item.starts_with("FS:") {
                        self.parse_fs(item)?;
                    } else if item.starts_with("F:") {
                        self.parse_f(item)?;
                    }
                }
            } else {
                // <Idle,MPos:0.000,0.000,0.000,WPos:0.000,0.000,0.000>
                let arr: Vec<&str> = rline.split(',').collect();
                if !arr.is_empty() {
                    self.parse_machine_status(arr[0])?;
                }
                if arr.len() > 3 {
                    let x = pf(&substring_from(arr[1], 5).ok_or("range")?)?;
                    self.set_mposition(GPoint::new(x, pf(arr[2])?, pf(arr[3])?));
                }
                if arr.len() > 6 {
                    let x = pf(&substring_from(arr[4], 5).ok_or("range")?)?;
                    self.compute_wco(GPoint::new(x, pf(arr[5])?, pf(arr[6])?));
                }
            }
            Ok(())
        })();
        if let Err(e) = r {
            logger::log_message("RealTimeStatus", &format!("Ex on [{shown}] message"));
            logger::log_exception("RealTimeStatus", &e);
        }
    }

    /// `ComputeWCO(wpos)`: WCO = MPos - WPos.
    fn compute_wco(&mut self, wpos: GPoint) {
        self.set_wco(self.mpos - wpos);
    }

    fn xyz(p: &str, skip: usize) -> PResult<Vec<String>> {
        let body = substring_from(p, skip).ok_or("range")?;
        Ok(body.split(',').map(str::to_string).collect())
    }

    /// `ParseWCO`.
    fn parse_wco(&mut self, p: &str) -> PResult<()> {
        let xyz = Self::xyz(p, 4)?;
        let v = GPoint::new(parse_float_at(&xyz, 0)?, parse_float_at(&xyz, 1)?, parse_float_at(&xyz, 2)?);
        self.set_wco(v);
        Ok(())
    }

    /// `ParseWPos`.
    fn parse_wpos(&mut self, p: &str) -> PResult<()> {
        let xyz = Self::xyz(p, 5)?;
        let v = GPoint::new(parse_float_at(&xyz, 0)?, parse_float_at(&xyz, 1)?, parse_float_at(&xyz, 2)?);
        self.set_mposition(self.wco + v);
        Ok(())
    }

    /// `ParseMPos`.
    fn parse_mpos(&mut self, p: &str) -> PResult<()> {
        let xyz = Self::xyz(p, 5)?;
        let v = GPoint::new(parse_float_at(&xyz, 0)?, parse_float_at(&xyz, 1)?, parse_float_at(&xyz, 2)?);
        self.set_mposition(v);
        Ok(())
    }

    /// `ParseBf`: `Bf:15,128` (blocks free, bytes free).
    fn parse_bf(&mut self, p: &str) -> PResult<()> {
        let ab = Self::xyz(p, 3)?;
        self.grbl_blocks = parse_int(ab.first().ok_or("index")?).ok_or("format")?;
        self.grbl_buffer = parse_int(ab.get(1).ok_or("index")?).ok_or("format")?;
        // do not force here, we cannot rely on values received from report (electrical noise?!)
        self.enlarge_buffer(self.grbl_buffer, false);
        Ok(())
    }

    /// `EnlargeBuffer(newval, force)`.
    pub(crate) fn enlarge_buffer(&mut self, newval: i32, force: bool) {
        let oldval = self.auto_buffer_size;
        if force {
            // this came from $I at connect, and so it is a verified buffer size
            self.auto_buffer_size = newval;
        } else if self.auto_buffer_size == DEFAULT_BUFFER_SIZE {
            // act only to change default value at first event; only allow known values
            // (do not accept errors in bf message caused by electrical noise)
            if matches!(newval, 128 | 255 | 256 | 10240 | 254) {
                self.auto_buffer_size = newval;
            }
        }
        if self.auto_buffer_size != oldval {
            logger::log_message("EnlargeBuffer", &format!("Buffer size changed to {} [{}]", self.auto_buffer_size, if force { "True" } else { "False" }));
        }
    }

    /// `ParseFS`: `FS:500,12000`.
    fn parse_fs(&mut self, p: &str) -> PResult<()> {
        let fs = Self::xyz(p, 3)?;
        let (f, s) = (parse_float_at(&fs, 0)?, parse_float_at(&fs, 1)?);
        self.set_fs(f, s);
        Ok(())
    }

    /// `ParseF` (virtual: Smoothie sends `F:a,b`).
    fn parse_f(&mut self, p: &str) -> PResult<()> {
        let f = substring_from(p, 2).ok_or("range")?;
        if self.firmware() == Firmware::Smoothie {
            let fs: Vec<&str> = f.split(',').collect();
            let a = pf(fs.first().ok_or("index")?)?;
            let b = pf(fs.get(1).ok_or("index")?)?;
            self.set_fs(a, b);
        } else {
            let a = pf(&f)?;
            self.set_fs(a, 0.0);
        }
        Ok(())
    }

    /// `SetFS(f, s)`.
    pub(crate) fn set_fs(&mut self, f: f32, s: f32) {
        laser_life::compute_laser_true_time(self.cur_s);
        self.cur_f = f;
        self.cur_s = s;
    }

    /// `SetMPosition(pos)`.
    pub(crate) fn set_mposition(&mut self, pos: GPoint) {
        if pos != self.mpos {
            self.mpos = pos;
            self.last_move_ms = Some(self.now_ms());
        }
    }

    /// `SetWCO(wco)`.
    fn set_wco(&mut self, wco: GPoint) {
        self.wco = wco;
        // remember last wco for job resume
        self.sh.tp().set_last_known_wco(wco);
    }

    /// `ManageBrokenOkMessage`.
    fn manage_broken_ok_message(&mut self, rline: &str) {
        self.push_message(GrblMessage::with_type("Handle broken ok!", MessageType::Warning));
        logger::log_message("CommandResponse", &format!("Broken \"ok\" message: [{rline}]"));
        self.manage_command_response("ok");
    }

    /// `ManageCommandResponse(rline)`: the heart of the buffer accounting.
    pub(crate) fn manage_command_response(&mut self, rline: &str) {
        // add a reset to prevent HangDetector trigger on G4
        self.last_move_ms = Some(self.now_ms());
        if self.has_pending_commands() {
            // peek first: the status must be known when the queue is seen empty (write config relies on it)
            let Some(pending) = self.pending.front().cloned() else { return };
            let decode = self.support_csv();
            pending.set_result(rline, decode);
            self.pending.pop_front();

            self.used_buffer = (self.used_buffer - crate::util::utf16_len(&pending.serial_data())).max(0);

            {
                let mut tp = self.sh.tp();
                // only if it is not a repetition update the time
                if tp.in_program() && pending.repeat_count() == 0 {
                    tp.job_executed(pending.time_offset());
                }
                if tp.in_program() && pending.status() == lg_gcode::CommandStatus::ResponseBad {
                    tp.job_error();
                }
            }

            if pending.is_write_eeprom() && pending.status() == lg_gcode::CommandStatus::ResponseGood {
                let mut conf = configuration::configuration();
                conf.add_or_update(&lg_gcode::GrblRow::get_decoded_message(&pending));
                // C# mutates the stored object in place
                configuration::set_configuration(&conf);
            }

            // repeat program errors if: no queue (maybe aligning after buf/sync change), error, not repeated too often
            if self.in_program()
                && self.current_streaming_mode() == StreamingMode::RepeatOnError
                && !self.has_pending_commands()
                && pending.status() == lg_gcode::CommandStatus::ResponseBad
                && pending.repeat_count() < 3
            {
                self.retry = Some(GrblCommand::with_options(pending.command(), pending.repeat_count() + 1, false));
            }
        }

        if self.in_program() && self.queue_ptr().is_empty() && !self.has_pending_commands() {
            self.on_program_end();
        }
    }

    // ---------------------------------------------------------------- status field parsers

    /// `ParseMachineStatus(data)` (virtual in Marlin).
    fn parse_machine_status(&mut self, data: &str) -> PResult<()> {
        if self.firmware() == Firmware::Marlin {
            return self.marlin_parse_machine_status(data);
        }
        let mut data = data;
        if let Some(i) = data.find(':') {
            data = &data[..i];
        }
        let mut var = MacStatus::parse(data).ok_or_else(|| format!("Requested value '{data}' was not found."))?;

        // bugfix for grbl sending Idle on G4
        if self.in_program() && var == MacStatus::Idle {
            var = MacStatus::Run;
        }
        if var == MacStatus::Hold && self.hold_by_cooling {
            var = MacStatus::Cooling;
        } else if var == MacStatus::Hold && !self.hold_by_user {
            var = MacStatus::AutoHold;
        }
        self.set_status(var);
        Ok(())
    }

    /// Marlin `ParseMachineStatus`.
    fn marlin_parse_machine_status(&mut self, data: &str) -> PResult<()> {
        let mut var = MacStatus::Disconnected;
        if data.contains("ok") {
            var = MacStatus::Idle;
        }
        if self.in_program() && var == MacStatus::Idle {
            var = MacStatus::Run;
        }
        if var == MacStatus::Hold && self.hold_by_cooling {
            var = MacStatus::Cooling;
        } else if var == MacStatus::Hold && !self.hold_by_user {
            var = MacStatus::AutoHold;
        }
        self.set_status(var);
        Ok(())
    }

    /// Marlin `ManageMarlinRealTimeStatus`: `X:10.00 Y:0.00 Z:0.00 E:0.00 Count X:1600 Y:0 Z:0`.
    fn manage_marlin_real_time_status(&mut self, rline: &str) {
        self.last_status_ms = Some(self.now_ms());
        // remove EOL (maybe not necessary, already done)
        let rline = rline.trim_matches(|c| c == '\r' || c == '\n' || c == ' ').to_string();
        let r = (|| -> PResult<()> {
            let arr: Vec<&str> = rline.split(' ').collect();
            if !arr.is_empty() {
                // force update of status
                self.parse_machine_status("ok")?;
                let part = |i: usize| -> PResult<f32> { pf(arr.get(i).ok_or("index")?.split(':').nth(1).ok_or("index")?) };
                let (x, y, z) = (part(0)?, part(1)?, part(2)?);
                self.set_mposition(GPoint::new(x, y, z));
            }
            Ok(())
        })();
        if let Err(e) = r {
            logger::log_message("ManageRealTimeStatus", &format!("Ex on [{rline}] message"));
            logger::log_exception("ManageRealTimeStatus", &e);
        }
    }

    /// `ForceStatusIdle` (Marlin only).
    fn force_status_idle(&mut self) {
        if self.firmware() == Firmware::Marlin {
            self.set_status(MacStatus::Idle);
        }
    }

    // ---------------------------------------------------------------- Vigo

    /// `ManageVigoStatus`: `<VSta:..|SBuf:..>`.
    fn manage_vigo_status(&mut self, rline: &str) {
        let r = (|| -> PResult<()> {
            let stripped = substring(rline, 1, rline.chars().count().saturating_sub(2)).ok_or("range")?;
            for item in stripped.split('|') {
                if item.starts_with("VSta:") {
                    self.parse_vsta(item)?;
                } else if item.starts_with("SBuf:") {
                    self.parse_sbuf(item)?;
                }
            }
            Ok(())
        })();
        if let Err(e) = r {
            logger::log_message("VigoStatus", &format!("Ex on [{rline}] message"));
            logger::log_exception("VigoStatus", &e);
        }
    }

    fn parse_vsta(&mut self, p: &str) -> PResult<()> {
        let data = substring_from(p, 5).ok_or("range")?;
        // (int)float truncation
        let cur = lg_dotnet::f32_to_i32(pf(&data)?);
        if self.vigo.old_sta == 2 && cur == 0 {
            self.vigo.injob = false;
        }
        self.vigo.old_sta = cur;
        Ok(())
    }

    fn parse_sbuf(&mut self, p: &str) -> PResult<()> {
        let data = substring_from(p, 5).ok_or("range")?;
        let xyz: Vec<&str> = data.split(',').collect();
        let get = |i: usize| -> PResult<i32> { Ok(lg_dotnet::f32_to_i32(pf(xyz.get(i).ok_or("index")?)?)) };
        let (cur_received, cur_managed, cur_error) = (get(0)?, get(1)?, get(2)?);

        if self.vigo.old_received > cur_received {
            // emergency reset
            self.vigo.old_received = 0;
            self.vigo.old_managed = 0;
            self.vigo.old_error = 0;
        }
        for _ in self.vigo.old_managed..cur_managed {
            self.manage_command_response("ok");
        }
        for _ in self.vigo.old_error..cur_error {
            self.manage_command_response("error:99");
        }
        self.vigo.old_received = cur_received;
        self.vigo.old_managed = cur_managed;
        self.vigo.old_error = cur_error;
        Ok(())
    }

    // ---------------------------------------------------------------- overrides

    /// `ManageOverrides()`: step the machine override toward the target.
    pub(crate) fn manage_overrides(&mut self) {
        if self.tar_ov_linear == 100 && self.cur_ov_linear != 100 {
            self.send_immediate(144, false);
        } else if self.tar_ov_linear - self.cur_ov_linear >= 10 {
            self.send_immediate(145, false);
        } else if self.cur_ov_linear - self.tar_ov_linear >= 10 {
            self.send_immediate(146, false);
        } else if self.tar_ov_linear - self.cur_ov_linear >= 1 {
            self.send_immediate(147, false);
        } else if self.cur_ov_linear - self.tar_ov_linear >= 1 {
            self.send_immediate(148, false);
        }

        if self.tar_ov_power == 100 && self.cur_ov_power != 100 {
            self.send_immediate(153, false);
        } else if self.tar_ov_power - self.cur_ov_power >= 10 {
            self.send_immediate(154, false);
        } else if self.cur_ov_power - self.tar_ov_power >= 10 {
            self.send_immediate(155, false);
        } else if self.tar_ov_power - self.cur_ov_power >= 1 {
            self.send_immediate(156, false);
        } else if self.cur_ov_power - self.tar_ov_power >= 1 {
            self.send_immediate(157, false);
        }

        if self.tar_ov_rapids == 100 && self.cur_ov_rapids != 100 {
            self.send_immediate(149, false);
        } else if self.tar_ov_rapids == 50 && self.cur_ov_rapids != 50 {
            self.send_immediate(150, false);
        } else if self.tar_ov_rapids == 25 && self.cur_ov_rapids != 25 {
            self.send_immediate(151, false);
        }
    }

    /// `ParseOverrides`: `Ov:100,100,100` (feed, rapids, spindle).
    fn parse_overrides(&mut self, data: &str) -> PResult<()> {
        let after = data.find(':').map_or(0, |i| i + 1);
        let arr: Vec<&str> = data[after..].split(',').collect();
        let get = |i: usize| -> PResult<i32> { parse_int(arr.get(i).ok_or("index")?).ok_or_else(|| "format".to_string()) };
        let (a, b, c) = (get(0)?, get(1)?, get(2)?);
        self.change_overrides(a, b, c);
        self.manage_overrides();
        Ok(())
    }

    fn change_overrides(&mut self, feed: i32, rapids: i32, spindle: i32) {
        let notify = feed != self.cur_ov_linear || rapids != self.cur_ov_rapids || spindle != self.cur_ov_power;
        self.cur_ov_linear = feed;
        self.cur_ov_rapids = rapids;
        self.cur_ov_power = spindle;
        if notify {
            self.emit(CoreEvent::OverrideChanged);
        }
    }

    /// `HotKeyOverride(action)`.
    pub(crate) fn hot_key_override(&mut self, action: OverrideAction) {
        match action {
            OverrideAction::PowerDefault => self.tar_ov_power = 100,
            OverrideAction::PowerUp => self.tar_ov_power = (self.tar_ov_power + 1).min(200),
            OverrideAction::PowerDown => self.tar_ov_power = (self.tar_ov_power - 1).max(10),
            OverrideAction::LinearDefault => self.tar_ov_linear = 100,
            OverrideAction::LinearUp => self.tar_ov_linear = (self.tar_ov_linear + 1).min(200),
            OverrideAction::LinearDown => self.tar_ov_linear = (self.tar_ov_linear - 1).max(10),
            OverrideAction::RapidDefault => self.tar_ov_rapids = 100,
            OverrideAction::RapidUp => self.tar_ov_rapids = (self.tar_ov_rapids * 2).min(100),
            OverrideAction::RapidDown => self.tar_ov_rapids = (self.tar_ov_rapids / 2).max(25),
        }
    }

    // ---------------------------------------------------------------- program run

    /// `OnProgramEnd()`.
    fn on_program_end(&mut self) {
        let file_count = self.sh.file.try_read().map(|f| f.count()).unwrap_or(0);
        let (looping, error_count, program_time) = {
            let mut tp = self.sh.tp();
            let ended = tp.job_end(self.loop_count == Decimal::ONE);
            (ended && self.loop_count > Decimal::ONE && self.machine_status != MacStatus::Check, tp.error_count(), tp.total_job_time())
        };
        let ts = |t: TimeSpan| time_span_to_string(t, TimePrecision::Second, TimePrecision::Second, ",", true);
        let summary = format!("[{} lines, {} errors, {}]", file_count, error_count, ts(program_time));

        if looping {
            logger::log_message("CycleEnd", &format!("Cycle Executed: {} lines, {} errors, {}", file_count, error_count, ts(program_time)));
            self.push_message(GrblMessage::with_type(&summary, MessageType::Diagnostic));
            self.emit(CoreEvent::ProgramEnded);
            let lc = self.loop_count - Decimal::ONE;
            self.set_loop_count(lc);
            self.run_program_from_start_inner(false, false, true);
        } else {
            logger::log_message("ProgramEnd", &format!("Job Executed: {} lines, {} errors, {}", file_count, error_count, ts(program_time)));
            self.push_message(GrblMessage::with_type(&summary, MessageType::Diagnostic));
            self.emit(CoreEvent::ProgramEnded);
            self.on_job_end();

            play_sound(EventId::Success);

            self.force_status_idle();
        }
    }

    /// `LoopCount` setter.
    pub(crate) fn set_loop_count(&mut self, v: Decimal) {
        self.loop_count = v;
        self.emit(CoreEvent::LoopCountChanged(v));
    }

    /// `OnJobBegin()` (virtual: Vigo wraps the job).
    fn on_job_begin(&mut self) {
        if self.firmware() == Firmware::VigoWork {
            self.vigo.injob = true;
            self.enqueue_command(&GrblCommand::with_options(">Ofk9gsd8IKjKBahP0OGS9BrhZCPeWCBALCbyGf", 0, true));
        }
        logger::log_message("EnqueueProgram", "Push Header");
        let code: String = lg_base::settings::get("GCode.CustomHeader", lg_gcode::GCODE_STD_HEADER.to_string());
        self.execute_custom_code(&code);
    }

    /// `OnJobEnd()` (virtual: Vigo appends its end sequence).
    fn on_job_end(&mut self) {
        logger::log_message("EnqueueProgram", "Push Footer");
        let code: String = lg_base::settings::get("GCode.CustomFooter", lg_gcode::GCODE_STD_FOOTER.to_string());
        self.execute_custom_code(&code);
        if self.firmware() == Firmware::VigoWork {
            self.enqueue_command(&GrblCommand::with_options("G0X0Y0M5", 0, true));
            self.enqueue_command(&GrblCommand::with_options(">NPredMjdFaeaJajf6OHy:hkRUygcpBXwtV", 0, true));
            // injob = false is not set: the machine does not answer this message, it resets (C# comment)
        }
    }

    /// `OnJobCycle()`.
    fn on_job_cycle(&mut self) {
        logger::log_message("EnqueueProgram", "Push Passes");
        let code: String = lg_base::settings::get("GCode.CustomPasses", lg_gcode::GCODE_STD_PASSES.to_string());
        self.execute_custom_code(&code);
    }

    /// `RunProgramFromStart(homing, first, pass)` without the safety countdown (UI side).
    ///
    /// `homing`: push `$H` first. `first`: first pass of the job (global reset, header). `pass`: loop pass
    /// (custom passes code). Returns false when the loaded file is busy and nothing was queued.
    pub(crate) fn run_program_from_start_inner(&mut self, homing: bool, first: bool, pass: bool) -> bool {
        self.clear_queue(true);
        self.sh.tp().reset(first);

        if homing {
            logger::log_message("EnqueueProgram", "Push Homing ($H)");
            self.queue_ptr().push_back(GrblCommand::new("$H"));
        }
        if first {
            self.on_job_begin();
        }
        if pass {
            self.on_job_cycle();
        }

        let Ok(file) = self.sh.file.try_read() else {
            logger::log_message("EnqueueProgram", "Loaded file is busy, program not queued");
            return false;
        };
        let count = file.count();
        let estimated = file.estimated_time();
        logger::log_message("EnqueueProgram", &format!("Push File, {count} lines"));
        let cmds: Vec<GrblCommand> = file.iter().cloned().collect();
        drop(file);
        self.queue_ptr().extend(cmds);

        let queued = self.queue_ptr().len();
        self.sh.tp().job_start(estimated, queued, first);
        logger::log_message("EnqueueProgram", &format!("Running program, {count} lines"));
        true
    }

    /// `ContinueProgramFromKnown(position, homing, setwco)` without the safety countdown.
    ///
    /// Returns Err(text) when the command analysis failed (C# throws).
    pub(crate) fn continue_program_from_known(&mut self, position: i32, homing: bool, setwco: bool) -> Result<(), String> {
        // leave the list of already sent items, if still present
        self.clear_queue(false);

        self.push_message(GrblMessage::new(&format!("[resume from #{}]", position + 1), false));
        logger::log_message("ResumeProgram", &format!("Resume program from #{}", position + 1));

        let mut spb = StatePositionBuilder::new();

        if homing {
            self.queue_ptr().push_back(GrblCommand::new("$H"));
        }

        if setwco {
            // compute current point and set offset
            let pos = if homing { GPoint::ZERO } else { self.mpos };
            let wco = self.sh.tp().last_known_wco();
            let cur = pos - wco;
            // NOTE(upstream): this one goes to mQueue, not to mQueuePtr
            self.queue.push_back(GrblCommand::new(&format!("G92 X{} Y{} Z{}", fmt_float(cur.x), fmt_float(cur.y), fmt_float(cur.z))));
        }

        let file = self.sh.file.try_read().map_err(|_| "loaded file is busy".to_string())?;
        let file_count = file.count();
        let estimated = file.estimated_time();
        let all: Vec<GrblCommand> = file.iter().cloned().collect();
        drop(file);

        // analyze up to the position
        let mut i = 0usize;
        while (i as i32) < position && i < file_count {
            let mut c = all[i].clone();
            spb.analyze_command(&mut c, false, None).map_err(|e| e.to_string())?;
            i += 1;
        }

        // absolute coordinates
        self.queue_ptr().push_back(GrblCommand::new("G90"));
        // fast go to the computed position with laser off and set speed and power
        let goto = format!("M5 G0 {} {} {} {} {}", el(spb.x()), el(spb.y()), el(spb.z()), el(spb.f()), el(spb.s()));
        self.queue_ptr().push_back(GrblCommand::new(&goto));
        let modals = spb.get_settled_modals();
        self.queue_ptr().push_back(GrblCommand::from_elements(modals.iter()));

        let added = self.queue_ptr().len();
        self.sh.tp().job_continue(estimated, file_count, position, added);

        // enqueue remaining commands
        let mut first = true;
        let start = position.max(0) as usize;
        for c in all.iter().skip(start) {
            if first {
                // check the very first movement command and ensure modal MotionMode is settled
                let mut cmd = c.clone();
                cmd.build_helper();
                let queued = if cmd.is_movement() && cmd.g().is_none() { GrblCommand::from_first_and_append(&spb.motion_mode, &cmd) } else {
                    cmd.delete_helper();
                    cmd.clone()
                };
                self.queue_ptr().push_back(queued);
                first = false;
            } else {
                self.queue_ptr().push_back(c.clone());
            }
        }
        Ok(())
    }

    /// `AbortProgram()`.
    pub(crate) fn abort_program(&mut self) {
        if self.can_abort_program() {
            logger::log_message("ManualAbort", "Program aborted by user action!");
            self.set_issue(DetectedIssue::ManualAbort);
            self.sh.tp().job_end(true);
            // flush the queue of items to send, then shut down the laser
            for c in self.queue.iter_mut() {
                c.dispose();
            }
            self.queue.clear();
            self.queue.push_back(GrblCommand::new("M5"));
        }
    }

    // ---------------------------------------------------------------- can* helpers

    pub(crate) fn has_program(&self) -> bool {
        self.sh.file.try_read().map(|f| f.count() > 0).unwrap_or(false)
    }

    pub(crate) fn queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn idle_or_check(&self) -> bool {
        self.machine_status == MacStatus::Idle || self.machine_status == MacStatus::Check
    }

    pub(crate) fn can_send_file(&self) -> bool {
        self.is_connected() && self.has_program() && self.idle_or_check() && self.queue_empty() && !self.doing_send
    }

    /// `CanAbortProgram` (NOTE(upstream): `a && b && c || !QueueEmpty` precedence kept).
    pub(crate) fn can_abort_program(&self) -> bool {
        (self.is_connected() && self.has_program() && (self.machine_status == MacStatus::Run || Self::is_any_hold_state(self.machine_status))) || !self.queue_empty()
    }

    pub(crate) fn can_read_write_config(&self) -> bool {
        self.is_connected() && !self.in_program() && (self.machine_status == MacStatus::Idle || self.machine_status == MacStatus::Alarm)
    }

    pub(crate) fn can_reset_grbl(&self) -> bool {
        self.is_connected() && self.machine_status != MacStatus::Disconnected
    }

    pub(crate) fn jog_enabled(&self) -> bool {
        if self.support_true_jogging() {
            self.is_connected() && (self.machine_status == MacStatus::Idle || self.machine_status == MacStatus::Jog)
        } else {
            self.is_connected() && (self.machine_status == MacStatus::Idle || self.machine_status == MacStatus::Run) && !self.in_program()
        }
    }

    // ---------------------------------------------------------------- custom code

    /// Snapshot for expression evaluation (`EvaluateCB` inputs).
    pub(crate) fn expr_vars(&self) -> ExprVars {
        let bounds = self.sh.file.try_read().map(|f| FileBounds::from_file(&f)).unwrap_or_default();
        ExprVars {
            bounds,
            jog_step: self.jog_step,
            jog_speed: self.jog_speed,
            working_offset: self.wco,
            machine_position: self.mpos,
            configuration: self.configuration(),
        }
    }

    /// `ExecuteCustomCode(buttoncode)`: one command (or immediate byte) per non-empty line.
    pub(crate) fn execute_custom_code(&mut self, buttoncode: &str) {
        let vars = self.expr_vars();
        for raw in buttoncode.trim().split('\n') {
            let str = raw.strip_suffix('\r').unwrap_or(raw);
            if !str.trim().is_empty() {
                let tosend = expr::evaluate_expression(&vars, str);
                if expr::is_immediate(&tosend) {
                    if let Some(b) = expr::get_immediate(&tosend) {
                        if b == b'!' {
                            self.hold_by_user = true;
                        }
                        self.send_immediate(b, false);
                    }
                } else {
                    self.enqueue_command(&GrblCommand::with_options(&tosend, 0, true));
                }
            }
        }
    }
}

/// `ParseFloat(string)`.
fn pf(s: &str) -> PResult<f32> {
    parse_float(s).ok_or_else(|| format!("Input string was not in a correct format: '{s}'"))
}

/// `ParseFloat(arr, idx, defval = 0)`: missing index gives the default.
fn parse_float_at(arr: &[String], idx: usize) -> PResult<f32> {
    match arr.get(idx) {
        Some(v) => pf(v),
        None => Ok(0.0),
    }
}
