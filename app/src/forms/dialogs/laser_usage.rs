// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `LaserUsage.cs` + `LaserUsage.Designer.cs`: "Laser Lifespan - Monitoring Tool".
//!
//! Uses `lasergrbl_core::laser_life` for the data and [`LaserLifeEdit`] for add/edit.

use chrono::NaiveDateTime;
use eframe::egui::{self, Ui, Vec2};
use lasergrbl_core::laser_life::{self, DEF_BRAND, DEF_MODEL, LaserLifeCounter};

use super::common::{ModalEvent, bitmap_button, list_view, modal, trf};
use crate::forms::form_util::load_texture;
use super::laser_life_edit::{LaserLifeEdit, LaserLifeEditOutcome};
use crate::ui::dialog::{DialogResult, MessageBox, MessageBoxButtons, MessageBoxIcon};
use crate::ui::tr;
use crate::ui::widgets::UsageClassPreview;

/// Result of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaserUsageOutcome {
    /// Closed.
    Closed,
}

/// What a pending message box asks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ask {
    Delete,
    Mark,
    Unmark,
    Error,
}

/// `string.Format("{0:0.0} h")` of a time span in hours.
///
/// `ts`: time span. Returns e.g. `"1.5 h"`.
pub fn hours_text(ts: lg_dotnet::TimeSpan) -> String {
    format!("{} h", lg_dotnet::fmt_double_custom(ts.total_hours(), "0.0"))
}

/// `ToShortDateString` for the invariant culture (`M/d/yyyy`); empty for no value.
///
/// `d`: optional date. Returns the text.
pub fn short_date(d: Option<NaiveDateTime>) -> String {
    d.map(|d| d.format("%-m/%-d/%Y").to_string()).unwrap_or_default()
}

/// Row texts of `LaserLifeCounterLVI`.
///
/// `llc`: counter. Returns the ten column texts in designer order.
pub fn row_texts(llc: &LaserLifeCounter) -> [String; 10] {
    let brand = llc.brand.as_deref().filter(|b| !b.trim().is_empty()).unwrap_or(DEF_BRAND).to_owned();
    let model = llc.model.as_deref().filter(|b| !b.trim().is_empty()).unwrap_or(DEF_MODEL).to_owned();
    [
        llc.name.clone().unwrap_or_default(),
        brand,
        model,
        hours_text(llc.time_in_run()),
        hours_text(llc.time_usage_normalized_power()),
        hours_text(llc.stress_time()),
        format!("{} %", lg_dotnet::fmt_double_custom(lg_dotnet::round_double(llc.average_power_factor() * 100.0, 0), "0")),
        short_date(llc.purchase_date),
        short_date(llc.monitoring_date),
        short_date(llc.death_date),
    ]
}

/// Button states of `RefreshButtons`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonStates {
    pub add: bool,
    pub remove: bool,
    pub edit: bool,
    pub mark_visible: bool,
    pub mark: bool,
    pub unmark: bool,
}

/// `RefreshButtons`.
///
/// `selected`: selected counter. Returns the states (Unmark is visible when Mark is not).
pub fn button_states(selected: Option<&LaserLifeCounter>) -> ButtonStates {
    let mark_visible = selected.is_none_or(|s| s.death_date.is_none());
    ButtonStates {
        add: true,
        remove: selected.is_some(),
        edit: selected.is_some(),
        mark_visible,
        mark: mark_visible && selected.is_some_and(|s| s.death_date.is_none()),
        unmark: !mark_visible && selected.is_some_and(|s| s.death_date.is_some()),
    }
}

/// The dialog.
pub struct LaserUsage {
    list: Vec<LaserLifeCounter>,
    selected: Option<usize>,
    edit: Option<(bool, LaserLifeEdit)>,
    ask: Option<(Ask, MessageBox)>,
    id: u64,
    /// Button bitmaps: unmark, mark, edit, remove, add, close.
    tex: Option<[Option<egui::TextureHandle>; 6]>,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl LaserUsage {
    /// Create the dialog (`LaserUsage_Load`): list loaded, last used laser preselected.
    pub fn open() -> Self {
        let mut f = LaserUsage { list: Vec::new(), selected: None, edit: None, ask: None, tex: None, id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed) };
        f.refresh_list(true);
        f
    }

    /// `RefreshList`.
    ///
    /// `select`: select the "Last laser used" row.
    fn refresh_list(&mut self, select: bool) {
        self.list = laser_life::get_list_clone().counters;
        if select {
            let last: Option<String> = lg_base::settings::get("Last laser used", None);
            self.selected = last.filter(|g| !g.is_empty()).and_then(|g| self.list.iter().position(|l| l.guid() == g));
        } else {
            self.selected = None;
        }
    }

    /// Button bitmaps from the designer resources, decoded once.
    ///
    /// `ctx`: egui context. Returns clones of the texture handles.
    fn icons(&mut self, ctx: &egui::Context) -> [Option<egui::TextureHandle>; 6] {
        self.tex
            .get_or_insert_with(|| {
                [
                    load_texture(ctx, "lu-unmark", include_bytes!("../../../../resources/grafica/laserusage_unmark.png")),
                    load_texture(ctx, "lu-mark", include_bytes!("../../../../resources/grafica/laserusage_mark.png")),
                    load_texture(ctx, "lu-edit", include_bytes!("../../../../resources/grafica/laserusage_edit.png")),
                    load_texture(ctx, "lu-remove", include_bytes!("../../../../resources/grafica/laserusage_remove.png")),
                    load_texture(ctx, "lu-add", include_bytes!("../../../../resources/grafica/laserusage_addnew.png")),
                    load_texture(ctx, "lu-close", include_bytes!("../../../../resources/grafica/laserusage_close.png")),
                ]
            })
            .clone()
    }

    /// `GetSelected`.
    fn selected_counter(&self) -> Option<LaserLifeCounter> {
        self.selected.and_then(|i| self.list.get(i)).cloned()
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns `Some(Closed)` on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<LaserUsageOutcome> {
        // sub dialogs first
        if let Some((is_new, f)) = &mut self.edit {
            if let Some(o) = f.show(ctx) {
                let is_new = *is_new;
                if let LaserLifeEditOutcome::Saved(llc) = o {
                    if is_new {
                        laser_life::add(llc);
                    } else {
                        laser_life::edit(&llc);
                    }
                    self.refresh_list(false);
                }
                self.edit = None;
            }
            return None;
        }
        if let Some((kind, mb)) = &mut self.ask {
            if let Some(r) = mb.show(ctx) {
                let kind = *kind;
                self.ask = None;
                if r == DialogResult::Yes {
                    self.confirmed(kind);
                }
            }
            return None;
        }
        let title = trf("LaserUsage.$this.Text", "Laser Lifespan - Monitoring Tool");
        let id = ("laserusage", self.id);
        match modal(ctx, id, &title, 1000.0, true, |ui| self.body(ui)) {
            Some(ModalEvent::Done(())) | Some(ModalEvent::Closed) => Some(LaserUsageOutcome::Closed),
            None => None,
        }
    }

    /// Apply a confirmed Yes/No question.
    ///
    /// `kind`: which question was answered with Yes.
    fn confirmed(&mut self, kind: Ask) {
        let Some(sel) = self.selected_counter() else { return };
        match kind {
            Ask::Delete => {
                if let Some(err) = laser_life::delete(&sel) {
                    self.ask = Some((Ask::Error, MessageBox::new(err, "Error").icon(MessageBoxIcon::Error)));
                }
            }
            Ask::Mark => laser_life::death(&sel),
            Ask::Unmark => laser_life::undeath(&sel),
            Ask::Error => {}
        }
        self.refresh_list(false);
    }

    /// Form body. Returns Some(()) on Close.
    fn body(&mut self, ui: &mut Ui) -> Option<()> {
        let t = |c: &str| tr(&format!("LaserUsage.{c}.Text")).to_owned();
        ui.add(egui::Label::new(t("label1")).wrap());
        ui.add_space(4.0);
        let st = button_states(self.selected_counter().as_ref());
        let heads: Vec<String> = ["ChName", "ChBrand", "ChModel", "ChRunTime", "ChPowerTime", "ChStressTime", "ChAvgPow", "ChDateBuy", "ChDateStartM", "ChDateEndL"].iter().map(|c| t(c)).collect();
        // column widths from the resx
        let widths = [80.0, 80.0, 80.0, 80.0, 80.0, 80.0, 80.0, 90.0, 90.0, 90.0];
        let rows: Vec<Vec<String>> = self.list.iter().map(|l| row_texts(l).to_vec()).collect();
        let mut out = None;
        let mut want: Option<&str> = None;
        let mut dbl = false;
        let tex = self.icons(ui.ctx());
        ui.horizontal_top(|ui| {
            // ListView 880x335
            // top-down layout explicitly: allocate_ui would inherit the horizontal one
            ui.allocate_ui_with_layout(Vec2::new(880.0, 335.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(880.0);
                let mut sel = self.selected;
                let r = list_view(ui, "laser-grid", &heads, &widths, &rows, &mut sel, 335.0);
                if r.double_clicked.is_some() {
                    dbl = true;
                }
                self.selected = sel;
            });
            ui.vertical(|ui| {
                // 114x32 buttons, 22px bitmap at the left (tableLayoutPanel4 rows)
                let sz = Vec2::new(114.0, 32.0);
                if bitmap_button(ui, st.edit, tex[2].as_ref(), &t("BtnEdit"), true, sz) {
                    want = Some("edit");
                }
                if bitmap_button(ui, st.add, tex[4].as_ref(), &t("BtnAddNew"), true, sz) {
                    want = Some("add");
                }
                if bitmap_button(ui, st.remove, tex[3].as_ref(), &t("BtnRemove"), true, sz) {
                    want = Some("remove");
                }
                if st.mark_visible {
                    if bitmap_button(ui, st.mark, tex[1].as_ref(), &t("BtnMark"), true, sz) {
                        want = Some("mark");
                    }
                } else if bitmap_button(ui, st.unmark, tex[0].as_ref(), &t("BtnUnmark"), true, sz) {
                    want = Some("unmark");
                }
                ui.add_space(8.0);
                let hours: Vec<f64> = self.selected_counter().map(|l| l.classes().iter().map(|c| c.total_hours()).collect()).unwrap_or_default();
                ui.add(UsageClassPreview::new(&hours).size(Vec2::new(114.0, 137.0)));
            });
        });
        if dbl && st.edit {
            want = Some("edit");
        }
        match want {
            Some("add") => self.edit = Some((true, LaserLifeEdit::open(&LaserLifeCounter::create_new()))),
            Some("edit") => {
                if let Some(s) = self.selected_counter() {
                    self.edit = Some((false, LaserLifeEdit::open(&s)));
                }
            }
            Some("remove") => {
                if let Some(s) = self.selected_counter() {
                    let text = if s.has_worked() {
                        "Are you sure you want to delete this laser and his history?\r\nIf your lasers has stopped working please use \"Mark as death\" button instead of delete."
                    } else {
                        "Are you sure you want to delete this laser?"
                    };
                    self.ask = Some((Ask::Delete, MessageBox::new(text, "Delete data?").buttons(MessageBoxButtons::YesNo).icon(MessageBoxIcon::Question)));
                }
            }
            Some("mark") => self.ask = Some((Ask::Mark, MessageBox::new("Are you sure your laser is death?", "Mark as death?").buttons(MessageBoxButtons::YesNo).icon(MessageBoxIcon::Question))),
            Some("unmark") => self.ask = Some((Ask::Unmark, MessageBox::new("Restore this laser?", "Confirm").buttons(MessageBoxButtons::YesNo).icon(MessageBoxIcon::Question))),
            _ => {}
        }
        ui.add_space(4.0);
        ui.add(egui::Label::new(t("label2")).wrap());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if bitmap_button(ui, true, tex[5].as_ref(), &t("BtnClose"), false, Vec2::new(95.0, 42.0)) {
                out = Some(());
            }
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lg_dotnet::TimeSpan;

    #[test]
    fn texts() {
        assert_eq!(hours_text(TimeSpan::from_minutes(90.0)), "1.5 h");
        assert_eq!(hours_text(TimeSpan::ZERO), "0.0 h");
        let d = NaiveDate::from_ymd_opt(2021, 3, 7).unwrap().and_hms_opt(0, 0, 0);
        assert_eq!(short_date(d), "3/7/2021");
        assert_eq!(short_date(None), "");
    }

    #[test]
    fn row_defaults() {
        let mut c = LaserLifeCounter::create_new();
        c.name = Some("L".into());
        c.brand = Some("  ".into());
        let r = row_texts(&c);
        assert_eq!(r[0], "L");
        assert_eq!(&r[1], DEF_BRAND);
        assert_eq!(&r[2], DEF_MODEL);
        assert_eq!(r[3], "0.0 h");
        assert_eq!(r[6], "0 %");
        assert_eq!(r[9], "");
        assert!(!r[8].is_empty());
    }

    #[test]
    fn buttons() {
        let none = button_states(None);
        assert!(none.add && !none.remove && !none.edit && none.mark_visible && !none.mark && !none.unmark);
        let mut c = LaserLifeCounter::create_new();
        let alive = button_states(Some(&c));
        assert!(alive.remove && alive.edit && alive.mark_visible && alive.mark && !alive.unmark);
        c.death_date = c.monitoring_date;
        let dead = button_states(Some(&c));
        assert!(!dead.mark_visible && !dead.mark && dead.unmark);
    }
}
