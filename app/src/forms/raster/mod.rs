#![allow(dead_code, clippy::collapsible_if, clippy::collapsible_match, clippy::too_many_arguments, clippy::type_complexity)]
//! Raster import dialogs (WP-U3): `RasterConverter/RasterToLaserForm.cs`,
//! `ConvertSizeAndOptionForm.cs`, `ResolutionHelperForm.cs`.
//!
//! Shared helpers live here: modal frame, tooltip helper, enum <-> setting tables.

use eframe::egui::{self, Response, Ui};
use lg_services::image_processor::{Direction, DitheringMode, Formula, InterpolationMode, Tool};
use serde_json::Value;
use std::fmt::Debug;
use std::hash::Hash;

use crate::ui::i18n::tr;

pub mod convert_size;
pub mod gallery_demo;
pub mod raster_to_laser;
pub mod resolution_helper;

pub use convert_size::ConvertSizeForm;
pub use raster_to_laser::RasterImport;
pub use resolution_helper::ResolutionHelperForm;

/// Usage counters touched by the raster import (mirror of core `UsageCounters` fields).
///
/// The core adds this to its counters after the dialog closed: `raster_file` is the
/// `UsageCounters.RasterFile++` that C# `OpenFile` does after `CreateAndShowDialog` returns.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UsageDelta {
    pub raster_file: i32,
    pub dithering: i32,
    pub line2line: i32,
    pub vectorization: i32,
    pub centerline: i32,
    pub passthrough: i32,
}

/// Result of a finished raster import dialog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RasterOutcome {
    /// G-code was generated into the shared file (`GenerationComplete` without error).
    pub generated: bool,
    /// Counter increments (see [`UsageDelta`]).
    pub usage: UsageDelta,
    /// Exception text of a failed generation (already shown to the user).
    pub error: Option<String>,
}

/// Show a modal form window (WinForms `ShowDialog`).
///
/// `ctx`: egui context. `id`: unique form id. `title`: form caption. `content`: draws the
/// body, returns `Some(result)` to close. Returns the result on the closing frame.
pub(crate) fn form_modal<T>(ctx: &egui::Context, id: impl Hash + Debug, title: &str, content: impl FnOnce(&mut Ui) -> Option<T>) -> Option<T> {
    form_modal_x(ctx, id, title, content).0
}

/// Like [`form_modal`] with the title-bar close box (every WinForms form has one).
///
/// `ctx`/`id`/`title`/`content`: see [`form_modal`]. Returns (content result, close box clicked).
pub(crate) fn form_modal_x<T>(ctx: &egui::Context, id: impl Hash + Debug, title: &str, content: impl FnOnce(&mut Ui) -> Option<T>) -> (Option<T>, bool) {
    let modal = egui::Modal::new(egui::Id::unique(id));
    let mut closed = false;
    let r = modal
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(crate::ui::fonts::bold(title));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(crate::ui::widgets::ImageButton::new("mdi-close").size(egui::Vec2::splat(16.0))).clicked() {
                        closed = true;
                    }
                });
            });
            ui.separator();
            content(ui)
        })
        .inner;
    (r, closed)
}

/// Attach the translated tooltip `<prefix>.<ctl>.ToolTip` when the key exists.
///
/// `r`: widget response. `prefix`: form resource prefix. `ctl`: control name. Returns the response.
pub(crate) fn tip(r: Response, prefix: &str, ctl: &str) -> Response {
    let key = format!("{prefix}.{ctl}.ToolTip");
    let t = tr(&key);
    if t == key || t.is_empty() { r } else { r.on_hover_text(t.to_owned()) }
}

/// Enum that is stored in the settings by variant name (C# stores the boxed enum).
pub(crate) trait SettingEnum: Copy + PartialEq + 'static {
    /// (value, C# name) of every variant.
    fn table() -> &'static [(Self, &'static str)];
    /// Numeric C# value.
    fn value(self) -> i32;
    /// C# `ToString()`.
    fn name(self) -> &'static str {
        Self::table().iter().find(|(v, _)| *v == self).map(|(_, n)| *n).unwrap_or("")
    }
}

macro_rules! setting_enum {
    ($t:ty, $conv:expr, [$($v:ident),+ $(,)?]) => {
        impl SettingEnum for $t {
            fn table() -> &'static [(Self, &'static str)] {
                &[$((<$t>::$v, stringify!($v))),+]
            }
            fn value(self) -> i32 {
                $conv(self)
            }
        }
    };
}

setting_enum!(Tool, |v: Tool| v as i32, [Line2Line, Dithering, Vectorize, Centerline, NoProcessing]);
setting_enum!(
    Direction,
    |v: Direction| v as i32,
    [None, Horizontal, Vertical, Diagonal, NewHorizontal, NewVertical, NewDiagonal, NewReverseDiagonal, NewGrid, NewDiagonalGrid, NewCross, NewDiagonalCross, NewSquares, NewZigZag, NewHilbert, NewInsetFilling]
);
setting_enum!(Formula, |v: Formula| v as i32, [SimpleAverage, WeightAverage, OpticalCorrect, Custom]);
setting_enum!(DitheringMode, |v: DitheringMode| v as i32, [Atkinson, FloydSteinberg, Burks, Jarvis, Random, Sierra2, Sierra3, SierraLight, Stucki]);
setting_enum!(InterpolationMode, |v: InterpolationMode| v as i32, [HighQualityBicubic, NearestNeighbor]);

/// `Settings.GetObject<TEnum>(key, default)`.
///
/// `key`: setting key. `default`: value when missing or unknown. Returns the stored enum
/// (variant name string or numeric value both accepted).
pub(crate) fn get_enum<T: SettingEnum>(key: &str, default: T) -> T {
    let v: Value = lg_base::settings::get(key, Value::Null);
    match v {
        Value::String(s) => T::table().iter().find(|(_, n)| *n == s).map(|(e, _)| *e).unwrap_or(default),
        Value::Number(n) => n.as_i64().and_then(|n| T::table().iter().find(|(e, _)| e.value() as i64 == n).map(|(e, _)| *e)).unwrap_or(default),
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_names_roundtrip() {
        assert_eq!(Tool::Centerline.name(), "Centerline");
        assert_eq!(Direction::NewInsetFilling.value(), 15);
        assert_eq!(InterpolationMode::NearestNeighbor.value(), 5);
        assert_eq!(Formula::Custom.name(), "Custom");
    }
}
