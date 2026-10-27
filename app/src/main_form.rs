// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(clippy::collapsible_if)]
//! Ports `MainForm.cs`: menu bar, status bar, left column (connect/log + jog), preview, and the
//! dialog flows (open/save/run/connect) that the C# `GrblCore` ran with blocking dialogs.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, TryRecvError};
use eframe::egui::{self, Frame, Margin, Ui};
use lasergrbl_core::laser_life::{self, OnConnectResult};
use lasergrbl_core::{CoreEvent, GrblCore, JogDirection, MacStatus, OpenFileOutcome, OverrideAction, RunDecision};
use lg_base::settings;
use lg_gcode::Firmware;
use lg_services::hotkeys::{keys, Actions, HotKeyEffect, JogDir};
use lg_services::project;

use crate::app_state::{AppState, REPAINT_INTERVAL};
use crate::forms::dialogs::emulator_ui::EmulatorWindow;
use crate::forms::dialogs::flash_grbl::{FlashGrbl, FlashOutcome};
use crate::forms::dialogs::hotkey_manager_form::{self, HotkeyOutcome};
use crate::forms::dialogs::issue_detector_form::IssueDetectorForm;
use crate::forms::dialogs::laser_selector::{LaserSelector, LaserSelectorOutcome};
use crate::forms::dialogs::laser_usage::LaserUsage;
use crate::forms::dialogs::ortur_wifi_config::{OrturWiFiConfig, WifiConfigOutcome};
use crate::forms::dialogs::ortur_wifi_config_prompt::{OrturWiFiConfigPrompt, WifiPromptOutcome};
use crate::forms::dialogs::resume_job_form::{ResumeJobForm, ResumeOutcome};
use crate::forms::dialogs::run_from_position_form::{RunFromPositionForm, RunFromPositionOutcome};
use crate::forms::dialogs::safety_countdown::{SafetyCountdown, SafetyOutcome};
use crate::forms::dialogs::settings_form::{SettingsForm, SettingsOutcome};
use crate::forms::dialogs::{discovery_form::DiscoveryForm, discovery_form::DiscoveryOutcome, grbl_config::GrblConfig};
use crate::forms::generators::cutting_test::CuttingTest;
use crate::forms::generators::power_vs_speed::PowerVsSpeedForm;
use crate::forms::generators::shake_test::ShakeTest;
use crate::forms::job_dock::{JobAction, JobDock};
use crate::forms::legal_disclaimer::{LegalDisclaimer, LegalOutcome};
use crate::forms::license::LicenseForm;
use crate::forms::main::connect_log::{ConnectLogAction, ConnectLogForm};
use crate::forms::main::dialogs::{AfterSafety, Overlay, OverlayResult};
use crate::forms::main::exception::{self, ExceptionDialog, ExceptionResult};
use crate::forms::main::jog::JogForm;
use crate::forms::main::menus::{self, MenuAction, MenuState};
use crate::forms::main::status_bar::{StatusAction, StatusBar};
use crate::forms::material::{default_presets, PresetEditor};
use crate::forms::preview_form::PreviewForm;
use crate::forms::raster::RasterImport;
use crate::forms::save_options::{SaveOptionForm, SaveOptionOutcome, SaveOptions};
use crate::forms::splash::SplashScreen;
use crate::ui::dialog::{filters, DialogResult, FileDialogOptions, MessageBox, MessageBoxButtons, MessageBoxHost, MessageBoxIcon, PendingFile};
use crate::ui::{tr, tr_fmt, Theme};

/// Continuations of message boxes.
#[derive(Clone, Debug)]
enum MsgTag {
    ExitAnyway,
    DisconnectAnyway,
    AbortProgram,
    UnlockStuck,
    LanguageRestart(String),
    RunMulti,
    Info,
}

/// What a pending file dialog is for.
enum FileTag {
    Open { append: bool },
    SaveProgram(SaveOptions),
    SaveProject,
    ComLog,
}

/// Why the classic raster import dialog is open.
struct ClassicCtx {
    /// Image of the dialog.
    path: PathBuf,
    /// The dialog appends to the program (otherwise it replaces it).
    append: bool,
    /// Opened from an image of the job list (cancel puts the image back), not for a `.lps` entry.
    from_item: bool,
}

/// `.lps` project restore loop (C# `OpenFile` project branch).
struct ProjectQueue {
    items: Vec<project::SettingsDict>,
    index: usize,
    temp: Option<PathBuf>,
}

/// Dev automation state (see `MainForm::dev_script`).
struct DevScript {
    active: bool,
    connect: bool,
    run: bool,
    shot: Option<PathBuf>,
    secs: f32,
    t0: Option<Instant>,
    did_connect: bool,
    did_run: bool,
    shot_requested: bool,
    /// `LASERGRBL_DEV_SEQ="6:hold,9:resume,10:cmd=G0 X999"`: timed actions (hold, resume, door, reset, emuhide, presets, license, layer=N, layeroff=N, fill=N, nofill=N, cross=N, nocross=N, pershape=N, nopershape=N, open=PATH, append=PATH, cmd=..).
    seq: Vec<(f32, String, bool)>,
}

impl DevScript {
    /// Read the env switches.
    fn from_env() -> Self {
        let shot = std::env::var_os("LASERGRBL_SHOT").map(PathBuf::from);
        let connect = std::env::var_os("LASERGRBL_DEV_CONNECT").is_some();
        let run = std::env::var_os("LASERGRBL_DEV_RUN").is_some();
        DevScript {
            active: shot.is_some() || connect || run || std::env::var_os("LASERGRBL_DEV_SEQ").is_some(),
            connect,
            run,
            secs: std::env::var("LASERGRBL_SHOT_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(8.0),
            shot,
            t0: None,
            did_connect: false,
            did_run: false,
            shot_requested: false,
            seq: std::env::var("LASERGRBL_DEV_SEQ")
                .unwrap_or_default()
                .split(',')
                .filter_map(|e| e.split_once(':').and_then(|(t, a)| Some((t.trim().parse().ok()?, a.trim().to_owned(), false))))
                .collect(),
        }
    }
}

/// Main window state.
pub struct MainForm {
    pub state: AppState,
    menu: MenuState,
    status: StatusBar,
    connect: ConnectLogForm,
    jog: JogForm,
    preview: PreviewForm,
    /// Right panel: the loaded files (SVG, images, G-code), the editor of the selected one and the job coordinator.
    job: JobDock,
    /// Image the classic import dialog is to open for (path, append), waiting until the image's block left the
    /// program.
    classic_wait: Option<(PathBuf, bool)>,
    /// Classic import dialog that is open now: what to do with its result.
    classic: Option<ClassicCtx>,
    /// Run was requested while the dock still had to generate G-code.
    run_after_apply: bool,
    emulator: std::sync::Arc<EmulatorWindow>,
    overlays: VecDeque<Overlay>,
    msgs: MessageBoxHost<MsgTag>,
    file: Option<(FileTag, PendingFile)>,
    connect_rx: Option<Receiver<Result<(), String>>>,
    project: Option<ProjectQueue>,
    args: Vec<String>,
    /// Startup (splash + legal) finished.
    started: bool,
    args_done: bool,
    prev_connected: bool,
    is_buffer_stuck: bool,
    prompt_wifi_shown: bool,
    multi_run_shown: bool,
    last_multi_check: Option<Instant>,
    exit_confirmed: bool,
    want_close: bool,
    restart_on_exit: bool,
    left_width: f32,
    params_width: f32,
    geometry: Option<(egui::Vec2, egui::Pos2, i32)>,
    title_set: bool,
    visible_checked: bool,
    pending_save: Option<SaveOptions>,
    want_open: Option<bool>,
    dev: DevScript,
}

impl MainForm {
    /// Build the window state: core, forms, splash and legal gate.
    ///
    /// `args`: command line arguments (without the program name).
    pub fn new(args: Vec<String>) -> Self {
        // presets auto-load at start
        let _ = default_presets();
        let firmware: Firmware = settings::get("Firmware Type", Firmware::Grbl);
        let state = AppState::new(firmware);
        let mut overlays = VecDeque::new();
        overlays.push_back(Overlay::Splash(SplashScreen::new()));
        if !settings::get("IsDisclamerAccepted", false) {
            overlays.push_back(Overlay::Legal(LegalDisclaimer::new()));
        }
        let left_width: f32 = settings::get("MainForm Splitter Position", 260.0f32);
        let params_width: f32 = settings::get("MainForm Params Width", 260.0f32);
        let job = JobDock::new(state.core.clone());
        MainForm {
            state,
            menu: MenuState::default(),
            status: StatusBar::default(),
            connect: ConnectLogForm::new(),
            jog: JogForm::new(),
            preview: PreviewForm::new(),
            job,
            classic_wait: None,
            classic: None,
            run_after_apply: false,
            emulator: EmulatorWindow::install(),
            overlays,
            msgs: MessageBoxHost::default(),
            file: None,
            connect_rx: None,
            project: None,
            args,
            started: false,
            args_done: false,
            prev_connected: false,
            is_buffer_stuck: false,
            prompt_wifi_shown: false,
            multi_run_shown: false,
            last_multi_check: None,
            exit_confirmed: false,
            want_close: false,
            restart_on_exit: false,
            left_width,
            params_width,
            geometry: None,
            title_set: false,
            visible_checked: false,
            pending_save: None,
            want_open: None,
            dev: DevScript::from_env(),
        }
    }

    // ------------------------------------------------------------------ frame

    /// Draw one frame.
    ///
    /// `ui`: root ui of the frame.
    pub fn ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        ctx.request_repaint_after(REPAINT_INTERVAL);
        self.window_housekeeping(&ctx);
        self.handle_close_request(&ctx);
        for r in exception::take_pending() {
            self.overlays.push_front(Overlay::Exception(ExceptionDialog::new(&r, Some(&self.state.core))));
        }
        if !self.started && self.overlays.is_empty() {
            self.started = true;
        }
        self.pump_events();
        // a modal form may write the program itself (classic import, generators): leave it alone meanwhile
        self.job.set_paused(!self.overlays.is_empty());
        self.poll_docks();
        self.poll_connect();
        self.poll_file(&ctx);
        if self.started && self.state.take_sincro_run() {
            self.request_run();
        }
        if self.started {
            self.timer_update();
        }
        let modal_open = !self.overlays.is_empty() || self.msgs.is_open();

        let th = Theme::current();
        let bar_frame = Frame::new().fill(th.form_back_color).inner_margin(Margin::symmetric(2, 1));
        let mut menu_actions = Vec::new();
        egui::Panel::top("main-menu").frame(bar_frame).show(ui, |ui| {
            menu_actions = menus::show_menu_bar(ui, &self.state.core, &mut self.menu);
        });
        let mut status_actions = Vec::new();
        egui::Panel::bottom("main-status").frame(Frame::new().fill(th.form_back_color).inner_margin(Margin::symmetric(4, 2))).show(ui, |ui| {
            status_actions = self.status.show(ui, &self.state.core, self.is_buffer_stuck);
        });
        let mut connect_actions = Vec::new();
        let panel = egui::Panel::left("main-left")
            .frame(Frame::new().fill(th.form_back_color).inner_margin(Margin::same(3)))
            .resizable(true)
            .default_size(self.left_width)
            .size_range(210.0..=700.0) // Panel1MinSize = 210
            .show(ui, |ui| {
                egui::Panel::bottom("main-jog").frame(Frame::NONE).show_separator_line(false).show(ui, |ui| {
                    self.jog.show(ui, &self.state.core);
                });
                connect_actions = self.connect.show(ui, &self.state.core);
            });
        // splitter moved: persist "MainForm Splitter Position"
        let w = panel.response.rect.width();
        if (w - self.left_width).abs() > 0.5 {
            self.left_width = w;
            settings::set("MainForm Splitter Position", w.round() as i32);
        }
        // Parameters dock: input is blocked while a modal is open
        let mut job_actions = Vec::new();
        let dock = egui::Panel::right("main-params")
            .frame(Frame::new().fill(th.form_back_color).inner_margin(Margin::same(3)))
            .resizable(true)
            .default_size(self.params_width)
            .size_range(200.0..=500.0)
            .show(ui, |ui| {
                ui.add_enabled_ui(!modal_open, |ui| {
                    job_actions = self.job.show(ui);
                });
            });
        // dock resized: persist "MainForm Params Width"
        let pw = dock.response.rect.width();
        if (pw - self.params_width).abs() > 0.5 {
            self.params_width = pw;
            settings::set("MainForm Params Width", pw.round() as i32);
        }
        let mut preview_actions = Vec::new();
        egui::CentralPanel::default().frame(Frame::new().fill(th.form_back_color)).show(ui, |ui| {
            preview_actions = self.preview.show(ui, &self.state.core, &mut self.state.custom_buttons, &self.state.hotkeys, self.job.layer_view());
        });
        let _ = preview_actions;
        for a in job_actions {
            match a {
                JobAction::OpenPresetEditor => self.overlays.push_back(Overlay::Material(PresetEditor::new(default_presets()))),
                JobAction::OpenClassic { id } => self.open_classic_from_item(id),
            }
        }

        self.emulator.show(&ctx);
        self.job.show_dialogs(&ctx);
        self.show_dialogs(&ctx);
        if self.started && !modal_open {
            self.hotkeys(&ctx);
        }
        for a in menu_actions {
            self.do_menu(&ctx, a);
        }
        for a in connect_actions {
            self.do_connect_log(a);
        }
        for a in status_actions {
            match a {
                StatusAction::UnlockFromStuck => self.msgs.push(
                    MsgTag::UnlockStuck,
                    MessageBox::new(tr("Strings.WarnBufferStuckUnlockText"), tr("Strings.WarnBufferStuckUnlockTitle")).buttons(MessageBoxButtons::OkCancel).icon(MessageBoxIcon::Information),
                ),
                StatusAction::LaserUsage => self.overlays.push_back(Overlay::LaserUsage(Box::new(LaserUsage::open()))),
            }
        }
        if let Some(append) = self.want_open.take() {
            self.open_file_dialog(&ctx, append);
        }
        self.dev_script(&ctx);
        if self.want_close {
            self.want_close = false;
            self.exit_confirmed = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Title, geometry tracking and off-screen guard (`MainForm_Load` / `RefreshFormTitle`).
    fn window_housekeeping(&mut self, ctx: &egui::Context) {
        if !self.title_set {
            self.title_set = true;
            let suffix = match self.state.core.firmware() {
                Firmware::Grbl => None,
                other => Some(lasergrbl_core::NamedEnum::variant_name(&other).to_string()),
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(crate::form_title(suffix.as_deref())));
        }
        let (inner, outer, maxi, mini, monitor) = ctx.input(|i| {
            let v = i.viewport();
            (v.inner_rect, v.outer_rect, v.maximized.unwrap_or(false), v.minimized.unwrap_or(false), v.monitor_size)
        });
        if let (Some(inner), Some(outer)) = (inner, outer) {
            let state = if mini { 1 } else if maxi { 2 } else { 0 };
            // keep the last normal rectangle when maximized (WinForms RestoreBounds)
            if state == 0 || self.geometry.is_none() {
                self.geometry = Some((inner.size(), outer.min, state));
            } else if let Some(g) = self.geometry.as_mut() {
                g.2 = state;
            }
            // IsVisibleOnAnyScreen
            if !self.visible_checked {
                self.visible_checked = true;
                if let Some(m) = monitor {
                    let off = outer.max.x < 0.0 || outer.max.y < 0.0 || outer.min.x > m.x || outer.min.y > m.y;
                    if off {
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1024.0, 768.0)));
                        if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
                            ctx.send_viewport_cmd(cmd);
                        }
                    }
                }
            }
        }
    }

    /// Close box: ask when a job runs (`MainFormFormClosing`).
    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) || self.exit_confirmed {
            return;
        }
        // splash/legal still open and declined, or plain exit
        if self.state.core.in_program() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self.msgs.is_open() {
                self.msgs.push(MsgTag::ExitAnyway, MessageBox::new(tr("Strings.ExitAnyway"), tr("Strings.WarnMessageBoxHeader")).buttons(MessageBoxButtons::YesNo).icon(MessageBoxIcon::Warning).default_button(1));
            }
        }
    }

    /// Cleanup at exit (`MainFormFormClosing` + `Program.Main` tail).
    pub fn on_exit(&mut self) {
        lasergrbl_core::sincro_start::stop_listen();
        self.state.core.exiting();
        if let Some((size, pos, st)) = self.geometry {
            settings::set("Mainform Size and Position", serde_json::json!([[size.x, size.y], [pos.x, pos.y], st]));
        }
        let core = &self.state.core;
        let uptime = lg_dotnet::TimeSpan::from_seconds(self.state.started.elapsed().as_secs_f64());
        let ver = core.grbl_version();
        let vendor = ver.as_ref().map(|v| v.vendor_name());
        lasergrbl_core::usage_stats::save_file(&core.usage_counters(), ver.as_ref(), vendor, uptime, &lasergrbl_core::usage_stats::data_file());
        settings::exiting();
        self.state.custom_buttons.save_file();
        lasergrbl_core::emulator::WebSocketEmulator::stop();
        lg_services::autotrace::cleanup_tmp_folder();
        if lg_base::logger::com::enabled() {
            if let Some(m) = lg_base::logger::com::stop_log() {
                lg_base::logger::log_message("ComLog", &m);
            }
        }
        lg_base::logger::stop();
        if self.restart_on_exit {
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe).args(&self.args).spawn();
            }
        }
    }

    /// Dev/test hook (env `LASERGRBL_DEV_CONNECT`, `LASERGRBL_DEV_RUN`, `LASERGRBL_SHOT`, `LASERGRBL_SHOT_SECS`):
    /// connect, run the loaded file, save a window screenshot, optionally exit. Not part of the C# app.
    fn dev_script(&mut self, ctx: &egui::Context) {
        if !self.dev.active || !self.started {
            return;
        }
        let t = self.dev.t0.get_or_insert_with(Instant::now).elapsed().as_secs_f32();
        let core = self.state.core.clone();
        if self.dev.connect && !self.dev.did_connect && t > 1.5 && self.overlays.is_empty() {
            self.dev.did_connect = true;
            self.connect_now();
        }
        if self.dev.run && !self.dev.did_run && core.machine_status() == MacStatus::Idle && core.has_program() && t > 3.0 && self.overlays.is_empty() {
            self.dev.did_run = true;
            self.request_run();
        }
        // timed actions
        let mut due = Vec::new();
        for (at, act, done) in self.dev.seq.iter_mut() {
            if !*done && t >= *at {
                *done = true;
                due.push(act.clone());
            }
        }
        for act in due {
            match act.as_str() {
                "hold" => core.feed_hold(false),
                "resume" => core.cycle_start_resume(false),
                "door" => core.safety_door(),
                "reset" => core.grbl_reset(),
                "emuhide" => lasergrbl_core::emulator::EmulatorUiHost::hide_ui(&*self.emulator),
                "presets" => self.overlays.push_back(Overlay::Material(PresetEditor::new(default_presets()))),
                "license" => self.overlays.push_back(Overlay::License(LicenseForm::new())),
                a => {
                    if let Some(n) = a.strip_prefix("layer=") {
                        self.job.dev_select_layer(n.parse().unwrap_or(0), true);
                    } else if let Some(n) = a.strip_prefix("layeroff=") {
                        self.job.dev_select_layer(n.parse().unwrap_or(0), false);
                    } else if let Some(n) = a.strip_prefix("fill=") {
                        self.job.dev_set_fill(n.parse().unwrap_or(0), Some(true), None, None);
                    } else if let Some(n) = a.strip_prefix("nofill=") {
                        self.job.dev_set_fill(n.parse().unwrap_or(0), Some(false), None, None);
                    } else if let Some(n) = a.strip_prefix("nocross=") {
                        self.job.dev_set_fill(n.parse().unwrap_or(0), None, Some(false), None);
                    } else if let Some(n) = a.strip_prefix("cross=") {
                        self.job.dev_set_fill(n.parse().unwrap_or(0), None, Some(true), None);
                    } else if let Some(n) = a.strip_prefix("nopershape=") {
                        self.job.dev_set_fill(n.parse().unwrap_or(0), None, None, Some(false));
                    } else if let Some(n) = a.strip_prefix("pershape=") {
                        self.job.dev_set_fill(n.parse().unwrap_or(0), None, None, Some(true));
                    } else if let Some(p) = a.strip_prefix("open=") {
                        self.open_path(Path::new(p), false);
                    } else if let Some(p) = a.strip_prefix("append=") {
                        self.open_path(Path::new(p), true);
                    } else if let Some(c) = a.strip_prefix("cmd=") {
                        core.enqueue_command(&lg_gcode::GrblCommand::with_options(c, 0, true));
                    }
                }
            }
        }
        if let Some(path) = self.dev.shot.clone() {
            if t > self.dev.secs && !self.dev.shot_requested {
                self.dev.shot_requested = true;
                let c2 = ctx.clone();
                ctx.request_screenshot(move |img| {
                    let raw: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                    if let Some(buf) = image::RgbaImage::from_raw(img.width() as u32, img.height() as u32, raw) {
                        let _ = buf.save(&path);
                    }
                    c2.send_viewport_cmd(egui::ViewportCommand::Close);
                });
            }
        }
    }

    // ------------------------------------------------------------------ events and timers

    /// Dispatch the core events (C# event handlers of `MainForm`, `ConnectLogForm`, `JogForm`).
    fn pump_events(&mut self) {
        for ev in self.state.drain_events() {
            self.preview.on_core_event(&ev, &self.state.core);
            match ev {
                CoreEvent::IssueDetected(issue) => {
                    if IssueDetectorForm::enabled() {
                        self.overlays.push_back(Overlay::Issue(IssueDetectorForm::open(issue)));
                    }
                }
                CoreEvent::MachineStatusChanged => self.on_machine_status(),
                CoreEvent::FileLoaded { filename, .. } => {
                    self.connect.on_file_loaded(&filename);
                    self.status.on_file_loaded(&self.state.core);
                }
                CoreEvent::LoopCountChanged(d) => self.connect.on_loop_count_changed(d),
                CoreEvent::JogStateChanged(j) => self.jog.home_visible = !j,
                _ => {}
            }
        }
    }

    /// `OnMachineStatus`: failed-connection help, laser life on (dis)connect.
    fn on_machine_status(&mut self) {
        let core = &self.state.core;
        if core.machine_status() == MacStatus::Disconnected && core.failed_connection_count() >= 3 {
            let wt: lasergrbl_core::com::WrapperType = settings::get("ComWrapper Protocol", lasergrbl_core::com::WrapperType::UsbSerial);
            use lasergrbl_core::com::WrapperType as W;
            let url = match wt {
                W::UsbSerial | W::UsbSerial2 | W::RJCPSerial => Some("https://lasergrbl.com/usage/arduino-connection/"),
                W::Telnet | W::LaserWebESP8266 => Some("https://lasergrbl.com/usage/wifi-with-esp8266/"),
                _ => None,
            };
            if let Some(url) = url {
                // Help button opens the URL (C# `MessageBox.Show(.., 0, url)`)
                self.msgs.push(MsgTag::Info, MessageBox::new(tr("Strings.ProblemConnectingText"), tr("Strings.ProblemConnectingTitle")).icon(MessageBoxIcon::Information).help_url(url));
            }
        }
        let connected = core.is_connected();
        if connected && !self.prev_connected {
            match laser_life::on_connect() {
                OnConnectResult::Selected(_) => {}
                OnConnectResult::NeedSelection(_) => self.overlays.push_back(Overlay::LaserSelector(LaserSelector::open())),
            }
        }
        if !connected && self.prev_connected {
            laser_life::on_disconnect();
        }
        self.prev_connected = connected;
    }

    /// Per-frame part of `UpdateTimer_Tick`: buffer-stuck state, multi instance probe, WiFi prompt, args.
    fn timer_update(&mut self) {
        let core = self.state.core.clone();
        self.is_buffer_stuck = core.is_buffer_stuck();
        // MultipleInstanceTimer (5 s)
        if self.last_multi_check.is_none_or(|t| t.elapsed() > Duration::from_secs(5)) {
            self.last_multi_check = Some(Instant::now());
            self.menu.run_multi_visible = lasergrbl_core::sincro_start::running() && lasergrbl_core::sincro_start::registered_ports().len() > 1;
        }
        // PromptForWiFi
        let wt: lasergrbl_core::com::WrapperType = settings::get("ComWrapper Protocol", lasergrbl_core::com::WrapperType::UsbSerial);
        let wifi_conf = matches!(wt, lasergrbl_core::com::WrapperType::LaserWebESP8266 | lasergrbl_core::com::WrapperType::Telnet);
        if !self.prompt_wifi_shown && !OrturWiFiConfigPrompt::suppressed() && core.grbl_version().is_some_and(|v| v.is_lucky_wifi()) && !wifi_conf && core.can_read_write_config() && self.overlays.is_empty() {
            self.prompt_wifi_shown = true;
            self.overlays.push_back(Overlay::WifiPrompt(OrturWiFiConfigPrompt::open(&core)));
        }
        // ManageCommandLineArgs once the startup dialogs are gone
        if !self.args_done && self.overlays.is_empty() {
            self.args_done = true;
            self.manage_command_line_args();
        }
        self.pump_project();
    }

    /// `ManageCommandLineArgs`: one argument = file to open (`.zbn` = custom button import).
    fn manage_command_line_args(&mut self) {
        if self.args.len() != 1 {
            return;
        }
        let f = PathBuf::from(&self.args[0]);
        if !f.is_file() {
            return;
        }
        if f.extension().is_some_and(|e| e.eq_ignore_ascii_case("zbn")) {
            self.preview.import_button(Some(f));
        } else if self.state.core.can_load_new_file() {
            self.open_path(&f, false);
        } else {
            self.msgs.push(MsgTag::Info, MessageBox::new(tr("Strings.MsgboxCannotOpenFileNow"), ""));
        }
    }

    // ------------------------------------------------------------------ dialogs

    /// Show the front message box or overlay and act on its result.
    fn show_dialogs(&mut self, ctx: &egui::Context) {
        if let Some((tag, res)) = self.msgs.show(ctx) {
            self.msg_answered(tag, res);
            return;
        }
        if self.msgs.is_open() {
            return;
        }
        let Some(front) = self.overlays.front_mut() else { return };
        let is_material = matches!(front, Overlay::Material(_));
        if let Some(res) = front.show(ctx) {
            self.overlays.pop_front();
            // presets may have changed
            if is_material {
                self.job.refresh_presets();
            }
            self.overlay_closed(res);
        }
    }

    /// Continuation of a message box.
    fn msg_answered(&mut self, tag: MsgTag, res: DialogResult) {
        let yes = matches!(res, DialogResult::Yes | DialogResult::Ok);
        match tag {
            MsgTag::ExitAnyway if yes => self.want_close = true,
            MsgTag::DisconnectAnyway if yes => {
                let _ = self.state.core.close_com(true);
            }
            MsgTag::AbortProgram if yes => self.state.core.abort_program(),
            MsgTag::UnlockStuck if yes => self.state.core.unlock_from_buffer_stuck(false),
            MsgTag::LanguageRestart(culture) => {
                // SetLanguage stored first, restart only on OK
                let _ = culture;
                if yes {
                    self.restart_on_exit = true;
                    self.want_close = true;
                }
            }
            MsgTag::RunMulti if yes => {
                lasergrbl_core::sincro_start::signal();
                self.multi_run_shown = true;
            }
            _ => {}
        }
    }

    /// Continuation of an overlay form.
    fn overlay_closed(&mut self, res: OverlayResult) {
        let core = self.state.core.clone();
        match res {
            OverlayResult::SplashDone => {}
            OverlayResult::Legal(LegalOutcome::Accepted) => settings::set("IsDisclamerAccepted", true),
            OverlayResult::Legal(LegalOutcome::Declined) => {
                self.overlays.clear();
                self.want_close = true;
            }
            OverlayResult::Settings(SettingsOutcome::Saved { restart }) => {
                if restart {
                    self.restart_on_exit = true;
                    self.want_close = true;
                }
            }
            OverlayResult::Hotkeys(HotkeyOutcome::Saved(list)) => hotkey_manager_form::apply_saved(&mut self.state.hotkeys, list),
            OverlayResult::SaveOptions(SaveOptionOutcome::Save(o)) => self.pick_save_program(o),
            OverlayResult::Raster(o) => {
                // classic dialog: its commands become a static file, a cancelled one gives the image back
                if let Some(ctx) = self.classic.take() {
                    self.classic_done(ctx, o.generated);
                }
                // raster_file is counted by the core in open_file
                core.update_usage_counters(|c| {
                    c.dithering += o.usage.dithering;
                    c.line2line += o.usage.line2line;
                    c.vectorization += o.usage.vectorization;
                    c.centerline += o.usage.centerline;
                    c.passthrough += o.usage.passthrough;
                });
            }
            OverlayResult::Resume(ResumeOutcome::Ok { position, homing, wco }) => {
                let next = if position == 0 { AfterSafety::ResumeFromStart { homing } } else { AfterSafety::Continue { position, homing, setwco: wco } };
                if position >= 0 {
                    self.countdown_then(next);
                }
            }
            OverlayResult::RunFromPosition(RunFromPositionOutcome::Ok { position, homing }) => {
                if position >= 0 {
                    self.countdown_then(AfterSafety::FromPosition { position, homing });
                }
            }
            OverlayResult::Safety(SafetyOutcome::Go, next) => self.execute_run(next),
            OverlayResult::LaserSelector(LaserSelectorOutcome::Selected(g)) => laser_life::select(g.as_deref()),
            OverlayResult::LaserSelector(LaserSelectorOutcome::Cancelled) => laser_life::select(None),
            OverlayResult::Flash(FlashOutcome::Flashed { .. }) | OverlayResult::Flash(FlashOutcome::Cancelled) => {}
            OverlayResult::Discovery(DiscoveryOutcome::Address(a)) => {
                if self.connect.config_from_discovery(Some(a), &core) {
                    self.connect_now();
                }
            }
            OverlayResult::WifiPrompt(WifiPromptOutcome::Yes) => self.show_wifi_config(),
            OverlayResult::OrturWifi(WifiConfigOutcome::ConnectViaWifi { ip }) => {
                // ShowWiFiConfig tail: switch protocol to Telnet and close the USB link
                let port = if core.is_ortur_board() { GrblCore::configuration().telnet_port() } else if core.grbl_version().is_some_and(|v| v.is_longer()) { Some("8847".to_owned()) } else { None };
                if let Some(p) = port {
                    self.connect.config_from_wifi_form(format!("{ip}:{p}"));
                }
                settings::set("ComWrapper Protocol", lasergrbl_core::com::WrapperType::Telnet);
                let _ = core.close_com(true);
            }
            OverlayResult::Exception(ExceptionResult::Abort) => {
                self.want_close = true;
            }
            _ => {}
        }
    }

    /// `ShowWiFiConfig`: open the Ortur/Longer WiFi configurator.
    fn show_wifi_config(&mut self) {
        self.overlays.push_back(Overlay::OrturWifi(OrturWiFiConfig::open(&self.state.core)));
    }

    // ------------------------------------------------------------------ connect / disconnect

    /// `Core.OpenCom()` on a worker thread; the error box appears when it returns.
    fn connect_now(&mut self) {
        if self.connect_rx.is_some() {
            return;
        }
        let core = self.state.core.clone();
        let (tx, rx) = crossbeam_channel::bounded(1);
        std::thread::spawn(move || {
            let _ = tx.send(core.open_com());
        });
        self.connect_rx = Some(rx);
    }

    /// Check the connect worker.
    fn poll_connect(&mut self) {
        let Some(rx) = &self.connect_rx else { return };
        match rx.try_recv() {
            Ok(r) => {
                self.connect_rx = None;
                if let Err(e) = r {
                    self.msgs.push(MsgTag::Info, MessageBox::new(e, tr("Strings.BoxConnectErrorTitle")).icon(MessageBoxIcon::Error));
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.connect_rx = None,
        }
    }

    /// Disconnect with the "transfer in progress" question.
    fn request_disconnect(&mut self) {
        if self.state.core.in_program() {
            self.msgs.push(MsgTag::DisconnectAnyway, MessageBox::new(tr("Strings.DisconnectAnyway"), tr("Strings.WarnMessageBoxHeader")).buttons(MessageBoxButtons::YesNo).icon(MessageBoxIcon::Warning).default_button(1));
        } else {
            let _ = self.state.core.close_com(true);
        }
    }

    // ------------------------------------------------------------------ files

    /// Open the file chooser (`OpenFileDialog` of `GrblCore.OpenFile`).
    fn open_file_dialog(&mut self, ctx: &egui::Context, append: bool) {
        if !self.state.core.can_load_new_file() || self.file.is_some() {
            return;
        }
        let last: Option<String> = settings::get("Core.LastOpenFile", None);
        let mut o = FileDialogOptions { filter: Some(filters::OPEN_ANY.into()), ..Default::default() };
        if let Some(l) = last.map(PathBuf::from).filter(|p| p.exists()) {
            o.directory = l.parent().map(Path::to_path_buf);
            o.file_name = l.file_name().map(|s| s.to_string_lossy().into_owned());
        }
        self.file = Some((FileTag::Open { append }, PendingFile::open(ctx, o)));
    }

    /// Initial save dialog options: last opened file name with a new extension.
    fn save_dialog_options(ext: &str, filter: &str) -> FileDialogOptions {
        let mut o = FileDialogOptions { filter: Some(filter.into()), ..Default::default() };
        let last: Option<String> = settings::get("Core.LastOpenFile", None);
        if let Some(l) = last.map(PathBuf::from) {
            o.directory = l.parent().map(Path::to_path_buf);
            o.file_name = l.file_stem().map(|s| format!("{}.{ext}", s.to_string_lossy()));
        }
        o
    }

    /// `SaveProgram` target chooser.
    fn pick_save_program(&mut self, opts: SaveOptions) {
        if !self.state.core.has_program() || self.file.is_some() {
            return;
        }
        self.pending_save = Some(opts);
    }

    /// Poll the pending file dialog and act on its result.
    fn poll_file(&mut self, ctx: &egui::Context) {
        if let Some(opts) = self.pending_save.take().filter(|_| self.state.core.has_program() && self.file.is_none()) {
            self.file = Some((FileTag::SaveProgram(opts), PendingFile::save(ctx, Self::save_dialog_options("nc", filters::GCODE_SAVE_NC))));
        }
        let Some((_, pf)) = self.file.as_mut() else { return };
        let Some(result) = pf.poll() else { return };
        let (tag, _) = self.file.take().expect("pending file");
        let Some(path) = result else { return };
        match tag {
            FileTag::Open { append } => {
                if !append {
                    project::clear_settings();
                }
                self.open_path(&path, append);
            }
            FileTag::SaveProgram(o) => {
                let path = with_ext(path, "nc");
                if let Err(e) = self.state.core.save_program(&path, o.header, o.footer, o.between, o.cycles, o.use_lf_line_endings) {
                    lg_base::logger::log_exception("SaveProgram", &e);
                }
            }
            FileTag::SaveProject => {
                let path = with_ext(path, "lps");
                if let Err(e) = project::store_settings(&path.to_string_lossy()) {
                    lg_base::logger::log_exception("SaveProject", &e);
                }
            }
            FileTag::ComLog => {
                let m = lg_base::logger::com::start_log(with_ext(path, "txt"));
                lg_base::logger::log_message("ComLog", &m);
            }
        }
    }

    /// `GrblCore.OpenFile(filename, append)` dispatch.
    ///
    /// `path`: file to open. `append`: keep the current program.
    fn open_path(&mut self, path: &Path, append: bool) {
        let outcome = self.state.core.open_file(path, append);
        self.open_outcome(outcome, append, false);
    }

    /// Act on what `open_file` decided.
    ///
    /// `outcome`: result of `open_file`. `append`: keep the current program. `from_project`: settings were
    /// just restored from a `.lps` entry (the dock reloads its values from them).
    fn open_outcome(&mut self, outcome: OpenFileOutcome, append: bool, from_project: bool) {
        let file = self.state.core.loaded_file();
        match outcome {
            // project entries carry the classic dialog settings (tool, vectorize, ...): classic dialog
            OpenFileOutcome::Raster(p) if from_project => match RasterImport::open(file, &p, append) {
                Ok(r) => {
                    self.classic = Some(ClassicCtx { path: p, append, from_item: false });
                    self.overlays.push_back(Overlay::Raster(Box::new(r)));
                }
                Err(e) => lg_base::logger::log_exception("RasterImport", &e),
            },
            // a plain raster image becomes an image item (a plain open replaces the files, an append adds one)
            OpenFileOutcome::Raster(p) => match self.job.add_image(&p, append) {
                Ok(_) => self.state.core.update_usage_counters(|c| c.line2line += 1),
                Err(e) => {
                    lg_base::logger::log_exception("ImageDock", &e);
                    self.msgs.push(MsgTag::Info, MessageBox::new(tr_fmt("ImageDock.LoadFailed", &[&e]), "Error").icon(MessageBoxIcon::Error));
                }
            },
            OpenFileOutcome::Svg(p) => {
                self.job.add_svg(&p, from_project, append);
            }
            OpenFileOutcome::Project(p) => match project::load_project(&p.to_string_lossy()) {
                Ok(items) => self.project = Some(ProjectQueue { items, index: 0, temp: None }),
                Err(e) => lg_base::logger::log_exception("OpenFile", &e),
            },
            OpenFileOutcome::Unsupported => self.msgs.push(MsgTag::Info, MessageBox::new(tr("Strings.UnsupportedFiletype"), "Error").icon(MessageBoxIcon::Error)),
            // the core loads G-code on its own thread: the new commands become a static file when it is done
            OpenFileOutcome::GCodeLoading => {
                if !append {
                    self.job.reset();
                }
                let last: Option<String> = settings::get("Core.LastOpenFile", None);
                let name = last.map(|l| file_name_of(Path::new(&l))).unwrap_or_default();
                self.job.expect_static(&name);
            }
            OpenFileOutcome::Busy => {}
        }
    }

    /// Take the result of the classic raster dialog into the file list.
    ///
    /// `ctx`: why the dialog was open. `generated`: it wrote G-code into the program.
    fn classic_done(&mut self, ctx: ClassicCtx, generated: bool) {
        if generated {
            // a dialog that replaced the program leaves nothing of the old files
            if !ctx.append {
                self.job.reset();
            }
            self.job.add_static_tail(&file_name_of(&ctx.path));
        } else if ctx.from_item {
            self.reopen_in_job(&ctx.path);
        }
    }

    /// Project loop: restore settings of the next entry once the previous dialog has closed.
    fn pump_project(&mut self) {
        // SVG generation is async: next entry would hit "busy"
        if !self.overlays.is_empty() || self.msgs.is_open() || self.job.job_running() || self.job.pending_auto() || self.classic_wait.is_some() || !self.state.core.can_load_new_file() {
            return;
        }
        let Some(mut q) = self.project.take() else { return };
        if let Some(t) = q.temp.take() {
            let _ = std::fs::remove_file(t);
        }
        if q.index >= q.items.len() {
            return;
        }
        let item = q.items[q.index].clone();
        let name = item.get("ImageName").and_then(|v| v.as_str()).map(|n| Path::new(n).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default());
        let b64 = item.get("ImageBase64").and_then(|v| v.as_str()).map(str::to_owned);
        if let (Some(name), Some(b64)) = (name, b64) {
            let img = std::env::temp_dir().join(name);
            if project::save_image(&b64, &img.to_string_lossy()).is_ok() {
                for (k, v) in item.iter() {
                    if k != "ImageName" && k != "ImageBase64" {
                        if let Some(j) = project_to_setting(v) {
                            settings::set(k, j);
                        }
                    }
                }
                let append = q.index > 0;
                let outcome = self.state.core.open_file(&img, append);
                q.temp = Some(img);
                self.open_outcome(outcome, append, true);
            }
        }
        q.index += 1;
        self.project = Some(q);
    }

    // ------------------------------------------------------------------ run flow

    /// `RunProgram(parent)`: resume question first, then the countdown (C# order).
    fn request_run(&mut self) {
        let core = self.state.core.clone();
        // edits not generated yet (auto regen running or failed): generate first, run when done
        if self.job.needs_apply() && core.is_connected() {
            // a file with invalid values: the program is stale and cannot be regenerated, do not run it
            if self.job.is_blocked() {
                return;
            }
            if self.job.is_dirty_idle() {
                if core.can_load_new_file() && self.job.apply() {
                    self.run_after_apply = true;
                }
            } else {
                self.run_after_apply = true;
            }
            return;
        }
        if !core.can_send_file() {
            return;
        }
        if core.run_would_ask_resume() {
            if let RunDecision::AskResume(info) = core.run_program() {
                self.overlays.push_back(Overlay::Resume(ResumeJobForm::open(info)));
            }
        } else {
            self.countdown_then(AfterSafety::Run);
        }
    }

    /// Check the job coordinator; start the deferred run when every file is generated, open the classic dialog
    /// once its image left the program.
    fn poll_docks(&mut self) {
        if let Some(ok) = self.job.poll() {
            if self.run_after_apply {
                if !ok {
                    self.run_after_apply = false;
                } else if self.job.needs_apply() {
                    // more files to generate: go on, give up when the rest cannot be generated
                    if self.job.is_blocked() || (self.job.is_dirty_idle() && !self.job.apply()) {
                        self.run_after_apply = false;
                    }
                } else {
                    self.run_after_apply = false;
                    self.request_run();
                }
            }
        }
        self.start_classic_dialog();
    }

    /// Image item "Classic import dialog…": its block is cut out first, the dialog opens afterwards.
    ///
    /// `id`: item id of the image.
    fn open_classic_from_item(&mut self, id: u64) {
        if self.classic_wait.is_some() || self.classic.is_some() {
            return;
        }
        if let Some(w) = self.job.begin_classic(id) {
            self.classic_wait = Some(w);
        }
    }

    /// Open the waiting classic dialog when the block removal is through.
    fn start_classic_dialog(&mut self) {
        if self.job.job_running() {
            return;
        }
        let Some((path, append)) = self.classic_wait.take() else { return };
        match RasterImport::open(self.state.core.loaded_file(), &path, append) {
            Ok(r) => {
                self.classic = Some(ClassicCtx { path, append, from_item: true });
                self.overlays.push_back(Overlay::Raster(Box::new(r)));
            }
            Err(e) => {
                lg_base::logger::log_exception("RasterImport", &e);
                self.reopen_in_job(&path);
            }
        }
    }

    /// Put an image back into the file list (classic dialog cancelled or failed to open).
    ///
    /// `path`: image file.
    fn reopen_in_job(&mut self, path: &Path) {
        if let Err(e) = self.job.add_image(path, true) {
            lg_base::logger::log_exception("ImageDock", &e);
        }
    }

    /// `SafetyCountdown.CanGo()` then `next`.
    fn countdown_then(&mut self, next: AfterSafety) {
        if SafetyCountdown::needed() {
            self.overlays.push_back(Overlay::Safety(SafetyCountdown::open(), next));
        } else {
            self.execute_run(next);
        }
    }

    /// Start the job after the countdown.
    fn execute_run(&mut self, next: AfterSafety) {
        let core = self.state.core.clone();
        match next {
            AfterSafety::Run => {
                if let RunDecision::AskResume(info) = core.run_program() {
                    self.overlays.push_back(Overlay::Resume(ResumeJobForm::open(info)));
                }
            }
            AfterSafety::ResumeFromStart { homing } => core.run_program_from_start_user(homing),
            AfterSafety::Continue { position, homing, setwco } => {
                if let Err(e) = core.continue_program(position, homing, setwco) {
                    lg_base::logger::log_exception("ResumeProgram", &e);
                }
            }
            AfterSafety::FromPosition { position, homing } => {
                if let Err(e) = core.continue_program(position, homing, false) {
                    lg_base::logger::log_exception("RunFromPosition", &e);
                }
            }
        }
    }

    // ------------------------------------------------------------------ actions

    /// Connect/log form requests.
    fn do_connect_log(&mut self, a: ConnectLogAction) {
        match a {
            ConnectLogAction::Connect => self.connect_now(),
            ConnectLogAction::Disconnect => self.request_disconnect(),
            ConnectLogAction::OpenFile => self.want_open = Some(false),
            ConnectLogAction::Run => self.request_run(),
            ConnectLogAction::Abort => self.msgs.push(
                MsgTag::AbortProgram,
                MessageBox::new(tr("Strings.BoxAbortProgramConfirm"), tr("Strings.WarnMessageBoxHeader")).buttons(MessageBoxButtons::YesNo).icon(MessageBoxIcon::Warning).default_button(1),
            ),
        }
    }

    /// Menu clicks (`MainForm` `*_Click` handlers).
    fn do_menu(&mut self, ctx: &egui::Context, a: MenuAction) {
        let core = self.state.core.clone();
        match a {
            MenuAction::Connect => self.connect_now(),
            MenuAction::Disconnect => self.request_disconnect(),
            MenuAction::WifiDiscovery => self.overlays.push_back(Overlay::Discovery(DiscoveryForm::open(&core))),
            MenuAction::GrblReset => core.grbl_reset(),
            MenuAction::GoHome => core.send_homing_command(),
            MenuAction::Unlock => core.send_unlock_command(),
            MenuAction::GrblConfig => self.overlays.push_back(Overlay::GrblConfig(GrblConfig::open(&core))),
            MenuAction::Settings => self.overlays.push_back(Overlay::Settings(SettingsForm::open(&core))),
            MenuAction::MaterialDb => self.overlays.push_back(Overlay::Material(PresetEditor::new(default_presets()))),
            MenuAction::LaserUsage => self.overlays.push_back(Overlay::LaserUsage(Box::new(LaserUsage::open()))),
            MenuAction::Hotkeys => self.overlays.push_back(Overlay::Hotkeys(hotkey_manager_form::HotkeyManagerForm::open(&self.state.hotkeys))),
            MenuAction::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            MenuAction::FileOpen => {
                project::clear_settings();
                self.open_file_dialog(ctx, false);
            }
            MenuAction::FileAppend => self.open_file_dialog(ctx, true),
            MenuAction::ReopenFile => {
                project::clear_settings();
                if let Some(o) = core.reopen_file() {
                    self.open_outcome(o, false, false);
                }
            }
            MenuAction::SaveProgram => self.pending_save = Some(SaveOptions { header: false, footer: false, between: false, cycles: 1, use_lf_line_endings: false }),
            MenuAction::AdvancedSave => self.overlays.push_back(Overlay::SaveOptions(SaveOptionForm::new())),
            MenuAction::SaveProject => {
                if core.has_program() && self.file.is_none() {
                    self.file = Some((FileTag::SaveProject, PendingFile::save(ctx, Self::save_dialog_options("lps", filters::PROJECT_SAVE))));
                }
            }
            MenuAction::FileSend => self.request_run(),
            MenuAction::StartFromPosition => {
                if let Some(info) = core.run_from_position_info() {
                    self.overlays.push_back(Overlay::RunFromPosition(RunFromPositionForm::open(&info)));
                }
            }
            MenuAction::RunMulti => {
                if self.multi_run_shown {
                    lasergrbl_core::sincro_start::signal();
                } else {
                    self.msgs.push(MsgTag::RunMulti, MessageBox::new(format!("Warning: this command will start/resume all job in any running {} instance!", lg_base::APP_NAME), "Warning").buttons(MessageBoxButtons::OkCancel).icon(MessageBoxIcon::Warning));
                }
            }
            MenuAction::PowerVsSpeed => self.overlays.push_back(Overlay::PowerVsSpeed(Box::new(PowerVsSpeedForm::new(core.loaded_file())))),
            MenuAction::CuttingTest => self.overlays.push_back(Overlay::CuttingTest(Box::new(CuttingTest::new(core.loaded_file())))),
            MenuAction::ShakeTest => self.overlays.push_back(Overlay::ShakeTest(Box::new(ShakeTest::new(core.loaded_file())))),
            MenuAction::AccuracyTest => {
                let p = lg_base::paths::resources_dir().join("generator").join("LaserGRBL-accuracy-test-file.svg");
                self.open_path(&p, false);
            }
            MenuAction::SetScheme(s) => {
                Theme::switch(ctx, s);
                self.preview.on_color_change();
                self.connect.on_color_change();
            }
            MenuAction::AutoSize => core.auto_size_drawing(),
            MenuAction::ZoomIn => core.zoom_in_drawing(),
            MenuAction::ZoomOut => core.zoom_out_drawing(),
            MenuAction::SetLanguage(c) => {
                crate::ui::i18n::set_language(Some(&c));
                self.msgs.push(MsgTag::LanguageRestart(c), MessageBox::new(tr("Strings.LanguageRequireRestartNow"), tr("Strings.LanguageRequireRestart")).buttons(MessageBoxButtons::OkCancel));
            }
            MenuAction::InstallDriver => {
                // Linux kernels ship ch341; no installer exists
                self.msgs.push(MsgTag::Info, MessageBox::new("The CH340/CH341 USB serial driver is built into the Linux kernel: no driver installation is needed.\nIf the port does not appear, check that your user is in the 'dialout' group.", "CH340").icon(MessageBoxIcon::Information));
            }
            MenuAction::FlashFirmware => {
                if core.machine_status() == MacStatus::Disconnected {
                    self.overlays.push_back(Overlay::Flash(FlashGrbl::open()));
                }
            }
            MenuAction::ConfigureWifi => self.show_wifi_config(),
            MenuAction::OpenSessionLog => {
                if lg_base::logger::exist_log() {
                    lg_base::logger::show_log();
                }
            }
            MenuAction::ToggleExtendedLog => {
                if lg_base::logger::com::enabled() {
                    if let Some(m) = lg_base::logger::com::stop_log() {
                        lg_base::logger::log_message("ComLog", &m);
                    }
                } else if self.file.is_none() {
                    let mut o = FileDialogOptions { filter: Some(filters::COMM_LOG.into()), title: Some("Select extended log filename".into()), ..Default::default() };
                    o.directory = dirs_desktop();
                    o.file_name = Some("comlog.txt".into());
                    self.file = Some((FileTag::ComLog, PendingFile::save(ctx, o)));
                }
            }
            MenuAction::ShowLicense => self.overlays.push_back(Overlay::License(LicenseForm::new())),
        }
        if let Some(append) = self.want_open.take() {
            self.open_file_dialog(ctx, append);
        }
    }

    // ------------------------------------------------------------------ hotkeys

    /// Route egui key events through `HotKeysManager::manage_hotkeys` (`ProcessCmdKey` / `OnKeyUp`).
    fn hotkeys(&mut self, ctx: &egui::Context) {
        if self.state.core.suspend_hk() {
            return;
        }
        let events: Vec<(i32, bool)> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key { key, pressed, repeat, modifiers, .. } => {
                        if !*pressed {
                            Some((keys::NONE, false))
                        } else if *repeat {
                            None
                        } else {
                            keys::combine(&format!("{key:?}"), modifiers.ctrl || modifiers.command, modifiers.shift, modifiers.alt).map(|k| (k, true))
                        }
                    }
                    _ => None,
                })
                .collect()
        });
        for (k, _) in events {
            let hctx = self.state.hotkey_context(self.preview.custom_button_count());
            let out = self.state.hotkeys.manage_hotkeys(k, hctx);
            for e in out.effects {
                self.hotkey_effect(ctx, e);
            }
        }
    }

    /// Execute one hotkey effect (`HotKeysManager.PerformAction`).
    fn hotkey_effect(&mut self, ctx: &egui::Context, e: HotKeyEffect) {
        let core = self.state.core.clone();
        match e {
            HotKeyEffect::Core(a) => match a {
                Actions::ConnectDisconnect => {
                    if core.is_connected() {
                        self.request_disconnect();
                    } else {
                        self.connect_now();
                    }
                }
                Actions::Connect => {
                    if !core.is_connected() {
                        self.connect_now();
                    }
                }
                Actions::Disconnect => {
                    if core.is_connected() {
                        self.request_disconnect();
                    }
                }
                Actions::OpenFile => self.open_file_dialog(ctx, false),
                Actions::ReopenLastFile => {
                    if let Some(o) = core.reopen_file() {
                        self.open_outcome(o, false, false);
                    }
                }
                Actions::SaveFile => self.pending_save = Some(SaveOptions { header: false, footer: false, between: false, cycles: 1, use_lf_line_endings: false }),
                Actions::ExecuteFile => self.request_run(),
                Actions::AbortFile => core.abort_program(),
                Actions::HelpOnline => lg_base::tools::utils::open_link("https://github.com/mechaot/L-ai-SERcraft/blob/master/docs/MANUAL.md"),
                Actions::AutoSizeDrawing => core.auto_size_drawing(),
                Actions::ZoomInDrawing => core.zoom_in_drawing(),
                Actions::ZoomOutDrawing => core.zoom_out_drawing(),
                Actions::Reset => core.grbl_reset(),
                Actions::Homing => core.grbl_homing(),
                Actions::Unlock => core.grbl_unlock(),
                Actions::PauseJob => core.feed_hold(false),
                Actions::ResumeJob => core.cycle_start_resume(false),
                Actions::SetNewZero => core.set_new_zero(),
                _ => {}
            },
            HotKeyEffect::Override(a) => {
                let m = match a {
                    Actions::OverridePowerDefault => OverrideAction::PowerDefault,
                    Actions::OverridePowerUp => OverrideAction::PowerUp,
                    Actions::OverridePowerDown => OverrideAction::PowerDown,
                    Actions::OverrideLinearDefault => OverrideAction::LinearDefault,
                    Actions::OverrideLinearUp => OverrideAction::LinearUp,
                    Actions::OverrideLinearDown => OverrideAction::LinearDown,
                    Actions::OverrideRapidDefault => OverrideAction::RapidDefault,
                    Actions::OverrideRapidUp => OverrideAction::RapidUp,
                    _ => OverrideAction::RapidDown,
                };
                core.hot_key_override(m);
            }
            HotKeyEffect::Jog(d) => {
                let dir = match d {
                    JogDir::Home => JogDirection::Home,
                    JogDir::N => JogDirection::N,
                    JogDir::NE => JogDirection::NE,
                    JogDir::E => JogDirection::E,
                    JogDir::SE => JogDirection::SE,
                    JogDir::S => JogDirection::S,
                    JogDir::SW => JogDirection::SW,
                    JogDir::W => JogDirection::W,
                    JogDir::NW => JogDirection::NW,
                    JogDir::Zup => JogDirection::Zup,
                    JogDir::Zdown => JogDirection::Zdown,
                };
                let _ = core.jog_to_direction_fast(dir, false);
            }
            HotKeyEffect::ChangeJogStep(up) => self.jog.change_jog_step_index_by(if up { 1 } else { -1 }),
            HotKeyEffect::ChangeJogSpeed(up) => self.jog.change_jog_speed_index_by(if up { 1 } else { -1 }),
            HotKeyEffect::JogAbort => core.jog_abort(),
            HotKeyEffect::CustomButtonDown(i) => self.preview.custom_button_down(i),
            HotKeyEffect::CustomButtonUp(i) => self.preview.custom_button_up(i),
        }
    }
}

/// File name part of a path. `p`: path. Returns the last component, or the whole path when it has none.
fn file_name_of(p: &Path) -> String {
    p.file_name().map_or_else(|| p.to_string_lossy().into_owned(), |n| n.to_string_lossy().into_owned())
}

/// Add a default extension when the user typed none (`AddExtension = true`).
///
/// `p`: chosen path. `ext`: extension without dot. Returns the path with an extension.
fn with_ext(p: PathBuf, ext: &str) -> PathBuf {
    if p.extension().is_some() { p } else { p.with_extension(ext) }
}

/// Desktop folder (`Environment.SpecialFolder.Desktop`), if any.
fn dirs_desktop() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Desktop")).filter(|p| p.is_dir())
}

/// Boxed project value to settings JSON (`Settings.SetObject(key, value)`); enums are stored by number.
///
/// `v`: value from the `.lps` dictionary. Returns the JSON value or None for null.
fn project_to_setting(v: &project::ProjectValue) -> Option<serde_json::Value> {
    use project::ProjectValue as P;
    Some(match v {
        P::Null => return None,
        P::Bool(b) => serde_json::Value::Bool(*b),
        P::I32(i) => (*i).into(),
        P::I64(i) => (*i).into(),
        P::F32(f) => serde_json::to_value(*f).ok()?,
        P::F64(f) => serde_json::to_value(*f).ok()?,
        P::Decimal(d) => serde_json::to_value(d).ok()?,
        P::Str(s) => serde_json::Value::String(s.clone()),
        P::Enum { value, .. } => (*value).into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext_added_only_when_missing() {
        assert_eq!(with_ext(PathBuf::from("/a/b"), "nc"), PathBuf::from("/a/b.nc"));
        assert_eq!(with_ext(PathBuf::from("/a/b.gcode"), "nc"), PathBuf::from("/a/b.gcode"));
    }

    #[test]
    fn enum_project_value_is_number() {
        assert_eq!(project_to_setting(&project::ProjectValue::tool(2)), Some(serde_json::json!(2)));
        assert_eq!(project_to_setting(&project::ProjectValue::Null), None);
    }
}
