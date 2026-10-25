// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Andreas Pösch

//! Preset picker of the raster import "P/S helper" button.
//!
//! Not part of the C# app (replaces `PSHelper/PSHelperForm.cs`): pick a saved layer preset, apply
//! its speed, power and laser mode.

use eframe::egui::{self, Align, Key, Layout, Ui};
use lg_services::layer_presets::LayerPreset;

use super::SharedPresetDb;
use crate::forms::raster::form_modal_x;
use crate::ui::i18n::{tr, tr_plain};
use crate::ui::widgets::flat_combo;

/// Resource prefix of the form.
const P: &str = "PSHelper.PSHelperForm";

/// Setting that stores the last picked preset name.
const LAST_KEY: &str = "Presets.LastRasterPick";

/// Preset picker form.
pub struct PresetPickerForm {
    uid: u64,
    /// Snapshot of the presets (sorted).
    presets: Vec<LayerPreset>,
    /// Selected index into `presets`.
    sel: Option<usize>,
}

impl PresetPickerForm {
    /// Ctor: snapshot the presets, restore the last pick.
    ///
    /// `db`: preset database. Returns the form.
    pub fn new(db: SharedPresetDb) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let presets = db.lock().unwrap_or_else(|e| e.into_inner()).presets().to_vec();
        // last pick by name, else first
        let last: Option<String> = lg_base::settings::get(LAST_KEY, None);
        let sel = last.and_then(|n| presets.iter().position(|p| p.name.eq_ignore_ascii_case(&n))).or(if presets.is_empty() { None } else { Some(0) });
        Self { uid: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed), presets, sel }
    }

    /// Draw the form; call every frame while open.
    ///
    /// `ctx`: egui context. Returns None while open; `Some(Some(preset))` after "Apply";
    /// `Some(None)` on cancel, Esc or the close box.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Option<LayerPreset>> {
        let uid = self.uid;
        let title = tr_plain(&format!("{P}.$this.Text"));
        let esc = ctx.input(|i| i.key_pressed(Key::Escape));
        let (r, closed) = form_modal_x(ctx, ("preset_picker", uid), &title, |ui| self.body(ui, uid));
        r.or(if esc || closed { Some(None) } else { None })
    }

    /// Form body.
    ///
    /// `ui`: parent. `uid`: form id. Returns the result once a button was clicked.
    fn body(&mut self, ui: &mut Ui, uid: u64) -> Option<Option<LayerPreset>> {
        ui.set_width(320.0);
        ui.horizontal(|ui| {
            ui.label(tr_plain("Presets.PickerLabel"));
            let names: Vec<String> = self.presets.iter().map(|p| p.name.clone()).collect();
            let w = ui.available_width().max(60.0);
            flat_combo(ui, ("preset_pick", uid), &mut self.sel, &names, w);
        });
        ui.add_space(4.0);
        // read-only values of the pick
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label(crate::ui::fonts::bold(tr(&format!("{P}.groupBox1.Text"))));
            egui::Grid::new(("preset_pick_res", uid)).num_columns(2).spacing([6.0, 4.0]).show(ui, |ui| {
                let (power, speed, passes, mode) = match self.sel.and_then(|i| self.presets.get(i)) {
                    Some(p) => (format!("{:.1} %", p.power_pct), format!("{} mm/min", p.speed), format!("{} pass", p.passes), p.laser_on.clone()),
                    None => Default::default(),
                };
                let rows = [(tr_plain(&format!("{P}.LblPower.Text")), power), (tr_plain(&format!("{P}.LblSpeed.Text")), speed), (tr_plain(&format!("{P}.LblPasses.Text")), passes), (tr_plain("Presets.ColMode"), mode)];
                for (lbl, mut txt) in rows {
                    ui.label(lbl);
                    ui.add(egui::TextEdit::singleline(&mut txt).desired_width(120.0).interactive(false));
                    ui.end_row();
                }
            });
        });
        ui.add_space(6.0);
        ui.add(egui::Label::new(tr("Presets.PickerNote")).wrap());
        ui.add_space(6.0);
        let mut out = None;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let picked = self.sel.and_then(|i| self.presets.get(i)).cloned();
            // Apply: remember the pick, return the preset
            let apply = ui.add_enabled(picked.is_some(), egui::Button::new(tr_plain(&format!("{P}.BtnApply.Text"))).min_size(egui::vec2(75.0, 22.0)));
            if apply.clicked()
                && let Some(p) = picked
            {
                lg_base::settings::set(LAST_KEY, p.name.clone());
                out = Some(Some(p));
            }
            if ui.add_sized([75.0, 22.0], egui::Button::new(tr_plain(&format!("{P}.BtnCancel.Text")))).clicked() {
                out = Some(None);
            }
        });
        out
    }
}
