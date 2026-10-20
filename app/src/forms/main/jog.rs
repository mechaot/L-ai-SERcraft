//! Ports `JogForm.cs` (+Designer): direction pad, Z buttons, speed and step sliders, and the `StepBar`,
//! `DirectionButton`, `DirectionStepButton` helper controls.
//!
//! `UserControls/Move2DControl.cs` is not used by any form of the original, so it is not ported (see PORTING.md).

use eframe::egui::{self, Align, Layout, Ui, Vec2};
use lasergrbl_core::{GrblCore, JogDirection};
use lg_base::settings;
use lg_dotnet::{decimal_to_f64, f64_to_decimal, fmt_decimal, Decimal};

use crate::ui::icons::icons;
use crate::ui::widgets::color_slider::Orientation;
use crate::ui::widgets::{ImageButton, TickStyle, TrackBar};
use crate::ui::{tr, Theme};

/// `StepBar.values`, in tenths (0.1, 0.2, 0.5, 1, 2, 5, 10, 20, 50, 100, 200).
const STEP_VALUES_TENTHS: [i64; 11] = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000];

/// Step value of a `StepBar` index.
///
/// `index`: 0..=10 (clamped). Returns the step in mm.
pub fn step_value(index: usize) -> Decimal {
    Decimal::new(STEP_VALUES_TENTHS[index.min(STEP_VALUES_TENTHS.len() - 1)], 1)
}

/// `StepBar.Value` setter: index of the value nearest to `v`.
///
/// `v`: wanted step. Returns the index (first of equally near values).
pub fn nearest_step_index(v: Decimal) -> usize {
    let mut found = 0;
    for i in 0..STEP_VALUES_TENTHS.len() {
        if (v - step_value(i)).abs() < (v - step_value(found)).abs() {
            found = i;
        }
    }
    found
}

/// State of `JogForm`.
pub struct JogForm {
    /// `TbSpeed.Value`.
    speed: i32,
    /// `TbSpeed.Maximum` (tracks the machine max rate).
    speed_max: i32,
    /// `StepBar` index.
    step_index: i32,
    /// Direction button currently held (for `JogAbort` on release).
    held: bool,
    /// `BtnHome.Visible = !jog`.
    pub home_visible: bool,
    /// Last (speed, step index) written to the settings.
    saved: (i32, i32),
}

impl Default for JogForm {
    fn default() -> Self {
        Self::new()
    }
}

impl JogForm {
    /// Constructor + `SetCore`: restore speed and step from the settings.
    pub fn new() -> Self {
        let min = 10;
        let mut s = JogForm { speed: 1000, speed_max: 4000, step_index: 6, held: false, home_visible: true, saved: (-1, -1) };
        s.update_fmax();
        s.speed = settings::get("Jog Speed", 1000).clamp(min, s.speed_max);
        let step: f64 = settings::get("Jog Step", 10.0);
        s.step_index = nearest_step_index(f64_to_decimal(step)) as i32;
        s
    }

    /// `UpdateFMax_Tick`: slider maximum follows `$110/$111`.
    fn update_fmax(&mut self) {
        let conf = GrblCore::configuration();
        let cur = decimal_to_f64(conf.max_rate_x().max(conf.max_rate_y())) as i32;
        self.speed_max = cur.max(10);
        self.speed = self.speed.clamp(10, self.speed_max);
    }

    /// `ChangeJogStepIndexBy`.
    ///
    /// `v`: +1 or -1.
    pub fn change_jog_step_index_by(&mut self, v: i32) {
        self.step_index = (self.step_index + v).clamp(0, 10);
    }

    /// `ChangeJogSpeedIndexBy`: moves by `LargeChange` per step.
    ///
    /// `v`: +1 or -1.
    pub fn change_jog_speed_index_by(&mut self, v: i32) {
        let large = (self.speed_max / 10).max(1);
        self.speed = (self.speed + large * v).clamp(10, self.speed_max);
    }

    /// Draw the jog pad (disabled as a whole while `JogEnabled` is false).
    ///
    /// `ui`: parent. `core`: the engine.
    pub fn show(&mut self, ui: &mut Ui, core: &GrblCore) {
        self.update_fmax();
        // push values to the core every frame; persist only on change (TbSpeed_ValueChanged / TbStep_ValueChanged)
        core.set_jog_speed(self.speed);
        core.set_jog_step(step_value(self.step_index as usize));
        if self.saved != (self.speed, self.step_index) {
            self.saved = (self.speed, self.step_index);
            settings::set("Jog Speed", self.speed);
            settings::set("Jog Step", decimal_to_f64(step_value(self.step_index as usize)));
        }

        let enabled = core.jog_enabled();
        let continuous = settings::get("Enable Continuous Jog", false);
        let z_control = settings::get("Enale Z Jog Control", false);
        let th = Theme::current();
        let _ = th;
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                // speed slider + label
                ui.vertical(|ui| {
                    let max = self.speed_max;
                    let r = ui.add(
                        TrackBar::new(&mut self.speed, 10..=max)
                            .orientation(Orientation::Vertical)
                            .tick_style(TickStyle::Both)
                            .tick_frequency(200)
                            .large_change((max / 10).max(1))
                            .small_change((max / 20).max(1))
                            .length(100.0),
                    );
                    r.on_hover_text(format!("{} {}", tr("Strings.SpeedSliderToolTip"), self.speed));
                    ui.label(format!("F{}", self.speed));
                });
                // direction pad
                self.pad(ui, core);
                // Z column
                if z_control {
                    self.z_column(ui, core);
                }
                // step slider
                if !continuous {
                    ui.vertical(|ui| {
                        let mut idx = self.step_index;
                        let r = ui.add(
                            TrackBar::new(&mut idx, 0..=10)
                                .orientation(Orientation::Vertical)
                                .tick_style(TickStyle::Both)
                                .small_change(1)
                                .large_change(1)
                                .length(100.0),
                        );
                        self.step_index = idx;
                        let step = step_value(self.step_index as usize);
                        r.on_hover_text(format!("{} {}", tr("Strings.StepSliderToolTip"), fmt_decimal(step)));
                        ui.label(fmt_decimal(step));
                    });
                }
            });
        });
        // button released anywhere: JogAbort (OnJogButtonMouseUp)
        if self.held && !ui.input(|i| i.pointer.any_down()) {
            self.held = false;
            core.jog_abort();
        }
    }

    /// 3x3 direction grid with the home button in the middle.
    ///
    /// `ui`: parent. `core`: the engine.
    fn pad(&mut self, ui: &mut Ui, core: &GrblCore) {
        use JogDirection::*;
        let grid: [[(JogDirection, &str); 3]; 3] = [
            [(NW, "mdi-arrow-top-left-bold-outline"), (N, "mdi-arrow-up-bold-outline"), (NE, "mdi-arrow-top-right-bold-outline")],
            [(W, "mdi-arrow-left-bold-outline"), (Home, "custom-home"), (E, "mdi-arrow-right-bold-outline")],
            [(SW, "mdi-arrow-bottom-left-bold-outline"), (S, "mdi-arrow-down-bold-outline"), (SE, "mdi-arrow-bottom-right-bold-outline")],
        ];
        egui::Grid::new("jog-pad").spacing([2.0, 2.0]).show(ui, |ui| {
            for row in grid {
                for (dir, icon) in row {
                    if dir == Home && !self.home_visible {
                        ui.allocate_space(Vec2::splat(33.0));
                        continue;
                    }
                    let r = ui.add(ImageButton::new(icon).size(Vec2::splat(33.0)));
                    // OnJogButtonMouseDown: right button = fast
                    if r.hovered() && ui.input(|i| i.pointer.any_pressed()) {
                        let fast = ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary));
                        let _ = core.jog_to_direction_fast(dir, fast);
                        self.held = true;
                    }
                }
                ui.end_row();
            }
        });
        let _ = icons();
    }

    /// Z up / down buttons (10, 1, 0.1) with the "Z" label between.
    ///
    /// `ui`: parent. `core`: the engine.
    fn z_column(&mut self, ui: &mut Ui, core: &GrblCore) {
        let up: [(&str, i64); 3] = [("custom-zup10", 100), ("custom-zup1", 10), ("custom-zup01", 1)];
        let down: [(&str, i64); 3] = [("custom-zdown01", 1), ("custom-zdown1", 10), ("custom-zdown10", 100)];
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let z = |ui: &mut Ui, icon: &str, tenths: i64, dir: JogDirection| {
                let r = ui.add(ImageButton::new(icon).size(Vec2::splat(16.0)));
                // designer tooltip: "0.1mm" / "1mm" / "10mm"
                let r = r.on_hover_text(format!("{}mm", fmt_decimal(Decimal::new(tenths, 1).normalize())));
                if r.hovered() && ui.input(|i| i.pointer.any_pressed()) {
                    // OnZJogButtonMouseDown: fixed step per button
                    let fast = ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary));
                    let _ = core.jog_to_direction_step_fast(dir, fast, Decimal::new(tenths, 1));
                }
            };
            for (icon, t) in up {
                z(ui, icon, t, JogDirection::Zup);
            }
            ui.label("Z");
            for (icon, t) in down {
                z(ui, icon, t, JogDirection::Zdown);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_values() {
        assert_eq!(step_value(0).to_string(), "0.1");
        assert_eq!(step_value(6).to_string(), "10.0");
        assert_eq!(step_value(10).to_string(), "200.0");
    }

    #[test]
    fn nearest_index() {
        assert_eq!(nearest_step_index(Decimal::new(10, 0)), 6);
        assert_eq!(nearest_step_index(Decimal::new(3, 1)), 1);
        assert_eq!(nearest_step_index(Decimal::new(1000, 0)), 10);
    }
}
