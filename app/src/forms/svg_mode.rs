// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Ports `SvgConverter/SvgModeForm.cs` (+ Designer): choose "Load as Vector" / "Load as Image" for an SVG.
//!
//! UNUSED in the C# flow: `GrblCore.OpenFile` forces `Mode.Vector` (the `CreateAndShow` call is commented out).
//! Ported anyway; both previews use the same resvg render (C# `svg.Draw()` vs `svg.Draw(true)`, the latter
//! has no counterpart here).

use eframe::egui::{self, Sense, TextureHandle, Ui, Vec2};
use std::path::Path;

use crate::forms::form_util::*;

/// `SvgModeForm.Mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgMode {
    Vector,
    Raster,
    /// Closed without a choice (`Result` default).
    None,
}

/// Render an SVG file to an RGBA preview (C# `svg.Ppi = 600; svg.Draw()` scaled to fit).
///
/// `path`: SVG file. `max_px`: longest edge of the result. Returns the image or None on error.
pub fn render_svg_preview(path: &Path, max_px: u32) -> Option<egui::ColorImage> {
    let data = std::fs::read(path).ok()?;
    let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default()).ok()?;
    let sz = tree.size();
    let scale = max_px as f32 / sz.width().max(sz.height()).max(1.0);
    let (w, h) = (((sz.width() * scale).ceil() as u32).max(1), ((sz.height() * scale).ceil() as u32).max(1));
    let mut pm = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pm.as_mut());
    Some(egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], pm.data()))
}

/// SVG mode dialog state.
pub struct SvgModeForm {
    filename: String,
    vector: Option<TextureHandle>,
    image: Option<TextureHandle>,
    loaded: bool,
}

impl SvgModeForm {
    /// `SvgModeForm.CreateAndShow(parent, filename)` setup.
    ///
    /// `filename`: SVG file to preview.
    pub fn new(filename: &str) -> Self {
        SvgModeForm { filename: filename.to_owned(), vector: None, image: None, loaded: false }
    }

    /// C# `CreatePreview`: render both previews once.
    fn create_preview(&mut self, ctx: &egui::Context) {
        self.loaded = true;
        if let Some(img) = render_svg_preview(Path::new(&self.filename), 600) {
            self.vector = Some(ctx.load_texture("svgmode-vector", img.clone(), egui::TextureOptions::LINEAR));
            self.image = Some(ctx.load_texture("svgmode-image", img, egui::TextureOptions::LINEAR));
        }
    }

    /// Draw the modal form; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the chosen mode once closed (`SvgMode::None` when dismissed).
    pub fn show(&mut self, ctx: &egui::Context) -> Option<SvgMode> {
        if !self.loaded {
            self.create_preview(ctx);
        }
        match run_modal(ctx, "svg-mode", "SvgModeForm", 723.0, true, |ui| self.ui(ui))? {
            ModalEvent::Done(m) => Some(m),
            ModalEvent::Closed => Some(SvgMode::None),
        }
    }

    /// Form body. Returns the picked mode.
    fn ui(&mut self, ui: &mut Ui) -> Option<SvgMode> {
        let mut out = None;
        ui.columns(2, |cols| {
            for (i, (tex, caption, mode)) in [(&self.vector, "Load as Vector", SvgMode::Vector), (&self.image, "Load as Image", SvgMode::Raster)].into_iter().enumerate() {
                let ui = &mut cols[i];
                ui.vertical_centered(|ui| {
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 360.0), Sense::click());
                    if let Some(t) = tex {
                        // PictureBoxSizeMode.Zoom
                        let r = egui::Rect::from_center_size(rect.center(), t.size_vec2() * (rect.width() / t.size_vec2().x).min(rect.height() / t.size_vec2().y));
                        // SVG.NET draws on a white bitmap
                        ui.painter().rect_filled(r, 0.0, egui::Color32::WHITE);
                        egui::Image::new(t).paint_at(ui, r);
                    }
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        out = Some(mode);
                    }
                    ui.label(caption);
                });
            }
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_preview() {
        let p = lg_base::paths::test_dir("svg_mode").join("mode.svg");
        std::fs::write(&p, r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect width="40" height="20" fill="red"/></svg>"#).unwrap();
        let img = render_svg_preview(&p, 100).unwrap();
        assert_eq!(img.size, [100, 50]);
        assert!(render_svg_preview(Path::new("/nonexistent.svg"), 100).is_none());
    }
}
