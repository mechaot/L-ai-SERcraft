#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Ports `SvgConverter/ConvertSizeAndOptionForm.cs` (+ Designer), class `SvgToGCodeForm`: SVG import options.

use eframe::egui::{self, Grid, Ui};
use lg_dotnet::Decimal;
use lg_gcode::GrblFile;
use lg_svg2gcode::ColorFilter;
use std::thread::JoinHandle;

use crate::forms::form_util::*;
use crate::ui::dialog::{MessageBox, MessageBoxHost, MessageBoxIcon};
use crate::ui::widgets::{flat_combo, grbl_group_box};
use crate::ui::{tr, tr_plain};

/// i18n prefix of the form (sub-folder form).
const P: &str = "SvgConverter.ConvertSizeAndOptionForm.";

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

/// How the form ended.
#[derive(Debug)]
pub struct SvgOutcome {
    /// OK pressed (`DialogResult.OK`): settings stored and the import started.
    pub accepted: bool,
    /// Worker running `GrblFile::load_imported_svg` (None when cancelled or the file was busy).
    pub load: Option<JoinHandle<()>>,
    /// Usage counter delta: C# `GrblCore.OpenFile` does `UsageCounters.SvgFile++` after the dialog closed,
    /// even on Cancel. Caller adds this to the counter.
    pub usage_svg_file: i32,
}

/// Message box tags.
#[derive(Clone, Copy)]
enum Msg {
    WrongLaserMode,
    Busy,
}

/// SVG import options dialog state.
pub struct SvgImport {
    file: SharedFile,
    filename: String,
    append: bool,
    support_pwm: bool,
    limits: Limits,
    border_speed: i32,
    min_power: i32,
    max_power: i32,
    /// `CBLaserON` index into [`LASER_OPTIONS`].
    laser_on: usize,
    /// `CBFilter` index into [`FILTER_OPTIONS`].
    filter: usize,
    /// `IIBorderTracing.MaxValue` / `IIMaxPower.MaxValue`.
    border_max: i32,
    power_max_limit: i32,
    msgs: MessageBoxHost<Msg>,
    /// Outcome waiting for the busy message box.
    pending: Option<SvgOutcome>,
    ps_helper_requested: bool,
}

impl SvgImport {
    /// `SvgToGCodeForm.CreateAndShowDialog(core, filename, append)` setup: ctor + `ShowDialogForm` value loading.
    ///
    /// `file`: shared program (C# `core.LoadedFile`). `filename`: SVG path. `append`: keep old commands.
    pub fn open(file: SharedFile, filename: &str, append: bool) -> Self {
        let limits = Limits::from_config();
        // `bool supportPWM = Settings.GetObject("Support Hardware PWM", true)`
        let support_pwm: bool = lg_base::settings::get("Support Hardware PWM", true);

        // ShowDialogForm: load persisted values
        let border_speed: i32 = lg_base::settings::get("GrayScaleConversion.VectorizeOptions.BorderSpeed", 1000);
        let laser_on_s: String = lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.LaserOn", "M3".to_owned());
        let laser_on = if laser_on_s == "M3" || !limits.laser_mode { 0 } else { 1 };
        let min_power: i32 = lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.PowerMin", 0);
        let max_power: i32 = lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.PowerMax", limits.max_pwm);

        SvgImport {
            file,
            filename: filename.to_owned(),
            append,
            support_pwm,
            limits,
            border_speed,
            min_power,
            max_power,
            laser_on,
            filter: 0,
            // AssignMinMaxLimit
            border_max: limits.max_rate_x,
            power_max_limit: limits.max_pwm,
            msgs: MessageBoxHost::default(),
            pending: None,
            ps_helper_requested: false,
        }
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

    /// C# `CBLaserON_SelectedIndexChanged` (user selection).
    fn on_laser_mode_changed(&mut self) {
        // warn: M4 selected but firmware not in laser mode
        if !self.limits.laser_mode && LASER_OPTIONS[self.laser_on].1 == "M4" {
            self.msgs.push(Msg::WrongLaserMode, MessageBox::new(tr("Strings.WarnWrongLaserMode"), tr("Strings.WarnWrongLaserModeTitle")).icon(MessageBoxIcon::Warning));
        }
    }

    /// True once after the user clicked the "Laser & Material DB" button (`BtnPSHelper_Click`).
    /// The host opens its material dialog and then calls [`SvgImport::apply_material`].
    ///
    /// Returns and clears the request flag.
    pub fn take_ps_helper_request(&mut self) -> bool {
        std::mem::take(&mut self.ps_helper_requested)
    }

    /// Apply a chosen material row (C# `BtnPSHelper_Click` tail).
    ///
    /// `speed`: `row.Speed` (mm/min). `power_percent`: `row.Power` (0..100).
    pub fn apply_material(&mut self, speed: i32, power_percent: i32) {
        self.border_speed = speed;
        self.max_power = self.power_max_limit * power_percent / 100;
    }

    /// OK: persist options and start `LoadImportedSVG` on a worker (C# `CreateAndShowDialog` after `DialogResult.OK`).
    fn accept(&mut self) -> SvgOutcome {
        use lg_base::settings::set;
        set("GrayScaleConversion.VectorizeOptions.BorderSpeed", self.border_speed);
        set("GrayScaleConversion.Gcode.LaserOptions.PowerMax", self.max_power);
        set("GrayScaleConversion.Gcode.LaserOptions.PowerMin", self.min_power);
        set("GrayScaleConversion.Gcode.LaserOptions.LaserOn", LASER_OPTIONS[self.laser_on].1);

        let filter = FILTER_OPTIONS[self.filter];
        let (name, append) = (self.filename.clone(), self.append);
        let load = GrblFile::spawn_load(&self.file, move |f| f.load_imported_svg(&name, append, filter));
        // C# SafeLoadFile -> CheckInUse(true) message
        if load.is_none() {
            self.msgs.push(Msg::Busy, already_loading_box());
        }
        SvgOutcome { accepted: true, load, usage_svg_file: 1 }
    }

    /// Draw the modal form; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome once the form closed (and a busy message was dismissed).
    pub fn show(&mut self, ctx: &egui::Context) -> Option<SvgOutcome> {
        // closing: wait for the busy message box
        if self.pending.is_some() {
            if self.msgs.is_open() {
                self.msgs.show(ctx);
                return None;
            }
            return self.pending.take();
        }
        let title = tr_plain(&format!("{P}$this.Text"));
        let ev = run_modal(ctx, "svg-import", &title, 349.0, true, |ui| self.ui(ui));
        // message boxes sit on top of the form
        if self.msgs.is_open() {
            self.msgs.show(ctx);
        }
        match ev? {
            ModalEvent::Done(true) => {
                let o = self.accept();
                self.pending = Some(o);
                ctx.request_repaint();
                None
            }
            ModalEvent::Done(false) | ModalEvent::Closed => Some(SvgOutcome { accepted: false, load: None, usage_svg_file: 1 }),
        }
    }

    /// Form body. Returns `Some(true)` on Create, `Some(false)` on Cancel.
    fn ui(&mut self, ui: &mut Ui) -> Option<bool> {
        let lim_rate = self.border_max;
        let pwm_max = self.power_max_limit;

        // speed group
        grbl_group_box(ui, &ctl_text(P, "GbSpeed"), |ui| {
            ui.set_width(335.0);
            ui.horizontal(|ui| {
                label(ui, P, "LblBorderTracing");
                int_input(ui, "svg-border", &mut self.border_speed, 1, lim_rate, false, 55.0);
                label(ui, P, "LblBorderTracingmm");
                // push the info button to the right edge
                ui.add_space((ui.available_width() - 17.0).max(0.0));
                if info_button(ui, &ctl_tip(P, "BtnPSHelper")) {
                    self.ps_helper_requested = true;
                }
            });
        });
        ui.add_space(4.0);

        // laser group
        grbl_group_box(ui, &ctl_text(P, "GbLaser"), |ui| {
            ui.set_width(335.0);
            let items: Vec<String> = LASER_OPTIONS.iter().map(|o| o.0.to_owned()).collect();
            Grid::new("svg-laser").num_columns(4).spacing([8.0, 4.0]).show(ui, |ui| {
                // row 0: laser mode
                label(ui, P, "label18");
                let before = self.laser_on;
                let mut sel = Some(self.laser_on);
                flat_combo(ui, "svg-laser-on", &mut sel, &items, 157.0);
                self.laser_on = sel.unwrap_or(before);
                ui.label("");
                if info_button(ui, &ctl_tip(P, "BtnOnOffInfo")) {
                    open_link("https://lasergrbl.com/usage/raster-image-import/target-image-size-and-laser-options/#laser-modes");
                }
                ui.end_row();
                if self.laser_on != before {
                    self.on_laser_mode_changed();
                }

                // row 1: S-MIN
                if self.support_pwm {
                    label(ui, P, "LblSmin");
                    if int_input(ui, "svg-smin", &mut self.min_power, 0, 999, false, 43.0).is_some() {
                        self.on_min_power_changed();
                    }
                } else {
                    ui.label("");
                    ui.label("");
                }
                ui.label(self.perc_text(self.min_power));
                if self.support_pwm && info_button(ui, &ctl_tip(P, "BtnModulationInfo")) {
                    open_link("https://lasergrbl.com/usage/raster-image-import/target-image-size-and-laser-options/#power-modulation");
                }
                ui.end_row();

                // row 2: S-MAX
                if self.support_pwm {
                    label(ui, P, "LblSmax");
                    if int_input(ui, "svg-smax", &mut self.max_power, 1, pwm_max, false, 43.0).is_some() {
                        self.on_max_power_changed();
                    }
                } else {
                    ui.label("");
                    ui.label("");
                }
                ui.label(self.perc_text(self.max_power));
                ui.end_row();
            });
        });
        ui.add_space(4.0);

        // filter group
        grbl_group_box(ui, &ctl_text(P, "gbFilter"), |ui| {
            ui.set_width(335.0);
            let items: Vec<String> = FILTER_OPTIONS.iter().map(|f| filter_name(*f).to_owned()).collect();
            ui.horizontal(|ui| {
                label(ui, P, "labelFilter");
                let mut sel = Some(self.filter);
                flat_combo(ui, "svg-filter", &mut sel, &items, 195.0);
                self.filter = sel.unwrap_or(self.filter);
                ui.add_space((ui.available_width() - 17.0).max(0.0));
                if info_button(ui, &ctl_tip(P, "BtnColorFilter")) {
                    open_link("https://lasergrbl.com/usage/raster-image-import/target-image-size-and-laser-options/#color-filter");
                }
            });
        });
        ui.add_space(6.0);
        ok_cancel_row(ui, P, "BtnCreate", Some("BtnCancel"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};

    fn new_file() -> SharedFile {
        Arc::new(RwLock::new(GrblFile::new()))
    }

    #[test]
    fn defaults_from_empty_settings() {
        let _g = settings_guard();
        let f = SvgImport::open(new_file(), "x.svg", false);
        assert_eq!((f.border_speed, f.min_power, f.max_power), (1000, 0, 1000));
        // empty config reports laser mode on, stored "M3" wins
        assert_eq!(f.laser_on, 0);
        assert_eq!(f.filter, 0);
        assert!(f.support_pwm);
    }

    #[test]
    fn laser_mode_m4_restored_only_in_laser_mode() {
        let _g = settings_guard();
        lg_base::settings::set("GrayScaleConversion.Gcode.LaserOptions.LaserOn", "M4");
        assert_eq!(SvgImport::open(new_file(), "x.svg", false).laser_on, 1);
    }

    #[test]
    fn power_limits_follow_user_edits() {
        let _g = settings_guard();
        let mut f = SvgImport::open(new_file(), "x.svg", false);
        f.min_power = 1000;
        f.on_min_power_changed();
        assert_eq!(f.max_power, 1001);
        f.max_power = 0;
        f.on_max_power_changed();
        assert_eq!(f.min_power, -1);
    }

    #[test]
    fn percent_text_like_dotnet() {
        let _g = settings_guard();
        let mut f = SvgImport::open(new_file(), "x.svg", false);
        assert_eq!(f.perc_text(1000), "100.0 %");
        f.limits.max_pwm = 0;
        assert_eq!(f.perc_text(10), "");
    }

    #[test]
    fn material_helper_scales_power() {
        let _g = settings_guard();
        let mut f = SvgImport::open(new_file(), "x.svg", false);
        f.apply_material(750, 25);
        assert_eq!((f.border_speed, f.max_power), (750, 250));
    }

    #[test]
    fn accept_persists_settings_and_loads() {
        let _g = settings_guard();
        let p = std::env::temp_dir().join(format!("lg-u4a-svg-{}.svg", std::process::id()));
        std::fs::write(&p, r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><path d="M1 1 L19 1 L19 19 Z" stroke="black" fill="none"/></svg>"#).unwrap();
        let file = new_file();
        let mut f = SvgImport::open(file.clone(), p.to_str().unwrap(), false);
        f.border_speed = 1500;
        f.min_power = 10;
        f.max_power = 800;
        f.laser_on = 1;
        let out = f.accept();
        assert!(out.accepted && out.usage_svg_file == 1);
        out.load.expect("load started").join().unwrap();
        assert!(file.read().unwrap().count() > 0);
        let get = |k: &str| lg_base::settings::get(k, -1i32);
        assert_eq!(get("GrayScaleConversion.VectorizeOptions.BorderSpeed"), 1500);
        assert_eq!(get("GrayScaleConversion.Gcode.LaserOptions.PowerMin"), 10);
        assert_eq!(get("GrayScaleConversion.Gcode.LaserOptions.PowerMax"), 800);
        assert_eq!(lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.LaserOn", String::new()), "M4");
        let _ = std::fs::remove_file(&p);
    }
}
