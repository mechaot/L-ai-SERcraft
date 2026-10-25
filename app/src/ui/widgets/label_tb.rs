// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `UserControls/LabelTB.cs`: override slider (Rapid / Linear / Power) with label and reset button.

use eframe::egui::{Response, Ui};

use super::color_slider::Orientation;
use super::track_bar::{TickStyle, TrackBar};
use crate::ui::icons::icon;

/// Which override a [`LabelTb`] controls (`function` ctor argument).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverrideKind {
    /// 0: Rapid; value 0/1/2 = 25%/50%/100%.
    Rapid,
    /// 1: Linear feed override 10..200.
    Linear,
    /// 2: Power (S) override 10..200.
    Power,
}

/// Result of one frame.
pub struct LabelTbOutput {
    pub response: Response,
    /// Value changed this frame (`TB_ValueChanged`).
    pub changed: bool,
    /// Mouse released on slider or reset pressed (`TB_MouseUp` -> `Core.ManageOverrides()`).
    pub commit: bool,
}

/// Override slider state.
pub struct LabelTb {
    pub kind: OverrideKind,
    /// Raw trackbar value (Rapid: 0..=2).
    pub value: i32,
    pub label_text: String,
}

impl LabelTb {
    /// New control from the current core override percent.
    ///
    /// `kind`: which override. `percent`: current override (Rapid: 25/50/100; others 10..200).
    pub fn new(kind: OverrideKind, percent: i32) -> Self {
        let (label, value) = match kind {
            OverrideKind::Rapid => ("Rapid", match percent { 25 => 0, 50 => 1, _ => 2 }),
            OverrideKind::Linear => ("Linear", percent),
            OverrideKind::Power => ("Power", percent),
        };
        LabelTb { kind, value, label_text: label.to_owned() }
    }

    /// Override percent to send to core (`TOverrideG0/G1/S`).
    pub fn percent(&self) -> i32 {
        match self.kind {
            OverrideKind::Rapid => match self.value { 0 => 25, 1 => 50, _ => 100 },
            _ => self.value,
        }
    }

    /// `RefreshText`: "Linear [1.00x]".
    pub fn text(&self) -> String {
        format!("{} [{:.2}x]", self.label_text, self.percent() as f64 / 100.0)
    }

    /// Draw. Double click on slider resets to 100 (Rapid: index 2), as C#.
    ///
    /// `ui`: parent. `length`: slider length in points. Returns events.
    pub fn show(&mut self, ui: &mut Ui, length: f32) -> LabelTbOutput {
        let (min, max, small, large, tick) = match self.kind {
            OverrideKind::Rapid => (0, 2, 1, 1, 1),
            _ => (10, 200, 5, 10, 10),
        };
        let reset_to = match self.kind { OverrideKind::Rapid => 2, _ => 100 };
        let before = self.value;
        let mut commit = false;
        let mut resp = None;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(self.text());
                let r = ui.add(
                    TrackBar::new(&mut self.value, min..=max)
                        .small_change(small)
                        .large_change(large)
                        .tick_frequency(tick)
                        .tick_style(TickStyle::BottomRight)
                        .orientation(Orientation::Horizontal)
                        .length(length),
                );
                if r.double_clicked() {
                    self.value = reset_to;
                }
                if r.drag_stopped() || r.double_clicked() || r.clicked() {
                    commit = true;
                }
                resp = Some(r);
            });
            // reset button (BtnReset)
            let b = ui.add(eframe::egui::Button::image(icon(ui.ctx(), "mdi-restore", 16.0)).frame(false));
            if b.clicked() {
                self.value = reset_to;
                commit = true;
            }
        });
        LabelTbOutput { response: resp.expect("track bar drawn"), changed: before != self.value, commit }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        let mut l = LabelTb::new(OverrideKind::Rapid, 50);
        assert_eq!(l.text(), "Rapid [0.50x]");
        l.value = 5;
        assert_eq!(l.percent(), 100);
        assert_eq!(LabelTb::new(OverrideKind::Power, 120).text(), "Power [1.20x]");
    }
}
