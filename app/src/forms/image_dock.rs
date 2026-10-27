// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Andreas Pösch

//! Raster-image dock editor for one image (right side of the main window). Replaces the modal `RasterToLaserForm` for plain
//! line-to-line engraving: the image loads at once, a histogram with a brightness to power map, the target
//! position and size, line settings and laser options are edited here, and every committed edit regenerates the G-code
//! of this image's block in the background. The classic dialog stays reachable through a button.
//! One instance edits one image. It owns no program and no worker: the job coordinator (`job_dock.rs`) takes its
//! [`GenRequest`], runs it and hands the result back.
//! Not part of the C# app (intentional deviation).

use eframe::egui::{self, Grid, Id, RichText, Ui, Vec2};
use image::RgbaImage;
use lg_dotnet::{decimal_to_f64, f64_to_decimal, fmt_double_custom, fmt_float_custom, Decimal};
use lg_gcode::{Direction, Firmware, PowerMap};
use lg_services::raster_dock::{self, RasterParams};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

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
}

/// What the dock asks the main window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDockAction {
    /// Hand the image to the classic raster import dialog (the coordinator cuts its block out first).
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

/// Image the editor regenerates G-code from.
struct ImageSource {
    /// Image path.
    path: String,
    /// Opaque gray version of the image (the one brightness definition of histogram and G-code).
    gray: Arc<RgbaImage>,
    /// Brightness histogram of `gray`.
    hist: Box<[u32; 256]>,
    /// Image size in px.
    img_w: u32,
    img_h: u32,
}

/// Raster-image editor state (one image).
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
    /// `edit_seq` of the last request taken (None = no attempt yet); one automatic attempt per edit.
    auto_seq: Option<u64>,
    /// `edit_seq` of the request that runs now.
    inflight: Option<u64>,
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
            auto_seq: None,
            inflight: None,
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
    #[cfg(test)]
    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }

    /// Load an image into this editor; the editor is dirty afterwards, so the coordinator generates at once.
    ///
    /// `path`: image file. Values come from the per-file store, else from the settings. Returns Err with the
    /// read / decode error text (nothing changed then).
    pub fn load_image(&mut self, path: &Path) -> Result<(), String> {
        let (img, _dpi) = raster_dock::load_image(path)?;
        let gray = raster_dock::gray_image(&img);
        let hist = raster_dock::histogram(&gray);
        let (img_w, img_h) = (gray.width(), gray.height());
        self.refresh_settings();
        let path_s = path.to_string_lossy().into_owned();
        // stored values of this file win over the last used ones
        let stored = load_sets().into_iter().find(|s| s.path == path_s).map(|s| s.values.sanitized(self.max_quality, &self.line_dirs));
        self.v = stored.unwrap_or_else(|| settings_values(&self.limits, self.max_quality, img_w, img_h));
        self.source = Some(ImageSource { path: path_s, gray: Arc::new(gray), hist: Box::new(hist), img_w, img_h });
        self.dirty = true;
        self.last_error = None;
        Ok(())
    }

    /// Test access: set the box as an edit would.
    ///
    /// `x0`/`y0`: lower left in mm. `x1`/`y1`: upper right in mm.
    #[cfg(test)]
    pub(crate) fn test_set_box(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        (self.v.x0, self.v.y0, self.v.x1, self.v.y1) = (x0, y0, x1, y1);
        self.mark_dirty();
    }

    /// Generator parameters of the current values. Returns the parameter set.
    pub(crate) fn params(&self) -> RasterParams {
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

    /// Take the work order for the latest edit: persist the values (settings shared with the classic dialog, map,
    /// per-file store) and hand out the producer of this image's block. The editor counts as generating until
    /// [`ImageDock::complete`] (or [`ImageDock::cancel_request`]).
    ///
    /// Returns None without an image, while a request is out, when nothing is dirty, or for an invalid layout
    /// (the red text says why).
    pub fn take_request(&mut self) -> Option<GenRequest> {
        if self.inflight.is_some() || self.source.is_none() || !self.dirty {
            return None;
        }
        // machine config may have changed while another file was edited
        self.refresh_settings();
        // invalid placement: no request, the red text says why
        if let Some(e) = self.layout_error() {
            self.last_error = Some(e);
            return None;
        }
        self.persist();
        let params = self.params();
        let src = self.source.as_ref()?;
        let (path, gray) = (src.path.clone(), src.gray.clone());
        let name = file_name(&path);
        let seq = self.edit_seq;
        self.inflight = Some(seq);
        self.auto_seq = Some(seq);
        self.last_error = None;
        // always appends: the coordinator cuts the old block out first
        let produce: ProduceFn = Box::new(move |f| {
            raster_dock::generate(f, &gray, &path, &params, true)?;
            Ok(Vec::new())
        });
        Some(GenRequest { seq, name, produce })
    }

    /// Give a taken request back unrun (the program was busy): the editor stays dirty and may be asked again.
    ///
    /// `seq`: the request's `seq`.
    pub fn cancel_request(&mut self, seq: u64) {
        if self.inflight == Some(seq) {
            self.inflight = None;
            self.auto_seq = None;
        }
    }

    /// Result of a request taken with [`ImageDock::take_request`].
    ///
    /// `seq`: the request's `seq`. `result`: Ok when the block was generated, else the error text.
    /// Edits made while the request ran keep the editor dirty. A failed generation is not retried by
    /// [`ImageDock::wants_auto`] until the next edit.
    pub fn complete(&mut self, seq: u64, result: Result<(), String>) {
        self.inflight = None;
        match result {
            Ok(()) => {
                self.dirty = self.edit_seq != seq;
                self.last_error = None;
            }
            Err(e) => self.last_error = Some(e),
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
        self.source.is_some() && (self.dirty || self.inflight.is_some())
    }

    /// Returns true when edits wait, no request is out and the values can be generated ([`ImageDock::take_request`] would yield one).
    pub fn is_dirty_idle(&self) -> bool {
        self.source.is_some() && self.dirty && self.inflight.is_none() && self.layout_error().is_none()
    }

    /// Returns true when the coordinator should start a request by itself: [`ImageDock::is_dirty_idle`] and no
    /// attempt yet for the latest edit (a failed generation waits for the next edit or for Run).
    pub fn wants_auto(&self) -> bool {
        self.is_dirty_idle() && self.auto_seq != Some(self.edit_seq)
    }

    /// Returns true when the program is stale and cannot be regenerated because the values are invalid.
    pub fn is_blocked(&self) -> bool {
        self.source.is_some() && self.dirty && self.inflight.is_none() && self.layout_error().is_some()
    }

    /// Returns true while a request of this editor runs.
    pub fn in_flight(&self) -> bool {
        self.inflight.is_some()
    }

    /// Returns the text of what is wrong with this image (failed generation, invalid size), None when fine.
    pub fn problem(&self) -> Option<String> {
        self.last_error.clone().or_else(|| self.source.as_ref().and_then(|_| self.layout_error()))
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

    /// Editor body (title and scrolling belong to the host).
    ///
    /// `ui`: parent ui. Returns requests for the main window. Edits mark the editor dirty; the coordinator
    /// starts the regeneration.
    pub fn show(&mut self, ui: &mut Ui) -> Vec<ImageDockAction> {
        let mut actions = Vec::new();
        self.refresh_settings();
        let mut edited = false;

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
        let classic = ui.add_enabled(self.inflight.is_none(), egui::Button::new(tr_plain("ImageDock.Classic")).min_size(egui::vec2(ui.available_width(), 22.0)));
        if classic.on_hover_text(tr_plain("ImageDock.ClassicTip")).clicked() {
            actions.push(ImageDockAction::OpenClassic);
        }
        ui.add_space(8.0);
        self.status_row(ui);
        if edited {
            self.mark_dirty();
        }
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
                if self.inflight.is_some() { format!("{size} - {}", tr_plain("ParamsDock.Generating")) } else { size }
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
    use crate::forms::job_test_harness::{Harness, Rig};
    use std::path::PathBuf;

    /// PNG in a scratch dir: `w` x `h` px, gray value per column from `col`. Returns the path.
    fn write_png(tag: &str, w: u32, h: u32, col: impl Fn(u32) -> u8) -> PathBuf {
        let p = lg_base::paths::test_dir(&format!("image_dock_{tag}")).join("img.png");
        let img = RgbaImage::from_fn(w, h, |x, _| {
            let g = col(x);
            image::Rgba([g, g, g, 255])
        });
        img.save(&p).unwrap();
        p
    }

    /// Take the editor's request, run it on the rig and hand the result back. Returns None without a request, else success.
    fn pump(rig: &mut Rig, d: &mut ImageDock) -> Option<bool> {
        let req = d.take_request()?;
        let (seq, r) = rig.run(req);
        let ok = r.is_ok();
        d.complete(seq, r.map(|_| ()));
        Some(ok)
    }

    /// Editor for the image at `p`, loaded. Panics when the image cannot be read.
    fn image_dock(p: &Path) -> ImageDock {
        let mut d = ImageDock::new();
        d.load_image(p).unwrap();
        d
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
        let mut rig = Rig::new(&[]);
        let mut d = image_dock(&p);
        // loaded: dirty, the coordinator asks for the first generation
        assert!(d.has_source() && d.needs_apply() && d.wants_auto());
        let req = d.take_request().expect("first request");
        assert!(d.in_flight() && req.name == "img.png");
        let (seq, r) = rig.run(req);
        assert!(r.as_ref().is_ok_and(|v| v.is_empty()), "images have no layers");
        d.complete(seq, r.map(|_| ()));
        assert!(!d.dirty && !d.needs_apply());
        // 2:1 image, 20 mm wide: upper right at (25, 17)
        assert_eq!((d.v.x1, d.v.y1), (25.0, 17.0));
        let l = rig.lines();
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
        assert!(d.load_image(Path::new("/nonexistent/none.png")).is_err());
        assert!(!d.has_source());
        let p = lg_base::paths::test_dir("image_dock_bad").join("bad.png");
        std::fs::write(&p, b"not an image").unwrap();
        assert!(d.load_image(&p).is_err());
        assert!(!d.has_source());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn edit_x1_regenerates_with_larger_max_x() {
        let _g = settings_guard();
        let p = write_png("x1", 40, 20, |_| 0);
        let mut rig = Rig::new(&[]);
        let mut d = image_dock(&p);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        let before = max_x(&rig.lines());
        assert!((before - 100.0).abs() < 1e-3, "{before}");
        d.v.x1 = 150.0;
        d.mark_dirty();
        // program busy: the coordinator hands the request back, the attempt is not consumed
        let req = d.take_request().unwrap();
        d.cancel_request(req.seq);
        assert!(!d.in_flight() && d.wants_auto());
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert!((max_x(&rig.lines()) - 150.0).abs() < 1e-3);
        // no new edit: nothing to do
        assert!(!d.wants_auto() && pump(&mut rig, &mut d).is_none());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn edit_during_request_keeps_dirty_and_failure_waits_for_an_edit() {
        let _g = settings_guard();
        let p = write_png("editjob", 40, 20, |_| 0);
        let mut rig = Rig::new(&[]);
        let mut d = image_dock(&p);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        d.mark_dirty();
        let req = d.take_request().unwrap();
        assert!(d.take_request().is_none(), "one request at a time");
        d.mark_dirty();
        let (seq, r) = rig.run(req);
        d.complete(seq, r.map(|_| ()));
        assert!(d.dirty && d.wants_auto());
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert!(!d.dirty);
        // a failed generation is not retried without a new edit
        d.mark_dirty();
        let req = d.take_request().unwrap();
        d.complete(req.seq, Err("boom".into()));
        assert!(d.is_dirty_idle() && !d.wants_auto() && d.problem().as_deref() == Some("boom"));
        d.mark_dirty();
        assert!(d.wants_auto());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn inverted_mapping_flips_which_pixels_burn_hard() {
        let _g = settings_guard();
        // left half gray 100, right half gray 200
        let p = write_png("inv", 40, 20, |x| if x < 20 { 100 } else { 200 });
        let mut rig = Rig::new(&[]);
        let mut d = image_dock(&p);
        d.v.min_power = 100;
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        // first burning value of the program (the left half, rows start there)
        let first_burn = |rig: &Rig| words(&rig.lines(), 'S').into_iter().find(|s| *s > 0.0);
        // normal: darker burns harder, both halves are inside [100, 200]
        d.v.start = 100;
        d.v.stop = 200;
        d.mark_dirty();
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert_eq!(first_burn(&rig), Some(1000.0), "left half (gray 100) burns with S-MAX");
        let levels = burn_levels(&rig.lines());
        assert!(levels.contains(&1000) && levels.contains(&100), "{levels:?}");
        // inverted: brighter burns harder
        d.v.start = 200;
        d.v.stop = 100;
        d.mark_dirty();
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert_eq!(first_burn(&rig), Some(100.0), "left half (gray 100) burns with S-MIN");
        let levels = burn_levels(&rig.lines());
        assert!(levels.contains(&1000) && levels.contains(&100), "{levels:?}");
        // out of range: nothing burns at all
        d.v.start = 20;
        d.v.stop = 60;
        d.mark_dirty();
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert!(burn_levels(&rig.lines()).is_empty());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn layout_error_blocks_request_until_fixed() {
        let _g = settings_guard();
        let p = write_png("err", 40, 20, |_| 0);
        let mut rig = Rig::new(&[]);
        let mut d = image_dock(&p);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        let good = rig.lines();
        // upper right left of lower left
        d.v.x1 = d.v.x0 - 1.0;
        d.mark_dirty();
        assert!(d.layout_error().is_some());
        assert!(!d.wants_auto(), "no automatic attempt for an invalid layout");
        assert!(d.take_request().is_none(), "no request for an invalid layout");
        assert_eq!(d.last_error, d.layout_error());
        assert_eq!(d.last_error.as_deref(), Some(tr_plain("ImageDock.ErrorPlacement").as_str()));
        // program stays as it was, run must be refused
        assert!(d.needs_apply() && !d.is_dirty_idle() && d.is_blocked());
        assert_eq!(rig.lines(), good);
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
        assert!(d.wants_auto());
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert!(!d.is_blocked() && d.last_error.is_none());
        assert_ne!(rig.lines(), good);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn default_size_follows_image_aspect_and_info_texts() {
        let _g = settings_guard();
        let p = write_png("fit", 60, 40, |_| 0);
        let mut d = image_dock(&p);
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
    fn request_persists_shared_keys_and_per_file_store_roundtrips() {
        let _g = settings_guard();
        let p = write_png("store", 40, 20, |_| 0);
        let mut rig = Rig::new(&[]);
        let mut d = image_dock(&p);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        d.v = ImageUi { x0: 3.0, y0: 4.0, x1: 63.0, y1: 34.0, quality: 4.5, direction: "Vertical".into(), start: 20, stop: 200, mark_speed: 1500, min_power: 50, max_power: 800, laser_on: 1 };
        d.mark_dirty();
        assert_eq!(pump(&mut rig, &mut d), Some(true));
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
        // same file in a fresh editor: everything restored
        let saved = d.v.clone();
        let d2 = image_dock(&p);
        assert_eq!(d2.v, saved);
        // another file starts from the last used settings, not from the stored set
        let q2 = write_png("store2", 40, 20, |_| 0);
        let d3 = image_dock(&q2);
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
    fn regeneration_keeps_the_head_and_does_not_duplicate() {
        let _g = settings_guard();
        let p = write_png("append", 40, 20, |_| 0);
        let mut rig = Rig::new(&["G0 X1", "G1 X2", "G1 X3"]);
        let mut d = image_dock(&p);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        let first = rig.count();
        assert!(first > 3);
        d.mark_dirty();
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert_eq!(rig.count(), first, "re-apply must not duplicate");
        assert_eq!(rig.lines()[..3].join("|"), "G0 X1|G1 X2|G1 X3");
        assert_eq!(rig.block, 3..first);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn request_without_source_does_nothing() {
        let _g = settings_guard();
        let mut d = ImageDock::new();
        assert!(d.take_request().is_none());
        assert!(!d.needs_apply() && !d.is_dirty_idle() && !d.is_blocked() && !d.wants_auto() && d.problem().is_none());
    }

    /// Values with only the box set, the rest default. `x0`/`y0`/`x1`/`y1`: corners in mm. Returns the values.
    fn boxed(x0: f32, y0: f32, x1: f32, y1: f32) -> ImageUi {
        ImageUi { x0, y0, x1, y1, quality: 3.0, direction: "Horizontal".into(), start: 0, stop: 254, mark_speed: 1000, min_power: 0, max_power: 1000, laser_on: 0 }
    }

    #[test]
    fn aspect_lock_is_on_for_every_editor_and_survives_load() {
        let _g = settings_guard();
        let mut d = ImageDock::new();
        assert!(d.keep_aspect);
        // user switches it off: loading an image does not touch it
        d.keep_aspect = false;
        let p = write_png("lock", 40, 20, |_| 0);
        d.load_image(&p).unwrap();
        assert!(!d.keep_aspect);
        // a new editor (every image has its own) is locked again, whatever was stored
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

    /// Open one image in a headless job dock and generate it. `h`: harness. `p`: image file.
    fn open_in_dock(h: &mut Harness, p: &Path) {
        h.job.add_image(p, false).unwrap();
        assert!(h.wait());
        h.frame(vec![]);
    }

    #[test]
    fn every_input_reaches_the_gcode() {
        let _g = settings_guard();
        let p = write_png("e2e", 40, 20, |_| 0);
        let mut h = Harness::new();
        open_in_dock(&mut h, &p);
        assert!(h.shows(&tr_plain("ImageDock.GroupMap")));
        // size row with the lock on, upper right calculated (x0 = 0: x1 equals w, so label + input = 2)
        assert!(h.shows(&tr_plain("ImageDock.SizeRow")) && h.image().keep_aspect);
        assert_eq!((h.count("100.00"), h.count("50.00")), (2, 2));
        assert!(h.shows(&tr_fmt("ImageDock.Aspect", &[&"2.000", &"2.000"])));
        assert!(h.shows(&tr_fmt("ImageDock.RasterSize", &[&300, &150])));
        let base = h.all_lines();
        assert!((max_x(&base) - 100.0).abs() < 1e-3);

        // width typed, lock on: height follows the 2:1 ratio, upper right follows, regenerated
        h.type_into("image-w", "150");
        h.settle();
        assert_eq!((h.image().v.x0, h.image().v.y0, h.image().v.x1, h.image().v.y1), (0.0, 0.0, 150.0, 75.0));
        assert!((max_x(&h.all_lines()) - 150.0).abs() < 1e-3);
        assert_eq!((h.count("150.00"), h.count("75.00")), (2, 2));
        assert!(h.shows(&tr_fmt("ImageDock.Aspect", &[&"2.000", &"2.000"])));
        assert!(h.shows(&tr_fmt("ImageDock.RasterSize", &[&450, &225])));

        // lock click: opens, toggling alone does not regenerate
        h.click_lock();
        assert!(!h.image().keep_aspect && !h.job.needs_apply());
        // height typed, lock off: width stays
        h.type_into("image-h", "40");
        h.settle();
        assert_eq!((h.image().v.x1, h.image().v.y1), (150.0, 40.0));
        assert!(h.shows(&tr_fmt("ImageDock.Aspect", &[&"3.750", &"2.000"])));
        assert!(h.shows(&tr_fmt("ImageDock.RasterSize", &[&450, &120])));
        // lock on again: width typed, height follows the new 3.75 ratio
        h.click_lock();
        assert!(h.image().keep_aspect && !h.job.needs_apply());
        h.type_into("image-w", "75");
        h.settle();
        assert_eq!((h.image().v.x1, h.image().v.y1), (75.0, 20.0));
        h.type_into("image-h", "40");
        h.settle();
        assert_eq!((h.image().v.x1, h.image().v.y1), (150.0, 40.0));

        // x0 typed: upper right moves, size stays, first move goes to the new X
        h.type_into("image-x0", "10");
        h.settle();
        let v = &h.image().v;
        assert_eq!((v.x0, v.x1, v.width(), v.height()), (10.0, 160.0, 150.0, 40.0));
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
        assert_eq!((h.image().v.quality, h.image().v.start, h.image().v.stop), (1.0, 0, 100));
        assert!(h.shows(&tr_fmt("ImageDock.RasterSize", &[&150, &40])));
        // start above black: black is outside the range and no longer burns
        h.type_into("image-start", "10");
        h.settle();
        assert!(burn_levels(&h.all_lines()).is_empty());
        let l = h.all_lines();

        // layout error: red text, program stays, no generation (lock is on: height goes to 0 as well)
        h.type_into("image-w", "0");
        assert!(h.shows(&tr_plain("ImageDock.ErrorPlacement")));
        assert!(!h.job.job_running() && h.job.is_blocked());
        assert_eq!(h.all_lines(), l);

        // classic button
        h.click_text(&tr_plain("ImageDock.Classic"));
        let id = h.job.selected_id().unwrap();
        assert_eq!(h.actions, vec![crate::forms::job_dock::JobAction::OpenClassic { id }]);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn direction_combo_and_laser_mode_reach_the_gcode() {
        let _g = settings_guard();
        let p = write_png("combo", 20, 20, |_| 0);
        let mut h = Harness::new();
        open_in_dock(&mut h, &p);
        let horizontal = h.all_lines();
        // combo texts are cut at "(" like in the classic dialog
        let text = |d| crate::ui::widgets::combo::enum_item_text(&translate_enum(d, "Direction"), true);
        h.click_text(&text(Direction::Horizontal));
        h.click_text(&text(Direction::Vertical));
        assert_eq!(h.image().v.direction, "Vertical");
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
        let mut h = Harness::new();
        open_in_dock(&mut h, &p);
        assert!(h.ctx.read_response(egui::Id::unique("image-smax")).is_none());
        assert!(h.ctx.read_response(egui::Id::unique("image-smin")).is_none());
        assert!(h.ctx.read_response(egui::Id::unique("image-start")).is_some());
        assert!(h.shows(&tr_fmt("ImageDock.MapOnOff", &[&0, &254])));
        let l = h.all_lines();
        assert!(l.iter().any(|x| x.starts_with("G1")));
        let _ = std::fs::remove_file(&p);
    }
}
