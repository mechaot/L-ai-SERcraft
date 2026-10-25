// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `Firmware/FlashGrbl.cs` + `FlashGrbl.Designer.cs`: flash a precompiled grbl `.hex` with avrdude.
//!
//! Logic (arguments, executable lookup, process streaming) lives in `lg_services::flash_grbl`; this is
//! the form; avrdude output is only written to the session log. The result message boxes of `MainForm` are shown
//! by the dialog itself, so the caller only needs to open it (menu enabled while disconnected).

use crossbeam_channel::{Receiver, unbounded};
use eframe::egui::{self, Color32, RichText, TextEdit, Ui, Vec2};
use lg_services::flash_grbl::{self as fg, BAUD_RATES, DEFAULT_BAUD, Firmware, SELECT_CUSTOM};
use std::path::PathBuf;

use super::common::{ModalEvent, button_en, info_button, modal, open_link, trf};
use crate::ui::dialog::{DialogResult, FileDialogOptions, MessageBox, MessageBoxButtons, MessageBoxIcon, PendingFile, filters};
use crate::ui::icons::{Tint, icons};
use crate::ui::tr;
use crate::ui::widgets::progress::BouncingMode;
use crate::ui::widgets::{WaitingProgressBar, flat_combo};

/// Result of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashOutcome {
    /// Closed without flashing.
    Cancelled,
    /// avrdude ran; `retval` is its exit code (1 when it could not start), 0 = success.
    Flashed { retval: i32 },
}

/// Messages from the avrdude worker.
enum Msg {
    /// Output chunk.
    Line(String),
    /// Finished with the exit code.
    Done(i32),
}

/// Pending message boxes.
enum Ask {
    Confirm,
    Result(i32),
}

/// `UpdateBtnOK` helper.
///
/// `port`: port text. `firmware`: selected firmware index. `target`: selected target index.
/// Returns true when the OK button is enabled.
pub fn ok_enabled(port: &str, firmware: Option<usize>, target: Option<usize>) -> bool {
    fg::can_flash(port, firmware.is_some(), target.is_some())
}

/// Whether the avrdude executable can be found (explicit path exists, or the name is on `PATH`).
///
/// `exe`: result of `avrdude_executable()`. Returns true when the program can be started.
pub fn exe_available(exe: &std::path::Path) -> bool {
    if exe.components().count() > 1 || exe.is_absolute() {
        return exe.is_file();
    }
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(exe).is_file() || d.join(format!("{}.exe", exe.display())).is_file()))
}

/// Text of the last log line segment: `\r` rewrites the current line like a console would.
///
/// `log`: accumulated text. `chunk`: new chunk from avrdude. Returns the updated text.
pub fn append_log(log: &str, chunk: &str) -> String {
    let mut out = log.to_owned();
    for c in chunk.chars() {
        match c {
            // carriage return: drop the current (unfinished) line
            '\r' => {
                let cut = out.rfind('\n').map(|i| i + 1).unwrap_or(0);
                out.truncate(cut);
            }
            _ => out.push(c),
        }
    }
    out
}

/// The dialog.
pub struct FlashGrbl {
    firmwares: Vec<Firmware>,
    fw_sel: Option<usize>,
    target_sel: Option<usize>,
    port: String,
    ports: Vec<String>,
    baud_sel: Option<usize>,
    exe: PathBuf,
    ask: Option<(Ask, MessageBox)>,
    file: Option<PendingFile>,
    worker: Option<Receiver<Msg>>,
    log: String,
    wait: WaitingProgressBar,
    id: u64,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl FlashGrbl {
    /// Create the dialog (C# ctor): firmware list, baud rates, serial ports.
    pub fn open() -> Self {
        let mut w = WaitingProgressBar::new(BouncingMode::PingPong);
        w.base.step = 4.0;
        w.interval_ms = 30.0;
        let mut f = FlashGrbl {
            firmwares: fg::list_firmwares(&fg::firmware_folder()),
            fw_sel: None,
            target_sel: None,
            port: String::new(),
            ports: Vec::new(),
            baud_sel: BAUD_RATES.iter().position(|b| *b == DEFAULT_BAUD),
            exe: fg::avrdude_executable(),
            ask: None,
            file: None,
            worker: None,
            log: String::new(),
            wait: w,
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        };
        f.init_ports();
        f
    }

    /// `InitPortCB`: list ports (issue 31 name purge), select the last one.
    fn init_ports(&mut self) {
        self.ports = lasergrbl_core::com::list_serial_ports().iter().map(|p| fg::purge_port_name(p)).collect();
        if let Some(last) = self.ports.last() {
            self.port = last.clone();
        }
    }

    /// Flash button enabled state.
    pub fn can_flash(&self) -> bool {
        ok_enabled(&self.port, self.fw_sel, self.target_sel)
    }

    /// `CbTarget_SelectedIndexChanged`: target preset sets the baud rate.
    ///
    /// `idx`: new target index.
    fn set_target(&mut self, idx: usize) {
        self.target_sel = Some(idx);
        if let Some(b) = fg::baud_for_target(idx) {
            self.baud_sel = BAUD_RATES.iter().position(|x| *x == b);
        }
    }

    /// Selected baud.
    pub fn baud(&self) -> i32 {
        self.baud_sel.map(|i| BAUD_RATES[i]).unwrap_or(DEFAULT_BAUD)
    }

    /// Items of the firmware combo: display names plus the "select custom" entry.
    fn fw_items(&self) -> Vec<String> {
        let mut v: Vec<String> = self.firmwares.iter().map(|f| f.display_name()).collect();
        v.push(SELECT_CUSTOM.to_owned());
        v
    }

    /// Start avrdude on a worker thread.
    fn start(&mut self, ctx: &egui::Context) {
        let Some(fw) = self.fw_sel.and_then(|i| self.firmwares.get(i)) else { return };
        let args = fg::avrdude_args(&self.port, self.baud(), &fw.path, None);
        let exe = self.exe.clone();
        let (tx, rx) = unbounded();
        let ctx = ctx.clone();
        self.log.clear();
        self.log.push_str(&format!("{} {}\n", exe.display(), args.join(" ")));
        let _ = std::thread::Builder::new().name("avrdude".into()).spawn(move || {
            let txl = tx.clone();
            let c2 = ctx.clone();
            let mut on_out = move |s: &str| {
                let _ = txl.send(Msg::Line(s.to_owned()));
                c2.request_repaint();
            };
            let code = match fg::run_avrdude(&exe, &args, &mut on_out) {
                Ok(c) => c,
                Err(m) => {
                    let _ = tx.send(Msg::Line(format!("{m}\n")));
                    1
                }
            };
            let _ = tx.send(Msg::Done(code));
            ctx.request_repaint();
        });
        self.worker = Some(rx);
        self.wait.start();
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<FlashOutcome> {
        // worker messages
        let mut finished = None;
        if let Some(rx) = &self.worker {
            while let Ok(m) = rx.try_recv() {
                match m {
                    Msg::Line(s) => self.log = append_log(&self.log, &s),
                    Msg::Done(c) => finished = Some(c),
                }
            }
            self.wait.tick();
        }
        if let Some(code) = finished {
            self.worker = None;
            self.wait.stop();
            // avrdude output goes to the session log only (original shows its own console)
            lg_base::logger::log_message("FlashGrbl", &format!("avrdude exit {code}\n{}", self.log));
            let mb = if code == 0 {
                MessageBox::new("Firmware flashed succesfull!", "Result").icon(MessageBoxIcon::Information)
            } else {
                MessageBox::new("Error: cannot flash firmware.", "Result").icon(MessageBoxIcon::Error)
            };
            self.ask = Some((Ask::Result(code), mb));
        }
        // custom firmware chooser
        if let Some(pf) = &mut self.file {
            if let Some(res) = pf.poll() {
                self.file = None;
                self.fw_sel = match res {
                    Some(p) if p.exists() => Some(fg::add_or_select(&mut self.firmwares, &p)),
                    _ => None,
                };
            }
        }
        if let Some((kind, mb)) = &mut self.ask {
            if let Some(r) = mb.show(ctx) {
                match kind {
                    Ask::Confirm => {
                        let go = r == DialogResult::Ok;
                        self.ask = None;
                        if go {
                            self.start(ctx);
                        }
                    }
                    Ask::Result(code) => return Some(FlashOutcome::Flashed { retval: *code }),
                }
            }
            return None;
        }
        let title = trf("Firmware.FlashGrbl.$this.Text", "Flash Grbl Firmware");
        let id = ("flashgrbl", self.id);
        let busy = self.worker.is_some();
        match modal(ctx, id, &title, 317.0, true, |ui| self.body(ui)) {
            Some(ModalEvent::Done(true)) => {
                // BtnOK_Click confirmation (default button: Cancel)
                self.ask = Some((
                    Ask::Confirm,
                    MessageBox::new("Are you sure to flash a new firmware?", "Confirm request!").buttons(MessageBoxButtons::OkCancel).icon(MessageBoxIcon::Warning).default_button(1),
                ));
                None
            }
            Some(ModalEvent::Done(false)) | Some(ModalEvent::Closed) if !busy => Some(FlashOutcome::Cancelled),
            _ => None,
        }
    }

    /// Form body. Returns Some(true)=OK, Some(false)=Cancel.
    fn body(&mut self, ui: &mut Ui) -> Option<bool> {
        let t = |c: &str| tr(&format!("Firmware.FlashGrbl.{c}.Text")).to_owned();
        let tip = |c: &str| tr(&format!("Firmware.FlashGrbl.{c}.ToolTip")).to_owned();
        let busy = self.worker.is_some();
        ui.horizontal_top(|ui| {
            ui.add(icons().image_tinted(ui.ctx(), "mdi-alert", 32.0, Tint::Color(Color32::from_rgb(242, 165, 1))));
            ui.vertical(|ui| {
                ui.label(RichText::new(t("LblWarning")).color(Color32::RED));
                ui.label(RichText::new(t("LblNotForOrtur")).color(Color32::RED));
                if !exe_available(&self.exe) {
                    // avrdude missing: the C# build shipped avrdude.exe
                    ui.label(RichText::new(format!("avrdude not found ({}). Install avrdude or set the \"{}\" setting.", self.exe.display(), fg::SETTING_AVRDUDE)).color(Color32::RED));
                }
            });
        });
        ui.add_space(6.0);
        ui.add_enabled_ui(!busy, |ui| {
            egui::Grid::new(("flash-grid", self.id)).num_columns(3).spacing([8.0, 6.0]).show(ui, |ui| {
                // target
                ui.label(t("LblTarget"));
                let items = vec![tr("Firmware.FlashGrbl.CbTarget.Items").to_owned(), tr("Firmware.FlashGrbl.CbTarget.Items1").to_owned()];
                let mut sel = self.target_sel;
                if flat_combo(ui, "flash-target", &mut sel, &items, 209.0).changed() {
                    if let Some(i) = sel {
                        self.set_target(i);
                    }
                }
                if info_button(ui, &tip("BtnTarget")) {
                    open_link("https://lasergrbl.com/usage/flash/#target");
                }
                ui.end_row();
                // firmware
                ui.label(t("LblFirmware"));
                let items = self.fw_items();
                let mut sel = self.fw_sel;
                if flat_combo(ui, "flash-fw", &mut sel, &items, 209.0).changed() {
                    if sel == Some(items.len() - 1) {
                        let o = FileDialogOptions { filter: Some(filters::HEX_FIRMWARE.into()), ..Default::default() };
                        self.file = Some(PendingFile::open(ui.ctx(), o));
                        sel = None;
                    }
                    self.fw_sel = sel;
                }
                if info_button(ui, &tip("BtnFirmware")) {
                    open_link("https://lasergrbl.com/usage/flash/#firmware");
                }
                ui.end_row();
                // port (editable combo)
                ui.label(t("LblPort"));
                // editable ComboBox: text box + drop arrow (egui has no editable combo)
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add(TextEdit::singleline(&mut self.port).desired_width(98.0));
                    let arrow = ui.add(egui::Button::new("").min_size(Vec2::new(17.0, 20.0)));
                    let c = arrow.rect.center();
                    ui.painter().add(egui::Shape::convex_polygon(
                        vec![c + Vec2::new(-4.0, -2.0), c + Vec2::new(4.0, -2.0), c + Vec2::new(0.0, 3.0)],
                        ui.visuals().text_color(),
                        egui::Stroke::NONE,
                    ));
                    egui::Popup::menu(&arrow).show(|ui| {
                        for p in self.ports.clone() {
                            if ui.button(&p).clicked() {
                                self.port = p;
                                ui.close();
                            }
                        }
                    });
                });
                ui.label("");
                ui.end_row();
                // baud
                ui.label(t("LblSpeed"));
                let items: Vec<String> = BAUD_RATES.iter().map(|b| b.to_string()).collect();
                flat_combo(ui, "flash-baud", &mut self.baud_sel, &items, 115.0);
                ui.label("");
                ui.end_row();
            });
        });
        ui.add_space(8.0);
        let mut out = None;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let sz = Vec2::new(80.0, 27.0);
            if button_en(ui, self.can_flash() && !busy && exe_available(&self.exe), &t("BtnOK"), None, sz) {
                out = Some(true);
            }
            if button_en(ui, !busy, &t("BtnCancel"), None, sz) {
                out = Some(false);
            }
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_button_rules() {
        assert!(!ok_enabled("", Some(0), Some(0)));
        assert!(!ok_enabled("  ", Some(0), Some(0)));
        assert!(!ok_enabled("COM3", None, Some(0)));
        assert!(!ok_enabled("COM3", Some(0), None));
        assert!(ok_enabled("COM3", Some(0), Some(1)));
    }

    #[test]
    fn target_sets_baud() {
        let mut f = FlashGrbl { firmwares: vec![], fw_sel: None, target_sel: None, port: String::new(), ports: vec![], baud_sel: None, exe: PathBuf::from("avrdude"), ask: None, file: None, worker: None, log: String::new(), wait: WaitingProgressBar::new(BouncingMode::PingPong), id: 0 };
        f.set_target(1);
        assert_eq!(f.baud(), 57600);
        f.set_target(0);
        assert_eq!(f.baud(), 115200);
    }

    #[test]
    fn log_carriage_return_rewrites_line() {
        let l = append_log("start\n", "Reading | 10%\r");
        let l = append_log(&l, "Reading | 50%\r");
        let l = append_log(&l, "Reading | 100%\n");
        assert_eq!(l, "start\nReading | 100%\n");
    }

    #[test]
    fn exe_lookup() {
        assert!(!exe_available(std::path::Path::new("definitely-not-avrdude-xyz")));
        assert!(!exe_available(std::path::Path::new("/nonexistent/avrdude")));
        assert!(exe_available(std::path::Path::new("sh")) || cfg!(windows));
    }
}
