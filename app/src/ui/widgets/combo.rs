// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `FlatComboBox.cs` and `ComboBoxEnumItem.cs` (`EnumComboBox`).

use eframe::egui::{self, Id, Response, Stroke, Ui};

use crate::ui::theme::Theme;

/// Apply flat themed colors to the ui style for a combo (C# colors: LogBack bg, border ControlsBorder).
fn flat_style(ui: &mut Ui) {
    let th = Theme::current();
    let v = &mut ui.style_mut().visuals;
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.bg_fill = th.log_back_color;
        w.weak_bg_fill = th.log_back_color;
        w.bg_stroke = Stroke::new(1.0, th.controls_border);
        w.fg_stroke = Stroke::new(1.0, th.form_fore_color);
        w.corner_radius = 0.into();
    }
    v.extreme_bg_color = th.log_back_color;
}

/// `FlatComboBox` over string items.
///
/// `ui`: parent. `id`: unique id salt. `selected`: selected index (None = nothing, `SelectedIndex = -1`).
/// `items`: item texts. `width`: control width. Returns response; `changed()` when selection changed.
pub fn flat_combo(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, selected: &mut Option<usize>, items: &[String], width: f32) -> Response {
    let mut changed = false;
    let text = selected.and_then(|i| items.get(i)).cloned().unwrap_or_default();
    let mut inner = ui
        .scope(|ui| {
            flat_style(ui);
            let cb = egui::ComboBox::from_id_salt(Id::unique(id)).width(width).selected_text(text).show_ui(ui, |ui| {
                for (i, it) in items.iter().enumerate() {
                    if ui.selectable_label(*selected == Some(i), it).clicked() && *selected != Some(i) {
                        *selected = Some(i);
                        changed = true;
                    }
                }
            });
            cb.response
        })
        .inner;
    if changed {
        inner.mark_changed();
    }
    inner
}

/// `EnumComboBox`: combo over a list of enum values with a label function (C# `GrblCore.TranslateEnum`).
///
/// `ui`: parent. `id`: id salt. `value`: selected value. `items`: all values (`AddItem` order).
/// `label`: value -> display text. `trim_parentheses`: cut text at first '(' (C# `trimparentesi`, only if idx>0).
/// `width`: control width. Returns response; `changed()` on selection.
pub fn enum_combo<T: PartialEq + Clone>(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    value: &mut T,
    items: &[T],
    label: impl Fn(&T) -> String,
    trim_parentheses: bool,
    width: f32,
) -> Response {
    let texts: Vec<String> = items.iter().map(|i| enum_item_text(&label(i), trim_parentheses)).collect();
    let mut sel = items.iter().position(|i| i == value);
    let r = flat_combo(ui, id, &mut sel, &texts, width);
    if r.changed() {
        if let Some(i) = sel {
            *value = items[i].clone();
        }
    }
    r
}

/// `ComboBoxEnumItem.ToString()` trimming.
///
/// `s`: translated text. `trim`: cut at '('. Returns text (not trimmed when '(' is first char).
pub fn enum_item_text(s: &str, trim: bool) -> String {
    if trim {
        if let Some(idx) = s.find('(') {
            if idx > 0 {
                return s[..idx].to_owned();
            }
        }
    }
    s.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim() {
        assert_eq!(enum_item_text("Line (fast)", true), "Line ");
        assert_eq!(enum_item_text("(x)", true), "(x)");
        assert_eq!(enum_item_text("Line (fast)", false), "Line (fast)");
    }
}
