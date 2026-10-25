// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Ports `SplashScreenForm.cs` (+ Designer): borderless, top-most splash with logo, version and 2 s timer.

use eframe::egui::{self, Align2, Color32, FontId, Frame, Sense, TextureHandle, Vec2};
use std::time::{Duration, Instant};

use crate::forms::form_util::load_texture;

/// `timer1.Interval`.
pub const SPLASH_MS: u64 = 2000;

/// Splash screen state.
pub struct SplashScreen {
    tex: Option<TextureHandle>,
    version: String,
    /// Set on first draw (C# `SplashScreenForm_Load` starts the timer).
    started: Option<Instant>,
}

impl Default for SplashScreen {
    /// Version text as in the ctor: `"v" + Program.CurrentVersion.ToString(3)`.
    fn default() -> Self {
        SplashScreen { tex: None, version: format!("v{}", env!("CARGO_PKG_VERSION")), started: None }
    }
}

impl SplashScreen {
    /// `new SplashScreenForm()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Draw the splash; call every frame while open.
    ///
    /// `ctx`: egui context. Returns true once closed (timer elapsed or clicked); stop calling then.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        if self.tex.is_none() {
            self.tex = load_texture(ctx, "splash", include_bytes!("../../../resources/branding/splash.png"));
        }
        let started = *self.started.get_or_insert_with(Instant::now);
        let left = Duration::from_millis(SPLASH_MS).saturating_sub(started.elapsed());
        if left.is_zero() {
            return true;
        }
        ctx.request_repaint_after(left);
        let mut clicked = false;
        egui::Modal::new(egui::Id::unique("splash")).frame(Frame::NONE).backdrop_color(Color32::TRANSPARENT).show(ctx, |ui| {
            let size = self.tex.as_ref().map_or(Vec2::new(756.0, 333.0), |t| t.size_vec2());
            let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
            if let Some(t) = &self.tex {
                egui::Image::new(t).paint_at(ui, rect);
            }
            // version text, light grey on dark art, 3 px from bottom-right
            ui.painter().text(rect.right_bottom() - Vec2::splat(3.0), Align2::RIGHT_BOTTOM, &self.version, FontId::proportional(11.0), Color32::from_rgb(0x9a, 0xa3, 0xb2));
            clicked = resp.clicked();
        });
        clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_text() {
        let s = SplashScreen::new();
        assert!(s.version.starts_with('v') && s.version.matches('.').count() == 2);
    }

    #[test]
    fn closes_after_timer() {
        let ctx = egui::Context::default();
        let mut s = SplashScreen::new();
        s.started = Some(Instant::now() - Duration::from_millis(SPLASH_MS + 1));
        // elapsed timer returns before any drawing
        let closed = s.show(&ctx);
        assert!(closed);
    }
}
