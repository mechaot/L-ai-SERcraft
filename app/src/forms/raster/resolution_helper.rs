// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `RasterConverter/ResolutionHelperForm.cs` (+ Designer): computes a line resolution that
//! matches the stepper resolution (`$100`).

use eframe::egui::{self, Align, Key, Layout, RichText, Ui};
use lg_dotnet::{decimal_to_f64, dec_div, f64_to_decimal, round_decimal, Decimal};

use crate::ui::i18n::{tr, tr_plain};
use crate::ui::widgets::{GrblButton, NumericUpDown};

use super::form_modal_x;

/// Resource prefix of the form.
const P: &str = "RasterConverter.ResolutionHelperForm";

/// `ResolutionHelperForm`.
pub struct ResolutionHelperForm {
    uid: u64,
    /// `UDDesired.Value`.
    desired: f64,
    /// `UDHardware.Value`.
    hardware: f64,
    /// `UDComputed.Value`.
    computed: f64,
    /// `UDDesired.Maximum` = `UDComputed.Maximum` (`GetMaxQuality`).
    max_quality: f64,
}

/// `GetMaxQuality()`: 50 with "Raster Hi-Res", else 20.
fn get_max_quality() -> f64 {
    if lg_base::settings::get("Raster Hi-Res", false) { 50.0 } else { 20.0 }
}

impl ResolutionHelperForm {
    /// `CreateAndShowDialog` setup part: fill the fields and run the first `Compute`.
    ///
    /// `oldval`: current quality (lines/mm) shown as desired resolution.
    /// Returns the form, ready for [`show`](Self::show).
    pub fn new(oldval: f64) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let max_quality = get_max_quality();
        let cfg = lg_gcode::configuration::configuration();
        let mut f = Self {
            uid: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            // UpDown ranges: desired 1..max, hardware 0.1..10000
            desired: oldval.clamp(1.0, max_quality),
            hardware: decimal_to_f64(cfg.resolution_x()).clamp(0.1, 10000.0),
            computed: 3.0,
            max_quality,
        };
        f.compute();
        f
    }

    /// `Compute`: nearest resolution that is a whole division of the hardware steps.
    fn compute(&mut self) {
        let hw = f64_to_decimal(self.hardware);
        let des = f64_to_decimal(self.desired);
        let max = f64_to_decimal(self.max_quality);
        let min = Decimal::ONE;
        // C# try/catch: divide by zero is logged, value stays
        let res = (|| {
            let ratio = dec_div(hw, des)?;
            let new_res = dec_div(hw, round_decimal(ratio, 0))?;
            if new_res > max {
                // MaxRes
                let m = dec_div(hw, dec_div(hw, max)?.ceil())?;
                Some(max.min(m))
            } else if new_res < min {
                // MinRes
                let m = dec_div(hw, dec_div(hw, min)?.floor())?;
                Some(min.max(m))
            } else {
                Some(new_res)
            }
        })();
        match res {
            Some(v) => self.computed = decimal_to_f64(v).clamp(1.0, self.max_quality),
            None => {
                lg_base::logger::log_message("ResolutionHelper", &format!("Ex with data [{}]HW [{}]DV", self.hardware, self.desired));
                lg_base::logger::log_exception("ResolutionHelper", &"Attempted to divide by zero.");
            }
        }
    }

    /// Draw the form; call every frame while open.
    ///
    /// `ctx`: egui context. Returns None while open; `Some(Some(v))` after "Set" (`mRetVal` =
    /// computed value); `Some(None)` on cancel.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Option<f64>> {
        let uid = self.uid;
        let title = tr_plain(&format!("{P}.$this.Text"));
        let esc = ctx.input(|i| i.key_pressed(Key::Escape));
        let (r, closed) = form_modal_x(ctx, ("resolution_helper", uid), &title, |ui| self.body(ui, uid));
        r.or(if esc || closed { Some(None) } else { None })
    }

    /// Form body: link, three numeric rows, buttons.
    fn body(&mut self, ui: &mut Ui, uid: u64) -> Option<Option<f64>> {
        ui.set_width(300.0);
        // linkLabel1
        let link = ui.add(egui::Label::new(RichText::new(tr(&format!("{P}.linkLabel1.Text"))).underline().color(egui::Color32::from_rgb(0, 102, 204))).wrap().sense(egui::Sense::click()));
        if link.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            lg_base::tools::utils::open_link("https://lasergrbl.com/usage/raster-image-import/setting-reliable-resolution/");
        }
        ui.add_space(4.0);
        let mut recompute = false;
        egui::Grid::new(("resolution_helper_grid", uid)).num_columns(3).spacing([6.0, 6.0]).show(ui, |ui| {
            // desired
            ui.label(tr_plain(&format!("{P}.LblBorderTracing.Text")));
            let before = self.desired;
            ui.add(NumericUpDown::new(&mut self.desired).range(1.0, self.max_quality).decimal_places(3).width(64.0).id(egui::Id::unique(("rh_desired", uid))));
            recompute |= before != self.desired;
            ui.label(tr_plain(&format!("{P}.LblBorderTracingmm.Text")));
            ui.end_row();
            // hardware
            ui.label(tr_plain(&format!("{P}.LblLinearFilling.Text")));
            let before = self.hardware;
            ui.add(NumericUpDown::new(&mut self.hardware).range(0.1, 10000.0).decimal_places(3).width(64.0).id(egui::Id::unique(("rh_hardware", uid))));
            recompute |= before != self.hardware;
            ui.label(tr_plain(&format!("{P}.LblLinearFillingmm.Text")));
            ui.end_row();
            // computed (no handler on its own change)
            ui.label(tr_plain(&format!("{P}.label19.Text")));
            ui.add(NumericUpDown::new(&mut self.computed).range(1.0, self.max_quality).decimal_places(3).width(64.0).id(egui::Id::unique(("rh_computed", uid))));
            ui.label(tr_plain(&format!("{P}.label20.Text")));
            ui.end_row();
        });
        if recompute {
            self.compute();
        }
        ui.add_space(6.0);
        let mut out = None;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // BtnCreate: remember hardware resolution, return computed
            if ui.add(GrblButton::new(tr_plain(&format!("{P}.BtnCreate.Text"))).icon("mdi-checkbox-marked").min_size(egui::vec2(74.0, 27.0))).clicked() {
                lg_base::settings::set("Hardware Resolution", f64_to_decimal(self.hardware));
                out = Some(Some(self.computed));
            }
            if ui.add(GrblButton::new(tr_plain(&format!("{P}.BtnCancel.Text"))).icon("mdi-close-box").min_size(egui::vec2(74.0, 27.0))).clicked() {
                out = Some(None);
            }
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_matching_resolution() {
        let mut f = ResolutionHelperForm { uid: 0, desired: 3.0, hardware: 250.0, computed: 0.0, max_quality: 20.0 };
        f.compute();
        // 250 / round(250/3 = 83.33) = 250 / 83
        assert!((f.computed - 250.0 / 83.0).abs() < 1e-9);
    }

    #[test]
    fn clamps_to_max() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let mut f = ResolutionHelperForm { uid: 0, desired: 20.0, hardware: 10.0, computed: 0.0, max_quality: 20.0 };
        f.compute();
        // 10 / round(0.5)=0 -> divide by zero path keeps value
        assert_eq!(f.computed, 0.0);
    }
}
