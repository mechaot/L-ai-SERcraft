//! Ports `SafetyCountdown.cs` + `SafetyCountdown.Designer.cs`: "Safety first!" 5 s countdown before a job.
//!
//! Settings: `DisableSafetyCountdown` (skip dialog), `QuietSafetyCountdown` (no beep).
//! Use [`SafetyCountdown::needed`] (C# `CanGo` short-cut) before opening the dialog, then run the
//! core continuation only on [`SafetyOutcome::Go`].

use eframe::egui::{self, RichText, Ui, Vec2};
use std::time::{Duration, Instant};

use super::common::{ModalEvent, button, modal, trf};
use crate::ui::icons::{Tint, icons};

/// Seconds of the countdown (`down = 5`).
const START_SECONDS: i32 = 5;

/// Result of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyOutcome {
    /// Start pressed or countdown reached 0 (`DialogResult.OK`).
    Go,
    /// Cancel / close box.
    Cancel,
}

/// Countdown logic without UI: `TimCountDown_Tick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Countdown {
    pub down: i32,
}

impl Countdown {
    /// New countdown at 5.
    pub fn new() -> Self {
        Countdown { down: START_SECONDS }
    }

    /// One timer tick: `down = max(0, down-1)`.
    ///
    /// Returns true when the dialog must close with OK (`down == 0`), else the caller beeps.
    pub fn tick(&mut self) -> bool {
        self.down = (self.down - 1).max(0);
        self.down == 0
    }

    /// Start button text, `"{lbl} ({down})"`.
    ///
    /// `lbl`: translated caption of the button.
    pub fn button_text(&self, lbl: &str) -> String {
        format!("{lbl} ({})", self.down)
    }
}

impl Default for Countdown {
    fn default() -> Self {
        Self::new()
    }
}

/// The countdown dialog.
pub struct SafetyCountdown {
    cd: Countdown,
    last_tick: Option<Instant>,
    id: u64,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl SafetyCountdown {
    /// C# `CanGo` pre-check: false when the countdown is disabled in the settings.
    ///
    /// Returns true when the dialog has to be shown before starting a job.
    pub fn needed() -> bool {
        !lg_base::settings::get("DisableSafetyCountdown", false)
    }

    /// Create the dialog.
    pub fn open() -> Self {
        SafetyCountdown { cd: Countdown::new(), last_tick: None, id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed) }
    }

    /// `DoBeep`: play `beep.wav` unless `QuietSafetyCountdown`.
    fn beep() {
        if !lg_base::settings::get("QuietSafetyCountdown", false) {
            lg_base::sound::play_file(&lg_base::paths::resources_dir().join("sound").join("beep.wav"));
        }
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<SafetyOutcome> {
        // Shown event: timer starts, first beep
        if self.last_tick.is_none() {
            self.last_tick = Some(Instant::now());
            Self::beep();
        }
        if let Some(t) = self.last_tick {
            if t.elapsed() >= Duration::from_secs(1) {
                self.last_tick = Some(t + Duration::from_secs(1));
                if self.cd.tick() {
                    return Some(SafetyOutcome::Go);
                }
                Self::beep();
            }
        }
        ctx.request_repaint_after(Duration::from_millis(100));
        let title = trf("SafetyCountdown.$this.Text", "Safety first!");
        let id = ("safety", self.id);
        match modal(ctx, id, &title, 500.0, true, |ui| self.body(ui)) {
            Some(ModalEvent::Done(true)) => Some(SafetyOutcome::Go),
            Some(ModalEvent::Done(false)) | Some(ModalEvent::Closed) => Some(SafetyOutcome::Cancel),
            None => None,
        }
    }

    /// Form body. Returns Some(true)=Start, Some(false)=Cancel.
    fn body(&mut self, ui: &mut Ui) -> Option<bool> {
        let mut out = None;
        ui.horizontal(|ui| {
            ui.add(icons().image(ui.ctx(), "mdi-safety-goggles", 128.0).tint(crate::ui::Theme::current().form_fore_color).fit_to_exact_size(Vec2::splat(128.0)));
            ui.vertical(|ui| {
                ui.set_min_height(128.0);
                ui.add_space(20.0);
                let text = trf("SafetyCountdown.label1.Text", "Engraving is about to start.\r\nPlease wear your safety goggles!");
                ui.label(RichText::new(text.replace("\r\n", "\n")).size(16.0));
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    let sz = Vec2::new(120.0, 30.0);
                    let cancel = trf("SafetyCountdown.BtnCancel.Text", "Cancel");
                    let start = self.cd.button_text(&trf("SafetyCountdown.BtnStart.Text", "Start"));
                    if button(ui, &cancel, Some("mdi-close-box"), sz) {
                        out = Some(false);
                    }
                    if button(ui, &start, Some("mdi-checkbox-marked"), sz) {
                        out = Some(true);
                    }
                });
            });
        });
        let _ = Tint::None;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::common::test_guard as settings_guard;

    #[test]
    fn countdown_ticks_to_ok() {
        let mut c = Countdown::new();
        assert_eq!(c.button_text("Start"), "Start (5)");
        for _ in 0..4 {
            assert!(!c.tick() || c.down == 0);
        }
        assert_eq!(c.down, 1);
        assert!(c.tick());
        assert_eq!(c.button_text("Start"), "Start (0)");
        // never negative
        assert!(c.tick());
        assert_eq!(c.down, 0);
    }

    #[test]
    fn disabled_by_setting() {
        let _g = settings_guard();
        assert!(SafetyCountdown::needed());
        lg_base::settings::set("DisableSafetyCountdown", true);
        assert!(!SafetyCountdown::needed());
    }
}
