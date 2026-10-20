//! Ports `WiFiConfigurator/OrturWiFiConfigPrompt.cs` (class `WiFiConfigPrompt`) + Designer:
//! "Configure machine for WiFi?" question shown after connecting to an Ortur/Longer board.

use eframe::egui::{self, Ui, Vec2};
use lasergrbl_core::GrblCore;
use std::sync::Arc;

use super::common::{ModalEvent, button_en, modal, trf};
use crate::ui::tr_fmt;

/// Setting key of the "Do not show again" checkbox.
pub const SUPPRESS_KEY: &str = "Suppress WiFI Prompt Message";

/// Result of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiPromptOutcome {
    /// "Yes please!": open `OrturWiFiConfig`.
    Yes,
    /// Cancel / close box.
    Cancel,
}

/// The dialog.
pub struct OrturWiFiConfigPrompt {
    machine: String,
    dont_show: bool,
    id: u64,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl OrturWiFiConfigPrompt {
    /// Prompt should be skipped because the user ticked "Do not show again" earlier.
    ///
    /// Returns true when `Suppress WiFI Prompt Message` is set.
    pub fn suppressed() -> bool {
        lg_base::settings::get(SUPPRESS_KEY, false)
    }

    /// Create the dialog (C# ctor).
    ///
    /// `core`: engine; `GrblVersion.MachineName` fills the `{0}` of the prompt.
    pub fn open(core: &Arc<GrblCore>) -> Self {
        let name = core.grbl_version().and_then(|v| v.machine_name().map(str::to_owned)).unwrap_or_default();
        Self::new(&name)
    }

    /// Same with an explicit machine name.
    ///
    /// `machine`: text for `{0}`.
    pub fn new(machine: &str) -> Self {
        OrturWiFiConfigPrompt { machine: machine.to_owned(), dont_show: Self::suppressed(), id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed) }
    }

    /// Prompt text with the machine name inserted.
    pub fn prompt_text(&self) -> String {
        tr_fmt("WiFiConfigurator.OrturWiFiConfigPrompt.LblPrompt.Text", &[&self.machine])
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<WifiPromptOutcome> {
        let title = trf("WiFiConfigurator.OrturWiFiConfigPrompt.$this.Text", "Configure machine for WiFi?");
        let id = ("wifiprompt", self.id);
        match modal(ctx, id, &title, 460.0, true, |ui| self.body(ui)) {
            Some(ModalEvent::Done(true)) => Some(WifiPromptOutcome::Yes),
            Some(ModalEvent::Done(false)) | Some(ModalEvent::Closed) => Some(WifiPromptOutcome::Cancel),
            None => None,
        }
    }

    /// Form body. Returns Some(true)=Yes, Some(false)=Cancel.
    fn body(&mut self, ui: &mut Ui) -> Option<bool> {
        let t = |c: &str| trf(&format!("WiFiConfigurator.OrturWiFiConfigPrompt.{c}.Text"), "");
        // pictureBox1 (64x64) left, prompt text right
        ui.horizontal_top(|ui| {
            ui.add(crate::ui::icons::icons().image(ui.ctx(), "mdi-wifi", 64.0));
            ui.add(egui::Label::new(self.prompt_text()).wrap());
        });
        ui.add_space(8.0);
        // CbDontShow_CheckedChanged stores the setting at once
        if ui.checkbox(&mut self.dont_show, t("CbDontShow")).changed() {
            lg_base::settings::set(SUPPRESS_KEY, self.dont_show);
        }
        let mut out = None;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // designer: 118x42 with image
            let sz = Vec2::new(118.0, 42.0);
            if button_en(ui, true, &t("BtnOK"), Some("mdi-checkbox-marked"), sz) {
                out = Some(true);
            }
            if button_en(ui, true, &t("BtnCancel"), Some("mdi-close-box"), sz) {
                out = Some(false);
            }
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::common::test_guard as settings_guard;

    #[test]
    fn prompt_has_machine_name_and_setting() {
        let _g = settings_guard();
        let p = OrturWiFiConfigPrompt::new("Ortur Laser Master 2");
        assert!(p.prompt_text().contains("Ortur Laser Master 2"));
        assert!(!OrturWiFiConfigPrompt::suppressed());
        lg_base::settings::set(SUPPRESS_KEY, true);
        assert!(OrturWiFiConfigPrompt::suppressed());
        assert!(OrturWiFiConfigPrompt::new("x").dont_show);
    }
}
