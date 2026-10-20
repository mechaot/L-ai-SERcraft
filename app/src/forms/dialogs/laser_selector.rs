//! Ports `LaserSelector.cs` + `LaserSelector.Designer.cs`: "Select Laser" dialog shown on connect when
//! more than one laser module is alive (`laser_life::on_connect` -> `NeedSelection`).

use eframe::egui::{self, Ui, Vec2};
use lasergrbl_core::laser_life::{self, LaserLifeCounter};

use super::common::{ModalEvent, button_en, modal, trf};
use crate::ui::widgets::flat_combo;

/// Result of the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaserSelectorOutcome {
    /// Cancel (C# returned null).
    Cancelled,
    /// Select: guid of the chosen counter (pass to `laser_life::select`).
    Selected(Option<String>),
}

/// Combo order of the constructor: last used first, then alive newest monitoring first, then dead.
///
/// `orig`: all counters. `last_guid`: setting "Last laser used". Returns the ordered list.
pub fn order_lasers(orig: &[LaserLifeCounter], last_guid: Option<&str>) -> Vec<LaserLifeCounter> {
    let last = last_guid.and_then(|g| orig.iter().find(|o| o.guid() == g));
    let rest = |dead: bool| {
        let mut v: Vec<&LaserLifeCounter> = orig.iter().filter(|o| last.is_none_or(|l| l.guid() != o.guid()) && o.death_date.is_some() == dead).collect();
        // OrderByDescending(MonitoringDate), stable
        v.sort_by_key(|a| std::cmp::Reverse(a.monitoring_date));
        v
    };
    let mut out: Vec<LaserLifeCounter> = Vec::new();
    out.extend(last.cloned());
    out.extend(rest(false).into_iter().cloned());
    out.extend(rest(true).into_iter().cloned());
    out
}

/// The dialog.
pub struct LaserSelector {
    items: Vec<LaserLifeCounter>,
    sel: Option<usize>,
    id: u64,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl LaserSelector {
    /// Create the dialog from the laser life store (C# ctor).
    pub fn open() -> Self {
        let last: Option<String> = lg_base::settings::get("Last laser used", None);
        Self::with_counters(&laser_life::get_list_clone().counters, last.as_deref())
    }

    /// Create the dialog from explicit data.
    ///
    /// `counters`: all lasers. `last_guid`: last used laser.
    pub fn with_counters(counters: &[LaserLifeCounter], last_guid: Option<&str>) -> Self {
        let items = order_lasers(counters, last_guid);
        LaserSelector { sel: if items.is_empty() { None } else { Some(0) }, items, id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed) }
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<LaserSelectorOutcome> {
        let title = trf("LaserSelector.$this.Text", "Select Laser");
        let id = ("lasersel", self.id);
        match modal(ctx, id, &title, 360.0, true, |ui| self.body(ui)) {
            Some(ModalEvent::Done(true)) => Some(LaserSelectorOutcome::Selected(self.sel.and_then(|i| self.items.get(i)).map(|l| l.guid().to_owned()))),
            Some(ModalEvent::Done(false)) | Some(ModalEvent::Closed) => Some(LaserSelectorOutcome::Cancelled),
            None => None,
        }
    }

    /// Form body. Returns Some(true)=Select, Some(false)=Cancel.
    fn body(&mut self, ui: &mut Ui) -> Option<bool> {
        ui.label(trf("LaserSelector.label1.Text", "Current Laser"));
        let names: Vec<String> = self.items.iter().map(|l| l.name.clone().unwrap_or_default()).collect();
        flat_combo(ui, "laser-selector", &mut self.sel, &names, 320.0);
        ui.add_space(8.0);
        let mut out = None;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // designer: Select 99x42, Cancel 107x42, both with a 32px image
            if button_en(ui, true, &trf("LaserSelector.BtnSelect.Text", "Select"), Some("mdi-checkbox-marked"), Vec2::new(99.0, 42.0)) {
                out = Some(true);
            }
            if button_en(ui, true, &trf("LaserSelector.BtnCancel.Text", "Cancel"), Some("mdi-close-box"), Vec2::new(107.0, 42.0)) {
                out = Some(false);
            }
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn counter(name: &str, mon: (i32, u32, u32), dead: bool) -> LaserLifeCounter {
        let mut c = LaserLifeCounter::create_new();
        c.name = Some(name.into());
        c.monitoring_date = NaiveDate::from_ymd_opt(mon.0, mon.1, mon.2).map(|d| d.and_hms_opt(0, 0, 0).unwrap());
        if dead {
            c.death_date = c.monitoring_date;
        }
        c
    }

    #[test]
    fn ordering() {
        let a = counter("a", (2020, 1, 1), false);
        let b = counter("b", (2022, 1, 1), false);
        let c = counter("c", (2021, 1, 1), true);
        let d = counter("d", (2023, 1, 1), true);
        let e = counter("e", (2019, 1, 1), false);
        let all = vec![a.clone(), b.clone(), c.clone(), d.clone(), e.clone()];
        // last used first, alive by newest monitoring, then dead by newest monitoring
        let names = |v: Vec<LaserLifeCounter>| v.iter().map(|l| l.name.clone().unwrap()).collect::<Vec<_>>();
        assert_eq!(names(order_lasers(&all, Some(e.guid()))), ["e", "b", "a", "d", "c"]);
        assert_eq!(names(order_lasers(&all, None)), ["b", "a", "e", "d", "c"]);
        assert_eq!(names(order_lasers(&all, Some("unknown"))), ["b", "a", "e", "d", "c"]);
        let s = LaserSelector::with_counters(&all, Some(d.guid()));
        assert_eq!(s.sel, Some(0));
        assert_eq!(s.items[0].name.as_deref(), Some("d"));
    }
}
