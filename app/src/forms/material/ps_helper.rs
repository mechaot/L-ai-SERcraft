//! Ports `PSHelper/PSHelperForm.cs` (+ Designer): cascading model -> material -> action ->
//! thickness selection that suggests power / speed / passes.

use eframe::egui::{self, Align, Key, Layout, Ui};
use lg_services::material_db::MaterialRow;

use super::SharedMaterialDb;
use crate::forms::raster::form_modal_x;
use crate::ui::i18n::{tr, tr_plain};
use crate::ui::widgets::flat_combo;

/// Resource prefix of the form.
const P: &str = "PSHelper.PSHelperForm";

/// `PSHelperForm`.
pub struct PsHelperForm {
    uid: u64,
    db: SharedMaterialDb,
    models: Vec<String>,
    materials: Vec<String>,
    actions: Vec<String>,
    thicknesses: Vec<String>,
    sel_model: Option<usize>,
    sel_material: Option<usize>,
    sel_action: Option<usize>,
    sel_thickness: Option<usize>,
    /// `current`: row of the selected combination.
    current: Option<MaterialRow>,
}

/// Which combo of the cascade changed.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Level {
    Model,
    Material,
    Action,
    Thickness,
}

/// Items of `list` for a new selection: keep `last` when still present, else first (C# handlers).
///
/// `list`: new items. `last`: previous selected text. Returns the new selected index.
fn keep_or_first(list: &[String], last: Option<&str>) -> Option<usize> {
    match last.and_then(|l| list.iter().position(|s| s == l)) {
        Some(i) => Some(i),
        None if !list.is_empty() => Some(0),
        None => None,
    }
}

impl PsHelperForm {
    /// Ctor: `InitModels()` then `SetDefault` for the four "DB Last Used ..." settings.
    ///
    /// `db`: material database. Returns the form.
    pub fn new(db: SharedMaterialDb) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let mut f = Self {
            uid: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            db,
            models: Vec::new(),
            materials: Vec::new(),
            actions: Vec::new(),
            thicknesses: Vec::new(),
            sel_model: None,
            sel_material: None,
            sel_action: None,
            sel_thickness: None,
            current: None,
        };
        // InitModels
        f.models = f.db.lock().unwrap_or_else(|e| e.into_inner()).materials.models();
        f.sel_model = if f.models.is_empty() { None } else { Some(0) };
        f.cascade(Level::Model, true);
        // SetDefault x4: select the stored value when it is in the list
        for (key, level) in [
            ("DB Last Used Laser Model", Level::Model),
            ("DB Last Used Material", Level::Material),
            ("DB Last Used Action", Level::Action),
            ("DB Last Used Thickness", Level::Thickness),
        ] {
            let value: Option<String> = lg_base::settings::get(key, None);
            let Some(value) = value else { continue };
            let (items, sel) = f.level_mut(level);
            if let Some(i) = items.iter().position(|s| *s == value) {
                if *sel != Some(i) {
                    *sel = Some(i);
                    f.cascade(level, false);
                }
            }
        }
        f
    }

    /// Items and selection of one combo.
    fn level_mut(&mut self, l: Level) -> (&Vec<String>, &mut Option<usize>) {
        match l {
            Level::Model => (&self.models, &mut self.sel_model),
            Level::Material => (&self.materials, &mut self.sel_material),
            Level::Action => (&self.actions, &mut self.sel_action),
            Level::Thickness => (&self.thicknesses, &mut self.sel_thickness),
        }
    }

    fn text(items: &[String], sel: Option<usize>) -> Option<String> {
        sel.and_then(|i| items.get(i)).cloned()
    }

    /// `Cb*_SelectedIndexChanged` chain: refill the dependent combos below `from`.
    ///
    /// `from`: combo that changed. `_init`: true for the very first fill (nothing to keep).
    fn cascade(&mut self, from: Level, _init: bool) {
        let db = self.db.clone();
        let g = db.lock().unwrap_or_else(|e| e.into_inner());
        let t = &g.materials;
        if from == Level::Model {
            let last = Self::text(&self.materials, self.sel_material);
            self.materials = t.materials(Self::text(&self.models, self.sel_model).as_deref().unwrap_or(""));
            self.sel_material = keep_or_first(&self.materials, last.as_deref());
        }
        if matches!(from, Level::Model | Level::Material) {
            let last = Self::text(&self.actions, self.sel_action);
            self.actions = t.actions(
                Self::text(&self.models, self.sel_model).as_deref().unwrap_or(""),
                Self::text(&self.materials, self.sel_material).as_deref().unwrap_or(""),
            );
            self.sel_action = keep_or_first(&self.actions, last.as_deref());
        }
        if matches!(from, Level::Model | Level::Material | Level::Action) {
            let last = Self::text(&self.thicknesses, self.sel_thickness);
            self.thicknesses = t.thickness(
                Self::text(&self.models, self.sel_model).as_deref().unwrap_or(""),
                Self::text(&self.materials, self.sel_material).as_deref().unwrap_or(""),
                Self::text(&self.actions, self.sel_action).as_deref().unwrap_or(""),
            );
            self.sel_thickness = keep_or_first(&self.thicknesses, last.as_deref());
        }
        // CbThickness_SelectedIndexChanged
        self.current = t
            .get_result(
                Self::text(&self.models, self.sel_model).as_deref().unwrap_or(""),
                Self::text(&self.materials, self.sel_material).as_deref().unwrap_or(""),
                Self::text(&self.actions, self.sel_action).as_deref().unwrap_or(""),
                Self::text(&self.thicknesses, self.sel_thickness).as_deref().unwrap_or(""),
            )
            .cloned();
    }

    /// Draw the form; call every frame while open.
    ///
    /// `ctx`: egui context. Returns None while open; `Some(Some(row))` after "Apply";
    /// `Some(None)` on cancel (C# `CreateAndShowDialog` result).
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Option<MaterialRow>> {
        let uid = self.uid;
        let title = tr_plain(&format!("{P}.$this.Text"));
        let esc = ctx.input(|i| i.key_pressed(Key::Escape));
        let (r, closed) = form_modal_x(ctx, ("ps_helper", uid), &title, |ui| self.body(ui, uid));
        r.or(if esc || closed { Some(None) } else { None })
    }

    /// Form body.
    fn body(&mut self, ui: &mut Ui, uid: u64) -> Option<Option<MaterialRow>> {
        ui.set_width(365.0);
        let mut changed: Option<Level> = None;
        ui.horizontal_top(|ui| {
            // left: tableLayoutPanel1
            egui::Grid::new(("ps_helper_grid", uid)).num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
                let rows = [
                    ("LblModel", Level::Model, "cb_model"),
                    ("LblMaterial", Level::Material, "cb_material"),
                    ("LblAction", Level::Action, "cb_action"),
                    ("LblThickness", Level::Thickness, "cb_thickness"),
                ];
                for (lbl, level, id) in rows {
                    ui.label(tr_plain(&format!("{P}.{lbl}.Text")));
                    let (items, sel) = match level {
                        Level::Model => (self.models.clone(), &mut self.sel_model),
                        Level::Material => (self.materials.clone(), &mut self.sel_material),
                        Level::Action => (self.actions.clone(), &mut self.sel_action),
                        Level::Thickness => (self.thicknesses.clone(), &mut self.sel_thickness),
                    };
                    // Enabled = Items.Count > 1
                    let r = ui.add_enabled_ui(items.len() > 1, |ui| flat_combo(ui, (id, uid), sel, &items, 121.0)).inner;
                    if r.changed() {
                        changed = Some(level);
                    }
                    ui.end_row();
                }
            });
            ui.add_space(8.0);
            // right: groupBox1 "Suggested settings"
            ui.group(|ui| {
                ui.label(crate::ui::fonts::bold(tr(&format!("{P}.groupBox1.Text"))));
                egui::Grid::new(("ps_helper_res", uid)).num_columns(2).spacing([6.0, 4.0]).show(ui, |ui| {
                    let (power, speed, passes) = match &self.current {
                        Some(c) => (format!("{} %", c.power), format!("{} mm/min", c.speed), format!("{} pass", c.cycles)),
                        None => Default::default(),
                    };
                    for (lbl, mut txt) in [("LblPower", power), ("LblSpeed", speed), ("LblPasses", passes)] {
                        ui.label(tr_plain(&format!("{P}.{lbl}.Text")));
                        ui.add(egui::TextEdit::singleline(&mut txt).desired_width(100.0).interactive(false));
                        ui.end_row();
                    }
                });
            });
        });
        if let Some(l) = changed {
            self.cascade(l, false);
        }
        ui.add_space(6.0);
        // label1: manufacturer warning
        ui.add(egui::Label::new(tr(&format!("{P}.label1.Text"))).wrap());
        ui.add_space(6.0);
        let mut out = None;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // BtnApply: remember selection, return row
            if ui.add_sized([75.0, 22.0], egui::Button::new(tr_plain(&format!("{P}.BtnApply.Text")))).clicked() {
                for (key, level) in [
                    ("DB Last Used Laser Model", Level::Model),
                    ("DB Last Used Material", Level::Material),
                    ("DB Last Used Action", Level::Action),
                    ("DB Last Used Thickness", Level::Thickness),
                ] {
                    let (items, sel) = self.level_mut(level);
                    if let Some(t) = sel.and_then(|i| items.get(i)) {
                        lg_base::settings::set(key, t.clone());
                    }
                }
                out = Some(self.current.clone());
            }
            if ui.add_sized([75.0, 22.0], egui::Button::new(tr_plain(&format!("{P}.BtnCancel.Text")))).clicked() {
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
    fn keep_selection() {
        let l = vec!["a".to_string(), "b".to_string()];
        assert_eq!(keep_or_first(&l, Some("b")), Some(1));
        assert_eq!(keep_or_first(&l, Some("x")), Some(0));
        assert_eq!(keep_or_first(&[], Some("x")), None);
    }
}
