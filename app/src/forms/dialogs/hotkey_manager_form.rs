// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `HotkeyManagerForm.cs` + `HotkeyManagerForm.Designer.cs`: shortcut editor.
//!
//! The form edits a copy of the list (`HotKeysManager::get_edit_list`); the caller applies the saved
//! list with [`apply_saved`] (C# `Core.WriteHotkeys`).

use eframe::egui::{self, Color32, Event, Key, Modifiers, RichText, Ui, Vec2};
use lg_services::hotkeys::{HotKey, HotKeysManager, keys};

use super::common::{ModalEvent, button_en, modal, trf};
use crate::ui::tr;
use crate::ui::tr_plain;
use crate::ui::widgets::{GridColumn, data_grid, grbl_group_box};

/// Result of the dialog.
#[derive(Debug, Clone, PartialEq)]
pub enum HotkeyOutcome {
    /// Cancel / close box.
    Cancelled,
    /// Save pressed: the edited list, pass it to [`apply_saved`].
    Saved(Vec<HotKey>),
}

/// `Core.WriteHotkeys(list)` plus persisting to settings.
///
/// `mgr`: the application's hotkey manager. `list`: list from [`HotkeyOutcome::Saved`].
pub fn apply_saved(mgr: &mut HotKeysManager, list: Vec<HotKey>) {
    mgr.write_hotkeys(list);
    mgr.save_to_settings();
}

/// `ComputeErrors`: which rows share their (non-empty) combination with another row.
///
/// `list`: edited bindings. Returns one flag per row, true = duplicated.
pub fn duplicated_rows(list: &[HotKey]) -> Vec<bool> {
    list.iter()
        .enumerate()
        .map(|(i, a)| a.combination() != keys::NONE && list.iter().enumerate().any(|(j, b)| i != j && a.combination() == b.combination()))
        .collect()
}

/// `ProcessCmdKey` logic: what a key press does to the selected row.
///
/// `combo`: WinForms `Keys` value of the pressed key + modifiers. Returns `Some(new_shortcut)`
/// for a valid shortcut or plain Delete (clear), None when the key is ignored.
pub fn captured_shortcut(combo: i32) -> Option<i32> {
    if HotKey::valid_shortcut(combo) {
        Some(combo)
    } else if combo == keys::DELETE {
        Some(keys::NONE)
    } else {
        None
    }
}

/// Convert an egui key press into a WinForms `Keys` value.
///
/// `key`: egui key. `m`: modifier state. Returns the combined value, None for unmapped keys.
pub fn keys_from_egui(key: Key, m: Modifiers) -> Option<i32> {
    keys::combine(&format!("{key:?}"), m.ctrl || m.command, m.shift, m.alt)
}

/// The hotkey editor.
pub struct HotkeyManagerForm {
    list: Vec<HotKey>,
    selected: Option<usize>,
    id: u64,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl HotkeyManagerForm {
    /// Create the dialog (C# ctor).
    ///
    /// `mgr`: the application's hotkey manager; its list is copied for editing.
    pub fn open(mgr: &HotKeysManager) -> Self {
        HotkeyManagerForm { list: mgr.get_edit_list(), selected: Some(0), id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed) }
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<HotkeyOutcome> {
        // key capture happens before the widgets see the events
        self.capture_keys(ctx);
        let title = trf("HotkeyManagerForm.$this.Text", "Hotkey Manager");
        let id = ("hotkeys", self.id);
        match modal(ctx, id, &title, 560.0, true, |ui| self.body(ui)) {
            Some(ModalEvent::Done(true)) => Some(HotkeyOutcome::Saved(self.list.clone())),
            Some(ModalEvent::Done(false)) | Some(ModalEvent::Closed) => Some(HotkeyOutcome::Cancelled),
            None => None,
        }
    }

    /// `ProcessCmdKey` on the current row.
    fn capture_keys(&mut self, ctx: &egui::Context) {
        let evs: Vec<(Key, Modifiers)> = ctx.input(|i| {
            i.events.iter().filter_map(|e| if let Event::Key { key, pressed: true, repeat: false, modifiers, .. } = e { Some((*key, *modifiers)) } else { None }).collect()
        });
        for (key, m) in evs {
            // plain Up/Down move the grid cursor like in a DataGridView
            if !m.any() && (key == Key::ArrowDown || key == Key::ArrowUp) {
                let n = self.list.len();
                if n > 0 {
                    let cur = self.selected.unwrap_or(0);
                    self.selected = Some(if key == Key::ArrowDown { (cur + 1).min(n - 1) } else { cur.saturating_sub(1) });
                }
                continue;
            }
            let (Some(sel), Some(combo)) = (self.selected, keys_from_egui(key, m)) else { continue };
            if let (Some(v), Some(hk)) = (captured_shortcut(combo), self.list.get_mut(sel)) {
                hk.set_shortcut(v);
            }
        }
    }

    /// Form body. Returns Some(true)=save, Some(false)=cancel.
    fn body(&mut self, ui: &mut Ui) -> Option<bool> {
        let dup = duplicated_rows(&self.list);
        grbl_group_box(ui, tr("HotkeyManagerForm.GB.Text"), |ui| {
            let cols = [
                GridColumn { header: tr("HotkeyManagerForm.Action.HeaderText").to_owned(), width: Some(260.0) },
                GridColumn { header: tr("HotkeyManagerForm.Shortcut.HeaderText").to_owned(), width: None },
            ];
            let list = &self.list;
            let mut sel = self.selected;
            data_grid(
                ui,
                "hotkey-grid",
                &cols,
                list.len(),
                &mut sel,
                |ui, r, c| {
                    if c == 0 {
                        // row ErrorText "Duplicated value!"
                        if dup[r] {
                            ui.label(RichText::new("\u{26A0}").color(Color32::from_rgb(0xE0, 0x30, 0x30))).on_hover_text("Duplicated value!");
                        }
                        ui.label(list[r].action_string());
                    } else {
                        ui.label(list[r].combination_string());
                    }
                },
                420.0,
            );
            self.selected = sel;
        });
        ui.add_space(4.0);
        let has_err = dup.iter().any(|d| *d);
        let mut out = None;
        // bottom row: hint label at the left, Cancel + Save (107x42) at the right
        ui.horizontal(|ui| {
            let sz = Vec2::new(107.0, 42.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if button_en(ui, !has_err, &tr_plain("HotkeyManagerForm.BtnSave.Text"), Some("mdi-checkbox-marked"), sz) {
                    out = Some(true);
                }
                if button_en(ui, true, &tr_plain("HotkeyManagerForm.BtnCancel.Text"), Some("mdi-close-box"), sz) {
                    out = Some(false);
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(tr("HotkeyManagerForm.LblConnect.Text")).wrap());
                });
            });
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lg_services::hotkeys::Actions;

    #[test]
    fn capture_rules() {
        assert_eq!(captured_shortcut(keys::CONTROL | keys::S), Some(keys::CONTROL | keys::S));
        assert_eq!(captured_shortcut(keys::F5), Some(keys::F5));
        // modifiers only / non standard keys are ignored
        assert_eq!(captured_shortcut(keys::CONTROL), None);
        assert_eq!(captured_shortcut(keys::SPACE), None);
        // plain Delete clears, Ctrl+Delete does not
        assert_eq!(captured_shortcut(keys::DELETE), Some(keys::NONE));
        assert_eq!(captured_shortcut(keys::DELETE | keys::CONTROL), None);
    }

    #[test]
    fn egui_key_mapping() {
        let m = Modifiers { ctrl: true, ..Default::default() };
        assert_eq!(keys_from_egui(Key::S, m), Some(keys::CONTROL | keys::S));
        assert_eq!(keys_from_egui(Key::F5, Modifiers::NONE), Some(keys::F5));
        assert_eq!(keys_from_egui(Key::Num5, Modifiers::SHIFT), Some(keys::SHIFT | keys::D5));
        assert_eq!(keys_from_egui(Key::Delete, Modifiers::NONE), Some(keys::DELETE));
    }

    #[test]
    fn duplicates_block_save() {
        let a = HotKey::new(Actions::ALL[0], keys::CONTROL | keys::A);
        let b = HotKey::new(Actions::ALL[1], keys::CONTROL | keys::A);
        let c = HotKey::new(Actions::ALL[2], keys::NONE);
        let d = HotKey::new(Actions::ALL[3], keys::NONE);
        assert_eq!(duplicated_rows(&[a, b, c, d]), vec![true, true, false, false]);
        assert_eq!(duplicated_rows(&[a, c, d]), vec![false, false, false]);
    }

    #[test]
    fn defaults_have_no_duplicates_and_apply() {
        let mgr = HotKeysManager::with_defaults();
        let list = mgr.get_edit_list();
        assert!(!duplicated_rows(&list).iter().any(|d| *d));
        let mut other = HotKeysManager::new();
        let mut l2 = list.clone();
        l2[0].set_shortcut(keys::NONE);
        other.write_hotkeys(l2.clone());
        assert_eq!(other.get_edit_list(), l2);
    }
}
