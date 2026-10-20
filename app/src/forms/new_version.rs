#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Ports `AutoUpdate/NewVersionForm.cs` (+ Designer). UI only: the update check and the download/self-update are
//! not ported (PLAN §6); "Download!" explains that to the user.

use eframe::egui::{self, Align, Grid, Layout, ProgressBar, Ui, Vec2};

use crate::forms::form_util::*;
use crate::ui::dialog::{MessageBox, MessageBoxHost, MessageBoxIcon};
use crate::ui::tr;

/// i18n prefix of the form (sub-folder form).
const P: &str = "AutoUpdate.NewVersionForm.";

/// How the form ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewVersionOutcome {
    /// Cancel / Esc / close box (C# `DialogResult.Cancel`).
    Cancelled,
}

/// New-version dialog state.
pub struct NewVersionForm {
    current: String,
    latest: String,
    page_url: String,
    download_url: String,
    msgs: MessageBoxHost<()>,
}

impl NewVersionForm {
    /// `CreateAndShowDialog(current, available, parent)` setup.
    ///
    /// `current`: current version text (`Version.ToString(3)`). `latest`: online version text.
    /// `pre_release`: appends " pre-release" like C#. `page_url`: release page (Website button).
    /// `download_url`: installer URL (kept for the API, unused: self-update is not available).
    pub fn new(current: &str, latest: &str, pre_release: bool, page_url: &str, download_url: &str) -> Self {
        let latest = if pre_release { format!("{latest} pre-release") } else { latest.to_owned() };
        NewVersionForm { current: current.to_owned(), latest, page_url: page_url.to_owned(), download_url: download_url.to_owned(), msgs: MessageBoxHost::default() }
    }

    /// Message of the "Download!" button in the Linux port.
    ///
    /// Returns the info box (title from `Strings.BoxAutoUpdateResult`).
    pub fn update_unavailable_box() -> MessageBox {
        MessageBox::new(
            "Automatic update is not available in the Linux port.\nPlease download the new version manually from the LaserGRBL website.",
            tr("Strings.BoxAutoUpdateResult"),
        )
        .icon(MessageBoxIcon::Information)
    }

    /// Draw the modal form; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome once the form closed.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<NewVersionOutcome> {
        let title = tr(&format!("{P}$this.Text")).to_owned();
        let ev = run_modal(ctx, "new-version", &title, 323.0, true, |ui| self.ui(ui));
        if self.msgs.is_open() {
            self.msgs.show(ctx);
            return None;
        }
        match ev? {
            ModalEvent::Done(()) | ModalEvent::Closed => Some(NewVersionOutcome::Cancelled),
        }
    }

    /// Form body. Returns `Some(())` when Cancel was pressed.
    fn ui(&mut self, ui: &mut Ui) -> Option<()> {
        ui.add(egui::Label::new(tr(&format!("{P}TxtHeader.Text"))).wrap());
        ui.add_space(4.0);
        Grid::new("newver").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
            label(ui, P, "TxtCurrentV");
            ui.label(&self.current);
            ui.end_row();
            label(ui, P, "TxtNewV");
            ui.label(&self.latest);
            ui.end_row();
        });
        ui.add_space(4.0);
        // download progress bar (never moves here)
        ui.add(ProgressBar::new(0.0).desired_width(ui.available_width()).desired_height(15.0));
        ui.add_space(6.0);
        let mut out = None;
        ui.horizontal(|ui| {
            use crate::ui::widgets::GrblButton;
            if ui.add(GrblButton::new(ctl_text(P, "BtnWebsite")).min_size(Vec2::new(74.0, 27.0))).clicked() {
                open_link(&self.page_url);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(GrblButton::new(ctl_text(P, "BtnUpdate")).min_size(Vec2::new(74.0, 27.0))).clicked() {
                    self.msgs.push((), Self::update_unavailable_box());
                }
                if ui.add(GrblButton::new(ctl_text(P, "BtnCancel")).min_size(Vec2::new(74.0, 27.0))).clicked() {
                    out = Some(());
                }
            });
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prerelease_suffix() {
        let f = NewVersionForm::new("7.14.1", "7.15.0", true, "https://x", "https://y");
        assert_eq!(f.latest, "7.15.0 pre-release");
        assert_eq!(NewVersionForm::new("7.14.1", "7.15.0", false, "", "").latest, "7.15.0");
    }
}
