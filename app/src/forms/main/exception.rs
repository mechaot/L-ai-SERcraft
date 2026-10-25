// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `ExceptionManager.cs` (+Designer): panic hook that logs and queues a report, and the dialog.

use std::sync::Mutex;

use eframe::egui::{self, Align, Layout, Ui};
use lasergrbl_core::{GrblCore, NamedEnum};

use crate::ui::widgets::GrblButton;
use crate::ui::{dialog, tr};

/// A captured error waiting for the UI (`CreateAndShow` arguments).
#[derive(Debug, Clone)]
pub struct Report {
    /// Exception data block (`AppendExceptionData`).
    pub body: String,
    /// `cancontinue`: the UI thread survived and can go on.
    pub can_continue: bool,
}

/// Reports queued by the panic hook, shown by the next UI frame.
static PENDING: Mutex<Vec<Report>> = Mutex::new(Vec::new());

/// Take the queued reports.
///
/// Returns the reports in arrival order.
pub fn take_pending() -> Vec<Report> {
    std::mem::take(&mut *PENDING.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Queue a report (`OnHandledException` / unhandled exception handlers).
///
/// `body`: report text. `can_continue`: show the Continue button.
pub fn push(body: String, can_continue: bool) {
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).push(Report { body, can_continue });
}

/// `ExceptionManager.RegisterHandler`: log every panic and queue a report for the dialog.
///
/// Panics on the `main` thread are caught by the app loop, so they can continue; other threads cannot.
pub fn register_handler() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("<unnamed>").to_owned();
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string panic payload>".to_owned());
        let loc = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_default();
        let bt = std::backtrace::Backtrace::force_capture();
        let mut sb = String::new();
        sb.push_str("TypeOf exception  [panic]\n");
        sb.push_str(&format!("Exception message [{msg}]\n"));
        sb.push_str(&format!("Exception source  [{loc}], thread [{name}]\n\n"));
        sb.push_str("   ----------- stack trace -----------\n");
        sb.push_str(&bt.to_string());
        sb.push('\n');
        lg_base::logger::log_message("Exception", &format!("{msg} at {loc} (thread {name})"));
        push(sb, name == "main");
        previous(info);
    }));
}

/// Header block of the report (version, firmware, wrapper, OS).
///
/// `core`: the engine when it exists. Returns the lines before the exception data.
pub fn header(core: Option<&GrblCore>) -> String {
    let mut sb = String::new();
    sb.push_str(&format!("{} v{}\n", lg_base::APP_NAME, env!("CARGO_PKG_VERSION")));
    if let Some(c) = core {
        let ver = GrblCore::configuration().grbl_version().map(|v| format!("{v:?}")).unwrap_or_default();
        sb.push_str(&format!("{} v{}\n", c.firmware().variant_name(), ver));
    }
    let wrapper: lasergrbl_core::com::WrapperType = lg_base::settings::get("ComWrapper Protocol", lasergrbl_core::com::WrapperType::UsbSerial);
    sb.push_str(&format!("Wrapper: {}\n", wrapper.name()));
    sb.push_str(&format!("{} ({})\n", lg_base::logger::os_info().replace('|', ", "), std::env::consts::ARCH));
    sb.push_str("Runtime: Rust, egui/eframe 0.37\n\n");
    sb
}

/// What the user chose in the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionResult {
    /// Dialog closed, keep running.
    Continue,
    /// `BtnAbort`: terminate (`Application.Exit()`).
    Abort,
}

/// The exception dialog for one report.
pub struct ExceptionDialog {
    text: String,
    can_continue: bool,
}

impl ExceptionDialog {
    /// Build the dialog.
    ///
    /// `report`: queued report. `core`: engine for the header. Returns the dialog.
    pub fn new(report: &Report, core: Option<&GrblCore>) -> Self {
        ExceptionDialog { text: format!("{}{}", header(core), report.body), can_continue: report.can_continue }
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the choice on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<ExceptionResult> {
        let mut res = None;
        let modal = egui::Modal::new(egui::Id::unique("exception-manager"));
        let r = modal.show(ctx, |ui: &mut Ui| {
            ui.set_width(620.0);
            ui.label(crate::ui::fonts::bold("Exception Manager"));
            ui.separator();
            ui.horizontal(|ui| {
                ui.add(crate::ui::icon(ui.ctx(), "mdi-alert-circle", 48.0));
                ui.vertical(|ui| {
                    ui.label(crate::ui::fonts::bold("Ooops! Something went wrong").size(18.0));
                    ui.add(egui::Label::new(tr("ExceptionManager.LblFormDescription.Text")).wrap());
                });
            });
            ui.add_space(6.0);
            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut self.text).desired_width(f32::INFINITY).desired_rows(14).font(egui::TextStyle::Monospace).interactive(true));
            });
            ui.add_space(6.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(GrblButton::new("Abort")).clicked() {
                    res = Some(ExceptionResult::Abort);
                }
                if self.can_continue && ui.add(GrblButton::new(" Continue")).clicked() {
                    res = Some(ExceptionResult::Continue);
                }
                if ui.add(GrblButton::new("Copy")).clicked() {
                    ui.ctx().copy_text(self.text.clone());
                }
            });
        });
        if res.is_none() && r.should_close() {
            res = Some(ExceptionResult::Continue);
        }
        let _ = dialog::bold("");
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_roundtrip() {
        push("x".into(), true);
        let v = take_pending();
        assert!(v.iter().any(|r| r.body == "x" && r.can_continue));
        assert!(take_pending().is_empty());
    }

    #[test]
    fn header_mentions_version() {
        assert!(header(None).starts_with("L-ai-SERctrl v1.2.1"));
    }
}
