// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `RasterConverter/ConvertSizeAndOptionForm.cs` (+ Designer): target size, offset,
//! speeds and laser options of the raster import (second page).

use eframe::egui::{self, Align, Key, Layout, Ui, Vec2};
use lg_dotnet::{dec_div, decimal_to_f32, fmt_decimal_custom, Decimal};
use lg_services::image_processor::{ImageProcessor, Tool};
use lg_services::layer_presets::LayerPreset;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{form_modal_x, tip};
use crate::forms::material::{PresetPickerForm, SharedPresetDb};
use crate::ui::dialog::{DialogResult, MessageBox, MessageBoxButtons, MessageBoxHost, MessageBoxIcon};
use crate::ui::i18n::{tr, tr_fmt, tr_plain};
use crate::ui::widgets::{flat_combo, grbl_group_box, DecimalInputRanged, GrblButton, ImageButton, IntegerInputRanged};

/// Resource prefix of the form.
const P: &str = "RasterConverter.ConvertSizeAndOptionForm";

/// C# `static bool ratiolock`: remembered between dialogs.
static RATIO_LOCK: AtomicBool = AtomicBool::new(true);
static NEXT_UID: AtomicU64 = AtomicU64::new(1);

/// `LaserOptions` (text, value).
const LASER_OPTIONS: [(&str, &str); 2] = [("M3 - Constant Power", "M3"), ("M4 - Dynamic Power", "M4")];

/// Continuation tags of the message boxes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Msg {
    UnlockProportion,
    SoftLimitNs,
    SoftLimitOob,
    WrongLaserMode,
}

/// `ConvertSizeAndOptionForm`.
pub struct ConvertSizeForm {
    uid: u64,
    support_pwm: bool,
    db: SharedPresetDb,
    // limits (AssignMinMaxLimit)
    table_w: i32,
    table_h: i32,
    max_rate_x: i32,
    max_pwm: i32,
    /// Firmware is in laser mode ($32=1).
    laser_mode: bool,
    off_min_x: f32,
    off_min_y: f32,
    // values
    border_speed: i32,
    mark_speed: i32,
    laser_sel: Option<usize>,
    min_power: i32,
    max_power: i32,
    autosize: bool,
    autosize_locked: bool,
    dpi: i32,
    size_w: f32,
    size_h: f32,
    off_x: f32,
    off_y: f32,
    keep_ratio: bool,
    // visibility of the speed rows
    show_filling: bool,
    show_border: bool,
    filling_vectorize: bool,
    // facts of the processor
    file_dpi: i32,
    true_width: u32,
    msgs: MessageBoxHost<Msg>,
    ps: Option<PresetPickerForm>,
}

impl ConvertSizeForm {
    /// Ctor + `ShowDialog(parent, processor)` initialisation.
    ///
    /// `ip`: image processor to configure (its `gcode` options are filled from settings).
    /// `db`: layer preset database for the P/S helper button. Returns the form ready to show.
    pub fn new(ip: &mut ImageProcessor, db: SharedPresetDb) -> Self {
        let support_pwm: bool = lg_base::settings::get("Support Hardware PWM", true);
        let cfg = lg_gcode::configuration::configuration();
        // AssignMinMaxLimit (the C# try/catch never fires: config getters have defaults)
        let table_w = dec_to_i32(cfg.table_width());
        let table_h = dec_to_i32(cfg.table_height());
        let (off_min_x, off_min_y) = if cfg.soft_limit() { (0.0, 0.0) } else { (-(table_w as f32), -(table_h as f32)) };
        let max_rate_x = dec_to_i32(cfg.max_rate_x());
        let max_pwm = dec_to_i32(cfg.max_pwm());
        let mut f = Self {
            uid: NEXT_UID.fetch_add(1, Ordering::Relaxed),
            support_pwm,
            db,
            table_w,
            table_h,
            max_rate_x,
            max_pwm,
            laser_mode: cfg.laser_mode(),
            off_min_x,
            off_min_y,
            border_speed: 1000,
            mark_speed: 1000,
            laser_sel: Some(0),
            min_power: 0,
            max_power: 1000,
            autosize: false,
            autosize_locked: false,
            dpi: 300,
            size_w: 0.0,
            size_h: 0.0,
            off_x: 0.0,
            off_y: 0.0,
            keep_ratio: true,
            show_filling: false,
            show_border: false,
            filling_vectorize: false,
            file_dpi: ip.file_dpi(),
            true_width: ip.true_original().width(),
            msgs: MessageBoxHost::default(),
            ps: None,
        };
        f.init(ip, cfg.laser_mode(), cfg.max_pwm());
        f
    }

    /// Body of C# `ShowDialog(parent, processor)` before the window opens.
    fn init(&mut self, ip: &mut ImageProcessor, laser_mode: bool, max_pwm: Decimal) {
        self.init_image_size(ip);
        self.border_speed = lg_base::settings::get("GrayScaleConversion.VectorizeOptions.BorderSpeed", 1000);
        self.mark_speed = lg_base::settings::get("GrayScaleConversion.Gcode.Speed.Mark", 1000);
        let laser_on: String = lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.LaserOn", "M3".to_string());
        // SelectedItem -> SelectedIndexChanged sets IP.LaserOn
        self.laser_sel = Some(if laser_on == "M3" || !laser_mode { 0 } else { 1 });
        ip.gcode.laser_off = "M5".into();
        self.min_power = lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.PowerMin", 0);
        self.max_power = lg_base::settings::get("GrayScaleConversion.Gcode.LaserOptions.PowerMax", dec_to_i32(max_pwm));
        let t = ip.selected_tool();
        self.show_filling = matches!(t, Tool::NoProcessing | Tool::Line2Line | Tool::Dithering) || (t == Tool::Vectorize && ip.filling_direction() != lg_services::image_processor::Direction::None);
        self.show_border = matches!(t, Tool::Vectorize | Tool::Centerline);
        self.filling_vectorize = t == Tool::Vectorize;
        self.off_x = lg_base::settings::get("GrayScaleConversion.Gcode.Offset.X", 0f32);
        self.off_y = lg_base::settings::get("GrayScaleConversion.Gcode.Offset.Y", 0f32);
        self.sync_ip(ip);
    }

    /// `InitImageSize()`.
    fn init_image_size(&mut self, ip: &ImageProcessor) {
        if ip.selected_tool() == Tool::NoProcessing {
            // CbAutosize.Checked = true; BtnDPI_Click; lock both
            self.autosize = true;
            self.dpi = self.file_dpi;
            self.compute_dpi_size(ip);
            self.autosize_locked = true;
        } else {
            self.keep_ratio = RATIO_LOCK.load(Ordering::Relaxed);
            let (w, h) = (ip.original().width(), ip.original().height());
            if self.keep_ratio {
                if h < w {
                    self.size_w = lg_base::settings::get("GrayScaleConversion.Gcode.ImageSize.W", 100f32);
                    self.size_h = ip.width_to_height(self.size_w);
                } else {
                    self.size_h = lg_base::settings::get("GrayScaleConversion.Gcode.ImageSize.H", 100f32);
                    self.size_w = ip.height_to_widht(self.size_h);
                }
            } else {
                self.size_w = lg_base::settings::get("GrayScaleConversion.Gcode.ImageSize.W", 100f32);
                self.size_h = lg_base::settings::get("GrayScaleConversion.Gcode.ImageSize.H", 100f32);
            }
        }
    }

    /// `ComputeDpiSize()` incl. the `BtnDPI.Enabled` rule (evaluated while drawing).
    fn compute_dpi_size(&mut self, ip: &ImageProcessor) {
        if self.autosize {
            self.size_w = (25.4 * self.true_width as f64 / self.dpi as f64) as f32;
            self.size_h = ip.width_to_height(self.size_w);
        }
    }

    /// Copy the control values to the processor (the `CurrentValueChanged` handlers).
    fn sync_ip(&self, ip: &mut ImageProcessor) {
        ip.gcode.target_size = (self.size_w, self.size_h);
        ip.gcode.target_offset = (self.off_x, self.off_y);
        ip.gcode.border_speed = self.border_speed;
        ip.gcode.mark_speed = self.mark_speed;
        ip.gcode.min_power = self.min_power;
        ip.gcode.max_power = self.max_power;
        ip.gcode.laser_on = LASER_OPTIONS[self.laser_sel.unwrap_or(0)].1.to_string();
    }

    /// `RefreshPerc` text for one power value.
    fn perc_text(&self, v: i32) -> String {
        let max = lg_gcode::configuration::configuration().max_pwm();
        if max > Decimal::ZERO {
            dec_div(Decimal::from(v), max).map(|d| fmt_decimal_custom(d, "P1")).unwrap_or_default()
        } else {
            String::new()
        }
    }

    /// `ConfirmOutOfBoundary()`, resumable. Returns true when the job may proceed now; false when a
    /// confirmation box was queued (its answer continues via [`on_answer`](Self::on_answer)).
    ///
    /// `after_ns`: skip the soft-limit question (already answered yes).
    fn confirm_out_of_boundary(&mut self, after_ns: bool) -> bool {
        let disabled: bool = lg_base::settings::get("DisableBoundaryWarning", false);
        if disabled {
            return true;
        }
        let cfg = lg_gcode::configuration::configuration();
        if !after_ns && (self.off_x < 0.0 || self.off_y < 0.0) && cfg.soft_limit() {
            self.msgs.push(Msg::SoftLimitNs, MessageBox::new(tr("Strings.WarnSoftLimitNS"), tr("Strings.WarnSoftLimitTitle")).buttons(MessageBoxButtons::YesNo).icon(MessageBoxIcon::Warning));
            return false;
        }
        let (tw, th) = (decimal_to_f32(cfg.table_width()), decimal_to_f32(cfg.table_height()));
        if self.off_x.max(0.0) + self.size_w > tw || self.off_y.max(0.0) + self.size_h > th {
            let text = tr_fmt("Strings.WarnSoftLimitOOB", &[&dec_to_i32(cfg.table_width()), &dec_to_i32(cfg.table_height())]);
            self.msgs.push(Msg::SoftLimitOob, MessageBox::new(text, tr("Strings.WarnSoftLimitTitle")).buttons(MessageBoxButtons::YesNo).icon(MessageBoxIcon::Warning));
            return false;
        }
        true
    }

    /// Continuation of a message box. Returns Some(Ok) when the dialog is accepted.
    fn on_answer(&mut self, tag: Msg, res: DialogResult, ip: &ImageProcessor) -> Option<DialogResult> {
        match tag {
            Msg::UnlockProportion => {
                // OK -> unlock, anything else (also click while unlocked) -> lock
                self.keep_ratio = res != DialogResult::Ok;
                if self.keep_ratio {
                    if ip.original().height() < ip.original().width() {
                        self.size_h = ip.width_to_height(self.size_w);
                    } else {
                        self.size_w = ip.height_to_widht(self.size_h);
                    }
                }
                None
            }
            Msg::SoftLimitNs if res == DialogResult::Yes => self.confirm_out_of_boundary(true).then_some(DialogResult::Ok),
            Msg::SoftLimitOob if res == DialogResult::Yes => Some(DialogResult::Ok),
            _ => None,
        }
    }

    /// Draw the form; call every frame while open (parent keeps drawing too).
    ///
    /// `ctx`: egui context. `ip`: processor being configured. Returns None while open, then the
    /// `DialogResult` (`Ok` = "Create!", `Cancel`).
    pub fn show(&mut self, ctx: &egui::Context, ip: &mut ImageProcessor) -> Option<DialogResult> {
        let uid = self.uid;
        let title = tr_plain(&format!("{P}.$this.Text"));
        let esc = ctx.input(|i| i.key_pressed(Key::Escape)) && !self.msgs.is_open() && self.ps.is_none();
        let (mut result, closed) = form_modal_x(ctx, ("convert_size", uid), &title, |ui| self.body(ui, ip));
        let esc = esc || closed;
        // P/S helper dialog
        if let Some(ps) = &mut self.ps {
            if let Some(r) = ps.show(ctx) {
                self.ps = None;
                if let Some(p) = r {
                    self.apply_material(&p);
                }
            }
        }
        if let Some((tag, res)) = self.msgs.show(ctx) {
            if let Some(r) = self.on_answer(tag, res, ip) {
                result = Some(r);
            }
        }
        if esc && result.is_none() {
            result = Some(DialogResult::Cancel);
        }
        self.sync_ip(ip);
        if result.is_some() {
            // ratiolock = KeepSizeRatio
            RATIO_LOCK.store(self.keep_ratio, Ordering::Relaxed);
        }
        result
    }

    /// `BtnPSHelper_Click` result handling (picked layer preset).
    ///
    /// `p`: preset; speed, power and laser mode are used, passes and filter are ignored.
    fn apply_material(&mut self, p: &LayerPreset) {
        if self.show_border {
            self.border_speed = p.speed;
        }
        if self.show_filling {
            self.mark_speed = p.speed;
        }
        self.max_power = p.power_s(self.max_pwm);
        // M4 only when firmware is in laser mode (same rule as init)
        let idx = LASER_OPTIONS.iter().position(|o| o.1 == p.laser_on).unwrap_or(0);
        self.laser_sel = Some(if idx == 1 && !self.laser_mode { 0 } else { idx });
    }

    /// Form body.
    fn body(&mut self, ui: &mut Ui, ip: &mut ImageProcessor) -> Option<DialogResult> {
        let uid = self.uid;
        ui.set_width(283.0);
        // --- GbSpeed ---
        grbl_group_box(ui, &tr_plain(&format!("{P}.GbSpeed.Text")), |ui| {
            ui.set_min_width(270.0);
            let mut first = true;
            egui::Grid::new(("cs_speed", uid)).num_columns(4).spacing([6.0, 4.0]).show(ui, |ui| {
                if self.show_border {
                    ui.label(tr_plain(&format!("{P}.LblBorderTracing.Text")));
                    ui.add(IntegerInputRanged::new(&mut self.border_speed).range(1, self.max_rate_x).width(55.0).id(egui::Id::unique(("cs_border", uid))));
                    ui.label(tr_plain(&format!("{P}.LblBorderTracingmm.Text")));
                    self.ps_button(ui);
                    first = false;
                    ui.end_row();
                }
                if self.show_filling {
                    // NOTE: C# overwrites the resource text with English literals
                    ui.label(if self.filling_vectorize { "Filling Speed" } else { "Engraving Speed" });
                    ui.add(IntegerInputRanged::new(&mut self.mark_speed).range(1, self.max_rate_x).width(55.0).id(egui::Id::unique(("cs_mark", uid))));
                    ui.label(tr_plain(&format!("{P}.LblLinearFillingmm.Text")));
                    if first {
                        self.ps_button(ui);
                    }
                    ui.end_row();
                }
            });
        });
        // --- GbLaser ---
        let mut warn_m4 = false;
        grbl_group_box(ui, &tr_plain(&format!("{P}.GbLaser.Text")), |ui| {
            ui.set_min_width(270.0);
            egui::Grid::new(("cs_laser", uid)).num_columns(4).spacing([6.0, 4.0]).show(ui, |ui| {
                ui.label(tr_plain(&format!("{P}.LblLaserMode.Text")));
                let items: Vec<String> = LASER_OPTIONS.iter().map(|o| o.0.to_string()).collect();
                let r = flat_combo(ui, ("cs_mode", uid), &mut self.laser_sel, &items, 150.0);
                if r.changed() && self.laser_sel == Some(1) && !lg_gcode::configuration::configuration().laser_mode() {
                    warn_m4 = true;
                }
                ui.label("");
                if ui.add(ImageButton::new("mdi-information-slab-box").size(Vec2::splat(17.0))).on_hover_text(tr(&format!("{P}.BtnOnOffInfo.ToolTip"))).clicked() {
                    lg_base::tools::utils::open_link("https://lasergrbl.com/usage/raster-image-import/target-image-size-and-laser-options/#laser-modes");
                }
                ui.end_row();
                if self.support_pwm {
                    ui.label(tr_plain(&format!("{P}.LblSmin.Text")));
                    let out = IntegerInputRanged::new(&mut self.min_power).range(0, 999).width(43.0).id(egui::Id::unique(("cs_minp", uid))).show(ui);
                    if let Some((_, nv)) = out.committed
                        && self.max_power <= nv
                    {
                        self.max_power = nv + 1;
                    }
                    ui.label(self.perc_text(self.min_power));
                    if ui.add(ImageButton::new("mdi-information-slab-box").size(Vec2::splat(17.0))).on_hover_text(tr(&format!("{P}.BtnModulationInfo.ToolTip"))).clicked() {
                        lg_base::tools::utils::open_link("https://lasergrbl.com/usage/raster-image-import/target-image-size-and-laser-options/#power-modulation");
                    }
                    ui.end_row();
                    ui.label(tr_plain(&format!("{P}.LblSmax.Text")));
                    let out = IntegerInputRanged::new(&mut self.max_power).range(1, self.max_pwm).width(43.0).id(egui::Id::unique(("cs_maxp", uid))).show(ui);
                    if let Some((_, nv)) = out.committed
                        && self.min_power >= nv
                    {
                        self.min_power = nv - 1;
                    }
                    ui.label(self.perc_text(self.max_power));
                    ui.end_row();
                }
            });
        });
        if warn_m4 {
            self.msgs.push(Msg::WrongLaserMode, MessageBox::new(tr("Strings.WarnWrongLaserMode"), tr("Strings.WarnWrongLaserModeTitle")).icon(MessageBoxIcon::Warning));
        }
        // --- GbSize ---
        let mut dpi_dirty = false;
        let mut unlock_click = false;
        grbl_group_box(ui, &tr_plain(&format!("{P}.GbSize.Text")), |ui| {
            ui.set_min_width(270.0);
            // tableLayoutPanel2: autosize, dpi, label, exif button
            ui.horizontal(|ui| {
                let was = self.autosize;
                ui.add_enabled(!self.autosize_locked, egui::Checkbox::new(&mut self.autosize, tr_plain(&format!("{P}.CbAutosize.Text"))));
                if was != self.autosize {
                    dpi_dirty = true;
                }
                let out = IntegerInputRanged::new(&mut self.dpi).range(1, 10000).width(55.0).id(egui::Id::unique(("cs_dpi", uid)));
                let enabled = self.autosize && !self.autosize_locked;
                let o = ui.add_enabled_ui(enabled, |ui| out.show(ui)).inner;
                if o.committed.is_some() {
                    dpi_dirty = true;
                }
                ui.label(tr_plain(&format!("{P}.label1.Text")));
                let btn_enabled = self.autosize && self.dpi != self.file_dpi;
                if ui.add_enabled(btn_enabled, ImageButton::new("custom-exif").size(Vec2::new(34.0, 20.0))).on_hover_text(tr(&format!("{P}.BtnDPI.ToolTip"))).clicked() {
                    // BtnDPI_Click
                    if self.autosize {
                        self.dpi = self.file_dpi;
                        dpi_dirty = true;
                    }
                }
            });
            let size_enabled = !self.autosize;
            egui::Grid::new(("cs_size", uid)).num_columns(6).spacing([6.0, 4.0]).show(ui, |ui| {
                // size row
                ui.label(tr_plain(&format!("{P}.label4.Text")));
                ui.label(tr_plain(&format!("{P}.label6.Text")));
                let out = ui.add_enabled_ui(size_enabled, |ui| DecimalInputRanged::new(&mut self.size_w).range(10.0, self.table_w as f32).decimals(1).width(55.0).id(egui::Id::unique(("cs_w", uid))).show(ui)).inner;
                // OnTheFly / commit by user: keep proportion
                if let Some(v) = out.on_the_fly.or(out.committed.map(|c| c.1))
                    && self.keep_ratio
                {
                    self.size_h = ip.width_to_height(v);
                }
                ui.label(tr_plain(&format!("{P}.label7.Text")));
                let out = ui.add_enabled_ui(size_enabled, |ui| DecimalInputRanged::new(&mut self.size_h).range(10.0, self.table_h as f32).decimals(1).width(55.0).id(egui::Id::unique(("cs_h", uid))).show(ui)).inner;
                if let Some(v) = out.on_the_fly.or(out.committed.map(|c| c.1))
                    && self.keep_ratio
                {
                    self.size_w = ip.height_to_widht(v);
                }
                let r = ui.add(ImageButton::new("mdi-lock").alt("mdi-lock-open-outline", !self.keep_ratio).size(Vec2::splat(17.0)));
                if tip(r, P, "BtnUnlockProportion").clicked() {
                    unlock_click = true;
                }
                ui.end_row();
                // offset row
                ui.label(tr_plain(&format!("{P}.label9.Text")));
                ui.label(tr_plain(&format!("{P}.label10.Text")));
                ui.add(DecimalInputRanged::new(&mut self.off_x).range(self.off_min_x, self.table_w as f32).decimals(1).width(55.0).id(egui::Id::unique(("cs_ox", uid))));
                ui.label(tr_plain(&format!("{P}.label11.Text")));
                ui.add(DecimalInputRanged::new(&mut self.off_y).range(self.off_min_y, self.table_h as f32).decimals(1).width(55.0).id(egui::Id::unique(("cs_oy", uid))));
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    // BtnReset
                    if ui.add(ImageButton::new("mdi-arrow-u-left-top-bold").size(Vec2::splat(17.0))).clicked() {
                        self.off_y = 0.0;
                        self.off_x = 0.0;
                    }
                    // BtnCenter
                    let r = ui.add(ImageButton::new("mdi-target").size(Vec2::splat(17.0)));
                    if tip(r, P, "BtnCenter").clicked() {
                        self.off_y = -(self.size_h / 2.0);
                        self.off_x = -(self.size_w / 2.0);
                    }
                });
                ui.end_row();
            });
        });
        if dpi_dirty {
            self.compute_dpi_size(ip);
        }
        if unlock_click {
            // BtnUnlockProportion_Click
            if self.keep_ratio {
                self.msgs.push(Msg::UnlockProportion, MessageBox::new(tr("Strings.WarnUnlockProportionText"), tr("Strings.WarnUnlockProportionTitle")).buttons(MessageBoxButtons::OkCancel).icon(MessageBoxIcon::Warning));
            } else {
                self.on_answer(Msg::UnlockProportion, DialogResult::Cancel, ip);
            }
        }
        // --- buttons ---
        ui.add_space(4.0);
        let mut out = None;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.add(GrblButton::new(tr_plain(&format!("{P}.BtnCreate.Text"))).icon("mdi-checkbox-marked").min_size(Vec2::new(74.0, 27.0))).clicked() && self.confirm_out_of_boundary(false) {
                out = Some(DialogResult::Ok);
            }
            if ui.add(GrblButton::new(tr_plain(&format!("{P}.BtnCancel.Text"))).icon("mdi-close-box").min_size(Vec2::new(74.0, 27.0))).clicked() {
                out = Some(DialogResult::Cancel);
            }
        });
        out
    }

    /// `BtnPSHelper` (opens the P/S helper).
    fn ps_button(&mut self, ui: &mut Ui) {
        let r = ui.add(ImageButton::new("mdi-database").size(Vec2::splat(17.0)));
        if tip(r, P, "BtnPSHelper").clicked() && self.ps.is_none() {
            self.ps = Some(PresetPickerForm::new(self.db.clone()));
        }
    }
}

/// `(int)decimal`: truncate toward zero.
fn dec_to_i32(d: Decimal) -> i32 {
    lg_dotnet::decimal_to_f64(d.trunc()) as i32
}
