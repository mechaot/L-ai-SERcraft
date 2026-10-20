//! Shared helpers of the WP-U4b dialogs. No C# counterpart: replaces WinForms boilerplate
//! (`ShowDialog`, `TableLayoutPanel` rows, caption lookups for forms without `.resx` entries).

use eframe::egui::{self, Align, Key, Layout, Modifiers, Ui, Vec2};
use std::fmt::Debug;
use std::hash::Hash;

use crate::ui::widgets::{GrblButton, ImageButton};
use crate::ui::{tr, tr_plain};

/// Result of a modal frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalEvent<T> {
    /// Content closure produced a result.
    Done(T),
    /// Esc or the close box (WinForms `DialogResult.Cancel`).
    Closed,
}

/// Caption with fallback: forms without `.resx` entries keep their English designer text.
///
/// `key`: i18n key. `fallback`: designer text used when the catalog has no entry. Returns the text.
pub fn trf(key: &str, fallback: &str) -> String {
    let t = tr(key);
    if t == key { fallback.to_owned() } else { t.to_owned() }
}

/// Like [`trf`] but strips WinForms mnemonics (`&`).
///
/// `key`: i18n key. `fallback`: designer text. Returns plain text.
pub fn trf_plain(key: &str, fallback: &str) -> String {
    let t = tr(key);
    if t == key { crate::ui::i18n::strip_mnemonic(fallback) } else { tr_plain(key) }
}

/// Show a fixed-width modal form with a title row.
///
/// `ctx`: egui context. `id`: unique form id. `title`: caption. `width`: client width in points.
/// `closable`: draw a close box. `content`: body, returns `Some(result)` to close.
/// Returns `Some(event)` on the closing frame. Esc closes when no popup (combo) is open.
pub fn modal<T>(
    ctx: &egui::Context,
    id: impl Hash + Debug,
    title: &str,
    width: f32,
    closable: bool,
    content: impl FnOnce(&mut Ui) -> Option<T>,
) -> Option<ModalEvent<T>> {
    let m = egui::Modal::new(egui::Id::unique(id));
    let r = m.show(ctx, |ui| {
        ui.set_width(width);
        let mut closed = false;
        // title row with optional close box
        ui.horizontal(|ui| {
            ui.label(crate::ui::fonts::bold(title));
            if closable {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(ImageButton::new("mdi-close").size(Vec2::splat(16.0))).clicked() {
                        closed = true;
                    }
                });
            }
        });
        ui.separator();
        if closed {
            return (None, true);
        }
        (content(ui), false)
    });
    let (inner, closed) = r.inner;
    if let Some(v) = inner {
        return Some(ModalEvent::Done(v));
    }
    let esc = r.is_top_modal && !r.any_popup_open && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
    if closed || esc { Some(ModalEvent::Closed) } else { None }
}

/// Themed button (`GrblButton`) with optional icon.
///
/// `ui`: parent. `text`: caption. `icon`: icon key or None. `size`: minimum size. Returns true if clicked.
pub fn button(ui: &mut Ui, text: &str, icon: Option<&str>, size: Vec2) -> bool {
    let mut b = GrblButton::new(text).min_size(size);
    if let Some(i) = icon {
        b = b.icon(i);
    }
    ui.add(b).clicked()
}

/// Same as [`button`] with an enabled flag.
///
/// `ui`: parent. `enabled`: false greys the button out and ignores clicks. Others: see [`button`].
pub fn button_en(ui: &mut Ui, enabled: bool, text: &str, icon: Option<&str>, size: Vec2) -> bool {
    ui.add_enabled_ui(enabled, |ui| button(ui, text, icon, size)).inner
}

/// 16x16 information button (`mdi-information-slab-box`) with a hover tooltip.
///
/// `ui`: parent. `tip`: tooltip, empty = none. Returns true when clicked.
pub fn info_button(ui: &mut Ui, tip: &str) -> bool {
    let r = ui.add(ImageButton::new("mdi-information-slab-box").size(Vec2::splat(18.0)));
    if tip.is_empty() { r.clicked() } else { r.on_hover_text(tip).clicked() }
}

/// Themed button with a coloured bitmap at the left (designer `Image` + `ImageAlign=MiddleLeft`).
///
/// `ui`: parent. `enabled`: false greys out and ignores clicks. `tex`: bitmap (None = no image).
/// `text`: caption (left aligned next to the image when `left_text`, else centered). `size`: button size.
/// Returns true when clicked.
pub fn bitmap_button(ui: &mut Ui, enabled: bool, tex: Option<&egui::TextureHandle>, text: &str, left_text: bool, size: Vec2) -> bool {
    use crate::ui::theme::{Theme, change_color_brightness};
    use eframe::egui::{Align2, FontId, Rect, Sense, Stroke, StrokeKind};
    let th = Theme::current();
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let mut back = th.log_back_color;
        if enabled && (resp.hovered() || resp.is_pointer_button_down_on()) {
            back = change_color_brightness(back, -0.15);
        }
        let p = ui.painter();
        p.rect_filled(rect, 0.0, th.form_back_color);
        let br = Rect::from_min_max(rect.min, rect.max - Vec2::splat(1.0));
        p.rect_filled(br, 5.0, back);
        p.rect_stroke(br, 5.0, Stroke::new(1.0, th.controls_border), StrokeKind::Inside);
        let mut left = rect.left() + 5.0;
        if let Some(t) = tex {
            let sz = t.size_vec2();
            let r = Rect::from_min_size(egui::pos2(left, rect.center().y - sz.y / 2.0), sz);
            let tint = if enabled { egui::Color32::WHITE } else { egui::Color32::from_white_alpha(90) };
            p.image(t.id(), r, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), tint);
            left += sz.x + 4.0;
        }
        let col = if enabled { th.form_fore_color } else { th.disabled_buttons };
        let font = FontId::proportional(13.0);
        if left_text {
            p.text(egui::pos2(left, rect.center().y), Align2::LEFT_CENTER, text, font, col);
        } else {
            p.text(egui::pos2(left + (rect.right() - left) / 2.0, rect.center().y), Align2::CENTER_CENTER, text, font, col);
        }
    }
    enabled && resp.clicked()
}

/// Settings-style row of a `TableLayoutPanel`: `[info] [control] [description]`.
///
/// `ui`: parent. `info`: Some(tooltip) draws the info button, `Some("")` draws it without tooltip,
/// None leaves the column blank. `ctl_w`: control column width. `control`: draws the control.
/// `desc`: wrapped description text. Returns true when the info button was clicked.
pub fn setting_row(ui: &mut Ui, info: Option<&str>, ctl_w: f32, control: impl FnOnce(&mut Ui), desc: &str) -> bool {
    let mut clicked = false;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.allocate_ui(Vec2::new(20.0, 20.0), |ui| {
            ui.set_min_width(20.0);
            if let Some(t) = info {
                clicked = info_button(ui, t);
            }
        });
        ui.allocate_ui(Vec2::new(ctl_w, 20.0), |ui| {
            ui.set_min_width(ctl_w);
            control(ui);
        });
        let w = ui.available_width().max(60.0);
        ui.allocate_ui(Vec2::new(w, 0.0), |ui| {
            ui.set_width(w);
            ui.add(egui::Label::new(desc).wrap());
        });
    });
    ui.add_space(4.0);
    clicked
}

/// `Tools.Utils.OpenLink(url)`.
///
/// `url`: http(s) link.
pub fn open_link(url: &str) {
    lg_base::tools::utils::open_link(url);
}

/// Shared settings test guard (see `form_util::test_guard`).
#[cfg(test)]
pub use crate::forms::form_util::test_guard;

/// Result of [`list_view`].
pub struct ListViewResult {
    /// Row clicked this frame.
    pub clicked: Option<usize>,
    /// Row double clicked this frame.
    pub double_clicked: Option<usize>,
}

/// `ListView` in `Details` view with `FullRowSelect` and `GridLines`.
///
/// `ui`: parent. `id`: id salt. `headers`: column captions. `widths`: fixed column widths (px).
/// `rows`: cell texts per row. `selected`: selected row index. `height`: body height.
/// Returns the clicked / double clicked row.
pub fn list_view(ui: &mut Ui, id: impl Hash + Debug, headers: &[String], widths: &[f32], rows: &[Vec<String>], selected: &mut Option<usize>, height: f32) -> ListViewResult {
    use egui_extras::{Column, TableBuilder};
    let mut res = ListViewResult { clicked: None, double_clicked: None };
    let mut tb = TableBuilder::new(ui).id_salt(id).striped(false).resizable(false).sense(egui::Sense::click()).max_scroll_height(height).min_scrolled_height(height);
    for w in widths {
        tb = tb.column(Column::exact(*w));
    }
    tb.header(20.0, |mut h| {
        for t in headers {
            h.col(|ui| {
                ui.label(t);
            });
        }
    })
    .body(|body| {
        body.rows(17.0, rows.len(), |mut row| {
            let i = row.index();
            row.set_selected(*selected == Some(i));
            for c in 0..widths.len() {
                row.col(|ui| {
                    ui.add(egui::Label::new(rows[i].get(c).map(String::as_str).unwrap_or("")).truncate());
                });
            }
            let r = row.response();
            if r.clicked() {
                *selected = Some(i);
                res.clicked = Some(i);
            }
            if r.double_clicked() {
                res.double_clicked = Some(i);
            }
        });
    });
    res
}
