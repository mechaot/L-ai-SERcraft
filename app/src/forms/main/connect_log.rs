// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `ConnectLogForm.cs` (+Designer): connection, file/progress and manual command / log group boxes.

use eframe::egui::{self, Align, Layout, RichText, Ui, Vec2};
use lasergrbl_core::com::{list_serial_ports, ComConfig, WrapperType};
use lasergrbl_core::GrblCore;
use lg_base::settings;
use lg_base::tools::utils::{time_span_to_string, TimePrecision};
use lg_dotnet::{decimal_to_f64, f64_to_decimal, Decimal};
use lg_gcode::GrblCommand;

use super::command_log::CommandLog;
use crate::ui::widgets::progress::Bar;
use crate::ui::widgets::{flat_combo, DoubleProgressBar, GrblTextBox, ImageButton, NumericUpDown, TextInput};
use crate::ui::{tr, tr_plain, Theme};

/// `baudRates`.
pub const BAUD_RATES: [i32; 10] = [4800, 9600, 19200, 38400, 57600, 115200, 230400, 256000, 460800, 921600];

/// What the form asks the main window to do (calls that open dialogs or need confirmation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectLogAction {
    /// `Core.OpenCom()` (connect button while disconnected).
    Connect,
    /// Disconnect; the main window asks for confirmation while a job runs.
    Disconnect,
    /// `Core.OpenFile()` (file dialog).
    OpenFile,
    /// `Core.RunProgram(ParentForm)`.
    Run,
    /// Abort button: confirmation, then `Core.AbortProgram()`.
    Abort,
}

/// Borderless WinForms `Panel` (GBConnection / GBFile / GBCommands have no border in the designer).
///
/// `ui`: parent. `add`: body. Returns the body's value.
fn panel<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new().inner_margin(egui::Margin::same(1)).show(ui, add).inner
}

/// Is this a serial wrapper (shows port and baud)?
///
/// `w`: wrapper type. Returns true for the three serial protocols.
fn is_serial(w: WrapperType) -> bool {
    matches!(w, WrapperType::UsbSerial | WrapperType::UsbSerial2 | WrapperType::RJCPSerial)
}

/// State of `ConnectLogForm`.
pub struct ConnectLogForm {
    /// `currentWrapper`.
    pub current_wrapper: WrapperType,
    ports: Vec<String>,
    port: Option<usize>,
    baud: Option<usize>,
    /// `TxtAddress.Text`.
    pub address: String,
    /// `mLoadedFileName` (full path).
    loaded_file_name: Option<String>,
    /// `TbFileName.Text`.
    file_name_text: String,
    /// `UDLoopCounter.Value`.
    loop_value: f64,
    manual: GrblTextBox,
    log: CommandLog,
    pb: DoubleProgressBar,
    /// Configuration must be pushed to the core (`UpdateConf` -> `ApplyConfig`).
    dirty: bool,
}

impl Default for ConnectLogForm {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectLogForm {
    /// Constructor + `SetCore`: restore the saved baud rate and address, list the ports.
    pub fn new() -> Self {
        let current_wrapper: WrapperType = settings::get("ComWrapper Protocol", WrapperType::UsbSerial);
        let speed: i32 = settings::get("Serial Speed", 115200);
        let baud = BAUD_RATES.iter().position(|b| *b == speed);
        // RestoreConf: address of the active wrapper only
        let address = match current_wrapper {
            WrapperType::Telnet => settings::get("Telnet Address", "127.0.0.1:23".to_string()),
            WrapperType::LaserWebESP8266 => settings::get("Websocket URL", "ws://127.0.0.1:81/".to_string()),
            _ => String::new(),
        };
        let mut s = ConnectLogForm {
            current_wrapper,
            ports: Vec::new(),
            port: None,
            baud,
            address,
            loaded_file_name: None,
            file_name_text: String::new(),
            loop_value: 1.0,
            manual: GrblTextBox::new(),
            log: CommandLog::new(),
            pb: DoubleProgressBar::default(),
            dirty: true,
        };
        s.init_port_cb();
        s.pb.base.draw_progress_string = true;
        s.pb.base.size = Vec2::new(109.0, 15.0);
        s
    }

    /// `InitPortCB`: refill the port list, keep the selection when still present, else select the last.
    fn init_port_cb(&mut self) {
        let current = self.port.and_then(|i| self.ports.get(i)).cloned();
        // FIX issue #31: strip a trailing letter from the port name
        self.ports = list_serial_ports();
        self.port = match current.and_then(|c| self.ports.iter().position(|p| *p == c)) {
            Some(i) => Some(i),
            None if !self.ports.is_empty() => Some(self.ports.len() - 1),
            None => None,
        };
    }

    /// `OnFileLoaded`: show the file name.
    ///
    /// `filename`: full path of the loaded file.
    pub fn on_file_loaded(&mut self, filename: &str) {
        self.loaded_file_name = Some(filename.to_owned());
        self.file_name_text = std::path::Path::new(filename).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    }

    /// `OnLoopCountChanged`.
    ///
    /// `current`: new loop count from the core.
    pub fn on_loop_count_changed(&mut self, current: Decimal) {
        self.loop_value = decimal_to_f64(current);
    }

    /// `ConfigFromDiscovery`: set the address and request a connect when idle.
    ///
    /// `config`: "ip:port" from the discovery dialog. Returns true when a connect must be started.
    pub fn config_from_discovery(&mut self, config: Option<String>, core: &GrblCore) -> bool {
        match config {
            Some(c) if self.address_visible() && !core.is_connected() => {
                self.address = c;
                self.dirty = true;
                core.machine_status() == lasergrbl_core::MacStatus::Disconnected
            }
            _ => false,
        }
    }

    /// `ConfigFromWiFiForm`.
    ///
    /// `config`: "ip:port" to put in the address box.
    pub fn config_from_wifi_form(&mut self, config: String) {
        self.address = config;
        self.dirty = true;
    }

    /// `TxtAddress.Visible`.
    fn address_visible(&self) -> bool {
        matches!(self.current_wrapper, WrapperType::Telnet | WrapperType::LaserWebESP8266)
    }

    /// `OnColorChange`: bar colours follow the scheme (re-read each frame, nothing cached).
    pub fn on_color_change(&mut self) {}

    /// `ApplyConfig`: push the connection parameters to the core.
    ///
    /// `core`: the engine.
    fn apply_config(&self, core: &GrblCore) {
        let w = self.current_wrapper;
        if is_serial(w) {
            if let (Some(p), Some(b)) = (self.port.and_then(|i| self.ports.get(i)), self.baud) {
                core.configure(w, &ComConfig::Serial { port_name: p.clone(), baud_rate: BAUD_RATES[b] });
            }
        } else if self.address_visible() {
            core.configure(w, &ComConfig::Address(self.address.clone()));
        } else if w == WrapperType::Emulator {
            core.configure(w, &ComConfig::None);
        }
    }

    /// `UpdateConf`: persist baud and address, then apply.
    ///
    /// `core`: the engine.
    fn update_conf(&mut self, core: &GrblCore) {
        if let Some(b) = self.baud {
            settings::set("Serial Speed", BAUD_RATES[b]);
        }
        if !self.address.trim().is_empty() {
            match self.current_wrapper {
                WrapperType::Telnet => settings::set("Telnet Address", self.address.clone()),
                WrapperType::LaserWebESP8266 => settings::set("Websocket URL", self.address.clone()),
                _ => {}
            }
        }
        self.apply_config(core);
        self.dirty = false;
    }

    /// `TimerUpdate`: progress bar, port list and wrapper change detection.
    ///
    /// `core`: the engine.
    fn timer_update(&mut self, core: &GrblCore) {
        let th = Theme::current();
        // rescan serial ports while disconnected
        if !core.is_connected() && list_serial_ports().len() != self.ports.len() {
            self.init_port_cb();
            self.dirty = true;
        }
        self.pb.base.set_range(0.0, core.program_target() as f64);
        self.pb.base.fill_color = th.log_back_color;
        self.pb.bars = vec![Bar { value: core.program_sent() as f64, color: th.preview_command_wait }, Bar { value: core.program_executed() as f64, color: th.preview_command_ok }];
        let val = time_span_to_string(core.program_time(), TimePrecision::Minute, TimePrecision::Second, " ,", true);
        self.pb.perc_string = if val != "now" {
            val
        } else if core.in_program() {
            "0 sec".to_owned()
        } else {
            String::new()
        };
        // protocol changed in the settings dialog
        if !core.is_connected() {
            let actual: WrapperType = settings::get("ComWrapper Protocol", WrapperType::UsbSerial);
            if actual != self.current_wrapper {
                self.current_wrapper = actual;
                self.dirty = true;
            }
        }
        if self.dirty {
            self.update_conf(core);
        }
    }

    /// Draw the whole control.
    ///
    /// `ui`: parent (left column). `core`: the engine. Returns the requests for the main window.
    pub fn show(&mut self, ui: &mut Ui, core: &GrblCore) -> Vec<ConnectLogAction> {
        let mut actions = Vec::new();
        let th = Theme::current();
        self.timer_update(core);
        let connected = core.is_connected();
        let w = self.current_wrapper;

        // GBConnection: com/baud row + address / emulator rows + connect button at the right (spans all rows)
        panel(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(2.0, 2.0);
                let btn_w = 24.0;
                let left_w = (ui.available_width() - btn_w - 4.0).max(100.0);
                ui.allocate_ui_with_layout(Vec2::new(left_w, 0.0), Layout::top_down(Align::Min), |ui| {
                    ui.set_width(left_w);
                    ui.add_enabled_ui(!connected, |ui| {
                        if is_serial(w) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                ui.label(tr("ConnectLogForm.LblComPort.Text"));
                                let mut sel = self.port;
                                let items = self.ports.clone();
                                if flat_combo(ui, "cb-port", &mut sel, &items, 66.0).changed() {
                                    self.port = sel;
                                    self.dirty = true;
                                }
                                ui.label(tr("ConnectLogForm.LblBaudRate.Text"));
                                let mut sel = self.baud;
                                let items: Vec<String> = BAUD_RATES.iter().map(|b| b.to_string()).collect();
                                if flat_combo(ui, "cb-baud", &mut sel, &items, 66.0).changed() {
                                    self.baud = sel;
                                    self.dirty = true;
                                }
                            });
                        }
                        if self.address_visible() {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                // LblAddress.Text is set in UpdateConf (not localized)
                                ui.label(if w == WrapperType::Telnet { "IP:PORT" } else { "Socket URL" });
                                if ui.add(TextInput::new(&mut self.address).width(168.0)).changed() {
                                    self.dirty = true;
                                }
                            });
                        }
                    });
                    if w == WrapperType::Emulator {
                        // TxtEmulator: disabled text box with fixed text
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            ui.label(tr("ConnectLogForm.LblEmulator.Text"));
                            let mut txt = tr("ConnectLogForm.TxtEmulator.Text").to_owned();
                            ui.add_enabled(false, TextInput::new(&mut txt).width(168.0));
                        });
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    let tip = if connected { tr("Strings.BtnDisconnectTT") } else { tr("Strings.BtnConnectTT") };
                    let b = ui.add(ImageButton::new("mdi-power-plug").alt("mdi-power-plug-off", connected).size(Vec2::splat(22.0)));
                    if b.on_hover_text(tip).clicked() {
                        actions.push(if core.machine_status() == lasergrbl_core::MacStatus::Disconnected { ConnectLogAction::Connect } else { ConnectLogAction::Disconnect });
                    }
                });
            });
        });

        // GBFile: filename row and progress row
        panel(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr("ConnectLogForm.LblFilename.Text"));
                let mut name = self.file_name_text.clone();
                let r = ui.add(TextInput::new(&mut name).read_only(true).width(120.0));
                if let Some(full) = &self.loaded_file_name {
                    r.on_hover_text(full.clone());
                }
                if ui.add_enabled(core.can_load_new_file(), ImageButton::new("mdi-folder").size(Vec2::splat(20.0))).on_hover_text(tr("ConnectLogForm.BtnOpen.ToolTip")).clicked() {
                    actions.push(ConnectLogAction::OpenFile);
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("ConnectLogForm.LblProgress.Text"));
                self.pb.show(ui);
                let before = self.loop_value;
                let r = ui.add(NumericUpDown::new(&mut self.loop_value).range(1.0, 99.0).width(48.0));
                r.on_hover_text(tr("ConnectLogForm.UDLoopCounter.ToolTip"));
                if (self.loop_value - before).abs() > f64::EPSILON {
                    core.set_loop_count(f64_to_decimal(self.loop_value));
                }
                if core.can_abort_program() {
                    if ui.add(ImageButton::new("mdi-stop-circle").size(Vec2::splat(20.0))).on_hover_text(tr("ConnectLogForm.BtnAbortProgram.ToolTip")).clicked() {
                        actions.push(ConnectLogAction::Abort);
                    }
                } else if ui.add_enabled(core.can_send_file(), ImageButton::new("mdi-play-circle").size(Vec2::splat(20.0))).on_hover_text(tr("ConnectLogForm.BtnRunProgram.ToolTip")).clicked() {
                    actions.push(ConnectLogAction::Run);
                }
            });
        });

        // GBCommands: manual command + log filling the rest
        let _ = &th;
        panel(ui, |ui| {
            let can_send = core.can_send_manual_command();
            let mut entered = None;
            ui.add_enabled_ui(can_send, |ui| {
                let (r, e) = self.manual.show(ui, &tr_plain("ConnectLogForm.TxtManualCommand.WaterMark"), ui.available_width() - 6.0);
                // TxtManualCommand_Enter / _Leave
                core.set_suspend_hk(r.has_focus());
                entered = e;
            });
            if let Some(cmd) = entered {
                core.enqueue_command(&GrblCommand::with_options(&cmd, 0, true));
            }
            let h = ui.available_height().max(40.0);
            ui.allocate_ui(Vec2::new(ui.available_width(), h), |ui| {
                self.log.show(ui, core);
            });
        });
        let _ = RichText::new("");
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_kinds() {
        assert!(is_serial(WrapperType::UsbSerial));
        assert!(is_serial(WrapperType::RJCPSerial));
        assert!(!is_serial(WrapperType::Telnet));
        assert!(!is_serial(WrapperType::Emulator));
    }

    #[test]
    fn baud_default_present() {
        assert!(BAUD_RATES.contains(&115200));
    }
}
