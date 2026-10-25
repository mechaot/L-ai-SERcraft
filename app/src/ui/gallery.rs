// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Widget gallery: shows every theme color, icon, widget and dialog for visual checks.
//! Start with `laiserctrl --gallery`.

use eframe::egui::{self, Color32, RichText, Ui, Vec2};

use super::dialog::{DialogResult, MessageBox, MessageBoxButtons, MessageBoxHost, MessageBoxIcon, PendingFile, FileDialogOptions, filters};
use super::i18n;
use super::icons::{icons, Tint};
use super::theme::{LogRowColor, Scheme, Theme};
use super::widgets::*;
use super::widgets::color_slider::SliderSchema;
use super::widgets::label_tb::OverrideKind;
use super::widgets::progress::{BouncingMode, Bar, Reference};

/// Gallery tab.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Theme,
    Icons,
    Widgets,
    Dialogs,
    I18n,
    Preview,
    PreviewForm,
    Forms,
    RasterImport,
    Dialogs2,
}

/// Gallery state.
pub struct Gallery {
    tab: Tab,
    icon_filter: String,
    icon_size: f32,
    // widgets
    slider: i32,
    slider_v: i32,
    track: i32,
    dec: f32,
    int: i32,
    nud: f64,
    text: String,
    text_in: String,
    placeholder: String,
    grbl_tb: GrblTextBox,
    last_cmd: String,
    override_tb: LabelTb,
    pb: ColorProgressBar,
    dpb: DoubleProgressBar,
    wpb: Vec<WaitingProgressBar>,
    combo_sel: Option<usize>,
    enum_sel: u8,
    picture: PictureBox,
    picture_tex: Option<egui::TextureHandle>,
    ib_alt: bool,
    // dialogs
    msgs: MessageBoxHost<&'static str>,
    input: Option<InputBox>,
    file: Option<PendingFile>,
    dialog_log: String,
    grid_sel: Option<usize>,
    i18n_key: String,
    preview: super::preview::demo::PreviewDemo,
    preview_form: Option<crate::forms::preview_form::gallery_demo::PreviewFormDemo>,
    forms: crate::forms::gallery_tab::FormsTab,
    raster: crate::forms::raster::gallery_demo::RasterDemo,
    dialogs2: crate::forms::dialogs::gallery_tab::Dialogs2Tab,
}

impl Default for Gallery {
    fn default() -> Self {
        let mut wpb: Vec<WaitingProgressBar> = [BouncingMode::PingPong, BouncingMode::FilledUpDown, BouncingMode::RollingStrip]
            .into_iter()
            .map(WaitingProgressBar::new)
            .collect();
        for w in &mut wpb {
            w.base.step = 5.0;
            w.interval_ms = 40.0;
            w.start();
        }
        let mut dpb = DoubleProgressBar::default();
        dpb.base.size = Vec2::new(200.0, 16.0);
        dpb.base.draw_progress_string = true;
        dpb.perc_string = "42% / 70%".into();
        dpb.bars = vec![Bar { value: 70.0, color: Color32::from_rgb(160, 200, 255) }, Bar { value: 42.0, color: Color32::from_rgb(255, 128, 128) }];
        dpb.references = vec![Reference { value: 60.0, color: Color32::YELLOW, border: Color32::BLACK, bottom: false }, Reference { value: 20.0, color: Color32::GREEN, border: Color32::BLACK, bottom: true }];
        let mut pb = ColorProgressBar::default();
        pb.value = 55.0;
        pb.draw_progress_string = true;
        Gallery {
            // dev: LASERGRBL_GALLERY_TAB=preview opens the preview tab
            tab: if std::env::var("LASERGRBL_GALLERY_TAB").is_ok_and(|v| v == "preview") { Tab::Preview } else if std::env::var("LASERGRBL_GALLERY_TAB").is_ok_and(|v| v == "previewform") { Tab::PreviewForm } else if std::env::var("LASERGRBL_GALLERY_TAB").is_ok_and(|v| v == "raster") { Tab::RasterImport } else if std::env::var("LASERGRBL_GALLERY_TAB").is_ok_and(|v| v == "dialogs2") { Tab::Dialogs2 } else { Tab::Widgets },
            icon_filter: String::new(),
            icon_size: 24.0,
            slider: 50,
            slider_v: 30,
            track: 100,
            dec: 1.5,
            int: 42,
            nud: 2.5,
            text: "text input".into(),
            text_in: String::new(),
            placeholder: String::new(),
            grbl_tb: GrblTextBox::new(),
            last_cmd: String::new(),
            override_tb: LabelTb::new(OverrideKind::Linear, 100),
            pb,
            dpb,
            wpb,
            combo_sel: Some(1),
            enum_sel: 1,
            picture: PictureBox::new(),
            picture_tex: None,
            ib_alt: false,
            msgs: MessageBoxHost::default(),
            input: None,
            file: None,
            dialog_log: String::new(),
            grid_sel: None,
            i18n_key: "MainForm.MnConnect.Text".into(),
            preview: Default::default(),
            preview_form: None,
            forms: Default::default(),
            raster: Default::default(),
            dialogs2: Default::default(),
        }
    }
}

impl Gallery {
    /// Draw the gallery as an egui window (call each frame).
    ///
    /// `ctx`: egui context. `open`: window open flag.
    pub fn window(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new("Widget gallery").open(open).default_size([760.0, 560.0]).show(ctx, |ui| self.ui(ui));
        self.dialogs(ctx);
    }

    /// Draw the gallery as full-panel content.
    ///
    /// `ui`: target ui.
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            for (t, n) in [(Tab::Theme, "Theme"), (Tab::Icons, "Icons"), (Tab::Widgets, "Widgets"), (Tab::Dialogs, "Dialogs"), (Tab::I18n, "i18n"), (Tab::Preview, "Preview"), (Tab::Forms, "Forms"), (Tab::RasterImport, "Raster import"), (Tab::Dialogs2, "Dialogs2")] {
                ui.selectable_value(&mut self.tab, t, n);
            }
        });
        ui.separator();
        // the preview needs the full remaining area, no scroll wrapper
        if self.tab == Tab::Preview {
            self.preview.ui(ui);
            return;
        }
        // PreviewForm owns its layout too; core is built on first visit
        if self.tab == Tab::PreviewForm {
            self.preview_form.get_or_insert_with(Default::default).ui(ui);
            return;
        }
        egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| match self.tab {
            Tab::Theme => self.theme_tab(ui),
            Tab::Icons => self.icons_tab(ui),
            Tab::Widgets => self.widgets_tab(ui),
            Tab::Dialogs => self.dialogs_tab(ui),
            Tab::I18n => self.i18n_tab(ui),
            Tab::Preview | Tab::PreviewForm => {}
            Tab::Forms => self.forms.ui(ui),
            Tab::RasterImport => self.raster.ui(ui),
            Tab::Dialogs2 => self.dialogs2.ui(ui),
        });
    }

    /// Theme tab: scheme switch + swatches + log colors.
    fn theme_tab(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            for s in Scheme::ALL {
                if ui.selectable_label(Theme::current_scheme() == s, s.name()).clicked() {
                    Theme::switch(ui.ctx(), s);
                }
            }
        });
        let t = Theme::current();
        let sw = |ui: &mut Ui, name: &str, c: Color32| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::new(40.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(r, 2.0, c);
                ui.label(format!("{name}  ({},{},{})", c.r(), c.g(), c.b()));
            });
        };
        ui.columns(2, |cols| {
            let items: [(&str, Color32); 35] = [
                ("FormBackColor", t.form_back_color), ("FormForeColor", t.form_fore_color),
                ("PreviewBackColor", t.preview_back_color), ("PreviewText", t.preview_text),
                ("PreviewRuler", t.preview_ruler), ("PreviewGrid", t.preview_grid),
                ("PreviewGridMinor", t.preview_grid_minor), ("PreviewJobRange", t.preview_job_range),
                ("PreviewFirstMovement", t.preview_first_movement), ("PreviewOtherMovement", t.preview_other_movement),
                ("PreviewLaserPower", t.preview_laser_power), ("PreviewCross", t.preview_cross),
                ("PreviewCommandOK", t.preview_command_ok), ("PreviewCommandKO", t.preview_command_ko),
                ("PreviewCommandWait", t.preview_command_wait), ("PreviewCrossCursor", t.preview_cross_cursor),
                ("LogBackColor", t.log_back_color), ("LogLeftCOMMAND", t.log_left_command),
                ("LogLeftSTARTUP", t.log_left_startup), ("LogLeftALARM", t.log_left_alarm),
                ("LogLeftCONFIG", t.log_left_config), ("LogLeftFEEDBACK", t.log_left_feedback),
                ("LogLeftPOSITION", t.log_left_position), ("LogLeftOTHERS", t.log_left_others),
                ("LogRightGOOD", t.log_right_good), ("LogRightBAD", t.log_right_bad),
                ("LogRightOTHERS", t.log_right_others), ("TextBoxColorOverride", t.text_box_color_override),
                ("LinkColor", t.link_color), ("VisitedLinkColor", t.visited_link_color),
                ("ControlsBorder", t.controls_border), ("ControlsBackDisabled", t.controls_back_disabled),
                ("DisabledButtons", t.disabled_buttons), ("PressedButtons", t.pressed_buttons),
                ("MenuHighlight", t.menu_highlight_color()),
            ];
            for (i, (n, c)) in items.iter().enumerate() {
                sw(&mut cols[i % 2], n, *c);
            }
        });
        ui.separator();
        // log rows on log background
        egui::Frame::new().fill(t.log_back_color).inner_margin(6).show(ui, |ui| {
            for (r, n) in [
                (LogRowColor::Command, "G0 X10 Y10"), (LogRowColor::Startup, "Grbl 1.1h ['$' for help]"), (LogRowColor::Alarm, "ALARM:1 Hard limit"),
                (LogRowColor::Config, "$100=250.000"), (LogRowColor::Feedback, "[MSG:Caution: Unlocked]"), (LogRowColor::Position, "<Idle|MPos:0,0,0>"),
                (LogRowColor::Others, "other"), (LogRowColor::RightGood, "ok"), (LogRowColor::RightBad, "error:20"), (LogRowColor::RightOthers, "...")
            ] {
                ui.label(RichText::new(format!("{r:?}: {n}")).color(t.log_color(r)).monospace());
            }
        });
        ui.hyperlink_to("link color", "https://lasergrbl.com");
    }

    /// Icons tab: filterable grid with tooltips.
    fn icons_tab(&mut self, ui: &mut Ui) {
        let ic = icons();
        ui.horizontal(|ui| {
            ui.label("filter:");
            ui.text_edit_singleline(&mut self.icon_filter);
            ui.add(egui::Slider::new(&mut self.icon_size, 12.0..=64.0).text("size"));
            ui.label(if ic.is_legacy() { "legacy loader" } else { "svg loader" });
        });
        let f = self.icon_filter.to_lowercase();
        let keys: Vec<&str> = ic.keys("").into_iter().filter(|k| f.is_empty() || k.contains(&f)).collect();
        ui.label(format!("{} icons", keys.len()));
        // only draw a bounded number to keep the frame cheap
        ui.horizontal_wrapped(|ui| {
            for k in keys.iter().take(600) {
                let plain = k.starts_with("flags-");
                let img = if plain { ic.image_plain(ui.ctx(), k, self.icon_size) } else { ic.image(ui.ctx(), k, self.icon_size) };
                ui.add(img).on_hover_text(*k);
            }
        });
        if keys.len() > 600 {
            ui.label("... (refine filter to see more)");
        }
    }

    /// Widgets tab.
    fn widgets_tab(&mut self, ui: &mut Ui) {
        let th = Theme::current();
        grbl_group_box(ui, "ColorSlider", |ui| {
            ui.horizontal(|ui| {
                for s in [SliderSchema::PerlBlueGreen, SliderSchema::PerlRedCoral, SliderSchema::PerlGold, SliderSchema::PerlRoyalColors] {
                    let mut v = self.slider;
                    ui.add(ColorSlider::new(&mut v, 0..=100).schema(s).length(120.0));
                }
            });
            ui.add(ColorSlider::new(&mut self.slider, 0..=100).length(300.0));
            ui.horizontal(|ui| {
                ui.add(ColorSlider::new(&mut self.slider_v, 0..=100).orientation(Orientation::Vertical).length(100.0));
                ui.add_enabled(false, ColorSlider::new(&mut self.slider_v.clone(), 0..=100).orientation(Orientation::Vertical).length(100.0));
                ui.label(format!("value {}", self.slider));
            });
            ui.add(TrackBar::new(&mut self.track, 10..=200).small_change(5).large_change(10).tick_frequency(10).tick_style(TickStyle::Both).length(300.0));
            let o = self.override_tb.show(ui, 220.0);
            let _ = o;
        });
        grbl_group_box(ui, "Buttons", |ui| {
            ui.horizontal(|ui| {
                for k in ["mdi-power-plug", "mdi-power-plug-off", "mdi-play", "mdi-stop", "custom-home", "custom-reset"] {
                    ui.add(ImageButton::new(k).size(Vec2::splat(32.0)));
                }
                ui.add(ImageButton::new("mdi-play").size(Vec2::splat(36.0)).rounded_borders(true));
                ui.add_enabled(false, ImageButton::new("mdi-play").size(Vec2::splat(36.0)).rounded_borders(true));
                let b = ui.add(ImageButton::new("mdi-lock").alt("mdi-lock-open", self.ib_alt).size(Vec2::splat(32.0)).caption("alt"));
                if b.clicked() {
                    self.ib_alt = !self.ib_alt;
                }
            });
            ui.horizontal(|ui| {
                ui.add(GrblButton::new("Connect").icon("mdi-power-plug"));
                ui.add(GrblButton::new("Plain"));
                ui.add_enabled(false, GrblButton::new("Disabled").icon("mdi-stop"));
            });
        });
        grbl_group_box(ui, "Text", |ui| {
            ui.add(TextInput::new(&mut self.text).width(200.0));
            ui.add(PlaceholderTextBox::new(&mut self.placeholder, "placeholder...").width(200.0));
            ui.add(TextInput::new(&mut self.text_in).multiline(true).rows(2).width(200.0));
            let (_, e) = self.grbl_tb.show(ui, "GRBL command (Enter, Up/Down history)", 300.0);
            if let Some(c) = e {
                self.last_cmd = c;
            }
            ui.label(format!("last command: {}", self.last_cmd));
        });
        grbl_group_box(ui, "Numeric", |ui| {
            ui.horizontal(|ui| {
                ui.add(DecimalInputRanged::new(&mut self.dec).range(0.0, 10.0).decimals(2).id(egui::Id::unique("g-dec")));
                ui.add(IntegerInputRanged::new(&mut self.int).range(0, 40).id(egui::Id::unique("g-int")));
                ui.add(NumericUpDown::new(&mut self.nud).range(0.0, 10.0).increment(0.5).decimal_places(1).id(egui::Id::unique("g-nud")));
                ui.add_enabled(false, DecimalInputRanged::new(&mut self.dec.clone()).id(egui::Id::unique("g-dec2")));
            });
            ui.label(format!("dec={} int={} nud={}", self.dec, self.int, self.nud));
        });
        grbl_group_box(ui, "Progress", |ui| {
            self.pb.show(ui);
            self.dpb.show(ui);
            for w in &mut self.wpb {
                w.show(ui);
            }
            ui.add(egui::Slider::new(&mut self.pb.value, 0.0..=100.0));
        });
        grbl_group_box(ui, "Combo / usage / grid", |ui| {
            let items: Vec<String> = vec!["Alpha".into(), "Beta".into(), "Gamma (note)".into()];
            flat_combo(ui, "g-combo", &mut self.combo_sel, &items, 140.0);
            let vals = [0u8, 1, 2];
            let mut v = self.enum_sel;
            enum_combo(ui, "g-enum", &mut v, &vals, |x| items[*x as usize].clone(), true, 140.0);
            self.enum_sel = v;
            let hours = [1.0, 3.0, 9.0, 12.0, 6.0, 4.0, 2.0, 1.0, 0.5, 0.2];
            ui.add(UsageClassPreview::new(&hours));
            let cols = [GridColumn { header: "Row".into(), width: Some(60.0) }, GridColumn { header: "Value".into(), width: None }];
            data_grid(ui, "g-grid", &cols, 5, &mut self.grid_sel, |ui, r, c| {
                ui.label(if c == 0 { r.to_string() } else { format!("cell {r}") });
            }, 120.0);
        });
        grbl_group_box(ui, "PictureBox (drag to draw fence)", |ui| {
            if self.picture_tex.is_none() {
                let img = icons().texture(ui.ctx(), "flags-it", 128, Tint::None);
                self.picture_tex = Some(img);
            }
            self.picture.show(ui, self.picture_tex.as_ref(), Vec2::new(300.0, 200.0));
        });
        let _ = th;
    }

    /// Dialogs tab: buttons that open dialogs.
    fn dialogs_tab(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("MessageBox OK").clicked() {
                self.msgs.push("ok", MessageBox::new("Hello from MessageBox.", "Title").icon(MessageBoxIcon::Information));
            }
            if ui.button("YesNo warning (default No)").clicked() {
                self.msgs.push("yn", MessageBox::new("Really do it?", "Confirm").buttons(MessageBoxButtons::YesNo).icon(MessageBoxIcon::Warning).default_button(1));
            }
            if ui.button("OkCancel error").clicked() {
                self.msgs.push("oc", MessageBox::new("Something failed.", "Error").buttons(MessageBoxButtons::OkCancel).icon(MessageBoxIcon::Error));
            }
            if ui.button("YesNoCancel question").clicked() {
                self.msgs.push("ync", MessageBox::new("Save changes?", "Question").buttons(MessageBoxButtons::YesNoCancel).icon(MessageBoxIcon::Question));
            }
            if ui.button("InputBox").clicked() {
                self.input = Some(InputBox::new("Enter a number:", "Input", "10", Some(Box::new(|t| if t.trim().parse::<f64>().is_ok() { Ok(()) } else { Err("not a number".into()) }))));
            }
            if ui.button("Open file").clicked() && self.file.is_none() {
                self.file = Some(PendingFile::open(ui.ctx(), FileDialogOptions { filter: Some(filters::OPEN_ANY.into()), ..Default::default() }));
            }
            if ui.button("Save file").clicked() && self.file.is_none() {
                self.file = Some(PendingFile::save(ui.ctx(), FileDialogOptions { filter: Some(filters::GCODE_SAVE_NC.into()), ..Default::default() }));
            }
        });
        ui.label(&self.dialog_log);
    }

    /// i18n tab: language list and key lookup.
    fn i18n_tab(&mut self, ui: &mut Ui) {
        ui.label(format!("active culture: {}", i18n::catalog().culture()));
        ui.horizontal_wrapped(|ui| {
            for c in i18n::cultures() {
                let r = ui.add(egui::Button::image_and_text(icons().image_plain(ui.ctx(), &c.flag, 16.0), &c.name));
                if r.clicked() {
                    i18n::set_language(Some(&c.culture));
                    self.dialog_log = format!("language '{}' saved (restart required)", c.culture);
                }
            }
        });
        ui.text_edit_singleline(&mut self.i18n_key);
        ui.label(format!("tr = {:?}", i18n::tr(&self.i18n_key)));
        ui.label(format!("tr_plain = {:?}", i18n::tr_plain(&self.i18n_key)));
        ui.label(i18n::tr_fmt("Strings.SpeedSliderToolTip", &[&123]));
        ui.label(&self.dialog_log);
    }

    /// Run open dialogs.
    pub fn dialogs(&mut self, ctx: &egui::Context) {
        self.forms.dialogs(ctx);
        self.dialogs2.dialogs(ctx);
        if let Some((tag, r)) = self.msgs.show(ctx) {
            self.dialog_log = format!("{tag}: {r:?}");
        }
        if let Some(ib) = &mut self.input {
            if let Some(r) = ib.show(ctx) {
                self.dialog_log = format!("input: ok={} text={:?}", r.ok, r.text);
                self.input = None;
            }
        }
        if let Some(f) = &mut self.file {
            if let Some(r) = f.poll() {
                self.dialog_log = format!("file: {r:?}");
                self.file = None;
            }
        }
        let _ = DialogResult::None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke: run every tab a few frames headless.
    #[test]
    fn renders_all_tabs() {
        let ctx = egui::Context::default();
        let mut g = Gallery::default();
        for s in Scheme::ALL {
            Theme::set_current(s);
            Theme::of(s).apply(&ctx);
            for t in [Tab::Theme, Tab::Icons, Tab::Widgets, Tab::Dialogs, Tab::I18n, Tab::Preview, Tab::Forms, Tab::RasterImport, Tab::Dialogs2] {
                g.tab = t;
                for _ in 0..2 {
                    let mut open = true;
                    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| g.window(&ui.ctx().clone(), &mut open));
                    out.textures_delta.clear();
                }
            }
        }
    }
}
