//! Ports `UserControls/GrblGroupBox.cs`: group box with bold title in a themed border.

use eframe::egui::{self, Align2, FontId, InnerResponse, Margin, Pos2, Stroke, Ui};

use crate::ui::theme::Theme;

/// Draw a themed group box around `add_contents`.
///
/// `ui`: parent. `title`: bold caption. `add_contents`: body. Returns the body's value.
pub fn grbl_group_box<R>(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let th = Theme::current();
    let font = FontId::proportional(13.0);
    let galley = ui.painter().layout_no_wrap(title.to_owned(), font.clone(), th.form_fore_color);
    let font_h = galley.size().y;
    let top = ui.cursor().top();
    let frame = egui::Frame::new().inner_margin(Margin { left: 6, right: 6, top: (font_h + 2.0) as i8, bottom: 6 });
    let out = frame.show(ui, add_contents);
    let r = out.response.rect;
    let p = ui.painter();
    let pen = Stroke::new(1.0, th.controls_border);
    let ly = top + (font_h / 2.0).floor();
    let tx = r.left() + 8.0;
    // left, bottom, right, and top segments around the caption gap (C# OnPaint)
    p.line_segment([Pos2::new(r.left(), ly), Pos2::new(r.left(), r.bottom() - 1.0)], pen);
    p.line_segment([Pos2::new(r.left(), r.bottom() - 1.0), Pos2::new(r.right() - 1.0, r.bottom() - 1.0)], pen);
    p.line_segment([Pos2::new(r.left(), ly), Pos2::new(tx, ly)], pen);
    p.line_segment([Pos2::new(tx + galley.size().x + 3.0, ly), Pos2::new(r.right() - 1.0, ly)], pen);
    p.line_segment([Pos2::new(r.right() - 1.0, ly), Pos2::new(r.right() - 1.0, r.bottom() - 1.0)], pen);
    // bold title: real bold font if found, else draw twice offset 0.6 px (fake bold)
    if crate::ui::fonts::have_bold() {
        let bf = FontId::new(13.0, egui::FontFamily::Name(crate::ui::fonts::BOLD_FAMILY.into()));
        p.text(Pos2::new(tx, top), Align2::LEFT_TOP, title, bf, th.form_fore_color);
    } else {
        p.text(Pos2::new(tx, top), Align2::LEFT_TOP, title, font.clone(), th.form_fore_color);
        p.text(Pos2::new(tx + 0.6, top), Align2::LEFT_TOP, title, font, th.form_fore_color);
    }
    out
}

/// Same as [`grbl_group_box`] but frame spans full available width.
///
/// `ui`: parent. `title`: bold caption. `add_contents`: body. Returns the body's value.
pub fn grbl_group_box_wide<R>(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    grbl_group_box(ui, title, |ui| {
        // inner width already excludes margins; fill it
        ui.set_min_width(ui.available_width());
        add_contents(ui)
    })
}
