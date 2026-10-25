// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `GrblConfig.cs` + `GrblConfig.Designer.cs`: grbl `$$` configuration editor.
//!
//! Blocking core calls (`refresh_config`, `write_config`) run on a worker thread while a
//! `WaitingProgressBar` animates; the result is applied on the UI thread.

use crossbeam_channel::{Receiver, TryRecvError, unbounded};
use eframe::egui::{self, Color32, RichText, TextEdit, Ui, Vec2};
use lasergrbl_core::{CoreError, GrblCore, MacStatus, RefreshCause};
use lg_gcode::config::{GrblConfParam, GrblConfST};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::common::{ModalEvent, button_en, modal, trf};
use crate::ui::dialog::{DialogResult, FileDialogOptions, MessageBox, MessageBoxButtons, MessageBoxIcon, PendingFile, filters};
use crate::ui::widgets::progress::BouncingMode;
use crate::ui::widgets::{GridColumn, WaitingProgressBar, data_grid, grbl_group_box};
use crate::ui::{tr, tr_fmt, tr_plain};

/// How long the "Action" label stays visible (`ActionTimer.Interval`).
const ACTION_MS: u64 = 5000;

/// Result of the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrblConfigOutcome {
    /// Dialog closed.
    Closed,
}

/// What a worker job reports back to the UI thread.
#[derive(Debug, Default)]
struct JobResult {
    /// Text for the "Action" label.
    action: Option<String>,
    /// Error `MessageBox` (text, title).
    error_box: Option<(String, String)>,
    /// Re-read `Configuration` into the local copy (`mLocalCopy = Configuration.ToList()`).
    reload: bool,
    /// A write finished without error.
    write_ok: bool,
}

/// Pending message boxes of the form.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BoxTag {
    Info,
    CloseWithChanges,
}

/// One displayed grid row: parameter plus cached csv columns.
#[derive(Clone)]
struct Row {
    param: GrblConfParam,
    name: String,
    unit: String,
    desc: String,
    /// Value when the cell got focus (for cancel-on-error).
    edit_start: Option<String>,
}

impl Row {
    /// Build a row from a parameter.
    ///
    /// `p`: parameter. Returns the row with csv texts.
    fn new(p: GrblConfParam) -> Self {
        Row { name: p.parameter().unwrap_or_default(), unit: p.unit().unwrap_or_default(), desc: p.description().unwrap_or_default(), param: p, edit_start: None }
    }
}

/// The grbl configuration dialog.
pub struct GrblConfig {
    core: Arc<GrblCore>,
    rows: Vec<Row>,
    selected: Option<usize>,
    action: Option<(String, Instant)>,
    worker: Option<Receiver<JobResult>>,
    wait: WaitingProgressBar,
    boxes: Vec<(BoxTag, MessageBox)>,
    file: Option<(FileKind, PendingFile)>,
    pending_close: bool,
    started: bool,
    id: u64,
}

/// Which file dialog is open.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FileKind {
    Export,
    Import,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// `GetChanges`: parameters of `local` that differ from the stored configuration.
///
/// `conf`: stored configuration. `local`: edited copy. Returns the changed parameters.
pub fn get_changes(conf: &GrblConfST, local: &[GrblConfParam]) -> Vec<GrblConfParam> {
    local.iter().filter(|p| conf.has_changes(p)).cloned().collect()
}

/// `HasChanges`: count mismatch or any changed parameter.
///
/// `conf`: stored configuration. `local`: edited copy. Returns true when the user changed something.
pub fn has_changes(conf: &GrblConfST, local: &[GrblConfParam]) -> bool {
    conf.count() != local.len() || local.iter().any(|p| conf.has_changes(p))
}

/// Parse an exported/imported config file (`BtnImport_Click` loop).
///
/// `text`: file content, one `$n=value` per line. Returns the parameters, or Err when any line
/// is malformed (C# throws and shows "Error reading file!") or no line exists.
pub fn parse_import(text: &str) -> Result<Vec<GrblConfParam>, String> {
    let mut conf = Vec::new();
    for line in text.lines() {
        let mut parts = line.split('=');
        let head = parts.next().unwrap_or("");
        // Substring(1) of an empty head throws in C#
        let mut it = head.chars();
        it.next().ok_or("empty line")?;
        let num = lg_dotnet::parse_int(it.as_str()).ok_or("bad number")?;
        let val = parts.next().ok_or("missing value")?.trim();
        conf.push(GrblConfParam::new(num, val));
    }
    if conf.is_empty() {
        return Err("File does not contain a valid configuration".into());
    }
    Ok(conf)
}

/// Export text (`sw.WriteLine("${0}={1}")` per parameter).
///
/// `conf`: parameters. Returns the file text.
pub fn export_text(conf: &[GrblConfParam]) -> String {
    conf.iter().map(|p| format!("${}={}{}", p.number, p.value, newline())).collect()
}

/// `Environment.NewLine`.
fn newline() -> &'static str {
    if cfg!(windows) { "\r\n" } else { "\n" }
}

/// `WriteConf`: send parameters and refresh afterwards (blocking).
///
/// `core`: engine. `conf`: parameters to write. `import`: message flavour. Returns the report.
fn write_conf(core: &GrblCore, conf: &[GrblConfParam], import: bool) -> JobResult {
    let mut r = JobResult::default();
    let n = conf.len();
    match core.write_config(conf) {
        Ok(()) => {
            let key = if import { "Strings.BoxImportConfigWithoutError" } else { "Strings.BoxWriteConfigWithoutError" };
            r.action = Some(tr_fmt(key, &[&n]));
            r.write_ok = true;
        }
        Err(CoreError::WriteConfig { message, errors }) => {
            let key = if import { "Strings.BoxImportConfigWithError" } else { "Strings.BoxWriteConfigWithError" };
            r.error_box = Some((format!("{}\n{}", tr_fmt(key, &[&n, &errors.len()]), message), tr("Strings.BoxExportConfigErrorTitle").to_owned()));
        }
        Err(_) => {
            let key = if import { "Strings.BoxImportConfigWithError" } else { "Strings.BoxWriteConfigWithError" };
            r.error_box = Some((tr_fmt(key, &[&n, &"unknown"]), tr("Strings.BoxExportConfigErrorTitle").to_owned()));
        }
    }
    // ensure to have the last conf at least in core
    let _ = core.refresh_config(RefreshCause::OnWriteEnd);
    r
}

impl GrblConfig {
    /// Create the dialog (C# ctor + `Load` event).
    ///
    /// `core`: engine used for reads/writes. Starts reading when the core allows it.
    pub fn open(core: &Arc<GrblCore>) -> Self {
        let mut w = WaitingProgressBar::new(BouncingMode::PingPong);
        w.base.step = 4.0;
        w.interval_ms = 30.0;
        let mut f = GrblConfig {
            core: core.clone(),
            rows: Vec::new(),
            selected: None,
            action: None,
            worker: None,
            wait: w,
            boxes: Vec::new(),
            file: None,
            pending_close: false,
            started: false,
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        };
        f.reload_local();
        f
    }

    /// `mLocalCopy = GrblCore.Configuration.ToList()`.
    fn reload_local(&mut self) {
        self.rows = GrblCore::configuration().to_list().into_iter().map(Row::new).collect();
        self.selected = None;
    }

    /// Current parameters of the grid.
    fn local(&self) -> Vec<GrblConfParam> {
        self.rows.iter().map(|r| r.param.clone()).collect()
    }

    /// `ActionResult`: show text for 5 s.
    fn action_result(&mut self, text: String) {
        self.action = Some((text, Instant::now()));
    }

    /// True while a worker job runs.
    fn busy(&self) -> bool {
        self.worker.is_some()
    }

    /// Run `job` on a worker thread.
    ///
    /// `ctx`: egui context (repaint on finish). `job`: blocking work returning the report.
    fn spawn(&mut self, ctx: &egui::Context, job: impl FnOnce(&GrblCore) -> JobResult + Send + 'static) {
        let (tx, rx) = unbounded();
        let core = self.core.clone();
        let ctx = ctx.clone();
        let _ = std::thread::Builder::new().name("GrblConfigJob".into()).spawn(move || {
            let r = job(&core);
            let _ = tx.send(r);
            ctx.request_repaint();
        });
        self.worker = Some(rx);
        self.wait.start();
    }

    /// `DoRead`.
    ///
    /// `cause`: refresh cause passed to the core.
    fn do_read(&mut self, ctx: &egui::Context, cause: RefreshCause) {
        self.spawn(ctx, move |core| {
            let mut r = JobResult { reload: true, ..Default::default() };
            match core.refresh_config(cause) {
                Ok(()) => r.action = Some(tr_fmt("Strings.BoxReadConfigSuccess", &[&GrblCore::configuration().count()])),
                Err(e) => r.action = Some(format!("{} {}", tr("Strings.BoxReadConfigError"), e)),
            }
            r
        });
    }

    /// `BtnWrite_Click`: refresh, diff against the grid, write changes.
    fn do_write(&mut self, ctx: &egui::Context) {
        let local = self.local();
        self.spawn(ctx, move |core| {
            let _ = core.refresh_config(RefreshCause::OnWriteBegin);
            let changes = get_changes(&GrblCore::configuration(), &local);
            if changes.is_empty() {
                return JobResult { action: Some(tr("Strings.BoxWriteConfigNoChange").to_owned()), ..Default::default() };
            }
            write_conf(core, &changes, false)
        });
    }

    /// Write after the close question answered "Yes" (`WriteConf(GetChanges())`).
    fn do_write_before_close(&mut self, ctx: &egui::Context) {
        let changes = get_changes(&GrblCore::configuration(), &self.local());
        self.pending_close = true;
        self.spawn(ctx, move |core| write_conf(core, &changes, false));
    }

    /// File chosen for export (`BtnExport_Click` after the dialog).
    ///
    /// `path`: target file.
    fn do_export(&mut self, ctx: &egui::Context, path: PathBuf) {
        self.spawn(ctx, move |core| {
            // internally skipped if not possible
            let _ = core.refresh_config(RefreshCause::OnExport);
            let toexport = GrblCore::configuration().to_list();
            let res = if toexport.is_empty() {
                Err("Nothing to export!".to_string())
            } else {
                std::fs::write(&path, export_text(&toexport)).map_err(|e| e.to_string())
            };
            match res {
                Ok(()) => JobResult { action: Some(tr_fmt("Strings.BoxExportConfigSuccess", &[&toexport.len()])), ..Default::default() },
                Err(e) => {
                    lg_base::logger::log_message("ExportConfig", &e);
                    JobResult { error_box: Some((tr("Strings.BoxExportConfigError").to_owned(), tr("Strings.BoxExportConfigErrorTitle").to_owned())), ..Default::default() }
                }
            }
        });
    }

    /// File chosen for import (`BtnImport_Click` after the dialog).
    ///
    /// `path`: source file.
    fn do_import(&mut self, ctx: &egui::Context, path: &Path) {
        let title = tr("Strings.BoxExportConfigErrorTitle").to_owned();
        if !path.exists() {
            self.boxes.push((BoxTag::Info, MessageBox::new(tr("Strings.BoxExportConfigFileNotFound"), title).icon(MessageBoxIcon::Error)));
            return;
        }
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string());
        let conf = text.and_then(|t| parse_import(&t));
        match conf {
            Err(e) => {
                lg_base::logger::log_message("ImportConfig", &e);
                self.boxes.push((BoxTag::Info, MessageBox::new(tr("Strings.BoxImportConfigFileError"), title).icon(MessageBoxIcon::Error)));
            }
            Ok(conf) => self.spawn(ctx, move |core| {
                let mut r = write_conf(core, &conf, true);
                // refresh actual conf
                let _ = core.refresh_config(RefreshCause::OnImport);
                r.reload = true;
                r
            }),
        }
    }

    /// Apply a worker report on the UI thread.
    ///
    /// `r`: report. Returns true when the dialog must close now.
    fn apply(&mut self, r: JobResult) -> bool {
        self.wait.stop();
        if r.reload {
            self.reload_local();
        }
        if let Some(a) = r.action {
            self.action_result(a);
        }
        if let Some((text, title)) = r.error_box {
            self.boxes.push((BoxTag::Info, MessageBox::new(text, title).icon(MessageBoxIcon::Error)));
        }
        let close = self.pending_close && r.write_ok;
        self.pending_close = false;
        close
    }

    /// `FormClosing`: ask about unwritten changes. Returns true when the dialog may close at once.
    fn request_close(&mut self) -> bool {
        let conf = GrblCore::configuration();
        if has_changes(&conf, &self.local()) && self.core.machine_status() == MacStatus::Idle {
            let mb = MessageBox::new(tr("Strings.BoxConfigDetectedChanges"), tr("Strings.BoxConfigDetectedChangesTitle"))
                .buttons(MessageBoxButtons::YesNoCancel)
                .icon(MessageBoxIcon::Question)
                .default_button(2);
            self.boxes.push((BoxTag::CloseWithChanges, mb));
            return false;
        }
        true
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns `Some(Closed)` on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<GrblConfigOutcome> {
        // initial load (GrblConfig_Load)
        if !self.started {
            self.started = true;
            if self.core.can_read_write_config() {
                self.do_read(ctx, RefreshCause::OnDialog);
            } else {
                self.action_result(tr("Strings.BoxReadConfigPleaseConnect").to_owned());
            }
        }
        // worker result
        if let Some(rx) = &self.worker {
            match rx.try_recv() {
                Ok(r) => {
                    self.worker = None;
                    if self.apply(r) {
                        return Some(GrblConfigOutcome::Closed);
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    self.worker = None;
                    self.wait.stop();
                }
                Err(TryRecvError::Empty) => {
                    self.wait.tick();
                    ctx.request_repaint_after(Duration::from_millis(30));
                }
            }
        }
        if let Some((_, t0)) = &self.action {
            if t0.elapsed() >= Duration::from_millis(ACTION_MS) {
                self.action = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(250));
            }
        }
        // message boxes first
        if let Some((tag, mb)) = self.boxes.first_mut() {
            if let Some(res) = mb.show(ctx) {
                let tag = *tag;
                self.boxes.remove(0);
                if tag == BoxTag::CloseWithChanges {
                    match res {
                        DialogResult::Yes => self.do_write_before_close(ctx),
                        DialogResult::No => return Some(GrblConfigOutcome::Closed),
                        _ => {}
                    }
                }
            }
            return None;
        }
        // file dialogs
        if let Some((kind, pf)) = &mut self.file {
            if let Some(res) = pf.poll() {
                let kind = *kind;
                self.file = None;
                if let Some(p) = res {
                    match kind {
                        FileKind::Export => self.do_export(ctx, if p.extension().is_none() { p.with_extension("nc") } else { p }),
                        FileKind::Import => self.do_import(ctx, &p),
                    }
                }
            }
        }
        let title = trf("GrblConfig.$this.Text", "Grbl");
        let id = ("grblconfig", self.id);
        match modal(ctx, id, &title, 820.0, true, |ui| self.body(ui)) {
            Some(ModalEvent::Done(true)) | Some(ModalEvent::Closed) => {
                if self.busy() || !self.request_close() {
                    None
                } else {
                    Some(GrblConfigOutcome::Closed)
                }
            }
            _ => None,
        }
    }

    /// Form body. Returns Some(true) when Close was pressed.
    fn body(&mut self, ui: &mut Ui) -> Option<bool> {
        let connected = self.core.is_connected();
        let can_rw = self.core.can_read_write_config() && !self.busy();
        let busy = self.busy();
        let gb_title = tr("GrblConfig.GB.Text").to_owned();
        let editable = connected && !busy;
        let mut value_check: Option<(usize, String)> = None;
        grbl_group_box(ui, &gb_title, |ui| {
            let cols = [
                GridColumn { header: tr("GrblConfig.DollarNumber.HeaderText").to_owned(), width: Some(40.0) },
                GridColumn { header: tr("GrblConfig.Parameter.HeaderText").to_owned(), width: Some(200.0) },
                GridColumn { header: tr("GrblConfig.Value.HeaderText").to_owned(), width: Some(90.0) },
                GridColumn { header: tr("GrblConfig.Unit.HeaderText").to_owned(), width: Some(80.0) },
                GridColumn { header: tr("GrblConfig.Description.HeaderText").to_owned(), width: None },
            ];
            let n = self.rows.len();
            let mut sel = self.selected;
            let rows = &mut self.rows;
            data_grid(
                ui,
                "grblconfig-grid",
                &cols,
                n,
                &mut sel,
                |ui, r, c| {
                    let row = &mut rows[r];
                    match c {
                        0 => {
                            ui.label(RichText::new(row.param.dollar_number()).monospace());
                        }
                        1 => {
                            ui.label(RichText::new(&row.name).monospace());
                        }
                        2 => {
                            // Value cell style: WhiteSmoke back, black text
                            let te = TextEdit::singleline(&mut row.param.value)
                                .desired_width(80.0)
                                .font(egui::TextStyle::Monospace)
                                .text_color(Color32::BLACK)
                                .background_color(Color32::from_rgb(0xF5, 0xF5, 0xF5))
                                .interactive(editable);
                            let resp = ui.add(te);
                            if resp.gained_focus() {
                                row.edit_start = Some(row.param.value.clone());
                            }
                            if resp.lost_focus() {
                                if let Some(orig) = row.edit_start.take() {
                                    if orig != row.param.value {
                                        value_check = Some((r, orig));
                                    }
                                }
                            }
                        }
                        3 => {
                            ui.label(RichText::new(&row.unit).monospace());
                        }
                        _ => {
                            ui.label(RichText::new(&row.desc).monospace());
                        }
                    }
                },
                340.0,
            );
            self.selected = sel;
            ui.horizontal(|ui| {
                // LblAction: red text on white, bordered
                if let Some((text, _)) = &self.action {
                    egui::Frame::new().fill(Color32::WHITE).stroke(egui::Stroke::new(1.0, Color32::BLACK)).inner_margin(2.0).show(ui, |ui| {
                        ui.label(RichText::new(text).color(Color32::RED));
                    });
                }
                if busy {
                    self.wait.show(ui);
                }
            });
        });
        // CellValidating: ValidateConfig
        if let Some((r, orig)) = value_check {
            let num = self.rows[r].param.number;
            if let Some(err) = self.core.validate_config(num) {
                self.rows[r].param.value = orig;
                self.boxes.push((BoxTag::Info, MessageBox::new(err, tr("Strings.WarnMessageBoxHeader")).icon(MessageBoxIcon::Warning)));
            }
        }
        // button row
        let mut out = None;
        let mut want_read = false;
        let mut want_write = false;
        let mut want_export = false;
        let mut want_import = false;
        ui.horizontal(|ui| {
            // designer: 107x42 buttons
            let sz = Vec2::new(107.0, 42.0);
            want_read = button_en(ui, can_rw, &tr_plain("GrblConfig.BtnRead.Text"), Some("mdi-text-box-search"), sz);
            want_write = button_en(ui, can_rw, &tr_plain("GrblConfig.BtnWrite.Text"), Some("mdi-text-box-edit"), sz);
            ui.add_space(10.0);
            want_export = button_en(ui, GrblCore::configuration().count() > 0 && !busy, &tr_plain("GrblConfig.BtnExport.Text"), Some("mdi-upload-box"), sz);
            want_import = button_en(ui, can_rw, &tr_plain("GrblConfig.BtnImport.Text"), Some("mdi-download-box"), sz);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if button_en(ui, true, &tr_plain("GrblConfig.BtnCancel.Text"), Some("mdi-close-box"), sz) {
                    out = Some(true);
                }
                if !connected {
                    ui.label(RichText::new(tr("GrblConfig.LblConnect.Text")).color(Color32::RED));
                }
            });
        });
        let ctx = ui.ctx().clone();
        if want_read {
            self.do_read(&ctx, RefreshCause::OnRead);
        }
        if want_write {
            self.do_write(&ctx);
        }
        if want_export {
            let o = FileDialogOptions { filter: Some(filters::GCODE_SAVE_NC.into()), ..Default::default() };
            self.file = Some((FileKind::Export, PendingFile::save(&ctx, o)));
        }
        if want_import {
            let o = FileDialogOptions { filter: Some(filters::GCODE_OPEN_CONFIG.into()), ..Default::default() };
            self.file = Some((FileKind::Import, PendingFile::open(&ctx, o)));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conf(items: &[(i32, &str)]) -> GrblConfST {
        let t: Vec<(i32, String)> = items.iter().map(|(k, v)| (*k, v.to_string())).collect();
        GrblConfST::with_table(None, &t)
    }

    #[test]
    fn changes_detected() {
        let c = conf(&[(0, "10"), (1, "25"), (30, "1000")]);
        let mut local = c.to_list();
        assert!(!has_changes(&c, &local));
        assert!(get_changes(&c, &local).is_empty());
        local[1].value = "26".into();
        assert!(has_changes(&c, &local));
        assert_eq!(get_changes(&c, &local), vec![GrblConfParam::new(1, "26")]);
        // count mismatch alone counts as a change
        local.pop();
        local[1].value = "25".into();
        assert!(has_changes(&c, &local));
    }

    #[test]
    fn import_parse_like_csharp() {
        let ok = parse_import("$0=10 (Step pulse time)\n$1=25\r\n$30=1000").unwrap();
        assert_eq!(ok.len(), 3);
        assert_eq!(ok[0], GrblConfParam::new(0, "10 (Step pulse time)"));
        assert_eq!(ok[2], GrblConfParam::new(30, "1000"));
        // blank line, missing '=' or bad number: C# exception -> "Error reading file!"
        assert!(parse_import("$0=1\n\n$1=2").is_err());
        assert!(parse_import("$0").is_err());
        assert!(parse_import("$x=1").is_err());
        assert!(parse_import("").is_err());
    }

    #[test]
    fn export_roundtrip() {
        let list = vec![GrblConfParam::new(0, "10"), GrblConfParam::new(32, "1")];
        let text = export_text(&list);
        assert!(text.starts_with("$0=10"));
        assert_eq!(parse_import(&text).unwrap(), list);
    }

    #[test]
    fn ortur_warning_on_33() {
        let mut v = lg_gcode::config::GrblVersionInfo::with_vendor(1, 1, 'f', Some("Ortur"), None, false);
        let _ = &mut v;
        let c = GrblConfST::with_version(Some(v));
        assert!(c.validate_config(33).is_some() == c.grbl_version().is_some_and(|v| v.is_ortur()));
        assert!(c.validate_config(32).is_none());
    }
}
