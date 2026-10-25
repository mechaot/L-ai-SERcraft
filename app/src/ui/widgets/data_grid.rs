// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Replacement for `MyDatagridView.cs` (a `DataGridView` with a WinForms crash workaround that has no
//! egui equivalent) -- here a small themed table helper on top of `egui_extras::TableBuilder`.

use eframe::egui::{Ui, Vec2};
use egui_extras::{Column, TableBuilder};

use crate::ui::theme::Theme;

/// Column definition.
pub struct GridColumn {
    pub header: String,
    /// Initial width; None = remainder.
    pub width: Option<f32>,
}

/// Draw a read-only/selectable grid.
///
/// `ui`: parent. `id`: id salt. `cols`: columns. `rows`: row count. `selected`: selected row.
/// `cell`: draws cell `(row, col)`. `height`: max body height. Returns clicked row, if any.
pub fn data_grid(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    cols: &[GridColumn],
    rows: usize,
    selected: &mut Option<usize>,
    mut cell: impl FnMut(&mut Ui, usize, usize),
    height: f32,
) -> Option<usize> {
    let th = Theme::current();
    let mut clicked = None;
    let mut tb = TableBuilder::new(ui).id_salt(id).striped(false).sense(eframe::egui::Sense::click()).max_scroll_height(height);
    for c in cols {
        tb = tb.column(match c.width {
            Some(w) => Column::initial(w).resizable(true),
            None => Column::remainder().resizable(true),
        });
    }
    tb.header(20.0, |mut h| {
        for c in cols {
            h.col(|ui| {
                ui.strong(&c.header);
            });
        }
    })
    .body(|body| {
        body.rows(20.0, rows, |mut row| {
            let i = row.index();
            row.set_selected(*selected == Some(i));
            for ci in 0..cols.len() {
                row.col(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                    cell(ui, i, ci);
                });
            }
            if row.response().clicked() {
                *selected = Some(i);
                clicked = Some(i);
            }
        });
    });
    let _ = th;
    clicked
}
