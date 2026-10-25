// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `LaserLifeEdit.cs` + `LaserLifeEdit.Designer.cs`: "Edit Laser" dialog.

use chrono::{Datelike, Local, Months, NaiveDate, NaiveDateTime};
use eframe::egui::{self, TextEdit, Ui, Vec2};
use lasergrbl_core::laser_life::LaserLifeCounter;

use super::common::{ModalEvent, button_en, modal, trf};
use crate::forms::form_util::{load_texture, native_image};
use crate::ui::tr;
use crate::ui::widgets::{DecimalInputRanged, NumericUpDown};

/// Result of the dialog.
#[derive(Debug, Clone, PartialEq)]
pub enum LaserLifeEditOutcome {
    /// Cancel / close box.
    Cancelled,
    /// Save: the edited counter (C# edited the passed object in place). Pass it to
    /// `laser_life::add` (new) or `laser_life::edit` (existing).
    Saved(LaserLifeCounter),
}

/// Earliest purchase date (`MinDate`).
pub fn min_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2015, 1, 1).unwrap_or(NaiveDate::MIN)
}

/// Latest purchase date (`MaxDate = Today.AddMonths(1)`).
///
/// `today`: current local date. Returns the limit.
pub fn max_date(today: NaiveDate) -> NaiveDate {
    today.checked_add_months(Months::new(1)).unwrap_or(today)
}

/// `DiPower_CurrentValueChanged`: values above 200 are taken as mW.
///
/// `v`: typed value. Returns watts.
pub fn normalize_power(v: f32) -> f32 {
    if v > 200.0 { v / 1000.0 } else { v }
}

/// `BtnSave_Click`: write the form fields into `target`.
///
/// `target`: counter to update. `name`,`brand`,`model`: text boxes. `power`: optical power in W
/// (0 = unset). `date`: purchase date.
pub fn apply_fields(target: &mut LaserLifeCounter, name: &str, brand: &str, model: &str, power: f32, date: NaiveDate) {
    target.name = Some(name.trim().to_owned());
    target.purchase_date = date.and_hms_opt(0, 0, 0);
    target.brand = if brand.trim().is_empty() { None } else { Some(brand.trim().to_owned()) };
    target.model = if model.trim().is_empty() { None } else { Some(model.trim().to_owned()) };
    target.optical_power = if power == 0.0 { None } else { Some(power as f64) };
}

/// The dialog.
pub struct LaserLifeEdit {
    target: LaserLifeCounter,
    name: String,
    brand: String,
    model: String,
    power: f32,
    date: NaiveDate,
    today: NaiveDate,
    id: u64,
    tex: Option<Option<egui::TextureHandle>>,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl LaserLifeEdit {
    /// Create the dialog (C# ctor).
    ///
    /// `llc`: counter to edit (cloned; the edited copy comes back in the outcome).
    pub fn open(llc: &LaserLifeCounter) -> Self {
        Self::with_today(llc, Local::now().date_naive())
    }

    /// Same with an explicit "today" (tests).
    ///
    /// `llc`: counter to edit. `today`: current date.
    pub fn with_today(llc: &LaserLifeCounter, today: NaiveDate) -> Self {
        let date = llc.purchase_date.map(|d: NaiveDateTime| d.date()).unwrap_or(today);
        LaserLifeEdit {
            target: llc.clone(),
            name: llc.name.clone().unwrap_or_default(),
            brand: llc.brand.clone().unwrap_or_default(),
            model: llc.model.clone().unwrap_or_default(),
            power: llc.optical_power.map(|p| p as f32).unwrap_or(0.0),
            date: date.clamp(min_date(), max_date(today)),
            today,
            tex: None,
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// `TbLaserName_TextChanged`: Save needs a non-blank name.
    pub fn can_save(&self) -> bool {
        !self.name.trim().is_empty()
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<LaserLifeEditOutcome> {
        let title = trf("LaserLifeEdit.$this.Text", "Edit Laser");
        let id = ("laseredit", self.id);
        match modal(ctx, id, &title, 440.0, true, |ui| self.body(ui)) {
            Some(ModalEvent::Done(true)) => {
                apply_fields(&mut self.target, &self.name, &self.brand, &self.model, self.power, self.date);
                Some(LaserLifeEditOutcome::Saved(self.target.clone()))
            }
            Some(ModalEvent::Done(false)) | Some(ModalEvent::Closed) => Some(LaserLifeEditOutcome::Cancelled),
            None => None,
        }
    }

    /// Date picker made of year/month/day spinners (no `DateTimePicker` in egui).
    fn date_field(&mut self, ui: &mut Ui) {
        let (mut y, mut m, mut d) = (self.date.year() as f64, self.date.month() as f64, self.date.day() as f64);
        ui.horizontal(|ui| {
            let a = ui.add(NumericUpDown::new(&mut y).range(min_date().year() as f64, max_date(self.today).year() as f64).width(64.0).id(egui::Id::unique(("le-y", self.id)))).changed();
            let b = ui.add(NumericUpDown::new(&mut m).range(1.0, 12.0).width(46.0).id(egui::Id::unique(("le-m", self.id)))).changed();
            let c = ui.add(NumericUpDown::new(&mut d).range(1.0, 31.0).width(46.0).id(egui::Id::unique(("le-d", self.id)))).changed();
            if a || b || c {
                self.date = clamp_ymd(y as i32, m as u32, d as u32, self.today);
            }
        });
    }

    /// Form body. Returns Some(true)=Save, Some(false)=Cancel.
    fn body(&mut self, ui: &mut Ui) -> Option<bool> {
        let t = |c: &str| tr(&format!("LaserLifeEdit.{c}.Text")).to_owned();
        // two columns like the designer: captions left (250px), inputs right (200px)
        egui::Grid::new(("laseredit-grid", self.id)).num_columns(2).min_col_width(240.0).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label(t("label1"));
            ui.add_sized([200.0, 20.0], TextEdit::singleline(&mut self.name));
            ui.end_row();
            ui.label(t("label3"));
            ui.add_sized([200.0, 20.0], TextEdit::singleline(&mut self.brand));
            ui.end_row();
            ui.label(t("label4"));
            ui.add_sized([200.0, 20.0], TextEdit::singleline(&mut self.model));
            ui.end_row();
            ui.label(t("label5"));
            ui.horizontal(|ui| {
                let o = DecimalInputRanged::new(&mut self.power).range(-100.0, 100.0).id(egui::Id::unique(("le-p", self.id))).show(ui);
                if o.committed.is_some() {
                    self.power = normalize_power(self.power);
                }
                ui.label(t("label6"));
            });
            ui.end_row();
            ui.label(t("label2"));
            self.date_field(ui);
            ui.end_row();
        });
        ui.add_space(6.0);
        // tableLayoutPanel5: illustration left, help text right
        if self.tex.is_none() {
            self.tex = Some(load_texture(ui.ctx(), "laserlifeedit", include_bytes!("../../../../resources/grafica/laserlifeedit.png")));
        }
        let tex = self.tex.clone().flatten();
        ui.horizontal_top(|ui| {
            native_image(ui, tex.as_ref());
            ui.vertical(|ui| {
                ui.set_width(276.0);
                ui.add(egui::Label::new(t("label7")).wrap());
            });
        });
        ui.add_space(6.0);
        let mut out = None;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // designer: Save 94x42, Cancel 107x42
            if button_en(ui, self.can_save(), &t("BtnSave"), Some("mdi-checkbox-marked"), Vec2::new(94.0, 42.0)) {
                out = Some(true);
            }
            if button_en(ui, true, &t("BtnCancel"), Some("mdi-close-box"), Vec2::new(107.0, 42.0)) {
                out = Some(false);
            }
        });
        out
    }
}

/// Build a date from spinner values, clamping the day to the month and the result to the limits.
///
/// `y`,`m`,`d`: spinner values. `today`: current date. Returns a valid date inside [min, max].
pub fn clamp_ymd(y: i32, m: u32, d: u32, today: NaiveDate) -> NaiveDate {
    let m = m.clamp(1, 12);
    let mut day = d.clamp(1, 31);
    // shrink the day until the date exists
    let date = loop {
        if let Some(dt) = NaiveDate::from_ymd_opt(y, m, day) {
            break dt;
        }
        day -= 1;
    };
    date.clamp(min_date(), max_date(today))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 19).unwrap()
    }

    #[test]
    fn date_limits() {
        assert_eq!(max_date(today()), NaiveDate::from_ymd_opt(2026, 11, 19).unwrap());
        assert_eq!(clamp_ymd(2026, 2, 31, today()), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
        assert_eq!(clamp_ymd(2000, 1, 1, today()), min_date());
        assert_eq!(clamp_ymd(2030, 1, 1, today()), max_date(today()));
    }

    #[test]
    fn power_normalisation() {
        assert_eq!(normalize_power(5.5), 5.5);
        assert_eq!(normalize_power(200.0), 200.0);
        assert!((normalize_power(5000.0) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn save_trims_and_nulls() {
        let mut c = LaserLifeCounter::create_new();
        apply_fields(&mut c, "  Diode  ", "  ", " X1 ", 0.0, today());
        assert_eq!(c.name.as_deref(), Some("Diode"));
        assert_eq!(c.brand, None);
        assert_eq!(c.model.as_deref(), Some("X1"));
        assert_eq!(c.optical_power, None);
        assert_eq!(c.purchase_date, today().and_hms_opt(0, 0, 0));
        apply_fields(&mut c, "D", "B", "", 7.5, today());
        assert_eq!(c.optical_power, Some(7.5));
    }

    #[test]
    fn open_prefills_and_validates_name() {
        let mut c = LaserLifeCounter::create_new();
        c.name = Some("N".into());
        c.optical_power = Some(10.0);
        let mut f = LaserLifeEdit::with_today(&c, today());
        assert!(f.can_save());
        assert_eq!(f.power, 10.0);
        assert_eq!(f.date, today());
        f.name = "   ".into();
        assert!(!f.can_save());
    }
}
