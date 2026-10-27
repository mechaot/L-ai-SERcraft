// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! "Parameters" dock editor for one SVG file (right side of the main window). Replaces the modal `SvgToGCodeForm`
//! (`SvgConverter/ConvertSizeAndOptionForm.cs`): SVG is loaded at once, power/speed are tweaked
//! here and every committed edit regenerates the G-code of this file's block in the background. Settings are kept per SVG layer (top-level `<g>`).
//! One instance edits one file; without a file it is the "(defaults)" editor. It owns no program and no worker:
//! the job coordinator (`job_dock.rs`) takes its [`GenRequest`], runs it and hands the result back.
//! Not part of the C# app (intentional deviation).

use eframe::egui::{self, Grid, RichText, Ui};
use lg_dotnet::Decimal;
use lg_services::layer_presets::LayerPreset;
use lg_svg2gcode::{ColorFilter, HatchFill, LayerInfo, LayerParams};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ops::Range;

use crate::forms::form_util::*;
use crate::forms::material::{default_presets, SharedPresetDb};
use crate::ui::dialog::{MessageBox, MessageBoxHost, MessageBoxIcon};
use crate::ui::widgets::{flat_combo, grbl_group_box_wide, DecimalInputRanged, ImageButton};
use crate::ui::{tr, tr_fmt, tr_plain};

/// i18n prefix of the old SVG form (captions and tooltips are reused).
const P: &str = "SvgConverter.ConvertSizeAndOptionForm.";

/// Setting that stores the selected preset name.
const PRESET_KEY: &str = "ParametersDock.Preset";

/// Setting that stores the per-file layer settings (JSON array of [`StoredSet`]).
const SETS_KEY: &str = "ParametersDock.LayerSets";

/// Most files kept in the per-file store.
const MAX_SETS: usize = 50;

/// Layer key of the pseudo-layer used without SVG.
const KEY_DEFAULTS: &str = "#defaults";

/// Layer key of the single layer used when the SVG cannot be listed.
const KEY_ALL: &str = "#all";

/// C# `FilterOptions`: `Enum.GetValues(typeof(ColorFilter))` in declaration order.
pub const FILTER_OPTIONS: [ColorFilter; 5] = [ColorFilter::All, ColorFilter::Red, ColorFilter::Green, ColorFilter::Blue, ColorFilter::Black];

/// Text of a filter item (`ColorFilter.ToString()`).
///
/// `f`: filter. Returns the enum member name.
pub fn filter_name(f: ColorFilter) -> &'static str {
    match f {
        ColorFilter::All => "All",
        ColorFilter::Red => "Red",
        ColorFilter::Green => "Green",
        ColorFilter::Blue => "Blue",
        ColorFilter::Black => "Black",
    }
}

/// Message box tags.
#[derive(Clone, Copy)]
enum Msg {
    WrongLaserMode,
}

/// What the dock asks the main window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamsAction {
    /// Open the layer preset list editor.
    OpenPresetEditor,
}

/// Editable values of one layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct LayerUi {
    /// Layer is converted (and shown) when true.
    enabled: bool,
    /// Repeat count, 1..=100.
    passes: i32,
    border_speed: i32,
    /// S value while burning (PWM mode). No S-MIN: vector paths burn at constant power.
    max_power: i32,
    /// Index into [`LASER_OPTIONS`].
    laser_on: usize,
    /// Index into [`FILTER_OPTIONS`].
    filter: usize,
    /// Name of the selected preset (old stored sets with `material_id` still load).
    #[serde(default)]
    preset: Option<String>,
    /// Hatch fill of closed shapes on.
    #[serde(default)]
    fill: bool,
    /// Fill line distance in mm.
    #[serde(default = "default_fill_distance")]
    fill_distance: f32,
    /// Fill angle in degrees, 0..=360.
    #[serde(default)]
    fill_angle: i32,
    /// Second hatch rotated +90 degrees.
    #[serde(default)]
    fill_cross: bool,
    /// Finish each connected fill area before the next.
    #[serde(default = "default_true")]
    fill_per_shape: bool,
}

/// Serde default for flags that are on unless stored otherwise. Returns true.
fn default_true() -> bool {
    true
}

/// Serde default of [`LayerUi::fill_distance`] (mm).
fn default_fill_distance() -> f32 {
    0.2
}

/// Saved layer values of one SVG file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct StoredSet {
    path: String,
    saved_unix_secs: u64,
    /// Layer key to values.
    layers: BTreeMap<String, LayerUi>,
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

/// SVG the editor regenerates G-code from.
struct SvgSource {
    /// SVG path.
    path: String,
    /// Layers could be listed: use the layered converter.
    layered: bool,
}

/// Command ranges per layer (relative to the start of this file's block).
type Ranges = LayerRanges;

/// Overlay data of one SVG for the preview; ranges are relative to the start of the file's block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SvgView {
    /// Ranges of layers the user disabled (not drawn).
    pub hidden: Vec<Range<u32>>,
    /// Range of the selected layer, only set when the SVG has more than one layer and it is enabled.
    pub highlight: Option<Range<u32>>,
}

/// Parameters editor state (one SVG file, or the defaults without a file).
pub struct ParamsDock {
    limits: Limits,
    support_pwm: bool,
    /// Layers and their values; one pseudo-layer without SVG.
    layers: Vec<(LayerInfo, LayerUi)>,
    /// Index of the edited layer.
    selected: usize,
    /// Upper limit of the speed input.
    border_max: i32,
    /// Upper limit of the S-MAX input (also the 100 % base of preset power).
    power_max_limit: i32,
    /// Shared preset database.
    presets_db: SharedPresetDb,
    /// Snapshot of the presets, sorted for the dropdown.
    presets: Vec<LayerPreset>,
    /// Edit buffer of the preset name field.
    preset_name: String,
    source: Option<SvgSource>,
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
    /// Command ranges of the layers in this file's block (from the last good generation).
    generated: Ranges,
    /// Bumped on every change the preview overlay depends on.
    revision: u64,
}

impl ParamsDock {
    /// Build the dock on the process wide preset DB.
    ///
    /// Returns the dock without a source.
    pub fn new() -> Self {
        Self::with_presets(default_presets())
    }

    /// Build the dock: values from settings, preset list, restored preset name.
    ///
    /// `db`: preset database. Returns the dock without a source.
    pub fn with_presets(db: SharedPresetDb) -> Self {
        let mut d = ParamsDock {
            limits: Limits::from_config(),
            support_pwm: true,
            layers: Vec::new(),
            selected: 0,
            border_max: 0,
            power_max_limit: 0,
            presets_db: db,
            presets: Vec::new(),
            preset_name: String::new(),
            source: None,
            dirty: false,
            edit_seq: 0,
            auto_seq: None,
            inflight: None,
            last_error: None,
            msgs: MessageBoxHost::default(),
            generated: Vec::new(),
            revision: 0,
        };
        d.load_values();
        d.refresh_presets();
        // restore selection by name, values stay as stored
        let name: Option<String> = lg_base::settings::get(PRESET_KEY, None);
        let found = name.and_then(|n| d.presets.iter().find(|p| p.name.eq_ignore_ascii_case(&n)).map(|p| p.name.clone()));
        if let Some(n) = &found {
            d.preset_name = n.clone();
        }
        d.cur_mut().preset = found;
        d
    }

    /// Values of the last-used settings (same keys/defaults as the old dialog).
    ///
    /// Returns a layer (enabled, one pass) built from the settings; refreshes limits as a side effect.
    fn settings_layer(&mut self) -> LayerUi {
        self.limits = Limits::from_config();
        self.support_pwm = lg_base::settings::get("Support Hardware PWM", true);
        let laser_on_s: String = lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.LaserOn", "M3".to_owned());
        self.border_max = self.limits.max_rate_x;
        self.power_max_limit = self.limits.max_pwm;
        LayerUi {
            enabled: true,
            passes: 1,
            border_speed: lg_base::settings::get("GrayScaleConversion.VectorizeOptions.BorderSpeed", 1000),
            max_power: lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.PowerMax", self.limits.max_pwm),
            laser_on: if laser_on_s == "M3" || !self.limits.laser_mode { 0 } else { 1 },
            filter: 0,
            preset: None,
            fill: false,
            fill_distance: default_fill_distance(),
            fill_angle: 0,
            fill_cross: false,
            fill_per_shape: true,
        }
    }

    /// Reset to a single "(defaults)" layer holding the persisted values.
    fn load_values(&mut self) {
        let l = self.settings_layer();
        self.layers = vec![(LayerInfo { key: KEY_DEFAULTS.into(), label: "(defaults)".into(), shapes: 0 }, l)];
        self.selected = 0;
    }

    /// Edited layer. Returns its values.
    fn cur(&self) -> &LayerUi {
        &self.layers[self.selected.min(self.layers.len() - 1)].1
    }

    /// Edited layer, mutable. Returns its values.
    fn cur_mut(&mut self) -> &mut LayerUi {
        let i = self.selected.min(self.layers.len() - 1);
        &mut self.layers[i].1
    }

    /// Editor for a new SVG file. The values seed like the old dock did: per-file store, else the edited layer
    /// of `seed_from` (or the last-used settings).
    ///
    /// `seed_from`: editor whose limits, presets and current layer values are copied (the defaults editor, or the
    /// editor of the SVG selected so far). `path`: SVG path. `from_settings`: take the values from the settings
    /// (project restore stored them just before) and ignore the per-file store.
    /// Returns an editor that is dirty, so the coordinator generates it at once.
    pub fn for_svg(seed_from: &ParamsDock, path: &str, from_settings: bool) -> ParamsDock {
        let mut d = ParamsDock {
            limits: seed_from.limits,
            support_pwm: seed_from.support_pwm,
            layers: vec![(LayerInfo { key: KEY_DEFAULTS.into(), label: "(defaults)".into(), shapes: 0 }, seed_from.cur().clone())],
            selected: 0,
            border_max: seed_from.border_max,
            power_max_limit: seed_from.power_max_limit,
            presets_db: seed_from.presets_db.clone(),
            presets: seed_from.presets.clone(),
            preset_name: seed_from.preset_name.clone(),
            source: None,
            dirty: false,
            edit_seq: 0,
            auto_seq: None,
            inflight: None,
            last_error: None,
            msgs: MessageBoxHost::default(),
            generated: Vec::new(),
            revision: 0,
        };
        d.set_svg(path, from_settings);
        d
    }

    /// List the layers of `path`, seed their values and mark the editor dirty.
    ///
    /// `path`: SVG path. `from_settings`: see [`ParamsDock::for_svg`].
    fn set_svg(&mut self, path: &str, from_settings: bool) {
        // seed values for layers without stored settings
        let seed = if from_settings {
            self.settings_layer()
        } else {
            LayerUi { enabled: true, passes: 1, ..self.cur().clone() }
        };
        let listed = lg_svg2gcode::list_layers_from_file(std::path::Path::new(path)).ok().filter(|l| !l.is_empty());
        let layered = listed.is_some();
        let infos = listed.unwrap_or_else(|| vec![LayerInfo { key: KEY_ALL.into(), label: "(all)".into(), shapes: 0 }]);
        let stored = if from_settings { None } else { load_sets().into_iter().find(|s| s.path == path) };
        self.layers = infos
            .into_iter()
            .map(|info| {
                let ui = stored.as_ref().and_then(|s| s.layers.get(&info.key)).cloned().unwrap_or_else(|| seed.clone());
                (info, ui)
            })
            .collect();
        self.selected = 0;
        self.generated.clear();
        self.revision += 1;
        self.source = Some(SvgSource { path: path.to_owned(), layered });
        self.dirty = true;
        self.last_error = None;
    }

    /// Take over the edited layer values of a dropped SVG editor as the plain defaults (the next SVG starts there).
    ///
    /// `from`: editor that goes away. Only meant for the defaults editor (no source).
    pub fn adopt_defaults(&mut self, from: &ParamsDock) {
        self.layers = vec![(LayerInfo { key: KEY_DEFAULTS.into(), label: "(defaults)".into(), shapes: 0 }, from.cur().clone())];
        self.selected = 0;
        self.preset_name = from.preset_name.clone();
        self.revision += 1;
    }

    /// Test access: command ranges of the layers in this file's block (relative to the block start).
    #[cfg(test)]
    pub(crate) fn generated_ranges(&self) -> &Ranges {
        &self.generated
    }

    /// Test access: set S-MAX of layer `layer` as an edit would.
    ///
    /// `layer`: layer index. `power`: new S-MAX.
    #[cfg(test)]
    pub(crate) fn test_set_power(&mut self, layer: usize, power: i32) {
        self.layers[layer].1.max_power = power;
        self.mark_dirty();
    }

    /// Test access: S-MAX of the edited layer. Returns the value.
    #[cfg(test)]
    pub(crate) fn test_power_of_current(&self) -> i32 {
        self.cur().max_power
    }

    /// Returns the SVG path of this editor, None for the defaults editor.
    #[cfg(test)]
    pub fn path(&self) -> Option<&str> {
        self.source.as_ref().map(|s| s.path.as_str())
    }

    /// Returns the number of layers (1 for the defaults pseudo-layer and for SVGs without groups).
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Dev hook (`LASERGRBL_DEV_SEQ` `fill=N` / `nofill=N` / `cross=N` / `nocross=N` / `pershape=N` / `nopershape=N`): set fill flags of layer `i`.
    ///
    /// `i`: layer index. `fill`: new fill flag (None = keep). `cross`: new cross flag (None = keep). `per_shape`: new per-shape flag (None = keep).
    pub fn dev_set_fill(&mut self, i: usize, fill: Option<bool>, cross: Option<bool>, per_shape: Option<bool>) {
        let Some((_, l)) = self.layers.get_mut(i) else { return };
        if let Some(f) = fill {
            l.fill = f;
        }
        if let Some(c) = cross {
            l.fill_cross = c;
        }
        if let Some(p) = per_shape {
            l.fill_per_shape = p;
        }
        self.mark_dirty();
        self.revision += 1;
    }

    /// Dev hook (`LASERGRBL_DEV_SEQ` `layer=N` / `layeroff=N`): select layer `i`, or disable it.
    ///
    /// `i`: layer index. `select`: true = select it, false = disable it (selection unchanged).
    pub fn dev_select_layer(&mut self, i: usize, select: bool) {
        if i >= self.layers.len() {
            return;
        }
        if select {
            self.selected = i;
        } else {
            self.layers[i].1.enabled = false;
            self.mark_dirty();
        }
        self.revision += 1;
    }

    /// True when at least one layer would be converted. Returns bool.
    fn any_enabled(&self) -> bool {
        self.layers.iter().any(|(_, l)| l.enabled)
    }

    /// Converter parameters of all layers. Returns one entry per layer, in layer order.
    pub(crate) fn layer_params(&self) -> Vec<LayerParams> {
        self.layers
            .iter()
            .map(|(_, l)| LayerParams {
                enabled: l.enabled,
                passes: l.passes.clamp(1, 100) as u32,
                border_speed: l.border_speed,
                power_max: l.max_power,
                laser_on: LASER_OPTIONS[l.laser_on].1.to_owned(),
                filter: FILTER_OPTIONS[l.filter],
                fill: l.fill.then_some(HatchFill { distance_mm: l.fill_distance as f64, angle_deg: l.fill_angle as f64, cross: l.fill_cross, per_shape: l.fill_per_shape }),
            })
            .collect()
    }

    /// Mark the dock dirty and bump the edit counter (an edit needs a regen).
    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.edit_seq += 1;
    }

    /// Take the work order for the latest edit: persist the settings and hand out the producer of this file's
    /// block. The editor counts as generating until [`ParamsDock::complete`] (or [`ParamsDock::cancel_request`]).
    ///
    /// Returns None without a source, while a request is out, when nothing is dirty, or when no layer is enabled
    /// (the red text says so).
    pub fn take_request(&mut self) -> Option<GenRequest> {
        use lg_base::settings::set;
        if self.inflight.is_some() || self.source.is_none() || !self.dirty {
            return None;
        }
        if !self.any_enabled() {
            self.last_error = Some(tr_plain("ParamsDock.NoLayerEnabled"));
            return None;
        }
        // selected layer becomes the "last used" values
        let c = self.cur().clone();
        set("GrayScaleConversion.VectorizeOptions.BorderSpeed", c.border_speed);
        set("GrayScaleConversion.Gcode.LaserOptions.PowerMax", c.max_power);
        set("GrayScaleConversion.Gcode.LaserOptions.LaserOn", LASER_OPTIONS[c.laser_on].1);
        self.save_set();

        let params = self.layer_params();
        let filter = FILTER_OPTIONS[c.filter];
        let src = self.source.as_ref()?;
        let (path, layered) = (src.path.clone(), src.layered);
        let name = file_name(&path);
        let seq = self.edit_seq;
        self.inflight = Some(seq);
        self.auto_seq = Some(seq);
        self.last_error = None;
        // always appends: the coordinator cuts the old block out first
        let produce: ProduceFn = Box::new(move |f| {
            if layered {
                f.load_imported_svg_layered(&path, true, &params)
            } else {
                f.load_imported_svg(&path, true, filter).map(|()| Vec::new())
            }
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

    /// Result of a request taken with [`ParamsDock::take_request`].
    ///
    /// `seq`: the request's `seq`. `result`: layer ranges relative to the block start, or the error text.
    /// Edits made while the request ran keep the editor dirty. A failed generation is not retried by
    /// [`ParamsDock::wants_auto`] until the next edit.
    pub fn complete(&mut self, seq: u64, result: Result<Ranges, String>) {
        self.inflight = None;
        match result {
            Ok(ranges) => {
                self.dirty = self.edit_seq != seq;
                self.last_error = None;
                self.generated = ranges;
            }
            Err(e) => {
                self.last_error = Some(e);
                // the block is gone
                self.generated.clear();
            }
        }
        self.revision += 1;
    }

    /// Save the layer values of the current file into the per-file store.
    fn save_set(&self) {
        let Some(src) = &self.source else { return };
        let layers = self.layers.iter().map(|(i, l)| (i.key.clone(), l.clone())).collect();
        let mut sets = load_sets();
        merge_set(&mut sets, StoredSet { path: src.path.clone(), saved_unix_secs: now_secs(), layers });
        lg_base::settings::set(SETS_KEY, sets);
    }

    /// Returns true when the G-code is out of date or being generated (SVG loaded).
    pub fn needs_apply(&self) -> bool {
        self.source.is_some() && (self.dirty || self.inflight.is_some())
    }

    /// Returns true when edits wait, no request is out and a layer is enabled ([`ParamsDock::take_request`] would yield one).
    pub fn is_dirty_idle(&self) -> bool {
        self.source.is_some() && self.dirty && self.inflight.is_none() && self.any_enabled()
    }

    /// Returns true when the coordinator should start a request by itself: [`ParamsDock::is_dirty_idle`] and no
    /// attempt yet for the latest edit (a failed generation waits for the next edit or for Run).
    pub fn wants_auto(&self) -> bool {
        self.is_dirty_idle() && self.auto_seq != Some(self.edit_seq)
    }

    /// Returns true when the program is stale and cannot be regenerated because every layer is off.
    pub fn is_blocked(&self) -> bool {
        self.source.is_some() && self.dirty && self.inflight.is_none() && !self.any_enabled()
    }

    /// Returns true while a request of this editor runs.
    pub fn in_flight(&self) -> bool {
        self.inflight.is_some()
    }

    /// Returns the text of what is wrong with this file (failed generation, no layer enabled), None when fine.
    pub fn problem(&self) -> Option<String> {
        self.last_error.clone().or_else(|| self.is_blocked().then(|| tr_plain("ParamsDock.NoLayerEnabled")))
    }

    /// Returns the counter that grows on every change the preview overlay depends on.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Overlay for the preview: highlight of the selected layer, hidden disabled layers.
    ///
    /// Returns ranges relative to the file's block; None without an SVG source.
    pub fn view(&self) -> Option<SvgView> {
        self.source.as_ref()?;
        let hidden = self
            .layers
            .iter()
            .zip(&self.generated)
            .filter(|((_, l), _)| !l.enabled)
            .filter_map(|(_, r)| r.clone())
            .collect();
        let sel_on = self.layers.get(self.selected).is_some_and(|(_, l)| l.enabled);
        let highlight = if self.layers.len() > 1 && sel_on { self.generated.get(self.selected).cloned().flatten() } else { None };
        Some(SvgView { hidden, highlight })
    }

    /// Rebuild the dropdown list from the preset database (after the editor closed).
    pub fn refresh_presets(&mut self) {
        self.presets = self.presets_db.lock().unwrap_or_else(|e| e.into_inner()).presets().to_vec();
    }

    /// Index into `presets` of the selected layer's preset. Returns None when none/vanished.
    fn preset_index(&self) -> Option<usize> {
        let n = self.cur().preset.as_ref()?;
        self.presets.iter().position(|p| p.name.eq_ignore_ascii_case(n))
    }

    /// Load all values of preset `i` into the selected layer and mark dirty.
    ///
    /// `i`: index into `presets`.
    fn select_preset(&mut self, i: usize) {
        let Some(p) = self.presets.get(i).cloned() else { return };
        let (max, border_max) = (self.power_max_limit, self.border_max);
        let new_laser = LASER_OPTIONS.iter().position(|o| o.1 == p.laser_on).unwrap_or(0);
        let new_filter = FILTER_OPTIONS.iter().position(|f| filter_name(*f) == p.filter).unwrap_or(0);
        let c = self.cur_mut();
        c.passes = p.passes.clamp(1, 100);
        c.border_speed = if border_max > 0 { p.speed.clamp(1, border_max) } else { p.speed };
        c.max_power = p.power_s(max);
        let laser_changed = c.laser_on != new_laser;
        c.laser_on = new_laser;
        c.filter = new_filter;
        c.fill = p.fill;
        c.fill_distance = p.fill_distance.clamp(0.01, 10.0) as f32;
        c.fill_angle = (p.fill_angle.round() as i32).clamp(0, 360);
        c.fill_cross = p.fill_cross;
        c.fill_per_shape = p.fill_per_shape;
        c.preset = Some(p.name.clone());
        lg_base::settings::set(PRESET_KEY, Some(p.name.clone()));
        self.preset_name = p.name;
        // keep the M4 warning working
        if laser_changed {
            self.on_laser_mode_changed();
        }
        self.mark_dirty();
    }

    /// Drop the preset selection (dropdown "(none)"); values stay, no regen.
    fn deselect_preset(&mut self) {
        self.cur_mut().preset = None;
        lg_base::settings::delete(PRESET_KEY);
    }

    /// Save the selected layer's values under the name in the name field (same name overwrites).
    ///
    /// No G-code change, so the dock is not marked dirty. Errors go to `last_error`.
    fn save_preset(&mut self) {
        let name = self.preset_name.trim().to_owned();
        let c = self.cur().clone();
        let p = LayerPreset {
            name: name.clone(),
            passes: c.passes,
            speed: c.border_speed,
            power_pct: LayerPreset::pct_from_s(c.max_power, self.power_max_limit),
            laser_on: LASER_OPTIONS[c.laser_on].1.to_owned(),
            filter: filter_name(FILTER_OPTIONS[c.filter]).to_owned(),
            fill: c.fill,
            fill_distance: round2(c.fill_distance as f64),
            fill_angle: c.fill_angle as f64,
            fill_cross: c.fill_cross,
            fill_per_shape: c.fill_per_shape,
        };
        let res = self.presets_db.lock().unwrap_or_else(|e| e.into_inner()).upsert(p);
        match res {
            Ok(()) => {
                self.refresh_presets();
                // adopt the stored spelling
                let stored = self.presets_db.lock().unwrap_or_else(|e| e.into_inner()).find(&name).map_or(name, |p| p.name.clone());
                lg_base::settings::set(PRESET_KEY, Some(stored.clone()));
                self.preset_name = stored.clone();
                self.cur_mut().preset = Some(stored);
                self.save_set();
                self.last_error = None;
            }
            Err(e) => self.last_error = Some(e),
        }
    }

    /// True when the layer values differ from the selected preset.
    ///
    /// Returns false without a selection.
    fn preset_modified(&self) -> bool {
        let c = self.cur();
        self.preset_index().and_then(|i| self.presets.get(i)).is_some_and(|p| {
            p.passes != c.passes
                || p.speed != c.border_speed
                || p.power_s(self.power_max_limit) != c.max_power
                || p.laser_on != LASER_OPTIONS[c.laser_on].1
                || p.filter != filter_name(FILTER_OPTIONS[c.filter])
                || p.fill != c.fill
                || round2(p.fill_distance) != round2(c.fill_distance as f64)
                || p.fill_angle.round() as i32 != c.fill_angle
                || p.fill_cross != c.fill_cross
                || p.fill_per_shape != c.fill_per_shape
        })
    }

    /// Copy the selected layer's values (not `enabled`) to every other layer and mark dirty.
    fn sync_layers(&mut self) {
        let src = self.cur().clone();
        let sel = self.selected;
        for (i, (_, l)) in self.layers.iter_mut().enumerate() {
            if i != sel {
                *l = LayerUi { enabled: l.enabled, ..src.clone() };
            }
        }
        self.mark_dirty();
    }

    /// C# `RefreshPerc`: "P1" text of a power value, empty without a valid MaxPWM.
    ///
    /// `v`: power value. Returns e.g. "100.0 %" formatted like .NET `P1` (invariant).
    fn perc_text(&self, v: i32) -> String {
        if self.limits.max_pwm > 0 {
            lg_dotnet::fmt_decimal_custom(Decimal::from(v) / Decimal::from(self.limits.max_pwm), "P1")
        } else {
            String::new()
        }
    }

    /// C# `CBLaserON_SelectedIndexChanged`: warn when M4 is picked but firmware is not in laser mode.
    fn on_laser_mode_changed(&mut self) {
        if !self.limits.laser_mode && LASER_OPTIONS[self.cur().laser_on].1 == "M4" {
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
    pub fn show(&mut self, ui: &mut Ui) -> Vec<ParamsAction> {
        let mut actions = Vec::new();
        // config changes after connecting
        self.limits = Limits::from_config();
        self.border_max = self.limits.max_rate_x;
        self.power_max_limit = self.limits.max_pwm;
        let mut edited = false;

        if self.source.is_some() {
            self.layer_group(ui, &mut edited);
            ui.add_space(4.0);
        }
        self.preset_group(ui, &mut actions, &mut edited);
        ui.add_space(4.0);
        self.speed_group(ui, &mut edited);
        ui.add_space(4.0);
        self.laser_group(ui, &mut edited);
        ui.add_space(4.0);
        self.fill_group(ui, &mut edited);
        ui.add_space(4.0);
        self.filter_group(ui, &mut edited);
        ui.add_space(8.0);
        self.status_row(ui);
        if edited {
            self.mark_dirty();
        }
        actions
    }

    /// Display text of a layer.
    ///
    /// `i`: layer index. Returns "label (shapes)", prefixed with "(off)" when the layer is disabled.
    fn layer_text(&self, i: usize) -> String {
        let (info, l) = &self.layers[i];
        let label = if info.key == "#root" { tr_plain("ParamsDock.Ungrouped") } else { info.label.clone() };
        let off = if l.enabled { "" } else { "(off) " };
        format!("{off}{label} ({})", info.shapes)
    }

    /// Layer picker, enable checkbox and the sync button.
    ///
    /// `ui`: parent. `edited`: set on any change that needs a re-apply.
    fn layer_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        grbl_group_box_wide(ui, &tr_plain("ParamsDock.Layer"), |ui| {
            let items: Vec<String> = (0..self.layers.len()).map(|i| self.layer_text(i)).collect();
            let mut sel = Some(self.selected);
            let w = ui.available_width().max(60.0);
            if flat_combo(ui, "params-layer", &mut sel, &items, w).changed() {
                self.selected = sel.unwrap_or(self.selected);
                self.revision += 1;
                // name field follows the layer's preset
                if let Some(n) = self.cur().preset.clone() {
                    self.preset_name = n;
                }
            }
            ui.horizontal(|ui| {
                let mut en = self.cur().enabled;
                if ui.checkbox(&mut en, tr_plain("ParamsDock.Enabled")).changed() {
                    self.cur_mut().enabled = en;
                    self.revision += 1;
                    *edited = true;
                }
            });
            let btn = ui.add_enabled(self.layers.len() > 1, egui::Button::new(tr_plain("ParamsDock.SyncLayers")).min_size(egui::vec2(ui.available_width(), 22.0)));
            if btn.on_hover_text(tr_plain("ParamsDock.SyncLayersTip")).clicked() {
                self.sync_layers();
            }
        });
    }

    /// Preset dropdown, info text, editor button and save-as-name row.
    ///
    /// `ui`: parent. `actions`: collects editor requests. `edited`: set on a preset pick.
    fn preset_group(&mut self, ui: &mut Ui, actions: &mut Vec<ParamsAction>, edited: &mut bool) {
        grbl_group_box_wide(ui, &tr_plain("ParamsDock.Preset"), |ui| {
            let mut items = vec![tr_plain("ParamsDock.PresetNone")];
            items.extend(self.presets.iter().map(|p| p.name.clone()));
            let mut sel = Some(self.preset_index().map_or(0, |i| i + 1));
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 44.0).max(60.0);
                let r = flat_combo(ui, "params-preset", &mut sel, &items, w);
                if r.changed() {
                    match sel.unwrap_or(0) {
                        0 => self.deselect_preset(),
                        n => {
                            self.select_preset(n - 1);
                            *edited = true;
                        }
                    }
                }
                if ui.add(ImageButton::new("mdi-database").size(egui::Vec2::splat(17.0))).on_hover_text(tr("ParamsDock.EditPresets")).clicked() {
                    actions.push(ParamsAction::OpenPresetEditor);
                }
            });
            // info of the selected preset
            if let Some(p) = self.preset_index().and_then(|i| self.presets.get(i)) {
                let info = tr_fmt("ParamsDock.PresetInfo", &[&p.passes, &p.speed, &format!("{:.1}", p.power_pct), &p.laser_on]);
                let mut info = info;
                if p.fill {
                    let cross = if p.fill_cross { " x" } else { "" };
                    info.push_str(&tr_fmt("ParamsDock.PresetFillInfo", &[&format!("{:.2}", p.fill_distance), &p.fill_angle.round(), &cross]));
                }
                ui.label(RichText::new(info).small());
                if self.preset_modified() {
                    ui.label(RichText::new(tr_plain("ParamsDock.Modified")).small().italics());
                }
            }
            // save row
            let mut save = false;
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 28.0).max(60.0);
                let r = ui.add(egui::TextEdit::singleline(&mut self.preset_name).id(egui::Id::unique("params-preset-name")).hint_text(tr_plain("ParamsDock.PresetNameHint")).desired_width(w));
                let can_save = !self.preset_name.trim().is_empty();
                // Enter in the field saves too
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_save {
                    save = true;
                }
                let btn = ui.add_enabled(can_save, ImageButton::new("mdi-content-save").size(egui::Vec2::splat(17.0)));
                if btn.on_hover_text(tr("ParamsDock.SavePresetTip")).clicked() {
                    save = true;
                }
            });
            if save {
                self.save_preset();
            }
        });
    }

    /// Border speed and passes inputs.
    ///
    /// `ui`: parent. `edited`: set on a committed edit.
    fn speed_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        let lim_rate = self.border_max;
        let mut v = self.cur().border_speed;
        let mut passes = self.cur().passes;
        // passes only per loaded layer: load_svg resets the defaults layer to 1
        let has_source = self.source.is_some();
        grbl_group_box_wide(ui, &ctl_text(P, "GbSpeed"), |ui| {
            ui.horizontal(|ui| {
                ui.label(tr_plain("ParamsDock.SpeedLabel"));
                if int_input(ui, "params-border", &mut v, 1, lim_rate, false, 55.0).is_some() {
                    *edited = true;
                }
                label(ui, P, "LblBorderTracingmm");
            });
            if has_source {
                ui.horizontal(|ui| {
                    ui.label(tr_plain("ParamsDock.PassesLabel"));
                    if int_input(ui, "params-passes", &mut passes, 1, 100, false, 40.0).is_some() {
                        *edited = true;
                    }
                });
            }
        });
        self.cur_mut().border_speed = v;
        self.cur_mut().passes = passes;
    }

    /// Laser mode, S-MIN and S-MAX.
    ///
    /// `ui`: parent. `edited`: set on any change.
    fn laser_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        let pwm_max = self.power_max_limit;
        grbl_group_box_wide(ui, &ctl_text(P, "GbLaser"), |ui| {
            let items: Vec<String> = LASER_OPTIONS.iter().map(|o| o.0.to_owned()).collect();
            // laser mode row
            ui.horizontal(|ui| {
                label(ui, P, "label18");
                let before = self.cur().laser_on;
                let mut sel = Some(before);
                // full width, no icon
                let w = ui.available_width().max(60.0);
                flat_combo(ui, "params-laser-on", &mut sel, &items, w);
                self.cur_mut().laser_on = sel.unwrap_or(before);
                if self.cur().laser_on != before {
                    *edited = true;
                    self.on_laser_mode_changed();
                }
            });
            if !self.support_pwm {
                return;
            }
            let mut max_p = self.cur().max_power;
            Grid::new("params-power").num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
                // S-MAX only: SVG paths burn at one power, S-MIN is never used
                label(ui, P, "LblSmax");
                if int_input(ui, "params-smax", &mut max_p, 1, pwm_max, false, 43.0).is_some() {
                    self.cur_mut().max_power = max_p;
                    *edited = true;
                }
                ui.label(self.perc_text(self.cur().max_power));
                ui.end_row();
            });
        });
    }

    /// Hatch fill of closed shapes: switch, line distance, angle, cross hatch.
    ///
    /// `ui`: parent. `edited`: set on any committed change.
    fn fill_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        grbl_group_box_wide(ui, &tr_plain("ParamsDock.Fill"), |ui| {
            let mut on = self.cur().fill;
            if ui.checkbox(&mut on, tr_plain("ParamsDock.FillEnabled")).changed() {
                self.cur_mut().fill = on;
                *edited = true;
            }
            // inputs only matter while fill is on
            ui.add_enabled_ui(on, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr_plain("ParamsDock.FillDistance"));
                    let mut d = self.cur().fill_distance;
                    let out = DecimalInputRanged::new(&mut d).range(0.01, 10.0).decimals(2).width(55.0).id(egui::Id::unique("params-fill-distance")).show(ui);
                    if out.committed.is_some() {
                        self.cur_mut().fill_distance = d;
                        *edited = true;
                    }
                    ui.label("mm");
                });
                ui.horizontal(|ui| {
                    ui.label(tr_plain("ParamsDock.FillAngle"));
                    let mut a = self.cur().fill_angle;
                    if int_input(ui, "params-fill-angle", &mut a, 0, 360, false, 43.0).is_some() {
                        self.cur_mut().fill_angle = a;
                        *edited = true;
                    }
                    ui.label("°");
                });
                let mut cross = self.cur().fill_cross;
                if ui.checkbox(&mut cross, tr_plain("ParamsDock.FillCross")).changed() {
                    self.cur_mut().fill_cross = cross;
                    *edited = true;
                }
                let mut per_shape = self.cur().fill_per_shape;
                if ui.checkbox(&mut per_shape, tr_plain("ParamsDock.FillPerShape")).on_hover_text(tr_plain("ParamsDock.FillPerShapeTip")).changed() {
                    self.cur_mut().fill_per_shape = per_shape;
                    *edited = true;
                }
            });
        });
    }

    /// Color filter combo.
    ///
    /// `ui`: parent. `edited`: set on change.
    fn filter_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        grbl_group_box_wide(ui, &ctl_text(P, "gbFilter"), |ui| {
            let items: Vec<String> = FILTER_OPTIONS.iter().map(|f| filter_name(*f).to_owned()).collect();
            ui.horizontal(|ui| {
                label(ui, P, "labelFilter");
                let mut sel = Some(self.cur().filter);
                let w = (ui.available_width() - 40.0).max(60.0);
                if flat_combo(ui, "params-filter", &mut sel, &items, w).changed() {
                    *edited = true;
                }
                self.cur_mut().filter = sel.unwrap_or(self.cur().filter);
                if info_button(ui, &ctl_tip(P, "BtnColorFilter")) {
                    open_link("https://lasergrbl.com/usage/raster-image-import/target-image-size-and-laser-options/#color-filter");
                }
            });
        });
    }

    /// Hint and status lines.
    ///
    /// `ui`: parent.
    fn status_row(&mut self, ui: &mut Ui) {
        if self.source.is_some() && !self.any_enabled() {
            ui.label(RichText::new(tr_plain("ParamsDock.NoLayerEnabled")).small().italics());
        }
        // status line
        let status = match &self.source {
            Some(s) if self.inflight.is_some() => format!("{} - {}", file_name(&s.path), tr_plain("ParamsDock.Generating")),
            Some(s) => file_name(&s.path),
            None => tr_plain("ParamsDock.NoSource"),
        };
        ui.label(RichText::new(status).small());
        if let Some(e) = &self.last_error {
            ui.colored_label(egui::Color32::from_rgb(200, 40, 40), e);
        }
    }
}

impl Default for ParamsDock {
    /// Same as [`ParamsDock::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// Round to 2 decimals. `v`: value. Returns the rounded value.
fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// File name part of a path text.
///
/// `p`: path. Returns the last component, or `p` when it has none.
fn file_name(p: &str) -> String {
    std::path::Path::new(p).file_name().map_or_else(|| p.to_owned(), |n| n.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::job_test_harness::Rig;
    use lg_services::layer_presets::{PresetDb, EXAMPLE_NAME};
    use std::sync::{Arc, Mutex};

    /// Defaults editor on an in-memory DB holding the example (never touches the real config file).
    fn new_dock() -> ParamsDock {
        ParamsDock::with_presets(Arc::new(Mutex::new(PresetDb::in_memory(vec![LayerPreset::example()]))))
    }

    /// Editor for the SVG at `path`, seeded from a fresh defaults editor.
    fn svg_dock(path: &std::path::Path) -> ParamsDock {
        ParamsDock::for_svg(&new_dock(), path.to_str().unwrap(), false)
    }

    /// Take the editor's request, run it on the rig and hand the result back. Returns None without a request, else success.
    fn pump(rig: &mut Rig, d: &mut ParamsDock) -> Option<bool> {
        let req = d.take_request()?;
        let (seq, r) = rig.run(req);
        let ok = r.is_ok();
        d.complete(seq, r);
        Some(ok)
    }

    fn write_svg(tag: &str) -> std::path::PathBuf {
        let p = lg_base::paths::test_dir(&format!("params_{tag}")).join("in.svg");
        std::fs::write(&p, r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><path d="M1 1 L19 1 L19 19 Z" stroke="black" fill="none"/></svg>"#).unwrap();
        p
    }

    #[test]
    fn edit_during_request_keeps_dirty() {
        let _g = settings_guard();
        let p = write_svg("editjob");
        let mut d = svg_dock(&p);
        let mut rig = Rig::new(&[]);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        d.mark_dirty();
        let req = d.take_request().expect("dirty editor hands out a request");
        assert!(d.in_flight() && d.take_request().is_none(), "one request at a time");
        // edit while the request is out
        d.mark_dirty();
        let (seq, r) = rig.run(req);
        d.complete(seq, r);
        assert!(d.dirty && d.wants_auto());
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert!(!d.dirty);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn auto_attempt_once_per_edit_and_cancel_gives_it_back() {
        let _g = settings_guard();
        let p = write_svg("autoonce");
        let mut d = svg_dock(&p);
        let mut rig = Rig::new(&[]);
        // the load itself is the first automatic attempt
        assert!(d.wants_auto());
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert!(!d.wants_auto());
        // program busy: the coordinator hands the request back, the attempt is not consumed
        d.mark_dirty();
        let req = d.take_request().unwrap();
        assert!(!d.wants_auto(), "a request is out");
        d.cancel_request(req.seq);
        assert!(!d.in_flight() && d.wants_auto());
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        // failing generation is not retried without a new edit ...
        let _ = std::fs::remove_file(&p);
        d.mark_dirty();
        assert!(d.wants_auto());
        assert_eq!(pump(&mut rig, &mut d), Some(false));
        assert!(!d.wants_auto() && d.is_dirty_idle());
        // the failure stays visible, the next edit makes the attempt automatic again
        d.mark_dirty();
        assert!(d.wants_auto());
        assert!(d.problem().is_some(), "the failure stays visible");
    }

    #[test]
    fn failed_generation_clears_the_block_ranges() {
        let _g = settings_guard();
        let p = write_svg("failed");
        let mut d = svg_dock(&p);
        let mut rig = Rig::new(&["G0 X1"]);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert!(!d.generated.is_empty());
        let _ = std::fs::remove_file(&p);
        d.mark_dirty();
        assert_eq!(pump(&mut rig, &mut d), Some(false));
        // the block is gone from the program, the head stays
        assert!(d.generated.is_empty() && rig.block.is_empty());
        assert_eq!(rig.lines(), ["G0 X1"]);
    }

    #[test]
    fn defaults_from_empty_settings() {
        let _g = settings_guard();
        let d = new_dock();
        assert_eq!((d.cur().border_speed, d.cur().max_power), (1000, 1000));
        // empty config reports laser mode on, stored "M3" wins
        assert_eq!(d.cur().laser_on, 0);
        assert_eq!(d.cur().filter, 0);
        assert!(d.support_pwm);
        assert!(!d.needs_apply());
    }

    #[test]
    fn laser_mode_m4_restored_only_in_laser_mode() {
        let _g = settings_guard();
        lg_base::settings::set("GrayScaleConversion.Gcode.LaserOptions.LaserOn", "M4");
        assert_eq!(new_dock().cur().laser_on, 1);
    }

    #[test]
    fn percent_text_like_dotnet() {
        let _g = settings_guard();
        let mut d = new_dock();
        assert_eq!(d.perc_text(1000), "100.0 %");
        d.limits.max_pwm = 0;
        assert_eq!(d.perc_text(10), "");
    }

    #[test]
    fn preset_pick_sets_all_values_and_tracks_modified() {
        let _g = settings_guard();
        let mut d = new_dock();
        d.select_preset(0);
        let c = d.cur();
        assert_eq!((c.border_speed, c.max_power, c.passes, c.laser_on, c.filter), (1000, 450, 1, 1, 0));
        assert_eq!(c.preset.as_deref(), Some(EXAMPLE_NAME));
        assert_eq!(d.preset_name, EXAMPLE_NAME);
        assert!(d.dirty && !d.preset_modified());
        d.cur_mut().border_speed = 800;
        assert!(d.preset_modified());
        // name persisted
        assert_eq!(lg_base::settings::get(PRESET_KEY, None::<String>), Some(EXAMPLE_NAME.to_owned()));
        d.deselect_preset();
        assert!(!d.preset_modified());
        assert_eq!(lg_base::settings::get(PRESET_KEY, None::<String>), None);
    }

    #[test]
    fn fill_values_roundtrip_through_preset() {
        let _g = settings_guard();
        let mut d = new_dock();
        {
            let c = d.cur_mut();
            (c.fill, c.fill_distance, c.fill_angle, c.fill_cross, c.fill_per_shape) = (true, 0.25, 45, true, false);
        }
        d.preset_name = "Hatch".into();
        d.save_preset();
        assert!(!d.preset_modified());
        let p = d.presets.iter().find(|p| p.name == "Hatch").unwrap().clone();
        assert_eq!((p.fill, p.fill_distance, p.fill_angle, p.fill_cross), (true, 0.25, 45.0, true));
        assert!(!p.fill_per_shape);
        // edit one value: modified
        d.cur_mut().fill_angle = 50;
        assert!(d.preset_modified());
        // loading the preset restores the values
        let i = d.presets.iter().position(|p| p.name == "Hatch").unwrap();
        d.deselect_preset();
        d.cur_mut().fill = false;
        d.select_preset(i);
        let c = d.cur();
        assert_eq!((c.fill, c.fill_angle, c.fill_cross, c.fill_per_shape), (true, 45, true, false));
        assert!((c.fill_distance - 0.25).abs() < 1e-6);
        // layer params carry the fill
        let lp = d.layer_params();
        assert_eq!(lp[0].fill, Some(HatchFill { distance_mm: 0.25f32 as f64, angle_deg: 45.0, cross: true, per_shape: false }));
    }

    #[test]
    fn old_stored_layer_loads_without_fill_fields() {
        let l: LayerUi = serde_json::from_str(r#"{"enabled":true,"passes":1,"border_speed":1000,"max_power":500,"laser_on":0,"filter":0}"#).unwrap();
        assert_eq!((l.fill, l.fill_distance, l.fill_angle, l.fill_cross), (false, 0.2, 0, false));
    }

    #[test]
    fn save_preset_overwrites_same_name() {
        let _g = settings_guard();
        let f = lg_base::paths::test_dir("params_presets").join("LayerPresets.json");
        let db = Arc::new(Mutex::new(PresetDb::load_from(&f)));
        let mut d = ParamsDock::with_presets(db);
        d.preset_name = "Test".into();
        d.cur_mut().border_speed = 500;
        d.save_preset();
        assert!(d.last_error.is_none());
        d.preset_name = "test".into();
        d.cur_mut().border_speed = 600;
        d.save_preset();
        let again = PresetDb::load_from(&f);
        let hits: Vec<_> = again.presets().iter().filter(|p| p.name.eq_ignore_ascii_case("test")).collect();
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].speed, hits[0].name.as_str()), (600, "test"));
        assert_eq!(d.presets.len(), 2);
        assert_eq!(d.cur().preset.as_deref(), Some("test"));
        assert!(!d.preset_modified());
        let _ = std::fs::remove_file(&f);
    }

    #[test]
    fn save_preset_rejects_empty_name() {
        let _g = settings_guard();
        let mut d = new_dock();
        d.preset_name = "  ".into();
        d.save_preset();
        assert!(d.last_error.is_some());
        assert_eq!(d.presets.len(), 1);
    }

    #[test]
    fn old_stored_layer_with_material_id_still_loads() {
        let json = r#"{"enabled":true,"passes":2,"border_speed":900,"max_power":300,"laser_on":0,"filter":0,"material_id":"abc"}"#;
        let l: LayerUi = serde_json::from_str(json).unwrap();
        assert_eq!((l.passes, l.border_speed, l.preset), (2, 900, None));
    }

    #[test]
    fn request_persists_settings_and_loads() {
        let _g = settings_guard();
        let p = write_svg("persist");
        let mut defaults = new_dock();
        defaults.cur_mut().border_speed = 1500;
        defaults.cur_mut().max_power = 800;
        defaults.cur_mut().laser_on = 1;
        // a new SVG starts from the edited defaults
        let mut d = ParamsDock::for_svg(&defaults, p.to_str().unwrap(), false);
        let mut rig = Rig::new(&[]);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert!(rig.count() > 0);
        let get = |k: &str| lg_base::settings::get(k, -1i32);
        assert_eq!(get("GrayScaleConversion.VectorizeOptions.BorderSpeed"), 1500);
        // S-MIN is not part of the dock: raster default stays untouched
        assert_eq!(get("GrayScaleConversion.Gcode.LaserOptions.PowerMin"), -1);
        assert_eq!(get("GrayScaleConversion.Gcode.LaserOptions.PowerMax"), 800);
        assert_eq!(lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.LaserOn", String::new()), "M4");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn request_clears_dirty_and_edit_sets_it() {
        let _g = settings_guard();
        let p = write_svg("dirty");
        let mut d = svg_dock(&p);
        let mut rig = Rig::new(&[]);
        assert!(d.needs_apply());
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert!(!d.dirty && !d.needs_apply());
        // edit marks dirty again
        d.dirty = true;
        assert!(d.needs_apply() && d.is_dirty_idle());
        // the defaults editor never asks for a request and adopts the values of a dropped file
        let mut defaults = new_dock();
        assert!(defaults.take_request().is_none() && !defaults.needs_apply());
        d.cur_mut().border_speed = 333;
        defaults.adopt_defaults(&d);
        assert_eq!(defaults.cur().border_speed, 333);
        assert!(defaults.path().is_none() && d.path().is_some());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn regeneration_keeps_the_head_and_does_not_duplicate() {
        let _g = settings_guard();
        let p = write_svg("append");
        let mut rig = Rig::new(&["G0 X1", "G1 X2", "G1 X3"]);
        let mut d = svg_dock(&p);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        let first = rig.count();
        assert!(first > 3);
        d.dirty = true;
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert_eq!(rig.count(), first);
        // old head untouched
        assert_eq!(rig.lines()[..3], ["G0 X1", "G1 X2", "G1 X3"]);
        assert_eq!(rig.block, 3..first);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn request_without_source_does_nothing() {
        let _g = settings_guard();
        let mut d = new_dock();
        assert!(d.take_request().is_none());
        assert!(!d.needs_apply() && !d.is_dirty_idle() && !d.wants_auto() && d.view().is_none());
    }

    /// Two-layer SVG in the scratchpad-independent temp dir. `tag`: file tag. Returns path.
    fn write_layer_svg(tag: &str) -> std::path::PathBuf {
        let p = lg_base::paths::test_dir(&format!("params_{tag}")).join("in.svg");
        std::fs::write(
            &p,
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="40mm" height="40mm" viewBox="0 0 40 40">
<g id="a" inkscape:label="Engrave"><rect x="2" y="2" width="10" height="10"/></g>
<g id="b" inkscape:label="Cut"><circle cx="25" cy="25" r="6"/></g></svg>"#,
        )
        .unwrap();
        p
    }

    #[test]
    fn layers_listed_and_sync_copies_all_but_enabled() {
        let _g = settings_guard();
        let p = write_layer_svg("sync");
        let mut d = svg_dock(&p);
        let mut rig = Rig::new(&[]);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        assert_eq!(d.layers.len(), 2);
        assert_eq!(d.layers[1].0.label, "Cut");
        d.layers[1].1.enabled = false;
        d.cur_mut().border_speed = 321;
        d.cur_mut().passes = 3;
        d.cur_mut().max_power = 77;
        d.cur_mut().preset = Some("m".into());
        d.dirty = false;
        d.sync_layers();
        assert!(d.dirty);
        let l = &d.layers[1].1;
        assert_eq!((l.border_speed, l.passes, l.max_power), (321, 3, 77));
        assert_eq!(l.preset.as_deref(), Some("m"));
        assert!(!l.enabled);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn per_file_store_roundtrip_restores_by_key() {
        let _g = settings_guard();
        let p = write_layer_svg("store");
        let path = p.to_str().unwrap();
        let mut d = svg_dock(&p);
        let mut rig = Rig::new(&[]);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        d.layers[1].1.border_speed = 555;
        d.layers[1].1.passes = 4;
        d.layers[0].1.enabled = false;
        d.dirty = true;
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        // reopen with a fresh editor
        let d2 = ParamsDock::for_svg(&new_dock(), path, false);
        assert_eq!((d2.layers[1].1.border_speed, d2.layers[1].1.passes), (555, 4));
        assert!(!d2.layers[0].1.enabled);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn project_restore_ignores_the_per_file_store() {
        let _g = settings_guard();
        let p = write_layer_svg("fromsettings");
        let path = p.to_str().unwrap();
        let mut d = svg_dock(&p);
        d.layers[1].1.border_speed = 555;
        d.dirty = true;
        let mut rig = Rig::new(&[]);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        lg_base::settings::set("GrayScaleConversion.VectorizeOptions.BorderSpeed", 1234);
        let d2 = ParamsDock::for_svg(&new_dock(), path, true);
        assert_eq!((d2.layers[0].1.border_speed, d2.layers[1].1.border_speed), (1234, 1234));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn store_dedups_by_path_newest_wins_and_caps() {
        let set = |path: &str, t: u64| StoredSet { path: path.into(), saved_unix_secs: t, layers: BTreeMap::new() };
        let mut v = Vec::new();
        merge_set(&mut v, set("a", 10));
        merge_set(&mut v, set("a", 20));
        merge_set(&mut v, set("a", 5));
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].saved_unix_secs, 20);
        for i in 0..60u64 {
            merge_set(&mut v, set(&format!("f{i}"), 100 + i));
        }
        assert_eq!(v.len(), MAX_SETS);
        // oldest dropped, newest first
        assert!(!v.iter().any(|s| s.path == "a"));
        assert_eq!(v[0].path, "f59");
    }

    #[test]
    fn all_layers_disabled_refuses_request() {
        let _g = settings_guard();
        let p = write_layer_svg("off");
        let mut d = svg_dock(&p);
        let mut rig = Rig::new(&[]);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        for (_, l) in d.layers.iter_mut() {
            l.enabled = false;
        }
        d.dirty = true;
        assert!(d.take_request().is_none());
        assert!(!d.in_flight() && d.last_error.is_some());
        assert!(!d.is_dirty_idle() && d.is_blocked() && d.problem().is_some());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn view_highlights_selected_and_hides_disabled() {
        let _g = settings_guard();
        let p = write_layer_svg("view");
        let mut d = svg_dock(&p);
        assert!(new_dock().view().is_none());
        let mut rig = Rig::new(&["G0 X1", "G1 X2"]);
        assert_eq!(pump(&mut rig, &mut d), Some(true));
        // ranges are relative to the block: small numbers although two lines sit in front
        assert!((d.generated[0].as_ref().unwrap().start as usize) < rig.block.len());
        let v = d.view().unwrap();
        assert_eq!(v.highlight, d.generated[0]);
        assert!(v.hidden.is_empty());
        // select layer 2, disable layer 1
        d.selected = 1;
        d.layers[0].1.enabled = false;
        let v2 = d.view().unwrap();
        assert_eq!(v2.highlight, d.generated[1]);
        assert_eq!(v2.hidden, vec![d.generated[0].clone().unwrap()]);
        // ranges are disjoint and inside the block
        let (a, b) = (d.generated[0].clone().unwrap(), d.generated[1].clone().unwrap());
        assert!(a.end <= b.start && b.end as usize <= rig.block.len());
        let _ = std::fs::remove_file(&p);
    }
}

/// End-to-end: drive the editor through egui input events inside the job dock, edit, let auto regen run, check the G-code.
#[cfg(test)]
mod e2e {
    use super::*;
    use crate::forms::job_test_harness::Harness;
    use lg_services::layer_presets::EXAMPLE_NAME;

    /// Three layers: ungrouped (X 5..10), Cut (red X 20..30, black X 45..50), Engrave (X 60..80, nested group).
    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="100mm" height="100mm" viewBox="0 0 100 100">
  <path d="M5 5 L10 5" style="stroke:#000000;fill:none"/>
  <g id="cut" inkscape:label="Cut" inkscape:groupmode="layer">
    <path d="M20 20 L30 20 L30 30" style="stroke:#ff0000;fill:none"/>
    <path d="M45 50 L50 50" style="stroke:#000000;fill:none"/>
  </g>
  <g id="engrave" inkscape:label="Engrave" inkscape:groupmode="layer">
    <path d="M60 60 L80 60 L80 80 L60 80 Z" style="stroke:#000000;fill:none"/>
    <g><path d="M62 90 L70 90" style="stroke:#000000;fill:none"/></g>
  </g>
</svg>"##;

    /// Values of word `w` (e.g. 'F') over `lines`. Returns the numbers.
    pub(super) fn words(lines: &[String], w: char) -> Vec<f64> {
        let mut out = Vec::new();
        for l in lines {
            for (i, c) in l.char_indices() {
                if c == w {
                    let rest: String = l[i + 1..].chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
                    if let Ok(v) = rest.parse() {
                        out.push(v);
                    }
                }
            }
        }
        out
    }

    /// Pen-down power lines (`S<n>` alone, n > 0). Returns their S values.
    pub(super) fn pen_downs(lines: &[String]) -> Vec<f64> {
        lines.iter().filter(|l| l.starts_with('S')).filter_map(|l| l[1..].trim().parse::<f64>().ok()).filter(|v| *v > 0.0).collect()
    }

    /// Distinct X values rounded to mm. Returns sorted set.
    pub(super) fn xs(lines: &[String]) -> Vec<i64> {
        let mut v: Vec<i64> = words(lines, 'X').into_iter().map(|x| x.round() as i64).collect();
        v.sort();
        v.dedup();
        v
    }

    /// Write the test SVG. `tag`: file tag. Returns the path.
    fn write_svg(tag: &str) -> std::path::PathBuf {
        let p = lg_base::paths::test_dir(&format!("e2e_{tag}")).join("in.svg");
        std::fs::write(&p, SVG).unwrap();
        p
    }

    /// Print a layer for debugging.
    fn dump(h: &Harness, i: usize) -> String {
        format!("{:?}", h.layer_lines(i))
    }

    #[test]
    fn every_input_reaches_the_gcode() {
        let _g = settings_guard();
        let p = write_svg("all");
        let mut h = Harness::new();
        h.job.add_svg(&p, false, false);
        assert!(h.wait());
        // layers listed: ungrouped, Cut, Engrave
        let labels: Vec<String> = (0..h.svg().layers.len()).map(|i| h.svg().layer_text(i)).collect();
        assert_eq!(labels, ["(ungrouped) (1)", "Cut (2)", "Engrave (2)"].map(|s| s.replace("(ungrouped)", &tr_plain("ParamsDock.Ungrouped"))));
        // defaults: every layer F1000 S1000
        for i in 0..3 {
            let l = h.layer_lines(i).unwrap();
            assert!(pen_downs(&l).iter().all(|s| *s == 1000.0), "layer {i}: {}", dump(&h, i));
        }
        assert_eq!(xs(&h.layer_lines(0).unwrap()), [5, 10]);
        assert_eq!(xs(&h.layer_lines(1).unwrap()), [20, 30, 45, 50]);
        assert_eq!(xs(&h.layer_lines(2).unwrap()), [60, 62, 70, 80]);

        // --- Cut: speed, S-MAX, mode, passes, filter via the UI
        h.pick(&labels[0], &labels[1]);
        assert_eq!(h.svg().selected, 1);
        h.type_into("params-border", "1500");
        h.type_into("params-smax", "400");
        h.pick("M3 - Constant Power", "M4 - Dynamic Power");
        h.type_into("params-passes", "3");
        h.pick("All", "Red");
        h.settle();
        let cut = h.layer_lines(1).unwrap();
        assert!(cut.iter().any(|l| l == "M4 S0"), "mode switch missing: {cut:?}");
        let all = h.all_lines();
        assert_eq!(all[0], "M3 S0", "program starts with the first layer's mode: {all:?}");
        assert_eq!(all.iter().filter(|l| l.starts_with("M3 S0") || l.starts_with("M4 S0")).count(), 3, "one mode line per change: {all:?}");
        assert!(words(&cut, 'F').iter().filter(|f| **f != 0.0).all(|f| *f == 1500.0), "{cut:?}");
        assert!(words(&cut, 'F').contains(&1500.0), "{cut:?}");
        assert_eq!(pen_downs(&cut), [400.0; 3], "3 passes of the red path only: {cut:?}");
        assert_eq!(xs(&cut), [20, 30], "red filter: {cut:?}");
        // other layers untouched, mode back to M3 for Engrave
        assert!(pen_downs(&h.layer_lines(0).unwrap()).iter().all(|s| *s == 1000.0));
        let eng = h.layer_lines(2).unwrap();
        assert!(eng.iter().any(|l| l == "M3 S0"), "M3 restore missing: {eng:?}");
        assert!(pen_downs(&eng).iter().all(|s| *s == 1000.0), "{eng:?}");
        assert!(words(&eng, 'F').contains(&1000.0), "engrave feed: {eng:?}");

        // --- Engrave disabled: gone from the program
        h.pick(&h.svg().layer_text(1), &h.svg().layer_text(2));
        assert_eq!(h.svg().selected, 2);
        h.click_text(&tr_plain("ParamsDock.Enabled"));
        assert!(!h.svg().layers[2].1.enabled);
        h.settle();
        assert!(h.layer_lines(2).is_none());
        assert!(xs(&h.all_lines()).iter().all(|x| *x < 60), "engrave still in program");

        // --- re-enable, Cut filter back to All, sync Cut to every layer
        h.click_text(&tr_plain("ParamsDock.Enabled"));
        h.pick(&h.svg().layer_text(2), &h.svg().layer_text(1));
        h.pick("Red", "All");
        h.click_text(&tr_plain("ParamsDock.SyncLayers"));
        h.settle();
        for i in 0..3 {
            let l = h.layer_lines(i).unwrap();
            let pd = pen_downs(&l);
            assert!(!pd.is_empty() && pd.iter().all(|s| *s == 400.0), "layer {i} power: {l:?}");
            assert!(words(&l, 'F').iter().filter(|f| **f != 0.0).all(|f| *f == 1500.0), "layer {i} feed: {l:?}");
        }
        // 3 passes everywhere: ungrouped 1 path, Cut 2, Engrave 2
        assert_eq!(pen_downs(&h.layer_lines(0).unwrap()).len(), 3);
        assert_eq!(pen_downs(&h.layer_lines(1).unwrap()).len(), 6);
        assert_eq!(pen_downs(&h.layer_lines(2).unwrap()).len(), 6);
        // all M4 now: one switch at the start of the program at most
        let all = h.all_lines();
        assert!(!all.iter().any(|l| l == "M3 S0"), "stale M3: {all:?}");

        // --- preset pick: passes, speed, power %, mode and filter of the example
        h.pick(&tr_plain("ParamsDock.PresetNone"), EXAMPLE_NAME);
        assert_eq!(h.svg().cur().preset.as_deref(), Some(EXAMPLE_NAME));
        h.settle();
        let cut = h.layer_lines(1).unwrap();
        assert_eq!(pen_downs(&cut), [450.0; 2], "2 paths x 1 pass at 45 %: {cut:?}");
        assert!(words(&cut, 'F').contains(&1000.0), "speed from preset: {cut:?}");
        // M4 was already active on all layers: no stale M3 appears
        assert!(!h.all_lines().iter().any(|l| l == "M3 S0"), "stale M3: {:?}", h.all_lines());

        // --- reopen: per-file store restores everything, identical G-code
        let before = h.all_lines();
        let mut h2 = Harness::new();
        h2.job.add_svg(&p, false, false);
        assert!(h2.wait());
        assert_eq!(h2.all_lines(), before);
        let _ = std::fs::remove_file(&p);
    }

    /// Modal G0/G1 moves of program lines. `lines`: G-code. Returns (is_g1, x, y) per move.
    fn moves(lines: &[String]) -> Vec<(bool, f64, f64)> {
        let (mut x, mut y, mut g1) = (0.0, 0.0, false);
        let mut out = Vec::new();
        for l in lines {
            if l.starts_with("G0") || l.starts_with("G1") {
                g1 = l.starts_with("G1");
            } else if !(l.starts_with('X') || l.starts_with('Y')) {
                continue;
            }
            x = words(std::slice::from_ref(l), 'X').first().copied().unwrap_or(x);
            y = words(std::slice::from_ref(l), 'Y').first().copied().unwrap_or(y);
            out.push((g1, x, y));
        }
        out
    }

    /// Rect 20x10 (X 10..30, Y 70..80 in G-code) and an open polyline (Y 90), one layer.
    const FILL_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">
  <g id="a"><rect x="10" y="20" width="20" height="10" style="stroke:#000000;fill:none"/>
  <polyline points="50,10 70,10 70,15" style="stroke:#000000;fill:none"/></g>
</svg>"##;

    /// Fill G1 segments of a program: moves not present in `base`, as (G0 start, G1 end) pairs.
    fn fill_pairs(all: &[String], base: &[String]) -> Vec<((f64, f64), (f64, f64))> {
        let (m, b) = (moves(all), moves(base));
        // outline and trailing moves are identical, fill moves sit between
        let pre = b.iter().zip(&m).take_while(|(a, c)| a == c).count();
        let extra = m.len() - b.len();
        m[pre..pre + extra].chunks(2).filter(|c| c.len() == 2 && !c[0].0 && c[1].0).map(|c| ((c[0].1, c[0].2), (c[1].1, c[1].2))).collect()
    }

    #[test]
    fn fill_and_cross_toggle_lines() {
        let _g = settings_guard();
        let p = lg_base::paths::test_dir("e2e_fill").join("in.svg");
        std::fs::write(&p, FILL_SVG).unwrap();
        let mut h = Harness::new();
        h.job.add_svg(&p, false, false);
        assert!(h.wait());
        h.frame(vec![]);
        let base = h.all_lines();
        let loads = h.file_loaded_events();
        assert!(loads >= 1, "initial load must raise FileLoaded");

        // fill on, 1 mm, 0 deg: horizontal rows inside the rect, none near the open polyline
        h.click_text(&tr_plain("ParamsDock.FillEnabled"));
        h.type_into("params-fill-distance", "1");
        h.type_into("params-fill-angle", "0");
        h.settle();
        let fill = h.all_lines();
        assert!(fill.len() > base.len());
        assert!(h.file_loaded_events() >= 1, "fill on must make the preview rebuild");
        let rows = fill_pairs(&fill, &base);
        assert_eq!(rows.len(), 10, "{rows:?}");
        for (a, b) in &rows {
            assert!((a.1 - b.1).abs() < 1e-3, "horizontal: {a:?} {b:?}");
            for q in [a, b] {
                assert!(q.0 >= 10.0 - 1e-3 && q.0 <= 30.0 + 1e-3 && q.1 >= 70.0 - 1e-3 && q.1 <= 80.0 + 1e-3, "inside rect: {q:?}");
            }
        }

        // cross on: vertical rows appear
        h.click_text(&tr_plain("ParamsDock.FillCross"));
        h.settle();
        let cross = h.all_lines();
        assert!(cross.len() > fill.len());
        assert!(h.file_loaded_events() >= 1, "cross on must make the preview rebuild");
        let rows = fill_pairs(&cross, &base);
        assert_eq!(rows.len(), 10 + 20, "{rows:?}");
        assert!(rows.iter().any(|(a, b)| (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() > 1.0), "vertical rows missing");

        // cross off: back to the plain fill, exactly
        h.click_text(&tr_plain("ParamsDock.FillCross"));
        h.settle();
        assert_eq!(h.all_lines(), fill);
        assert!(h.file_loaded_events() >= 1, "cross off must make the preview rebuild");

        // fill off: back to the plain outline, exactly
        h.click_text(&tr_plain("ParamsDock.FillEnabled"));
        h.settle();
        assert_eq!(h.all_lines(), base);
        assert!(h.file_loaded_events() >= 1, "fill off must make the preview rebuild");
        let _ = std::fs::remove_file(&p);
    }

    /// Two rects 20x10 (X 10..30 and 50..70, Y 70..80 in G-code) and an open polyline, one layer.
    const TWO_RECTS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">
  <g id="a"><rect x="10" y="20" width="20" height="10" style="stroke:#000000;fill:none"/>
  <rect x="50" y="20" width="20" height="10" style="stroke:#000000;fill:none"/>
  <polyline points="50,10 70,10 70,15" style="stroke:#000000;fill:none"/></g>
</svg>"##;

    /// Pen-down G1 segments of a program as sorted unordered endpoint pairs (rounded to 1e-3).
    fn burned(lines: &[String]) -> Vec<[i64; 4]> {
        let r = |v: f64| (v * 1e3).round() as i64;
        let m = moves(lines);
        let mut v: Vec<[i64; 4]> = m
            .windows(2)
            .filter(|w| w[1].0)
            .map(|w| {
                let (p, q) = ((r(w[0].1), r(w[0].2)), (r(w[1].1), r(w[1].2)));
                let (lo, hi) = if p <= q { (p, q) } else { (q, p) };
                [lo.0, lo.1, hi.0, hi.1]
            })
            .collect();
        v.sort();
        v
    }

    #[test]
    fn fill_per_shape_toggle_keeps_lines() {
        let _g = settings_guard();
        let p = lg_base::paths::test_dir("e2e_pershape").join("in.svg");
        std::fs::write(&p, TWO_RECTS_SVG).unwrap();
        let mut h = Harness::new();
        h.job.add_svg(&p, false, false);
        assert!(h.wait());
        h.frame(vec![]);
        let base = h.all_lines();

        // fill on (per shape is on by default)
        h.click_text(&tr_plain("ParamsDock.FillEnabled"));
        h.type_into("params-fill-distance", "1");
        h.type_into("params-fill-angle", "0");
        h.settle();
        let a = h.all_lines();
        let rows_a = fill_pairs(&a, &base);
        assert_eq!(rows_a.len(), 20, "{rows_a:?}");
        // left rect (x < 40) fully before right rect
        assert!(rows_a[..10].iter().all(|(s, e)| s.0 < 40.0 && e.0 < 40.0), "{rows_a:?}");
        assert!(rows_a[10..].iter().all(|(s, e)| s.0 > 40.0 && e.0 > 40.0), "{rows_a:?}");

        // per shape off: row by row, shapes interleave
        h.click_text(&tr_plain("ParamsDock.FillPerShape"));
        h.settle();
        let b = h.all_lines();
        assert_ne!(a, b);
        assert_eq!(burned(&a), burned(&b));
        let rows_b = fill_pairs(&b, &base);
        assert_eq!(rows_b.len(), 20);
        assert!(rows_b[0].0.0 < 40.0 && rows_b[1].0.0 > 40.0, "{rows_b:?}");

        // on again: identical to the first program
        h.click_text(&tr_plain("ParamsDock.FillPerShape"));
        h.settle();
        assert_eq!(h.all_lines(), a);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn no_smin_input() {
        let _g = settings_guard();
        let p = write_svg("smin");
        let mut h = Harness::new();
        h.job.add_svg(&p, false, false);
        assert!(h.wait());
        h.frame(vec![]);
        assert!(h.shows(&ctl_text(P, "LblSmax")));
        assert!(!h.shows(&ctl_text(P, "LblSmin")));
        assert!(h.ctx.read_response(egui::Id::unique("params-smin")).is_none());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn without_hardware_pwm_mode_switches_per_pen_down() {
        let _g = settings_guard();
        lg_base::settings::set("Support Hardware PWM", false);
        let p = write_svg("nopwm");
        let mut h = Harness::new();
        h.job.add_svg(&p, false, false);
        assert!(h.wait());
        // no power input without PWM
        assert!(!h.shows(&ctl_text(P, "LblSmax")));
        let labels: Vec<String> = (0..3).map(|i| h.svg().layer_text(i)).collect();
        h.pick(&labels[0], &labels[1]);
        h.type_into("params-border", "700");
        h.pick("M3 - Constant Power", "M4 - Dynamic Power");
        h.settle();
        let all = h.all_lines();
        // laser on = M3/M4 line per pen down, laser off = M5
        let cut = h.layer_lines(1).unwrap();
        assert!(cut.iter().filter(|l| l.starts_with("M4")).count() == 2, "{cut:?}");
        assert!(!cut.iter().any(|l| l.starts_with("M3")), "{cut:?}");
        assert!(words(&cut, 'F').contains(&700.0), "{cut:?}");
        let eng = h.layer_lines(2).unwrap();
        assert!(eng.iter().filter(|l| l.starts_with("M3")).count() == 2, "{eng:?}");
        assert!(words(&eng, 'F').contains(&1000.0), "{eng:?}");
        assert!(!all.iter().any(|l| l == "M4 S0" || l == "M3 S0"), "no PWM mode lines: {all:?}");
        let _ = std::fs::remove_file(&p);
    }
}
