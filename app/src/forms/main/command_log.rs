//! Ports `UserControls/CommandLog.cs`: virtualised list of sent commands and board messages.

use std::sync::OnceLock;

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, ScrollArea, Sense, TextureHandle, Ui, Vec2};
use lasergrbl_core::GrblCore;
use lg_gcode::{GrblRow, RowColor};

use crate::ui::theme::{named, LogRowColor, Theme};

/// Icons of the C# `ImageList IL`, by `ImageIndex`.
const ICON_QUEUED: &[u8] = include_bytes!("../../../../resources/icons/logqueued.png");
const ICON_OK: &[u8] = include_bytes!("../../../../resources/icons/logok.png");
const ICON_KO: &[u8] = include_bytes!("../../../../resources/icons/logko.png");
const ICON_INFO: &[u8] = include_bytes!("../../../../resources/icons/loginfo.png");
const ICON_WARN: &[u8] = include_bytes!("../../../../resources/icons/logwarning.png");
const ICON_DIAG: &[u8] = include_bytes!("../../../../resources/icons/diagnostic.png");

/// Tooltip wrap width, `WordWrap.WrapString(.., 60)`.
const TOOLTIP_WIDTH: i32 = 60;

/// Decode a PNG into an egui texture.
///
/// `ctx`: egui context. `name`: texture debug name. `bytes`: PNG data. Returns the handle or None.
fn load_png(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(name, ci, egui::TextureOptions::LINEAR))
}

/// Map the core row colour to the theme colour (`IGrblRow.LeftColor/RightColor`).
///
/// `c`: row colour from `lg_gcode`. Returns the colour of the current scheme.
pub fn row_color(th: &Theme, c: RowColor) -> Color32 {
    let m = match c {
        RowColor::LogLeftCommand => LogRowColor::Command,
        RowColor::LogRightGood => LogRowColor::RightGood,
        RowColor::LogRightBad => LogRowColor::RightBad,
        RowColor::LogRightOthers => LogRowColor::RightOthers,
        RowColor::LogLeftStartup => LogRowColor::Startup,
        RowColor::LogLeftAlarm => LogRowColor::Alarm,
        RowColor::LogLeftConfig => LogRowColor::Config,
        RowColor::LogLeftFeedback => LogRowColor::Feedback,
        RowColor::LogLeftPosition => LogRowColor::Position,
        RowColor::LogLeftOthers => LogRowColor::Others,
        RowColor::Black => return named::BLACK,
    };
    th.log_color(m)
}

/// The command log control.
#[derive(Default)]
pub struct CommandLog {
    /// Icon textures, index = `ImageIndex` (loaded lazily).
    icons: OnceLock<Vec<Option<TextureHandle>>>,
}

impl CommandLog {
    /// Empty log control.
    pub fn new() -> Self {
        Self::default()
    }

    /// Texture of an `ImageIndex`.
    ///
    /// `ctx`: egui context. `idx`: image index 0..=5. Returns the texture if loaded.
    fn icon(&self, ctx: &egui::Context, idx: i32) -> Option<&TextureHandle> {
        let v = self.icons.get_or_init(|| {
            vec![
                load_png(ctx, "log-queued", ICON_QUEUED),
                load_png(ctx, "log-ok", ICON_OK),
                load_png(ctx, "log-ko", ICON_KO),
                load_png(ctx, "log-info", ICON_INFO),
                load_png(ctx, "log-warning", ICON_WARN),
                load_png(ctx, "log-diagnostic", ICON_DIAG),
            ]
        });
        usize::try_from(idx).ok().and_then(|i| v.get(i)).and_then(|t| t.as_ref())
    }

    /// Draw the log filling the remaining space of `ui`.
    ///
    /// `ui`: parent. `core`: the engine (rows, `SupportCSV`). Autoscroll sticks to the end while the view
    /// is at the end (C# `ScrollBar.Value >= Maximum - 10`).
    pub fn show(&self, ui: &mut Ui, core: &GrblCore) {
        let th = Theme::current();
        // CmdLog.Font = Consolas 8.25pt (11 px); RowHeight = Font.Height + 2 (Consolas 8.25pt: 15 px)
        let font = FontId::monospace(11.0);
        let row_h = 17.0;
        let total = core.executed();
        let support_csv = core.support_csv();
        // control background + border (CmdLog_Paint / OnPaintBackground)
        let outer = ui.available_rect_before_wrap();
        ui.painter().rect_filled(outer, 0.0, th.log_back_color);
        let inner = ui.scope_builder(egui::UiBuilder::new().max_rect(outer.shrink(1.0)), |ui| {
            ScrollArea::vertical().auto_shrink([false, false]).stick_to_bottom(true).show_rows(ui, row_h, total, |ui, range| {
                // rows of the visible window only
                let first = range.start as i64;
                let rows = core.sent_command(first, range.len() as i64);
                let width = ui.available_width();
                for (i, row) in rows.iter().enumerate() {
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, row_h), Sense::hover());
                    self.paint_row(ui, rect, row, support_csv, &th, &font);
                    // tooltip with wrapped text (SetPosition)
                    if resp.hovered() {
                        if let Some(tip) = row.get_tool_tip(support_csv) {
                            resp.on_hover_text(lg_base::tools::word_wrap::wrap_string(&tip, TOOLTIP_WIDTH));
                        }
                    }
                    let _ = i;
                }
            });
        });
        let _ = inner;
        ui.painter().rect_stroke(outer, 0.0, egui::Stroke::new(1.0, th.controls_border), egui::StrokeKind::Inside);
    }

    /// Paint one row: icon, message (left, ellipsis), result (right).
    ///
    /// `ui`: target. `rect`: row rectangle. `row`: the row. `csv`: decode flag. `th`: theme. `font`: log font.
    fn paint_row(&self, ui: &Ui, rect: Rect, row: &lasergrbl_core::SentRow, csv: bool, th: &Theme, font: &FontId) {
        let p = ui.painter_at(rect);
        let h = rect.height();
        let image_index = row.image_index();
        // result text on the right (erroronly=true as C# passes mUseImages)
        let mut result_w = 0.0;
        if let Some(result) = row.get_result(csv, true) {
            let g = p.layout_no_wrap(result, font.clone(), row_color(th, row.right_color()));
            result_w = g.size().x;
            let pos = Pos2::new(rect.right() - 2.0 - g.size().x, rect.top() + 2.0);
            p.galley(pos, g, Color32::WHITE);
        }
        // message on the left, clipped to the free width
        let msg = row.get_decoded_message();
        if !msg.is_empty() {
            let left = rect.left() + if image_index >= 0 { h + 1.0 } else { 1.0 };
            let max_w = (rect.right() - left - result_w - 4.0).max(10.0);
            let mut job = egui::text::LayoutJob::simple_singleline(msg, font.clone(), row_color(th, row.left_color()));
            job.wrap = egui::text::TextWrapping::truncate_at_width(max_w);
            let g = p.layout_job(job);
            p.galley(Pos2::new(left, rect.top() + 1.0), g, Color32::WHITE);
        }
        // status icon
        if image_index >= 0 {
            if let Some(tex) = self.icon(ui.ctx(), image_index) {
                let side = h - 2.0;
                let r = Rect::from_min_size(Pos2::new(rect.left() + 1.0, rect.top() + (h - side) / 2.0), Vec2::splat(side));
                p.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            }
        }
        let _ = Align2::LEFT_TOP;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_map() {
        let th = Theme::of(crate::ui::theme::Scheme::CADDark);
        assert_eq!(row_color(&th, RowColor::LogLeftCommand), th.log_left_command);
        assert_eq!(row_color(&th, RowColor::LogRightBad), th.log_right_bad);
        assert_eq!(row_color(&th, RowColor::Black), named::BLACK);
    }

    #[test]
    fn icons_decode() {
        for b in [ICON_QUEUED, ICON_OK, ICON_KO, ICON_INFO, ICON_WARN, ICON_DIAG] {
            assert!(image::load_from_memory(b).is_ok());
        }
    }
}
