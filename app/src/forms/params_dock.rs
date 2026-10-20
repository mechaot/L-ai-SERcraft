//! "Parameters" dock (right side of the main window). Replaces the modal `SvgToGCodeForm`
//! (`SvgConverter/ConvertSizeAndOptionForm.cs`): SVG is loaded at once, power/speed are tweaked
//! here and "Apply" regenerates the G-code. Not part of the C# app (intentional deviation).

use eframe::egui::{self, Grid, RichText, Ui};
use lasergrbl_core::GrblCore;
use lg_dotnet::Decimal;
use lg_gcode::GrblFile;
use lg_services::material_db::{culture_compare, MaterialRow};
use lg_svg2gcode::ColorFilter;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::forms::form_util::*;
use crate::forms::material::default_db;
use crate::ui::dialog::{MessageBox, MessageBoxHost, MessageBoxIcon};
use crate::ui::widgets::{flat_combo, grbl_group_box, ImageButton};
use crate::ui::{tr, tr_fmt, tr_plain};

/// i18n prefix of the old SVG form (captions and tooltips are reused).
const P: &str = "SvgConverter.ConvertSizeAndOptionForm.";

/// Setting that stores the selected material row id.
const MATERIAL_KEY: &str = "ParametersDock.MaterialId";

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
    Busy,
}

/// What the dock asks the main window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamsAction {
    /// Open the material database editor.
    OpenMaterialEditor,
}

/// SVG the dock regenerates G-code from.
struct SvgSource {
    /// Shared program.
    file: SharedFile,
    /// SVG path.
    path: String,
    /// Import was an append.
    append: bool,
    /// Commands that existed before the import (kept on every re-apply when `append`).
    base_len: usize,
}

/// Running G-code generation.
struct ApplyJob {
    handle: JoinHandle<()>,
    /// Set by the worker when the conversion succeeded.
    ok: Arc<AtomicBool>,
}

/// Parameters dock state.
pub struct ParamsDock {
    limits: Limits,
    support_pwm: bool,
    border_speed: i32,
    min_power: i32,
    max_power: i32,
    /// Index into [`LASER_OPTIONS`].
    laser_on: usize,
    /// Index into [`FILTER_OPTIONS`].
    filter: usize,
    /// Upper limit of the speed input.
    border_max: i32,
    /// Upper limit of the S-MAX input (also the 100 % base of material power).
    power_max_limit: i32,
    /// Visible material rows, sorted for the dropdown.
    materials: Vec<MaterialRow>,
    /// Selected index into `materials`.
    material: Option<usize>,
    source: Option<SvgSource>,
    /// Edited since the last successful generation.
    dirty: bool,
    job: Option<ApplyJob>,
    last_error: Option<String>,
    msgs: MessageBoxHost<Msg>,
}

impl ParamsDock {
    /// Build the dock: values from settings, material list, restored material id.
    ///
    /// Returns the dock without a source.
    pub fn new() -> Self {
        let mut d = ParamsDock {
            limits: Limits::from_config(),
            support_pwm: true,
            border_speed: 1000,
            min_power: 0,
            max_power: 0,
            laser_on: 0,
            filter: 0,
            border_max: 0,
            power_max_limit: 0,
            materials: Vec::new(),
            material: None,
            source: None,
            dirty: false,
            job: None,
            last_error: None,
            msgs: MessageBoxHost::default(),
        };
        d.load_values();
        d.refresh_materials();
        // restore selection by id, values stay as stored
        let id: Option<String> = lg_base::settings::get(MATERIAL_KEY, None);
        d.material = id.and_then(|id| d.materials.iter().position(|r| r.id == id));
        d
    }

    /// Load the four persisted values (same keys/defaults as the old dialog) and refresh limits.
    fn load_values(&mut self) {
        self.limits = Limits::from_config();
        self.support_pwm = lg_base::settings::get("Support Hardware PWM", true);
        self.border_speed = lg_base::settings::get("GrayScaleConversion.VectorizeOptions.BorderSpeed", 1000);
        let laser_on_s: String = lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.LaserOn", "M3".to_owned());
        self.laser_on = if laser_on_s == "M3" || !self.limits.laser_mode { 0 } else { 1 };
        self.min_power = lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.PowerMin", 0);
        self.max_power = lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.PowerMax", self.limits.max_pwm);
        self.border_max = self.limits.max_rate_x;
        self.power_max_limit = self.limits.max_pwm;
    }

    /// Set a new SVG source and generate at once.
    ///
    /// `file`: shared program. `path`: SVG path. `append`: keep the commands already in `file`.
    /// `from_settings`: reload the values from settings first (project restore stored them just before).
    pub fn load_svg(&mut self, file: SharedFile, path: &str, append: bool, from_settings: bool) {
        if from_settings {
            self.load_values();
        }
        // commands that stay in front of the import
        let base_len = if append { file.read().map(|f| f.count()).unwrap_or(0) } else { 0 };
        self.source = Some(SvgSource { file, path: path.to_owned(), append, base_len });
        self.dirty = true;
        self.last_error = None;
        self.apply();
    }

    /// Forget the SVG (another file type was opened). Values stay, Apply gets disabled.
    pub fn clear_source(&mut self) {
        self.source = None;
        self.dirty = false;
        self.last_error = None;
    }

    /// Persist the settings and start the G-code generation on a worker.
    ///
    /// Returns false when nothing was started (no source, or file busy: busy box queued).
    pub fn apply(&mut self) -> bool {
        use lg_base::settings::set;
        if self.job.is_some() {
            return false;
        }
        let Some(src) = &self.source else { return false };
        set("GrayScaleConversion.VectorizeOptions.BorderSpeed", self.border_speed);
        set("GrayScaleConversion.Gcode.LaserOptions.PowerMax", self.max_power);
        set("GrayScaleConversion.Gcode.LaserOptions.PowerMin", self.min_power);
        set("GrayScaleConversion.Gcode.LaserOptions.LaserOn", LASER_OPTIONS[self.laser_on].1);

        let filter = FILTER_OPTIONS[self.filter];
        let (path, append, base_len) = (src.path.clone(), src.append, src.base_len);
        let ok = Arc::new(AtomicBool::new(false));
        let ok_w = ok.clone();
        let handle = GrblFile::spawn_load(&src.file, move |f| {
            // drop the previous import, keep what was there before it
            if append {
                f.truncate_list(base_len);
            }
            f.load_imported_svg(&path, append, filter)?;
            ok_w.store(true, Ordering::SeqCst);
            Ok(())
        });
        match handle {
            Some(handle) => {
                self.job = Some(ApplyJob { handle, ok });
                self.last_error = None;
                true
            }
            None => {
                self.msgs.push(Msg::Busy, already_loading_box());
                false
            }
        }
    }

    /// Returns true when the G-code is out of date or being generated (SVG loaded).
    pub fn needs_apply(&self) -> bool {
        self.source.is_some() && (self.dirty || self.job.is_some())
    }

    /// Returns true when edits wait and no generation runs (an `apply()` call would start one).
    pub fn is_dirty_idle(&self) -> bool {
        self.source.is_some() && self.dirty && self.job.is_none()
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
            self.dirty = false;
            self.last_error = None;
        } else {
            self.last_error = Some(tr_plain("ParamsDock.ApplyFailed"));
        }
        Some(ok)
    }

    /// Rebuild the dropdown rows from the material database (after the editor closed).
    pub fn refresh_materials(&mut self) {
        let selected_id = self.material.and_then(|i| self.materials.get(i)).map(|r| r.id.clone());
        let db = default_db();
        let mut rows: Vec<MaterialRow> = db.lock().unwrap_or_else(|e| e.into_inner()).materials.rows().iter().filter(|r| r.visible).cloned().collect();
        rows.sort_by(|a, b| {
            culture_compare(&a.model, &b.model)
                .then_with(|| culture_compare(&a.material, &b.material))
                .then_with(|| culture_compare(&a.action, &b.action))
                .then_with(|| culture_compare(&a.thickness, &b.thickness))
        });
        self.materials = rows;
        // keep selection when the row survived
        self.material = selected_id.and_then(|id| self.materials.iter().position(|r| r.id == id));
    }

    /// Dropdown text of a material row.
    ///
    /// `r`: row. Returns "model · material · action · thickness".
    fn material_text(r: &MaterialRow) -> String {
        format!("{} · {} · {} · {}", r.model, r.material, r.action, r.thickness)
    }

    /// Take speed and power of material row `i` (C# `BtnPSHelper_Click` tail) and mark dirty.
    ///
    /// `i`: index into `materials`.
    fn select_material(&mut self, i: usize) {
        let Some(r) = self.materials.get(i) else { return };
        self.border_speed = r.speed;
        self.max_power = self.power_max_limit * r.power / 100;
        self.material = Some(i);
        lg_base::settings::set(MATERIAL_KEY, Some(r.id.clone()));
        self.dirty = true;
    }

    /// Drop the material selection (dropdown "(none)"); values stay.
    fn deselect_material(&mut self) {
        self.material = None;
        lg_base::settings::delete(MATERIAL_KEY);
    }

    /// True when speed or S-MAX differ from the selected material row.
    ///
    /// Returns false without a selection.
    fn material_modified(&self) -> bool {
        self.material.and_then(|i| self.materials.get(i)).is_some_and(|r| r.speed != self.border_speed || self.power_max_limit * r.power / 100 != self.max_power)
    }

    /// C# `IIMinPowerCurrentValueChanged` (user edit).
    fn on_min_power_changed(&mut self) {
        if self.max_power <= self.min_power {
            self.max_power = self.min_power + 1;
        }
    }

    /// C# `IIMaxPowerCurrentValueChanged` (user edit).
    fn on_max_power_changed(&mut self) {
        if self.min_power >= self.max_power {
            self.min_power = self.max_power - 1;
        }
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
        if !self.limits.laser_mode && LASER_OPTIONS[self.laser_on].1 == "M4" {
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
    /// `ui`: panel ui. `core`: for the Apply enable state. Returns requests for the main window.
    pub fn show(&mut self, ui: &mut Ui, core: &GrblCore) -> Vec<ParamsAction> {
        let mut actions = Vec::new();
        // config changes after connecting
        self.limits = Limits::from_config();
        self.border_max = self.limits.max_rate_x;
        self.power_max_limit = self.limits.max_pwm;
        let mut edited = false;

        ui.label(RichText::new(tr_plain("ParamsDock.Title")).strong());
        ui.add_space(2.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            self.material_group(ui, &mut actions, &mut edited);
            ui.add_space(4.0);
            self.speed_group(ui, &mut edited);
            ui.add_space(4.0);
            self.laser_group(ui, &mut edited);
            ui.add_space(4.0);
            self.filter_group(ui, &mut edited);
            ui.add_space(8.0);
            self.apply_row(ui, core);
        });
        if edited {
            self.dirty = true;
        }
        actions
    }

    /// Material dropdown, info text and editor button.
    ///
    /// `ui`: parent. `actions`: collects editor requests. `edited`: set on a material pick.
    fn material_group(&mut self, ui: &mut Ui, actions: &mut Vec<ParamsAction>, edited: &mut bool) {
        grbl_group_box(ui, &tr_plain("ParamsDock.Material"), |ui| {
            let mut items = vec![tr_plain("ParamsDock.MaterialNone")];
            items.extend(self.materials.iter().map(Self::material_text));
            let mut sel = Some(self.material.map_or(0, |i| i + 1));
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 44.0).max(60.0);
                let r = flat_combo(ui, "params-material", &mut sel, &items, w);
                if r.changed() {
                    match sel.unwrap_or(0) {
                        0 => self.deselect_material(),
                        n => {
                            self.select_material(n - 1);
                            *edited = true;
                        }
                    }
                }
                if ui.add(ImageButton::new("mdi-database").size(egui::Vec2::splat(17.0))).on_hover_text(tr("ParamsDock.EditMaterials")).clicked() {
                    actions.push(ParamsAction::OpenMaterialEditor);
                }
            });
            // passes + remarks of the selected row
            if let Some(r) = self.material.and_then(|i| self.materials.get(i)) {
                let mut info = tr_fmt("ParamsDock.Passes", &[&r.cycles]);
                if !r.remarks().is_empty() {
                    info = format!("{info} - {}", r.remarks());
                }
                ui.label(RichText::new(info).small());
                if self.material_modified() {
                    ui.label(RichText::new(tr_plain("ParamsDock.Modified")).small().italics());
                }
            }
        });
    }

    /// Border speed input.
    ///
    /// `ui`: parent. `edited`: set on a committed edit.
    fn speed_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        let lim_rate = self.border_max;
        grbl_group_box(ui, &ctl_text(P, "GbSpeed"), |ui| {
            ui.horizontal(|ui| {
                label(ui, P, "LblBorderTracing");
                if int_input(ui, "params-border", &mut self.border_speed, 1, lim_rate, false, 55.0).is_some() {
                    *edited = true;
                }
                label(ui, P, "LblBorderTracingmm");
            });
        });
    }

    /// Laser mode, S-MIN and S-MAX.
    ///
    /// `ui`: parent. `edited`: set on any change.
    fn laser_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        let pwm_max = self.power_max_limit;
        grbl_group_box(ui, &ctl_text(P, "GbLaser"), |ui| {
            let items: Vec<String> = LASER_OPTIONS.iter().map(|o| o.0.to_owned()).collect();
            // laser mode row
            ui.horizontal(|ui| {
                label(ui, P, "label18");
                let before = self.laser_on;
                let mut sel = Some(self.laser_on);
                let w = (ui.available_width() - 40.0).max(60.0);
                flat_combo(ui, "params-laser-on", &mut sel, &items, w);
                self.laser_on = sel.unwrap_or(before);
                if info_button(ui, &ctl_tip(P, "BtnOnOffInfo")) {
                    open_link("https://lasergrbl.com/usage/raster-image-import/target-image-size-and-laser-options/#laser-modes");
                }
                if self.laser_on != before {
                    *edited = true;
                    self.on_laser_mode_changed();
                }
            });
            if !self.support_pwm {
                return;
            }
            Grid::new("params-power").num_columns(4).spacing([8.0, 4.0]).show(ui, |ui| {
                // S-MIN
                label(ui, P, "LblSmin");
                if int_input(ui, "params-smin", &mut self.min_power, 0, 999, false, 43.0).is_some() {
                    self.on_min_power_changed();
                    *edited = true;
                }
                ui.label(self.perc_text(self.min_power));
                if info_button(ui, &ctl_tip(P, "BtnModulationInfo")) {
                    open_link("https://lasergrbl.com/usage/raster-image-import/target-image-size-and-laser-options/#power-modulation");
                }
                ui.end_row();
                // S-MAX
                label(ui, P, "LblSmax");
                if int_input(ui, "params-smax", &mut self.max_power, 1, pwm_max, false, 43.0).is_some() {
                    self.on_max_power_changed();
                    *edited = true;
                }
                ui.label(self.perc_text(self.max_power));
                ui.end_row();
            });
        });
    }

    /// Color filter combo.
    ///
    /// `ui`: parent. `edited`: set on change.
    fn filter_group(&mut self, ui: &mut Ui, edited: &mut bool) {
        grbl_group_box(ui, &ctl_text(P, "gbFilter"), |ui| {
            let items: Vec<String> = FILTER_OPTIONS.iter().map(|f| filter_name(*f).to_owned()).collect();
            ui.horizontal(|ui| {
                label(ui, P, "labelFilter");
                let mut sel = Some(self.filter);
                let w = (ui.available_width() - 40.0).max(60.0);
                if flat_combo(ui, "params-filter", &mut sel, &items, w).changed() {
                    *edited = true;
                }
                self.filter = sel.unwrap_or(self.filter);
                if info_button(ui, &ctl_tip(P, "BtnColorFilter")) {
                    open_link("https://lasergrbl.com/usage/raster-image-import/target-image-size-and-laser-options/#color-filter");
                }
            });
        });
    }

    /// Apply button and status lines.
    ///
    /// `ui`: parent. `core`: for `can_load_new_file`.
    fn apply_row(&mut self, ui: &mut Ui, core: &GrblCore) {
        let enabled = self.dirty && self.source.is_some() && self.job.is_none() && core.can_load_new_file();
        let btn = egui::Button::new(tr_plain("ParamsDock.Apply")).min_size(egui::vec2(ui.available_width(), 26.0));
        if ui.add_enabled(enabled, btn).clicked() {
            self.apply();
        }
        // status line
        let status = match &self.source {
            Some(s) if self.job.is_some() => format!("{} - {}", file_name(&s.path), tr_plain("ParamsDock.Generating")),
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

/// File name part of a path text.
///
/// `p`: path. Returns the last component, or `p` when it has none.
fn file_name(p: &str) -> String {
    std::path::Path::new(p).file_name().map_or_else(|| p.to_owned(), |n| n.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::RwLock;
    use std::time::{Duration, Instant};

    fn new_file() -> SharedFile {
        Arc::new(RwLock::new(GrblFile::new()))
    }

    fn write_svg(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("lg-params-{tag}-{}.svg", std::process::id()));
        std::fs::write(&p, r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><path d="M1 1 L19 1 L19 19 Z" stroke="black" fill="none"/></svg>"#).unwrap();
        p
    }

    /// Poll until the worker finished (5 s limit).
    fn wait(d: &mut ParamsDock) -> bool {
        let t = Instant::now();
        loop {
            if let Some(ok) = d.poll() {
                return ok;
            }
            assert!(t.elapsed() < Duration::from_secs(5), "worker timeout");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn row(id: &str, power: i32, speed: i32) -> MaterialRow {
        MaterialRow { id: id.into(), visible: true, model: "M".into(), material: "Wood".into(), thickness: "3".into(), action: "Cut".into(), power, speed, cycles: 2, remarks: None }
    }

    #[test]
    fn defaults_from_empty_settings() {
        let _g = settings_guard();
        let d = ParamsDock::new();
        assert_eq!((d.border_speed, d.min_power, d.max_power), (1000, 0, 1000));
        // empty config reports laser mode on, stored "M3" wins
        assert_eq!(d.laser_on, 0);
        assert_eq!(d.filter, 0);
        assert!(d.support_pwm);
        assert!(!d.needs_apply());
    }

    #[test]
    fn laser_mode_m4_restored_only_in_laser_mode() {
        let _g = settings_guard();
        lg_base::settings::set("GrayScaleConversion.Gcode.LaserOptions.LaserOn", "M4");
        assert_eq!(ParamsDock::new().laser_on, 1);
    }

    #[test]
    fn power_limits_follow_user_edits() {
        let _g = settings_guard();
        let mut d = ParamsDock::new();
        d.min_power = 1000;
        d.on_min_power_changed();
        assert_eq!(d.max_power, 1001);
        d.max_power = 0;
        d.on_max_power_changed();
        assert_eq!(d.min_power, -1);
    }

    #[test]
    fn percent_text_like_dotnet() {
        let _g = settings_guard();
        let mut d = ParamsDock::new();
        assert_eq!(d.perc_text(1000), "100.0 %");
        d.limits.max_pwm = 0;
        assert_eq!(d.perc_text(10), "");
    }

    #[test]
    fn material_pick_scales_power_and_tracks_modified() {
        let _g = settings_guard();
        let mut d = ParamsDock::new();
        d.materials = vec![row("a", 25, 750)];
        d.select_material(0);
        assert_eq!((d.border_speed, d.max_power), (750, 250));
        assert!(d.dirty && !d.material_modified());
        d.border_speed = 800;
        assert!(d.material_modified());
        // id persisted
        assert_eq!(lg_base::settings::get(MATERIAL_KEY, None::<String>), Some("a".to_owned()));
        d.deselect_material();
        assert!(!d.material_modified());
        assert_eq!(lg_base::settings::get(MATERIAL_KEY, None::<String>), None);
    }

    #[test]
    fn apply_persists_settings_and_loads() {
        let _g = settings_guard();
        let p = write_svg("persist");
        let file = new_file();
        let mut d = ParamsDock::new();
        d.border_speed = 1500;
        d.min_power = 10;
        d.max_power = 800;
        d.laser_on = 1;
        d.load_svg(file.clone(), p.to_str().unwrap(), false, false);
        assert!(d.job_running());
        assert!(wait(&mut d));
        assert!(file.read().unwrap().count() > 0);
        let get = |k: &str| lg_base::settings::get(k, -1i32);
        assert_eq!(get("GrayScaleConversion.VectorizeOptions.BorderSpeed"), 1500);
        assert_eq!(get("GrayScaleConversion.Gcode.LaserOptions.PowerMin"), 10);
        assert_eq!(get("GrayScaleConversion.Gcode.LaserOptions.PowerMax"), 800);
        assert_eq!(lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.LaserOn", String::new()), "M4");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn apply_clears_dirty_and_edit_sets_it() {
        let _g = settings_guard();
        let p = write_svg("dirty");
        let mut d = ParamsDock::new();
        d.load_svg(new_file(), p.to_str().unwrap(), false, false);
        assert!(d.needs_apply());
        assert!(wait(&mut d));
        assert!(!d.dirty && !d.needs_apply());
        // edit marks dirty again
        d.dirty = true;
        assert!(d.needs_apply() && d.is_dirty_idle());
        // clear_source disables apply
        d.clear_source();
        assert!(!d.needs_apply());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn append_reapply_does_not_duplicate() {
        let _g = settings_guard();
        let p = write_svg("append");
        let file = new_file();
        for l in ["G0 X1", "G1 X2", "G1 X3"] {
            file.write().unwrap().commands_mut().push(lg_gcode::GrblCommand::new(l));
        }
        let mut d = ParamsDock::new();
        d.load_svg(file.clone(), p.to_str().unwrap(), true, false);
        assert!(wait(&mut d));
        let first = file.read().unwrap().count();
        assert!(first > 3);
        d.dirty = true;
        assert!(d.apply());
        assert!(wait(&mut d));
        assert_eq!(file.read().unwrap().count(), first);
        // old head untouched
        let g = file.read().unwrap();
        assert_eq!(g.get(0).map(|c| c.command()), Some("G0 X1"));
        assert_eq!(g.get(2).map(|c| c.command()), Some("G1 X3"));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn apply_without_source_does_nothing() {
        let _g = settings_guard();
        let mut d = ParamsDock::new();
        assert!(!d.apply());
        assert!(d.poll().is_none());
    }
}
