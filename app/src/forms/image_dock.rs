// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Andreas Pösch

//! Raster-image dock (right side of the main window). Replaces the modal `RasterToLaserForm` for plain
//! line-to-line engraving: the image loads at once, a histogram with a brightness to power map, the target
//! position and size, line settings and laser options are edited here, and every committed edit regenerates the G-code in
//! the background. The classic dialog stays reachable through a button.
//! Not part of the C# app (intentional deviation).

use eframe::egui::{self, Grid, Id, RichText, Ui, Vec2};
use image::RgbaImage;
use lasergrbl_core::GrblCore;
use lg_dotnet::{decimal_to_f64, f64_to_decimal, fmt_double_custom, fmt_float_custom, Decimal};
use lg_gcode::{Direction, Firmware, GrblFile, PowerMap};
use lg_services::raster_dock::{self, RasterParams};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::forms::form_util::*;
use crate::forms::raster::{get_enum, translate_enum, SettingEnum};
use crate::ui::dialog::{MessageBox, MessageBoxHost, MessageBoxIcon};
use crate::ui::widgets::{enum_combo, flat_combo, grbl_group_box_wide, BrightnessHistogram, DecimalInputRanged, ImageButton, PowerScale};
use crate::ui::{tr, tr_fmt, tr_plain};

/// i18n prefix of the classic raster size / laser form (captions and tooltips are reused).
const R: &str = "RasterConverter.ConvertSizeAndOptionForm.";

/// i18n prefix of the classic raster dialog (direction / quality captions are reused).
const RTL: &str = "RasterConverter.RasterToLaserForm.";

/// Setting that stores the per-file values (JSON array of [`StoredSet`]).
const SETS_KEY: &str = "ImageDock.ImageSets";

/// Settings of the brightness map (last used).
const MAP_START_KEY: &str = "ImageDock.MapStart";
const MAP_STOP_KEY: &str = "ImageDock.MapStop";

/// Most files kept in the per-file store.
const MAX_SETS: usize = 50;

/// Message box tags.
#[derive(Clone, Copy)]
enum Msg {
    WrongLaserMode,
    Busy,
}

/// What the dock asks the main window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDockAction {
    /// Hand the image to the classic raster import dialog (see [`ImageDock::take_classic`]).
    OpenClassic,
}

/// Editable values of the dock (all stored per file).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ImageUi {
    /// Lower-left corner in mm.
    x0: f32,
    y0: f32,
    /// Upper-right corner in mm.
    x1: f32,
    y1: f32,
    /// Lines per mm.
    quality: f64,
    /// `Direction` enum name (a raster filling).
    direction: String,
    /// Brightness that burns with S-MAX.
    start: u8,
    /// Brightness that burns with S-MIN.
    stop: u8,
    mark_speed: i32,
    min_power: i32,
    max_power: i32,
    /// Index into [`LASER_OPTIONS`].
    laser_on: usize,
}

impl ImageUi {
    /// Make stored values safe to use (old or hand-edited settings).
    ///
    /// `max_quality`: upper quality limit. `dirs`: allowed directions. Returns the repaired values.
    fn sanitized(mut self, max_quality: f64, dirs: &[Direction]) -> ImageUi {
        // quality inside the dialog range, NaN falls back to 3
        self.quality = if self.quality.is_finite() { self.quality.clamp(0.1, max_quality) } else { 3.0 };
        if !dirs.iter().any(|d| d.name() == self.direction) {
            self.direction = Direction::Horizontal.name().to_owned();
        }
        self.laser_on = self.laser_on.min(LASER_OPTIONS.len() - 1);
        self.mark_speed = self.mark_speed.max(1);
        self
    }

    /// Returns the target width in mm (upper right minus lower left).
    fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    /// Returns the target height in mm (upper right minus lower left).
    fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    /// Move the lower-left X; the width stays, upper right follows.
    ///
    /// `x0`: new lower-left X in mm.
    fn move_x(&mut self, x0: f32) {
        let w = self.width();
        self.x0 = x0;
        self.x1 = round2(x0 + w);
    }

    /// Move the lower-left Y; the height stays, upper right follows.
    ///
    /// `y0`: new lower-left Y in mm.
    fn move_y(&mut self, y0: f32) {
        let h = self.height();
        self.y0 = y0;
        self.y1 = round2(y0 + h);
    }

    /// Set the width; with `keep_aspect` the height follows the ratio from before the edit.
    ///
    /// `w`: new width in mm. `keep_aspect`: lock is on. `native_ratio`: image height / width, used when the
    /// current size has no usable ratio (zero, negative, NaN).
    fn set_width(&mut self, w: f32, keep_aspect: bool, native_ratio: f32) {
        let (old_w, old_h) = (self.width(), self.height());
        self.x1 = round2(self.x0 + w);
        if keep_aspect {
            // current ratio, else the image's own
            let ratio = if usable_size(old_w, old_h) { old_h / old_w } else { native_ratio };
            self.y1 = round2(self.y0 + w * ratio);
        }
    }

    /// Set the height; with `keep_aspect` the width follows the ratio from before the edit.
    ///
    /// `h`: new height in mm. `keep_aspect`: lock is on. `native_ratio`: image height / width, used when the
    /// current size has no usable ratio (zero, negative, NaN).
    fn set_height(&mut self, h: f32, keep_aspect: bool, native_ratio: f32) {
        let (old_w, old_h) = (self.width(), self.height());
        self.y1 = round2(self.y0 + h);
        if keep_aspect {
            // current ratio, else the image's own
            let ratio = if usable_size(old_w, old_h) { old_w / old_h } else { 1.0 / native_ratio };
            self.x1 = round2(self.x0 + h * ratio);
        }
    }
}

/// Returns true when a size has a ratio: both sides finite and above zero.
///
/// `w`/`h`: sides in mm.
fn usable_size(w: f32, h: f32) -> bool {
    w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0
}

/// Saved values of one image file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct StoredSet {
    path: String,
    saved_unix_secs: u64,
    values: ImageUi,
}

/// Read the per-file store. Returns the saved sets (empty when none/unreadable).
fn load_sets() -> Vec<StoredSet> {
    lg_base::settings::get(SETS_KEY, Vec::new())
}

/// Add a set to the store: one entry per path (newest timestamp wins), newest first, capped.
///
/// `sets`: store content, modified in place. `new`: set to add.
fn merge_set(sets: &mut Vec<StoredSet>, new: StoredSet) {
    match sets.iter().position(|s| s.path == new.path) {
        Some(i) if sets[i].saved_unix_secs > new.saved_unix_secs => {}
        Some(i) => sets[i] = new,
        None => sets.push(new),
    }
    sets.sort_by_key(|s| std::cmp::Reverse(s.saved_unix_secs));
    sets.truncate(MAX_SETS);
}

/// Current time as unix seconds. Returns 0 on a clock before the epoch.
fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Round a position to 2 decimals (what the input box shows).
///
/// `v`: value in mm. Returns the rounded value.
fn round2(v: f32) -> f32 {
    ((f64::from(v) * 100.0).round() / 100.0) as f32
}

/// Round a quality to 3 decimals.
///
/// `v`: lines per mm. Returns the rounded value.
fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// Raster fillings in the order of the classic dialog's direction combo.
///
/// Returns Horizontal, Vertical, Diagonal (and any other `raster_filling` direction).
fn raster_directions() -> Vec<Direction> {
    Direction::table().iter().map(|(d, _)| *d).filter(|d| lg_gcode::raster_filling(*d)).collect()
}

/// Direction from its stored name.
///
/// `name`: `Direction` variant name. Returns the direction; Horizontal when unknown or not a raster filling.
fn dir_from_name(name: &str) -> Direction {
    Direction::table().iter().find(|(_, n)| *n == name).map(|(d, _)| *d).filter(|d| lg_gcode::raster_filling(*d)).unwrap_or(Direction::Horizontal)
}

/// Start values from the settings shared with the classic dialog.
///
/// `limits`: machine limits (default S-MAX). `max_quality`: quality limit. `img_w`/`img_h`: image size in px.
/// Returns the values; size like the classic dialog with kept aspect ratio (landscape: width setting, else height).
fn settings_values(limits: &Limits, max_quality: f64, img_w: u32, img_h: u32) -> ImageUi {
    use lg_base::settings::get;
    let laser_on: String = get("GrayScaleConversion.Gcode.LaserOptions.LaserOn", "M3".to_owned());
    let (x0, y0): (f32, f32) = (get("GrayScaleConversion.Gcode.Offset.X", 0.0), get("GrayScaleConversion.Gcode.Offset.Y", 0.0));
    // size with kept ratio (ConvertSizeForm::init_image_size)
    let (iw, ih) = (img_w.max(1) as f32, img_h.max(1) as f32);
    let (w, h) = if ih < iw {
        let w: f32 = get("GrayScaleConversion.Gcode.ImageSize.W", 100.0);
        (w, w * ih / iw)
    } else {
        let h: f32 = get("GrayScaleConversion.Gcode.ImageSize.H", 100.0);
        (h * iw / ih, h)
    };
    let (x0, y0) = (round2(x0), round2(y0));
    let direction: String = get_enum("GrayScaleConversion.Line2LineOptions.Direction", Direction::Horizontal).name().to_owned();
    let quality = decimal_to_f64(get::<Decimal>("GrayScaleConversion.Line2LineOptions.Quality", f64_to_decimal(3.0)));
    ImageUi {
        x0,
        y0,
        x1: round2(x0 + w),
        y1: round2(y0 + h),
        quality,
        direction,
        start: get::<i32>(MAP_START_KEY, 0).clamp(0, 255) as u8,
        stop: get::<i32>(MAP_STOP_KEY, 254).clamp(0, 255) as u8,
        mark_speed: get("GrayScaleConversion.Gcode.Speed.Mark", 1000),
        min_power: get("GrayScaleConversion.Gcode.LaserOptions.PowerMin", 0),
        max_power: get("GrayScaleConversion.Gcode.LaserOptions.PowerMax", limits.max_pwm),
        laser_on: if laser_on == "M3" || !limits.laser_mode { 0 } else { 1 },
    }
    .sanitized(max_quality, &raster_directions())
}

/// Image the dock regenerates G-code from.
struct ImageSource {
    /// Shared program.
    file: SharedFile,
    /// Image path.
    path: String,
    /// Import was an append.
    append: bool,
    /// Commands that existed before the import (kept on every re-apply when `append`).
    base_len: usize,
    /// Opaque gray version of the image (the one brightness definition of histogram and G-code).
    gray: Arc<RgbaImage>,
    /// Brightness histogram of `gray`.
    hist: Box<[u32; 256]>,
    /// Image size in px.
    img_w: u32,
    img_h: u32,
}

/// Running G-code generation.
struct ApplyJob {
    /// `edit_seq` when the job started.
    seq: u64,
    handle: JoinHandle<()>,
    /// Set by the worker when the generation succeeded.
    ok: Arc<AtomicBool>,
}

/// Raster-image dock state.
pub struct ImageDock {
    limits: Limits,
    support_pwm: bool,
    /// Upper limit of the quality input.
    max_quality: f64,
    firmware: Firmware,
    /// Raster fillings offered in the direction combo.
    line_dirs: Vec<Direction>,
    /// Edited values.
    v: ImageUi,
    /// Size lock: width and height keep the target ratio. On at every start, not stored.
    keep_aspect: bool,
    source: Option<ImageSource>,
    /// Edited since the last successful generation.
    dirty: bool,
    /// Bumped on every edit that needs a regen.
    edit_seq: u64,
    /// `edit_seq` of the last automatic attempt.
    auto_seq: u64,
    job: Option<ApplyJob>,
    last_error: Option<String>,
    msgs: MessageBoxHost<Msg>,
}

impl ImageDock {
    /// Build the dock with values from the settings.
    ///
    /// Returns the dock without a source.
    pub fn new() -> Self {
        let limits = Limits::from_config();
        let mut d = ImageDock {
            limits,
            support_pwm: true,
            max_quality: 20.0,
            firmware: Firmware::Grbl,
            line_dirs: raster_directions(),
            v: settings_values(&limits, 20.0, 1, 1),
            keep_aspect: true,
            source: None,
            dirty: false,
            edit_seq: 0,
            auto_seq: 0,
            job: None,
            last_error: None,
            msgs: MessageBoxHost::default(),
        };
        d.refresh_settings();
        d.v = settings_values(&d.limits, d.max_quality, 1, 1);
        d
    }

    /// Re-read the settings the dock depends on (config changes after connecting, Settings dialog).
    fn refresh_settings(&mut self) {
        self.limits = Limits::from_config();
        self.support_pwm = lg_base::settings::get("Support Hardware PWM", true);
        self.max_quality = if lg_base::settings::get("Raster Hi-Res", false) { 50.0 } else { 20.0 };
        self.firmware = lg_base::settings::get("Firmware Type", Firmware::Grbl);
    }

    /// Returns true when an image is loaded.
    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }

    /// Load an image and generate at once.
    ///
    /// `file`: shared program. `path`: image file. `append`: keep the commands already in `file`.
    /// Values come from the per-file store, else from the settings. Returns Err with the read / decode error text
    /// (nothing changed then).
    pub fn load_image(&mut self, file: SharedFile, path: &Path, append: bool) -> Result<(), String> {
        let (img, _dpi) = raster_dock::load_image(path)?;
        let gray = raster_dock::gray_image(&img);
        let hist = raster_dock::histogram(&gray);
        let (img_w, img_h) = (gray.width(), gray.height());
        self.refresh_settings();
        let path_s = path.to_string_lossy().into_owned();
        // stored values of this file win over the last used ones
        let stored = load_sets().into_iter().find(|s| s.path == path_s).map(|s| s.values.sanitized(self.max_quality, &self.line_dirs));
        self.v = stored.unwrap_or_else(|| settings_values(&self.limits, self.max_quality, img_w, img_h));
        // commands that stay in front of the import
        let base_len = if append { file.read().map(|f| f.count()).unwrap_or(0) } else { 0 };
        self.source = Some(ImageSource { file, path: path_s, append, base_len, gray: Arc::new(gray), hist: Box::new(hist), img_w, img_h });
        self.dirty = true;
        self.last_error = None;
        self.apply();
        Ok(())
    }

    /// Forget the image (another file type was opened). Values stay, auto regen stops.
    pub fn clear_source(&mut self) {
        self.source = None;
        self.dirty = false;
        self.last_error = None;
    }

    /// Give the image to the classic import dialog and forget it here.
    ///
    /// Returns `(program, image path, append)`, or None without an image or while a generation runs. When the
    /// import was an append, the program is cut back to the commands that existed before it.
    pub fn take_classic(&mut self) -> Option<(SharedFile, PathBuf, bool)> {
        if self.job.is_some() {
            return None;
        }
        let src = self.source.take()?;
        self.dirty = false;
        self.last_error = None;
        if src.append {
            // back to the program before the import
            let mut g = src.file.write().unwrap_or_else(|e| e.into_inner());
            g.truncate_list(src.base_len);
            if let Err(e) = g.analyze() {
                lg_base::logger::log_exception("ImageDock", &e);
            }
        }
        Some((src.file, PathBuf::from(src.path), src.append))
    }

    /// Generator parameters of the current values. Returns the parameter set.
    fn params(&self) -> RasterParams {
        let v = &self.v;
        RasterParams {
            x0: v.x0,
            y0: v.y0,
            x1: v.x1,
            y1: v.y1,
            quality: v.quality,
            direction: dir_from_name(&v.direction),
            map: PowerMap { start: v.start, stop: v.stop },
            mark_speed: v.mark_speed,
            min_power: v.min_power,
            max_power: v.max_power,
            laser_on: LASER_OPTIONS[v.laser_on.min(LASER_OPTIONS.len() - 1)].1.to_owned(),
            // only M5 is supported, like the classic dialog
            laser_off: "M5".to_owned(),
            pwm: self.support_pwm,
            firmware: self.firmware,
        }
    }

    /// Why the current values cannot be generated.
    ///
    /// Returns the translated message, or None when the layout is fine.
    fn layout_error(&self) -> Option<String> {
        let p = self.params();
        raster_dock::raster_layout(&p).err().map(|_| {
            if p.x1 <= p.x0 || p.y1 <= p.y0 { tr_plain("ImageDock.ErrorPlacement") } else { tr_plain("ImageDock.ErrorInvalid") }
        })
    }

    /// Mark the dock dirty and bump the edit counter (an edit needs a regen).
    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.edit_seq += 1;
    }

    /// Start a regen for the latest edit, at most once per edit, never in a loop.
    ///
    /// `can_load`: `core.can_load_new_file()`; false while a program runs, then wait (attempt not consumed).
    /// A failed generation is not retried until the next edit; Run retries via `request_run`.
    fn auto_apply(&mut self, can_load: bool) {
        // nothing new, program busy, or a job runs / nothing to do
        if self.auto_seq == self.edit_seq || !can_load || !self.is_dirty_idle() {
            return;
        }
        self.auto_seq = self.edit_seq;
        self.apply();
    }

    /// Persist the values (settings shared with the classic dialog, map, per-file store) and start the
    /// G-code generation on a worker.
    ///
    /// Returns false when nothing was started (no source, job running, invalid layout: error set, or file busy:
    /// busy box queued).
    pub fn apply(&mut self) -> bool {
        if self.job.is_some() || self.source.is_none() {
            return false;
        }
        // invalid placement: no job, the red text says why
        if let Some(e) = self.layout_error() {
            self.last_error = Some(e);
            return false;
        }
        self.persist();
        let params = self.params();
        let Some(src) = &self.source else { return false };
        let (path, append, base_len, gray) = (src.path.clone(), src.append, src.base_len, src.gray.clone());
        let ok = Arc::new(AtomicBool::new(false));
        let ok_w = ok.clone();
        let handle = GrblFile::spawn_load(&src.file, move |f| {
            // drop the previous import, keep what was there before it
            if append {
                f.truncate_list(base_len);
            }
            raster_dock::generate(f, &gray, &path, &params, append)?;
            ok_w.store(true, Ordering::SeqCst);
            Ok(())
        });
        match handle {
            Some(handle) => {
                self.job = Some(ApplyJob { seq: self.edit_seq, handle, ok });
                self.last_error = None;
                true
            }
            None => {
                self.msgs.push(Msg::Busy, already_loading_box());
                false
            }
        }
    }

    /// Write the values as "last used" (settings shared with the classic dialog) and into the per-file store.
    fn persist(&self) {
        use lg_base::settings::set;
        let v = &self.v;
        set("GrayScaleConversion.Line2LineOptions.Direction", v.direction.clone());
        set("GrayScaleConversion.Line2LineOptions.Quality", f64_to_decimal(v.quality));
        set("GrayScaleConversion.Gcode.Speed.Mark", v.mark_speed);
        set("GrayScaleConversion.Gcode.LaserOptions.LaserOn", LASER_OPTIONS[v.laser_on.min(LASER_OPTIONS.len() - 1)].1);
        set("GrayScaleConversion.Gcode.LaserOptions.PowerMin", v.min_power);
        set("GrayScaleConversion.Gcode.LaserOptions.PowerMax", v.max_power);
        set("GrayScaleConversion.Gcode.Offset.X", v.x0);
        set("GrayScaleConversion.Gcode.Offset.Y", v.y0);
        set("GrayScaleConversion.Gcode.ImageSize.W", v.x1 - v.x0);
        set("GrayScaleConversion.Gcode.ImageSize.H", v.y1 - v.y0);
        set(MAP_START_KEY, i32::from(v.start));
        set(MAP_STOP_KEY, i32::from(v.stop));
        // per-file store
        if let Some(src) = &self.source {
            let mut sets = load_sets();
            merge_set(&mut sets, StoredSet { path: src.path.clone(), saved_unix_secs: now_secs(), values: v.clone() });
            set(SETS_KEY, sets);
        }
    }

    /// Returns true when the G-code is out of date or being generated (image loaded).
    pub fn needs_apply(&self) -> bool {
        self.source.is_some() && (self.dirty || self.job.is_some())
    }

    /// Returns true when edits wait, no generation runs and the values can be generated (`apply()` would start one).
    pub fn is_dirty_idle(&self) -> bool {
        self.source.is_some() && self.dirty && self.job.is_none() && self.layout_error().is_none()
    }

    /// Returns true when the program is stale and cannot be regenerated because the values are invalid.
    pub fn is_blocked(&self) -> bool {
        self.source.is_some() && self.dirty && self.job.is_none() && self.layout_error().is_some()
    }

    /// Returns true while the worker runs.
    pub fn job_running(&self) -> bool {
        self.job.is_some()
    }

    /// Check the worker; call every frame.
    ///
    /// Returns `Some(success)` once when a generation finished.
    pub fn poll(&mut self) -> Option<bool> {
        if !self.job.as_ref().is_some_and(|j| j.handle.is_finished()) {
            return None;
        }
        let job = self.job.take()?;
        let _ = job.handle.join();
        let ok = job.ok.load(Ordering::SeqCst);
        if ok {
            // edits made during the job keep the dock dirty
            self.dirty = self.edit_seq != job.seq;
            self.last_error = None;
        } else {
            self.last_error = Some(tr_plain("ParamsDock.ApplyFailed"));
        }
        Some(ok)
    }

    /// "P1" text of a power value, like the classic dialog's `RefreshPerc`.
    ///
    /// `v`: power value. Returns e.g. "100.0 %" formatted like .NET `P1`; empty without a valid MaxPWM.
    fn perc_text(&self, v: i32) -> String {
        if self.limits.max_pwm > 0 {
            lg_dotnet::fmt_decimal_custom(Decimal::from(v) / Decimal::from(self.limits.max_pwm), "P1")
        } else {
            String::new()
        }
    }

    /// Warn when M4 is picked but the firmware is not in laser mode (`CBLaserON_SelectedIndexChanged`).
    fn on_laser_mode_changed(&mut self) {
        if !self.limits.laser_mode && LASER_OPTIONS[self.v.laser_on.min(LASER_OPTIONS.len() - 1)].1 == "M4" {
            self.msgs.push(Msg::WrongLaserMode, MessageBox::new(tr("Strings.WarnWrongLaserMode"), tr("Strings.WarnWrongLaserModeTitle")).icon(MessageBoxIcon::Warning));
        }
    }

    /// Draw the dock's message boxes; call every frame.
    ///
    /// `ctx`: egui context.
    pub fn show_dialogs(&mut self, ctx: &egui::Context) {
        if self.msgs.is_open() {
            let _ = self.msgs.show(ctx);
        }
    }

    /// Dock body.
    ///
    /// `ui`: panel ui. `core`: for `can_load_new_file` (auto regen). Returns requests for the main window.
    pub fn show(&mut self, ui: &mut Ui, core: &GrblCore) -> Vec<ImageDockAction> {
        let mut actions = Vec::new();
        self.refresh_settings();
        let mut edited = false;

        ui.label(RichText::new(tr_plain("ParamsDock.Title")).strong());
        ui.add_space(2.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            self.map_group(ui, &mut edited);
            ui.add_space(4.0);
            self.position_group(ui, &mut edited);
            ui.add_space(4.0);
            self.line_group(ui, &mut edited);
            ui.add_space(4.0);
            self.speed_group(ui, &mut edited);
            ui.add_space(4.0);
            self.laser_group(ui, &mut edited);
            ui.add_space(6.0);
            let classic = ui.add_enabled(self.job.is_none(), egui::Button::new(tr_plain("ImageDock.Classic")).min_size(egui::vec2(ui.available_width(), 22.0)));
            if classic.on_hover_text(tr_plain("ImageDock.ClassicTip")).clicked() {
                actions.push(ImageDockAction::OpenClassic);
            }
            ui.add_space(8.0);
            self.status_row(ui);
        });
        if edited {
            self.mark_dirty();
        }
        self.auto_apply(core.can_load_new_file());
        actions
    }

    /// One-line explanation of the brightness map.
    ///
    /// `start`/`stop`: marker values. Returns the translated text (slope, inverted, single value, on/off).
    fn map_info(&self, start: u8, stop: u8) -> String {
        if !self.support_pwm {
            tr_fmt("ImageDock.MapOnOff", &[&start.min(stop), &start.max(stop)])
        } else if start == stop {
            tr_fmt("ImageDock.MapSingle", &[&start])
        } else if start < stop {
            tr_fmt("ImageDock.MapNormal", &[&start, &stop])
        } else {
            tr_fmt("ImageDock.MapInverted", &[&start, &stop])
        }
    }

    /// Histogram with the map markers, start / stop inputs and the slope text.
    ///
    /// `ui`: parent. `edited`: set on a committed change (drag released, input committed).
    fn map_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        grbl_group_box_wide(ui, &tr_plain("ImageDock.GroupMap"), |ui| {
            let (mut start, mut stop) = (self.v.start, self.v.stop);
            if let Some(src) = &self.source {
                let mut h = BrightnessHistogram::new(&src.hist, &mut start, &mut stop).id(Id::unique("image-hist")).labels(&tr_plain("ImageDock.Start"), &tr_plain("ImageDock.Stop"));
                if self.support_pwm {
                    h = h.power(PowerScale { s_min: self.v.min_power, s_max: self.v.max_power, s_limit: self.limits.max_pwm.max(1) });
                }
                if h.show(ui).committed {
                    *edited = true;
                }
            }
            let (mut a, mut b) = (i32::from(start), i32::from(stop));
            ui.horizontal(|ui| {
                ui.label(tr_plain("ImageDock.Start")).on_hover_text(tr_plain("ImageDock.StartTip"));
                if int_input(ui, "image-start", &mut a, 0, 255, true, 43.0).is_some() {
                    *edited = true;
                }
                ui.label(tr_plain("ImageDock.Stop")).on_hover_text(tr_plain("ImageDock.StopTip"));
                if int_input(ui, "image-stop", &mut b, 0, 255, true, 43.0).is_some() {
                    *edited = true;
                }
            });
            (self.v.start, self.v.stop) = (a.clamp(0, 255) as u8, b.clamp(0, 255) as u8);
            ui.label(RichText::new(self.map_info(self.v.start, self.v.stop)).small());
        });
    }

    /// Image height / width (ratio of the pixels). Returns 1 without an image.
    fn native_ratio(&self) -> f32 {
        self.source.as_ref().map_or(1.0, |s| s.img_h.max(1) as f32 / s.img_w.max(1) as f32)
    }

    /// Lower-left and size inputs with the aspect lock, the calculated upper right and the read-only facts.
    ///
    /// `ui`: parent. `edited`: set on a committed change (a lock toggle alone is no edit).
    fn position_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        grbl_group_box_wide(ui, &tr_plain("ImageDock.GroupPosition"), |ui| {
            let mut v = self.v.clone();
            let (mut x0, mut y0, mut w, mut h) = (v.x0, v.y0, v.width(), v.height());
            let (lock, native) = (self.keep_aspect, self.native_ratio());
            let mut lock_clicked = false;
            Grid::new("image-pos").num_columns(6).min_col_width(0.0).spacing([6.0, 4.0]).show(ui, |ui| {
                // lower left: moving keeps the size
                ui.label(tr_plain("ImageDock.LowerLeft"));
                ui.label("X");
                if let Some(nv) = mm_input(ui, "image-x0", &mut x0, -10000.0, 10000.0) {
                    v.move_x(nv);
                    *edited = true;
                }
                ui.label("");
                ui.label("Y");
                if let Some(nv) = mm_input(ui, "image-y0", &mut y0, -10000.0, 10000.0) {
                    v.move_y(nv);
                    *edited = true;
                }
                ui.end_row();
                // size: lock sits between width and height
                ui.label(tr_plain("ImageDock.SizeRow"));
                ui.label("W");
                if let Some(nv) = mm_input(ui, "image-w", &mut w, 0.01, 10000.0) {
                    v.set_width(nv, lock, native);
                    *edited = true;
                }
                let r = ui.add(ImageButton::new("mdi-lock").alt("mdi-lock-open-outline", !lock).size(Vec2::splat(17.0)));
                lock_clicked = r.on_hover_text(tr_plain("ImageDock.LockAspectTip")).clicked();
                ui.label("H");
                if let Some(nv) = mm_input(ui, "image-h", &mut h, 0.01, 10000.0) {
                    v.set_height(nv, lock, native);
                    *edited = true;
                }
                ui.end_row();
                // upper right: calculated, read-only
                ui.label(tr_plain("ImageDock.UpperRight"));
                ui.label("X");
                mm_label(ui, v.x1);
                ui.label("");
                ui.label("Y");
                mm_label(ui, v.y1);
                ui.end_row();
            });
            if lock_clicked {
                self.keep_aspect = !self.keep_aspect;
            }
            // read-only facts: aspect ratios, raster size
            let (w, h) = (v.width(), v.height());
            let (img_w, img_h) = self.source.as_ref().map_or((1, 1), |s| (s.img_w.max(1), s.img_h.max(1)));
            let target_aspect = if w > 0.0 && h > 0.0 { fmt_double_custom(f64::from(w / h), "0.000") } else { "-".to_owned() };
            let img_aspect = fmt_double_custom(f64::from(img_w) / f64::from(img_h), "0.000");
            ui.label(tr_fmt("ImageDock.Aspect", &[&target_aspect, &img_aspect]));
            let layout = raster_dock::raster_layout(&self.params_with(&v));
            if let Ok((_, (pw, ph))) = &layout {
                ui.label(tr_fmt("ImageDock.RasterSize", &[pw, ph]));
            }
            if layout.is_err() {
                let msg = if w <= 0.0 || h <= 0.0 { tr_plain("ImageDock.ErrorPlacement") } else { tr_plain("ImageDock.ErrorInvalid") };
                ui.colored_label(ui.visuals().error_fg_color, msg);
            }
            self.v = v;
        });
    }

    /// Generator parameters for a set of values (read-only facts use the live, not yet committed numbers).
    ///
    /// `v`: values. Returns the parameter set.
    fn params_with(&self, v: &ImageUi) -> RasterParams {
        let mut p = self.params();
        (p.x0, p.y0, p.x1, p.y1) = (v.x0, v.y0, v.x1, v.y1);
        p
    }

    /// Direction and quality.
    ///
    /// `ui`: parent. `edited`: set on a change.
    fn line_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        grbl_group_box_wide(ui, &tr_plain("Strings.Line2LineOptions"), |ui| {
            ui.horizontal(|ui| {
                ui.label(tr_plain(&format!("{RTL}label27.Text")));
                let mut dir = dir_from_name(&self.v.direction);
                let before = dir;
                let w = (ui.available_width() - 6.0).clamp(60.0, 170.0);
                let r = enum_combo(ui, "image-dir", &mut dir, &self.line_dirs, |d| translate_enum(*d, "Direction"), true, w);
                r.on_hover_text(tr(&format!("{RTL}CbDirections.ToolTip")).to_owned());
                if dir != before {
                    self.v.direction = dir.name().to_owned();
                    *edited = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr_plain(&format!("{RTL}label5.Text")));
                let mut q = self.v.quality as f32;
                let out = DecimalInputRanged::new(&mut q).range(0.1, self.max_quality as f32).decimals(3).width(55.0).id(Id::unique("image-quality")).show(ui);
                out.response.on_hover_text(tr(&format!("{RTL}UDQuality.ToolTip")).to_owned());
                if out.committed.is_some() {
                    self.v.quality = round3(f64::from(q));
                    *edited = true;
                }
                ui.label(tr_plain(&format!("{RTL}label8.Text")));
            });
        });
    }

    /// Engraving speed input.
    ///
    /// `ui`: parent. `edited`: set on a committed change.
    fn speed_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        let lim = self.limits.max_rate_x.max(1);
        grbl_group_box_wide(ui, &tr_plain(&format!("{R}GbSpeed.Text")), |ui| {
            ui.horizontal(|ui| {
                ui.label(tr_plain("ImageDock.EngraveSpeed"));
                let mut s = self.v.mark_speed;
                if int_input(ui, "image-speed", &mut s, 1, lim, false, 55.0).is_some() {
                    self.v.mark_speed = s;
                    *edited = true;
                }
                ui.label(tr_plain(&format!("{R}LblLinearFillingmm.Text")));
            });
        });
    }

    /// Laser mode, S-MIN and S-MAX (the power inputs only with hardware PWM).
    ///
    /// `ui`: parent. `edited`: set on any change.
    fn laser_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        let pwm_max = self.limits.max_pwm.max(1);
        grbl_group_box_wide(ui, &ctl_text(R, "GbLaser"), |ui| {
            let items: Vec<String> = LASER_OPTIONS.iter().map(|o| o.0.to_owned()).collect();
            // laser mode row
            ui.horizontal(|ui| {
                label(ui, R, "LblLaserMode");
                let before = self.v.laser_on;
                let mut sel = Some(before);
                // full width, no icon
                let w = ui.available_width().max(60.0);
                flat_combo(ui, "image-laser-on", &mut sel, &items, w);
                self.v.laser_on = sel.unwrap_or(before);
                if self.v.laser_on != before {
                    *edited = true;
                    self.on_laser_mode_changed();
                }
            });
            if !self.support_pwm {
                return;
            }
            Grid::new("image-power").num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
                // S-MIN
                label(ui, R, "LblSmin");
                let mut min_p = self.v.min_power;
                if let Some((_, nv)) = int_input(ui, "image-smin", &mut min_p, 0, (pwm_max - 1).max(0), false, 43.0) {
                    self.v.min_power = min_p;
                    // keep S-MIN below S-MAX like the classic dialog
                    if self.v.max_power <= nv {
                        self.v.max_power = nv + 1;
                    }
                    *edited = true;
                }
                ui.label(self.perc_text(self.v.min_power));
                ui.end_row();
                // S-MAX
                label(ui, R, "LblSmax");
                let mut max_p = self.v.max_power;
                if let Some((_, nv)) = int_input(ui, "image-smax", &mut max_p, 1, pwm_max, false, 43.0) {
                    self.v.max_power = max_p;
                    if self.v.min_power >= nv {
                        self.v.min_power = nv - 1;
                    }
                    *edited = true;
                }
                ui.label(self.perc_text(self.v.max_power));
                ui.end_row();
            });
        });
    }

    /// File name, generation state and errors.
    ///
    /// `ui`: parent.
    fn status_row(&mut self, ui: &mut Ui) {
        let status = match &self.source {
            Some(s) => {
                let name = file_name(&s.path);
                let size = format!("{name} ({} × {} px)", s.img_w, s.img_h);
                if self.job.is_some() { format!("{size} - {}", tr_plain("ParamsDock.Generating")) } else { size }
            }
            None => tr_plain("ImageDock.NoSource"),
        };
        ui.label(RichText::new(status).small());
        if let Some(e) = &self.last_error {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
    }
}

impl Default for ImageDock {
    /// Same as [`ImageDock::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// One millimetre input (2 decimals).
///
/// `ui`: parent. `id`: unique id text. `v`: bound value. `min`/`max`: range (out of range shows the red border).
/// Returns the new value when one was committed, else None.
fn mm_input(ui: &mut Ui, id: &str, v: &mut f32, min: f32, max: f32) -> Option<f32> {
    let out = DecimalInputRanged::new(v).range(min, max).decimals(2).width(52.0).id(Id::unique(id)).show(ui);
    out.committed.map(|(_, new)| new)
}

/// Read-only millimetre value, sized like an input.
///
/// `ui`: parent. `v`: value in mm, shown with 2 decimals.
fn mm_label(ui: &mut Ui, v: f32) {
    // left aligned with the text of the inputs next to it
    let size = egui::vec2(52.0, ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.add_space(3.0);
        ui.label(fmt_float_custom(v, "0.00"));
    });
}

/// File name part of a path text.
///
/// `p`: path. Returns the last component, or `p` when it has none.
fn file_name(p: &str) -> String {
    Path::new(p).file_name().map_or_else(|| p.to_owned(), |n| n.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::RwLock;
    use std::time::{Duration, Instant};

    fn new_file() -> SharedFile {
        Arc::new(RwLock::new(GrblFile::new()))
    }

    /// PNG in the temp dir: `w` x `h` px, gray value per column from `col`. Returns the path.
    fn write_png(tag: &str, w: u32, h: u32, col: impl Fn(u32) -> u8) -> PathBuf {
        let p = std::env::temp_dir().join(format!("lg-image-dock-{tag}-{}.png", std::process::id()));
        let img = RgbaImage::from_fn(w, h, |x, _| {
            let g = col(x);
            image::Rgba([g, g, g, 255])
        });
        img.save(&p).unwrap();
        p
    }

    /// Poll until the worker finished (5 s limit).
    fn wait(d: &mut ImageDock) -> bool {
        let t = Instant::now();
        loop {
            if let Some(ok) = d.poll() {
                return ok;
            }
            assert!(t.elapsed() < Duration::from_secs(5), "worker timeout");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Program lines of a file. `f`: shared program. Returns the command texts.
    fn lines(f: &SharedFile) -> Vec<String> {
        f.read().unwrap().commands().iter().map(|c| c.command().to_owned()).collect()
    }

    /// Values of word `w` over `lines`. Returns the numbers.
    fn words(lines: &[String], w: char) -> Vec<f64> {
        lines.iter().flat_map(|l| l.split_whitespace().filter_map(|t| t.strip_prefix(w).and_then(|v| v.parse::<f64>().ok())).collect::<Vec<_>>()).collect()
    }

    /// Largest X of the program. `lines`: program text.
    fn max_x(lines: &[String]) -> f64 {
        words(lines, 'X').into_iter().fold(f64::MIN, f64::max)
    }

    /// Distinct S values above 0, sorted.
    fn burn_levels(lines: &[String]) -> Vec<i64> {
        let mut v: Vec<i64> = words(lines, 'S').into_iter().filter(|s| *s > 0.0).map(|s| s as i64).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    #[test]
    fn defaults_from_empty_settings() {
        let _g = settings_guard();
        let d = ImageDock::new();
        let v = &d.v;
        assert_eq!((v.x0, v.y0, v.x1, v.y1), (0.0, 0.0, 100.0, 100.0));
        assert_eq!((v.quality, v.direction.as_str()), (3.0, "Horizontal"));
        // white (255) never burns by default
        assert_eq!((v.start, v.stop), (0, 254));
        assert_eq!((v.mark_speed, v.min_power, v.max_power, v.laser_on), (1000, 0, 1000, 0));
        assert!(d.support_pwm && !d.has_source() && !d.needs_apply());
        assert_eq!(d.line_dirs, vec![Direction::Horizontal, Direction::Vertical, Direction::Diagonal]);
    }

    #[test]
    fn size_from_settings_keeps_aspect_ratio() {
        let _g = settings_guard();
        lg_base::settings::set("GrayScaleConversion.Gcode.Offset.X", 5.0f32);
        lg_base::settings::set("GrayScaleConversion.Gcode.Offset.Y", 7.0f32);
        lg_base::settings::set("GrayScaleConversion.Gcode.ImageSize.W", 80.0f32);
        lg_base::settings::set("GrayScaleConversion.Gcode.ImageSize.H", 30.0f32);
        let lim = Limits::from_config();
        // landscape: width setting, height from aspect
        let v = settings_values(&lim, 20.0, 400, 200);
        assert_eq!((v.x0, v.y0, v.x1, v.y1), (5.0, 7.0, 85.0, 47.0));
        // portrait: height setting, width from aspect
        let v = settings_values(&lim, 20.0, 200, 400);
        assert_eq!((v.x1 - v.x0, v.y1 - v.y0), (15.0, 30.0));
    }

    #[test]
    fn stored_values_are_sanitized() {
        let dirs = raster_directions();
        let v = ImageUi { x0: 0.0, y0: 0.0, x1: 1.0, y1: 1.0, quality: 99.0, direction: "NewHilbert".into(), start: 0, stop: 1, mark_speed: 0, min_power: 0, max_power: 5, laser_on: 7 }.sanitized(20.0, &dirs);
        assert_eq!((v.quality, v.direction.as_str(), v.mark_speed, v.laser_on), (20.0, "Horizontal", 1, 1));
        let v = ImageUi { quality: f64::NAN, ..v }.sanitized(20.0, &dirs);
        assert_eq!(v.quality, 3.0);
        let v = ImageUi { quality: 0.0, ..v }.sanitized(20.0, &dirs);
        assert_eq!(v.quality, 0.1);
    }

    #[test]
    fn load_generates_and_first_move_hits_origin() {
        let _g = settings_guard();
        lg_base::settings::set("GrayScaleConversion.Gcode.Offset.X", 5.0f32);
        lg_base::settings::set("GrayScaleConversion.Gcode.Offset.Y", 7.0f32);
        lg_base::settings::set("GrayScaleConversion.Gcode.ImageSize.W", 20.0f32);
        let p = write_png("load", 40, 20, |_| 0);
        let file = new_file();
        let mut d = ImageDock::new();
        d.load_image(file.clone(), &p, false).unwrap();
        assert!(d.job_running() && d.has_source());
        assert!(wait(&mut d));
        assert!(!d.dirty && !d.needs_apply());
        // 2:1 image, 20 mm wide: upper right at (25, 17)
        assert_eq!((d.v.x1, d.v.y1), (25.0, 17.0));
        let l = lines(&file);
        assert!(l.len() > 5);
        assert_eq!(l[0], "G0 X5 Y7 F1000");
        assert!(words(&l, 'X').iter().all(|x| (5.0 - 1e-3..=25.0 + 1e-3).contains(x)));
        assert!(words(&l, 'Y').iter().all(|y| (7.0 - 1e-3..=17.0 + 1e-3).contains(y)));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn unreadable_image_is_an_error_and_changes_nothing() {
        let _g = settings_guard();
        let mut d = ImageDock::new();
        assert!(d.load_image(new_file(), Path::new("/nonexistent/none.png"), false).is_err());
        assert!(!d.has_source());
        let p = std::env::temp_dir().join(format!("lg-image-dock-bad-{}.png", std::process::id()));
        std::fs::write(&p, b"not an image").unwrap();
        assert!(d.load_image(new_file(), &p, false).is_err());
        assert!(!d.has_source());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn edit_x1_regenerates_with_larger_max_x() {
        let _g = settings_guard();
        let p = write_png("x1", 40, 20, |_| 0);
        let file = new_file();
        let mut d = ImageDock::new();
        d.load_image(file.clone(), &p, false).unwrap();
        assert!(wait(&mut d));
        let before = max_x(&lines(&file));
        assert!((before - 100.0).abs() < 1e-3, "{before}");
        d.v.x1 = 150.0;
        d.mark_dirty();
        // program busy: attempt waits, then runs once
        d.auto_apply(false);
        assert!(!d.job_running());
        d.auto_apply(true);
        assert!(d.job_running());
        assert!(wait(&mut d));
        assert!((max_x(&lines(&file)) - 150.0).abs() < 1e-3);
        // no new edit: no new job
        d.auto_apply(true);
        assert!(!d.job_running());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn inverted_mapping_flips_which_pixels_burn_hard() {
        let _g = settings_guard();
        // left half gray 100, right half gray 200
        let p = write_png("inv", 40, 20, |x| if x < 20 { 100 } else { 200 });
        let file = new_file();
        let mut d = ImageDock::new();
        d.load_image(file.clone(), &p, false).unwrap();
        d.v.min_power = 100;
        assert!(wait(&mut d));
        // first burning value of the program (the left half, rows start there)
        let first_burn = |f: &SharedFile| words(&lines(f), 'S').into_iter().find(|s| *s > 0.0);
        // normal: darker burns harder, both halves are inside [100, 200]
        d.v.start = 100;
        d.v.stop = 200;
        d.mark_dirty();
        d.auto_apply(true);
        assert!(wait(&mut d));
        assert_eq!(first_burn(&file), Some(1000.0), "left half (gray 100) burns with S-MAX");
        let levels = burn_levels(&lines(&file));
        assert!(levels.contains(&1000) && levels.contains(&100), "{levels:?}");
        // inverted: brighter burns harder
        d.v.start = 200;
        d.v.stop = 100;
        d.mark_dirty();
        d.auto_apply(true);
        assert!(wait(&mut d));
        assert_eq!(first_burn(&file), Some(100.0), "left half (gray 100) burns with S-MIN");
        let levels = burn_levels(&lines(&file));
        assert!(levels.contains(&1000) && levels.contains(&100), "{levels:?}");
        // out of range: nothing burns at all
        d.v.start = 20;
        d.v.stop = 60;
        d.mark_dirty();
        d.auto_apply(true);
        assert!(wait(&mut d));
        assert!(burn_levels(&lines(&file)).is_empty());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn layout_error_blocks_apply_until_fixed() {
        let _g = settings_guard();
        let p = write_png("err", 40, 20, |_| 0);
        let file = new_file();
        let mut d = ImageDock::new();
        d.load_image(file.clone(), &p, false).unwrap();
        assert!(wait(&mut d));
        let good = lines(&file);
        // upper right left of lower left
        d.v.x1 = d.v.x0 - 1.0;
        d.mark_dirty();
        assert!(d.layout_error().is_some());
        d.auto_apply(true);
        assert!(!d.job_running(), "no job for an invalid layout");
        assert!(!d.apply());
        assert_eq!(d.last_error, d.layout_error());
        assert_eq!(d.last_error.as_deref(), Some(tr_plain("ImageDock.ErrorPlacement").as_str()));
        // program stays as it was, run must be refused
        assert!(d.needs_apply() && !d.is_dirty_idle() && d.is_blocked());
        assert_eq!(lines(&file), good);
        // zero height: same message; NaN: generic message
        d.v.x1 = d.v.x0 + 10.0;
        d.v.y1 = d.v.y0;
        assert_eq!(d.layout_error().as_deref(), Some(tr_plain("ImageDock.ErrorPlacement").as_str()));
        d.v.y1 = f32::NAN;
        assert_eq!(d.layout_error().as_deref(), Some(tr_plain("ImageDock.ErrorInvalid").as_str()));
        d.v.y1 = d.v.y0 + 10.0;
        d.v.quality = 0.0;
        assert_eq!(d.layout_error().as_deref(), Some(tr_plain("ImageDock.ErrorInvalid").as_str()));
        // fix: generates again
        d.v.quality = 3.0;
        d.mark_dirty();
        d.auto_apply(true);
        assert!(d.job_running());
        assert!(wait(&mut d));
        assert!(!d.is_blocked() && d.last_error.is_none());
        assert_ne!(lines(&file), good);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn default_size_follows_image_aspect_and_info_texts() {
        let _g = settings_guard();
        let p = write_png("fit", 60, 40, |_| 0);
        let mut d = ImageDock::new();
        d.load_image(new_file(), &p, false).unwrap();
        assert!(wait(&mut d));
        // 3:2 image, width 100: height 66.67
        assert_eq!((d.v.x1 - d.v.x0, d.v.y1 - d.v.y0), (100.0, 66.67));
        assert_eq!(d.map_info(16, 210), tr_fmt("ImageDock.MapNormal", &[&16, &210]));
        assert_eq!(d.map_info(210, 16), tr_fmt("ImageDock.MapInverted", &[&210, &16]));
        assert_eq!(d.map_info(7, 7), tr_fmt("ImageDock.MapSingle", &[&7]));
        d.support_pwm = false;
        assert_eq!(d.map_info(210, 16), tr_fmt("ImageDock.MapOnOff", &[&16, &210]));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn apply_persists_shared_keys_and_per_file_store_roundtrips() {
        let _g = settings_guard();
        let p = write_png("store", 40, 20, |_| 0);
        let mut d = ImageDock::new();
        d.load_image(new_file(), &p, false).unwrap();
        assert!(wait(&mut d));
        d.v = ImageUi { x0: 3.0, y0: 4.0, x1: 63.0, y1: 34.0, quality: 4.5, direction: "Vertical".into(), start: 20, stop: 200, mark_speed: 1500, min_power: 50, max_power: 800, laser_on: 1 };
        d.mark_dirty();
        d.auto_apply(true);
        assert!(wait(&mut d));
        let get = |k: &str| lg_base::settings::get(k, -1i32);
        assert_eq!(get("GrayScaleConversion.Gcode.Speed.Mark"), 1500);
        assert_eq!(get("GrayScaleConversion.Gcode.LaserOptions.PowerMin"), 50);
        assert_eq!(get("GrayScaleConversion.Gcode.LaserOptions.PowerMax"), 800);
        assert_eq!((get(MAP_START_KEY), get(MAP_STOP_KEY)), (20, 200));
        assert_eq!(lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.LaserOn", String::new()), "M4");
        assert_eq!(lg_base::settings::get("GrayScaleConversion.Line2LineOptions.Direction", String::new()), "Vertical");
        // the classic dialog reads quality as a decimal
        let q = decimal_to_f64(lg_base::settings::get::<Decimal>("GrayScaleConversion.Line2LineOptions.Quality", Decimal::ZERO));
        assert_eq!(q, 4.5);
        assert_eq!(lg_base::settings::get("GrayScaleConversion.Gcode.Offset.X", -1.0f32), 3.0);
        assert_eq!(lg_base::settings::get("GrayScaleConversion.Gcode.ImageSize.W", -1.0f32), 60.0);
        // same file in a fresh dock: everything restored
        let saved = d.v.clone();
        let mut d2 = ImageDock::new();
        d2.load_image(new_file(), &p, false).unwrap();
        assert!(wait(&mut d2));
        assert_eq!(d2.v, saved);
        // another file starts from the last used settings, not from the stored set
        let q2 = write_png("store2", 40, 20, |_| 0);
        let mut d3 = ImageDock::new();
        d3.load_image(new_file(), &q2, false).unwrap();
        assert!(wait(&mut d3));
        assert_eq!((d3.v.direction.as_str(), d3.v.start, d3.v.stop, d3.v.mark_speed), ("Vertical", 20, 200, 1500));
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(&q2);
    }

    #[test]
    fn store_dedups_by_path_newest_wins_and_caps() {
        let v = ImageUi { x0: 0.0, y0: 0.0, x1: 1.0, y1: 1.0, quality: 3.0, direction: "Horizontal".into(), start: 0, stop: 254, mark_speed: 1000, min_power: 0, max_power: 1000, laser_on: 0 };
        let set = |path: &str, t: u64| StoredSet { path: path.into(), saved_unix_secs: t, values: v.clone() };
        let mut s = Vec::new();
        merge_set(&mut s, set("a", 10));
        merge_set(&mut s, set("a", 20));
        merge_set(&mut s, set("a", 5));
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].saved_unix_secs, 20);
        for i in 0..60u64 {
            merge_set(&mut s, set(&format!("f{i}"), 100 + i));
        }
        assert_eq!(s.len(), MAX_SETS);
        assert!(!s.iter().any(|x| x.path == "a"));
        assert_eq!(s[0].path, "f59");
    }

    #[test]
    fn append_reapply_keeps_head_and_take_classic_restores_it() {
        let _g = settings_guard();
        let p = write_png("append", 40, 20, |_| 0);
        let file = new_file();
        for l in ["G0 X1", "G1 X2", "G1 X3"] {
            file.write().unwrap().commands_mut().push(lg_gcode::GrblCommand::new(l));
        }
        let mut d = ImageDock::new();
        d.load_image(file.clone(), &p, true).unwrap();
        assert!(wait(&mut d));
        let first = file.read().unwrap().count();
        assert!(first > 3);
        d.mark_dirty();
        d.auto_apply(true);
        assert!(wait(&mut d));
        assert_eq!(file.read().unwrap().count(), first, "re-apply must not duplicate");
        assert_eq!(lines(&file)[..3].join("|"), "G0 X1|G1 X2|G1 X3");
        // classic hand-over: busy job refuses, idle gives the image back and cuts the import
        d.mark_dirty();
        d.auto_apply(true);
        assert!(d.take_classic().is_none());
        assert!(wait(&mut d));
        let (f, path, append) = d.take_classic().expect("idle dock hands over");
        assert!(Arc::ptr_eq(&f, &file) && append);
        assert_eq!(path, p);
        assert!(!d.has_source() && !d.needs_apply());
        assert_eq!(file.read().unwrap().count(), 3);
        // nothing left to give
        assert!(d.take_classic().is_none());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn take_classic_without_append_keeps_the_program() {
        let _g = settings_guard();
        let p = write_png("plain", 40, 20, |_| 0);
        let file = new_file();
        let mut d = ImageDock::new();
        d.load_image(file.clone(), &p, false).unwrap();
        assert!(wait(&mut d));
        let n = file.read().unwrap().count();
        let (_, _, append) = d.take_classic().unwrap();
        assert!(!append);
        assert_eq!(file.read().unwrap().count(), n);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn clear_source_stops_regen() {
        let _g = settings_guard();
        let p = write_png("clear", 40, 20, |_| 0);
        let mut d = ImageDock::new();
        d.load_image(new_file(), &p, false).unwrap();
        assert!(wait(&mut d));
        d.mark_dirty();
        assert!(d.needs_apply() && d.is_dirty_idle());
        d.clear_source();
        assert!(!d.needs_apply() && !d.is_dirty_idle() && !d.is_blocked());
        assert!(!d.apply());
        let _ = std::fs::remove_file(&p);
    }

    /// Values with only the box set, the rest default. `x0`/`y0`/`x1`/`y1`: corners in mm. Returns the values.
    fn boxed(x0: f32, y0: f32, x1: f32, y1: f32) -> ImageUi {
        ImageUi { x0, y0, x1, y1, quality: 3.0, direction: "Horizontal".into(), start: 0, stop: 254, mark_speed: 1000, min_power: 0, max_power: 1000, laser_on: 0 }
    }

    #[test]
    fn aspect_lock_is_on_at_start_and_survives_load_and_clear() {
        let _g = settings_guard();
        let mut d = ImageDock::new();
        assert!(d.keep_aspect);
        // user switches it off: loading another image or dropping the source does not touch it
        d.keep_aspect = false;
        let p = write_png("lock", 40, 20, |_| 0);
        d.load_image(new_file(), &p, false).unwrap();
        assert!(wait(&mut d));
        assert!(!d.keep_aspect);
        d.clear_source();
        assert!(!d.keep_aspect);
        // a new dock (app start) is locked again, whatever was stored
        assert!(ImageDock::new().keep_aspect);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn move_keeps_size_and_size_edits_follow_the_lock() {
        // x0 / y0: image moves, size stays
        let mut v = boxed(0.0, 0.0, 100.0, 50.0);
        v.move_x(10.0);
        v.move_y(-5.5);
        assert_eq!((v.x0, v.y0, v.x1, v.y1), (10.0, -5.5, 110.0, 44.5));
        assert_eq!((v.width(), v.height()), (100.0, 50.0));
        // width, lock on: height follows the current 2:1 ratio
        v.set_width(60.0, true, 0.5);
        assert_eq!((v.x1, v.y1), (70.0, 24.5));
        // height, lock on: width follows
        v.set_height(10.0, true, 0.5);
        assert_eq!((v.x1, v.y1), (30.0, 4.5));
        // lock off: the other side stays
        v.set_width(100.0, false, 0.5);
        assert_eq!((v.x1, v.y1), (110.0, 4.5));
        v.set_height(33.33, false, 0.5);
        assert_eq!((v.x1, v.y1), (110.0, 27.83));
        // a ratio that is not 2:1 is kept as it is, not forced to the image's
        let mut v = boxed(0.0, 0.0, 30.0, 10.0);
        v.set_width(60.0, true, 0.5);
        assert_eq!((v.width(), v.height()), (60.0, 20.0));
    }

    #[test]
    fn aspect_lock_falls_back_to_image_ratio_without_a_usable_size() {
        // image 3:2 wide: height / width = 2/3
        let native = 2.0 / 3.0;
        for bad in [(0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (-4.0, -2.0), (f32::NAN, 5.0)] {
            let mut v = boxed(0.0, 0.0, 1.0, 1.0);
            (v.x1, v.y1) = bad;
            v.set_width(30.0, true, native);
            assert_eq!((v.width(), v.height()), (30.0, 20.0), "width from {bad:?}");
            let mut v = boxed(0.0, 0.0, 1.0, 1.0);
            (v.x1, v.y1) = bad;
            v.set_height(20.0, true, native);
            assert_eq!((v.width(), v.height()), (30.0, 20.0), "height from {bad:?}");
        }
    }

    /// Headless egui driver around one dock (same technique as the SVG dock's e2e tests).
    mod e2e {
        use super::*;
        use lg_gcode::Firmware;

        pub(super) struct Harness {
            pub ctx: egui::Context,
            pub core: Arc<GrblCore>,
            pub dock: ImageDock,
            /// Text shapes of the last frame: (text, screen rect).
            texts: Vec<(String, egui::Rect)>,
            /// Actions of the last frame.
            pub actions: Vec<ImageDockAction>,
        }

        /// Collect text shapes recursively. `s`: shape. `out`: sink.
        fn collect(s: &egui::Shape, out: &mut Vec<(String, egui::Rect)>) {
            match s {
                egui::Shape::Text(t) => out.push((t.galley.text().to_owned(), t.galley.rect.translate(t.pos.to_vec2()))),
                egui::Shape::Vec(v) => v.iter().for_each(|c| collect(c, out)),
                _ => {}
            }
        }

        impl Harness {
            /// Build core + dock. Returns the harness.
            pub fn new() -> Self {
                let (tx, _rx) = crossbeam_channel::unbounded();
                let core = GrblCore::new(Firmware::Grbl, tx);
                let mut h = Harness { ctx: egui::Context::default(), core, dock: ImageDock::new(), texts: Vec::new(), actions: Vec::new() };
                h.frame(vec![]);
                h
            }

            /// Run one frame. `events`: input events.
            pub fn frame(&mut self, events: Vec<egui::Event>) {
                let ri = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 1100.0))), events, ..Default::default() };
                let (dock, core) = (&mut self.dock, &self.core);
                let mut acts = Vec::new();
                let mut out = self.ctx.run_ui(ri, |ui| {
                    acts = dock.show(ui, core);
                });
                self.actions.extend(acts);
                out.textures_delta.clear();
                self.texts.clear();
                for cs in &out.shapes {
                    collect(&cs.shape, &mut self.texts);
                }
            }

            /// Click at `p` (press frame + release frame + settle frame).
            pub fn click_at(&mut self, p: egui::Pos2) {
                let btn = |pressed| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
                self.frame(vec![egui::Event::PointerMoved(p), btn(true)]);
                self.frame(vec![btn(false)]);
                self.frame(vec![]);
            }

            /// Two quiet frames: grids settle their column widths before the next rect is read.
            fn stabilize(&mut self) {
                self.frame(vec![]);
                self.frame(vec![]);
            }

            /// Click the last drawn text equal to `t`. Panics when absent.
            pub fn click_text(&mut self, t: &str) {
                self.stabilize();
                let r = self.texts.iter().rev().find(|(s, _)| s == t).map(|(_, r)| *r).unwrap_or_else(|| panic!("text {t:?} not on screen: {:?}", self.texts.iter().map(|x| &x.0).collect::<Vec<_>>()));
                self.click_at(r.center());
            }

            /// Type `v` into the numeric field `id` and press Enter.
            pub fn type_into(&mut self, id: &str, v: &str) {
                self.stabilize();
                let r = self.ctx.read_response(egui::Id::unique(id)).unwrap_or_else(|| panic!("field {id} not shown")).rect;
                self.click_at(r.center());
                let all = egui::Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND };
                self.frame(vec![all, egui::Event::Text(v.into())]);
                let enter = |p| egui::Event::Key { key: egui::Key::Enter, physical_key: None, pressed: p, repeat: false, modifiers: egui::Modifiers::NONE };
                self.frame(vec![enter(true), enter(false)]);
                self.frame(vec![]);
            }

            /// True when a text is on screen. `t`: text.
            pub fn shows(&self, t: &str) -> bool {
                self.texts.iter().any(|(s, _)| s.trim() == t.trim())
            }

            /// Returns how often a text is on screen. `t`: text.
            pub fn count(&self, t: &str) -> usize {
                self.texts.iter().filter(|(s, _)| s.trim() == t.trim()).count()
            }

            /// Click the aspect lock (right of the width input, one grid gap away).
            pub fn click_lock(&mut self) {
                self.stabilize();
                let w = self.ctx.read_response(egui::Id::unique("image-w")).expect("width input not shown").rect;
                self.click_at(egui::pos2(w.max.x + 6.0 + 8.5, w.center().y));
            }

            /// Wait until auto regen caught up with all edits. Panics when none was triggered or one failed.
            pub fn settle(&mut self) {
                assert!(self.dock.job.is_some() || self.dock.dirty, "edit did not trigger regen");
                let t = Instant::now();
                while !(self.dock.job.is_none() && !self.dock.dirty) {
                    if let Some(ok) = self.dock.poll() {
                        assert!(ok, "generation failed");
                    }
                    assert!(t.elapsed() < Duration::from_secs(10), "settle timeout");
                    std::thread::sleep(Duration::from_millis(5));
                    self.frame(vec![]);
                }
            }

            /// Whole program text lines.
            pub fn all_lines(&self) -> Vec<String> {
                lines(&self.core.loaded_file())
            }
        }
    }

    #[test]
    fn every_input_reaches_the_gcode() {
        let _g = settings_guard();
        let p = write_png("e2e", 40, 20, |_| 0);
        let mut h = e2e::Harness::new();
        h.dock.load_image(h.core.loaded_file(), &p, false).unwrap();
        assert!(wait(&mut h.dock));
        h.frame(vec![]);
        assert!(h.shows(&tr_plain("ImageDock.GroupMap")));
        // size row with the lock on, upper right calculated (x0 = 0: x1 equals w, so label + input = 2)
        assert!(h.shows(&tr_plain("ImageDock.SizeRow")) && h.dock.keep_aspect);
        assert_eq!((h.count("100.00"), h.count("50.00")), (2, 2));
        assert!(h.shows(&tr_fmt("ImageDock.Aspect", &[&"2.000", &"2.000"])));
        assert!(h.shows(&tr_fmt("ImageDock.RasterSize", &[&300, &150])));
        let base = h.all_lines();
        assert!((max_x(&base) - 100.0).abs() < 1e-3);

        // width typed, lock on: height follows the 2:1 ratio, upper right follows, regenerated
        h.type_into("image-w", "150");
        h.settle();
        assert_eq!((h.dock.v.x0, h.dock.v.y0, h.dock.v.x1, h.dock.v.y1), (0.0, 0.0, 150.0, 75.0));
        assert!((max_x(&h.all_lines()) - 150.0).abs() < 1e-3);
        assert_eq!((h.count("150.00"), h.count("75.00")), (2, 2));
        assert!(h.shows(&tr_fmt("ImageDock.Aspect", &[&"2.000", &"2.000"])));
        assert!(h.shows(&tr_fmt("ImageDock.RasterSize", &[&450, &225])));

        // lock click: opens, toggling alone does not regenerate
        h.click_lock();
        assert!(!h.dock.keep_aspect && !h.dock.needs_apply());
        // height typed, lock off: width stays
        h.type_into("image-h", "40");
        h.settle();
        assert_eq!((h.dock.v.x1, h.dock.v.y1), (150.0, 40.0));
        assert!(h.shows(&tr_fmt("ImageDock.Aspect", &[&"3.750", &"2.000"])));
        assert!(h.shows(&tr_fmt("ImageDock.RasterSize", &[&450, &120])));
        // lock on again: width typed, height follows the new 3.75 ratio
        h.click_lock();
        assert!(h.dock.keep_aspect && !h.dock.needs_apply());
        h.type_into("image-w", "75");
        h.settle();
        assert_eq!((h.dock.v.x1, h.dock.v.y1), (75.0, 20.0));
        h.type_into("image-h", "40");
        h.settle();
        assert_eq!((h.dock.v.x1, h.dock.v.y1), (150.0, 40.0));

        // x0 typed: upper right moves, size stays, first move goes to the new X
        h.type_into("image-x0", "10");
        h.settle();
        assert_eq!((h.dock.v.x0, h.dock.v.x1, h.dock.v.width(), h.dock.v.height()), (10.0, 160.0, 150.0, 40.0));
        assert!(h.shows("160.00") && h.shows("10.00"));
        let l = h.all_lines();
        assert!(l[0].starts_with("G0 X10 "), "{}", l[0]);
        assert!((max_x(&l) - 160.0).abs() < 1e-3);

        // quality, speed, S-MAX, stop (black stays in range)
        h.type_into("image-quality", "1");
        h.type_into("image-speed", "1800");
        h.type_into("image-smax", "400");
        h.type_into("image-stop", "100");
        h.settle();
        let l = h.all_lines();
        assert!(words(&l, 'F').contains(&1800.0), "speed: {:?}", &l[..3]);
        assert_eq!(burn_levels(&l), vec![400]);
        assert_eq!((h.dock.v.quality, h.dock.v.start, h.dock.v.stop), (1.0, 0, 100));
        assert!(h.shows(&tr_fmt("ImageDock.RasterSize", &[&150, &40])));
        // start above black: black is outside the range and no longer burns
        h.type_into("image-start", "10");
        h.settle();
        assert!(burn_levels(&h.all_lines()).is_empty());
        let l = h.all_lines();

        // layout error: red text, program stays, no generation (lock is on: height goes to 0 as well)
        h.type_into("image-w", "0");
        assert!(h.shows(&tr_plain("ImageDock.ErrorPlacement")));
        assert!(!h.dock.job_running() && h.dock.is_blocked());
        assert_eq!(h.all_lines(), l);

        // classic button
        h.click_text(&tr_plain("ImageDock.Classic"));
        assert_eq!(h.actions, vec![ImageDockAction::OpenClassic]);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn direction_combo_and_laser_mode_reach_the_gcode() {
        let _g = settings_guard();
        let p = write_png("combo", 20, 20, |_| 0);
        let mut h = e2e::Harness::new();
        h.dock.load_image(h.core.loaded_file(), &p, false).unwrap();
        assert!(wait(&mut h.dock));
        h.frame(vec![]);
        let horizontal = h.all_lines();
        // combo texts are cut at "(" like in the classic dialog
        let text = |d| crate::ui::widgets::combo::enum_item_text(&translate_enum(d, "Direction"), true);
        h.click_text(&text(Direction::Horizontal));
        h.click_text(&text(Direction::Vertical));
        assert_eq!(h.dock.v.direction, "Vertical");
        h.settle();
        assert_ne!(h.all_lines(), horizontal);
        h.click_text("M3 - Constant Power");
        h.click_text("M4 - Dynamic Power");
        h.settle();
        assert!(h.all_lines().iter().any(|l| l.starts_with("M4")), "{:?}", &h.all_lines()[..3]);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn without_hardware_pwm_no_power_inputs() {
        let _g = settings_guard();
        lg_base::settings::set("Support Hardware PWM", false);
        let p = write_png("nopwm", 20, 20, |_| 0);
        let mut h = e2e::Harness::new();
        h.dock.load_image(h.core.loaded_file(), &p, false).unwrap();
        assert!(wait(&mut h.dock));
        h.frame(vec![]);
        assert!(h.ctx.read_response(egui::Id::unique("image-smax")).is_none());
        assert!(h.ctx.read_response(egui::Id::unique("image-smin")).is_none());
        assert!(h.ctx.read_response(egui::Id::unique("image-start")).is_some());
        assert!(h.shows(&tr_fmt("ImageDock.MapOnOff", &[&0, &254])));
        let l = h.all_lines();
        assert!(l.iter().any(|x| x.starts_with("G1")));
        let _ = std::fs::remove_file(&p);
    }
}
