//! Ports `RasterConverter/RasterToLaserForm.cs` (+ Designer): the image import dialog (tool
//! selection, conversion parameters, crop / fill / outline tools, live preview) which hands over to
//! [`ConvertSizeForm`] and finally drives `ImageProcessor::generate_gcode`.
//!
//! The WinForms modal `ShowDialog` becomes an immediate mode struct: `RasterImport::open(..)`
//! then `show(ctx)` each frame until it returns the [`RasterOutcome`]. The static events of the
//! C# `ImageProcessor` arrive through a channel and are drained at the start of every frame.

use crossbeam_channel::Receiver;
use eframe::egui::{self, Align, Color32, ColorImage, Key, Layout, PointerButton, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2};
use image::{Rgba, RgbaImage};
use lg_dotnet::{decimal_to_f64, f64_to_decimal, Decimal};
use lg_gcode::GrblFile;
use lg_services::image_processor::{Color, Direction, DitheringMode, Formula, ImageProcessor, ImageProcessorEvent, InterpolationMode, Point, Rect as IpRect, Tool};
use lg_services::project::{self, ProjectValue, SettingsDict};
use serde_json::Value;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use super::{form_modal_x, get_enum, tip, ConvertSizeForm, RasterOutcome, ResolutionHelperForm, SettingEnum, UsageDelta};
use crate::forms::material::{default_db, SharedMaterialDb};
use crate::ui::dialog::{DialogResult, MessageBox, MessageBoxButtons, MessageBoxHost, MessageBoxIcon};
use crate::ui::i18n::{tr, tr_plain};
use crate::ui::theme::Theme;
use crate::ui::widgets::{enum_combo, flat_combo, grbl_group_box, ColorSlider, GrblButton, ImageButton, NumericUpDown, SliderColors, TextInput, WaitingProgressBar};

/// Resource prefix of the form.
const P: &str = "RasterConverter.RasterToLaserForm";
/// `PbConverted.Size` (designer).
const PB_W: i32 = 512;
const PB_H: i32 = 484;

static NEXT_UID: AtomicU64 = AtomicU64::new(1);

/// `Strings.<Type><Value>` translation of an enum (`GrblCore.TranslateEnum`), fallback = C# name.
fn translate_enum<T: SettingEnum>(v: T, type_name: &str) -> String {
    let key = format!("Strings.{type_name}{}", v.name());
    let t = tr(&key);
    if t == key || t.is_empty() { v.name().to_string() } else { t.to_string() }
}

fn dir_text(d: &Direction) -> String {
    translate_enum(*d, "Direction")
}

/// Value of one entry of `GetActualSettings()`.
#[derive(Clone, Debug)]
enum SVal {
    Tool(Tool),
    Dir(Direction),
    Formula(Formula),
    Dither(DitheringMode),
    Interp(InterpolationMode),
    Dec(f64),
    Bool(bool),
    I32(i32),
    F32(f32),
    Str(String),
}

impl SVal {
    /// Settings store representation (`Settings.SetObject`).
    fn to_setting(&self) -> Value {
        match self {
            SVal::Tool(v) => Value::String(v.name().into()),
            SVal::Dir(v) => Value::String(v.name().into()),
            SVal::Formula(v) => Value::String(v.name().into()),
            SVal::Dither(v) => Value::String(v.name().into()),
            SVal::Interp(v) => Value::String(v.name().into()),
            SVal::Dec(v) => serde_json::to_value(f64_to_decimal(*v)).unwrap_or(Value::Null),
            SVal::Bool(v) => Value::Bool(*v),
            SVal::I32(v) => Value::from(*v),
            SVal::F32(v) => serde_json::to_value(*v).unwrap_or(Value::Null),
            SVal::Str(v) => Value::String(v.clone()),
        }
    }

    /// Boxed value of the project dictionary.
    fn to_project(&self) -> ProjectValue {
        match self {
            SVal::Tool(v) => ProjectValue::tool(v.value()),
            SVal::Dir(v) => ProjectValue::direction(v.value()),
            SVal::Formula(v) => ProjectValue::formula(v.value()),
            SVal::Dither(v) => ProjectValue::dithering_mode(v.value()),
            SVal::Interp(v) => ProjectValue::interpolation_mode(v.value()),
            SVal::Dec(v) => ProjectValue::Decimal(f64_to_decimal(*v)),
            SVal::Bool(v) => ProjectValue::Bool(*v),
            SVal::I32(v) => ProjectValue::I32(*v),
            SVal::F32(v) => ProjectValue::F32(*v),
            SVal::Str(v) => ProjectValue::Str(v.clone()),
        }
    }
}

/// Message box continuations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Msg {
    /// "could be very time consuming" question before the second page.
    TimeConsuming,
    /// Exception text of the generator.
    GenError,
    /// Project settings could not be stored.
    ProjectError,
}

/// Which quality field the resolution helper edits.
#[derive(Clone, Copy, PartialEq, Eq)]
enum QualityTarget {
    Line,
    Filling,
}

/// State of every control of the form (the C# controls are the only state there).
struct Ctl {
    support_pwm: bool,
    is_gray: bool,
    max_quality: f64,
    // radios
    tool: Tool,
    // line2line / dithering options
    line_dirs: Vec<Direction>,
    direction: Direction,
    quality: f64,
    line_preview: bool,
    dither: DitheringMode,
    // vectorize options
    use_spot_removal: bool,
    spot_removal: f64,
    use_smoothing: bool,
    smoothing: f64,
    use_optimize: bool,
    optimize: f64,
    use_down_sample: bool,
    down_sample: f64,
    adaptive_quality: bool,
    optimize_fast: bool,
    fill_dirs: Vec<Direction>,
    filling_direction: Direction,
    filling_quality: f64,
    // centerline
    use_line_threshold: bool,
    line_threshold: i32,
    use_corner_threshold: bool,
    corner_threshold: i32,
    // parameters
    interpolation: InterpolationMode,
    formula: Formula,
    red: i32,
    green: i32,
    blue: i32,
    brightness: i32,
    contrast: i32,
    use_threshold: bool,
    threshold: i32,
    white_clip: i32,
}

impl Ctl {
    /// Constructor part of the form: combo item lists, UD maximum.
    fn new(is_gray: bool) -> Self {
        let support_pwm: bool = lg_base::settings::get("Support Hardware PWM", true);
        let max_quality = if lg_base::settings::get("Raster Hi-Res", false) { 50.0 } else { 20.0 };
        // CbDirections: raster fillings; CbFillingDirection: None, vector fillings, raster fillings
        let all: Vec<Direction> = Direction::table().iter().map(|(d, _)| *d).collect();
        let line_dirs: Vec<Direction> = all.iter().copied().filter(|d| lg_gcode::raster_filling(*d)).collect();
        let mut fill_dirs = vec![Direction::None];
        fill_dirs.extend(all.iter().copied().filter(|d| lg_gcode::vector_filling(*d)));
        fill_dirs.extend(all.iter().copied().filter(|d| lg_gcode::raster_filling(*d)));
        Self {
            support_pwm,
            is_gray,
            max_quality,
            tool: Tool::Line2Line,
            direction: line_dirs[0],
            line_dirs,
            quality: 3.0,
            line_preview: true,
            dither: DitheringMode::Atkinson,
            use_spot_removal: false,
            spot_removal: 2.0,
            use_smoothing: false,
            smoothing: 1.0,
            use_optimize: false,
            optimize: 0.2,
            use_down_sample: false,
            down_sample: 1.0,
            adaptive_quality: false,
            optimize_fast: false,
            filling_direction: fill_dirs[0],
            fill_dirs,
            filling_quality: 3.0,
            use_line_threshold: false,
            line_threshold: 10,
            use_corner_threshold: false,
            corner_threshold: 110,
            interpolation: InterpolationMode::HighQualityBicubic,
            formula: Formula::SimpleAverage,
            red: 100,
            green: 100,
            blue: 100,
            brightness: 100,
            contrast: 100,
            use_threshold: false,
            threshold: 50,
            white_clip: 5,
        }
    }

    /// `LoadSettings()`: read the settings into the controls and the processor.
    fn load_settings(&mut self, ip: &mut ImageProcessor) {
        use lg_base::settings::get;
        let dec = |key: &str, default: f64| -> f64 { decimal_to_f64(get::<Decimal>(key, f64_to_decimal(default))) };
        // tool radios (NoProcessing is never stored and falls to Vectorize like C#)
        let t = get_enum("GrayScaleConversion.RasterConversionTool", Tool::Line2Line);
        self.tool = if t == Tool::NoProcessing { Tool::Vectorize } else { t };
        ip.set_selected_tool(self.tool);

        self.direction = get_enum("GrayScaleConversion.Line2LineOptions.Direction", Direction::Horizontal);
        ip.set_line_direction(self.direction);
        self.quality = dec("GrayScaleConversion.Line2LineOptions.Quality", 3.0).min(self.max_quality);
        ip.set_quality(f64_to_decimal(self.quality));
        self.line_preview = get("GrayScaleConversion.Line2LineOptions.Preview", false);
        ip.set_line_preview(self.line_preview);

        self.use_spot_removal = get("GrayScaleConversion.VectorizeOptions.SpotRemoval.Enabled", false);
        ip.set_use_spot_removal(self.use_spot_removal);
        self.spot_removal = dec("GrayScaleConversion.VectorizeOptions.SpotRemoval.Value", 2.0);
        ip.set_spot_removal(f64_to_decimal(self.spot_removal));
        self.use_smoothing = get("GrayScaleConversion.VectorizeOptions.Smooting.Enabled", false);
        ip.set_use_smoothing(self.use_smoothing);
        self.smoothing = dec("GrayScaleConversion.VectorizeOptions.Smooting.Value", 1.0);
        ip.set_smoothing(f64_to_decimal(self.smoothing));
        self.use_optimize = get("GrayScaleConversion.VectorizeOptions.Optimize.Enabled", false);
        ip.set_use_optimize(self.use_optimize);
        self.adaptive_quality = get("GrayScaleConversion.VectorizeOptions.UseAdaptiveQuality.Enabled", false);
        ip.set_use_adaptive_quality(self.adaptive_quality);
        self.optimize = dec("GrayScaleConversion.VectorizeOptions.Optimize.Value", 0.2);
        ip.set_optimize(f64_to_decimal(self.optimize));
        self.use_down_sample = get("GrayScaleConversion.VectorizeOptions.DownSample.Enabled", false);
        ip.set_use_down_sampling(self.use_down_sample);
        self.down_sample = dec("GrayScaleConversion.VectorizeOptions.DownSample.Value", 2.0);
        ip.set_down_sampling(f64_to_decimal(self.down_sample));
        self.optimize_fast = get("GrayScaleConversion.VectorizeOptions.OptimizeFast.Enabled", false);
        ip.set_optimize_fast(self.optimize_fast);

        self.filling_direction = get_enum("GrayScaleConversion.VectorizeOptions.FillingDirection", Direction::None);
        ip.set_filling_direction(self.filling_direction);
        self.filling_quality = dec("GrayScaleConversion.VectorizeOptions.FillingQuality", 3.0).min(self.max_quality);
        ip.set_filling_quality(f64_to_decimal(self.filling_quality));

        self.interpolation = get_enum("GrayScaleConversion.Parameters.Interpolation", InterpolationMode::HighQualityBicubic);
        ip.set_interpolation(self.interpolation);
        self.formula = get_enum("GrayScaleConversion.Parameters.Mode", Formula::SimpleAverage);
        ip.set_formula(self.formula);
        self.red = get("GrayScaleConversion.Parameters.R", 100);
        ip.set_red(self.red);
        self.green = get("GrayScaleConversion.Parameters.G", 100);
        ip.set_green(self.green);
        self.blue = get("GrayScaleConversion.Parameters.B", 100);
        ip.set_blue(self.blue);
        self.brightness = get("GrayScaleConversion.Parameters.Brightness", 100);
        ip.set_brightness(self.brightness);
        self.contrast = get("GrayScaleConversion.Parameters.Contrast", 100);
        ip.set_contrast(self.contrast);
        self.use_threshold = get("GrayScaleConversion.Parameters.Threshold.Enabled", false);
        ip.set_use_threshold(self.use_threshold);
        self.threshold = get("GrayScaleConversion.Parameters.Threshold.Value", 50);
        ip.set_threshold(self.threshold);
        self.white_clip = get("GrayScaleConversion.Parameters.WhiteClip", 5);
        ip.set_white_clip(self.white_clip);

        // only the combo changes the processor (no IP assignment in C#)
        self.dither = get_enum("GrayScaleConversion.DitheringOptions.DitheringMode", DitheringMode::FloydSteinberg);
        ip.set_dithering_mode(self.dither);

        self.use_line_threshold = get("GrayScaleConversion.VectorizeOptions.LineThreshold.Enabled", true);
        ip.set_use_line_threshold(self.use_line_threshold);
        self.line_threshold = get("GrayScaleConversion.VectorizeOptions.LineThreshold.Value", 10);
        ip.set_line_threshold(self.line_threshold);
        self.use_corner_threshold = get("GrayScaleConversion.VectorizeOptions.CornerThreshold.Enabled", true);
        ip.set_use_corner_threshold(self.use_corner_threshold);
        self.corner_threshold = get("GrayScaleConversion.VectorizeOptions.CornerThreshold.Value", 110);
        ip.set_corner_threshold(self.corner_threshold);

        if self.tool == Tool::Line2Line && !self.support_pwm {
            self.tool = Tool::Dithering;
            ip.set_selected_tool(Tool::Dithering);
        }
    }

    /// `GetActualSettings()`: key / value list in the C# insertion order.
    fn actual_settings(&self, ip: &ImageProcessor) -> Vec<(&'static str, SVal)> {
        let g = &ip.gcode;
        let tool = match self.tool {
            Tool::Line2Line => Tool::Line2Line,
            Tool::Dithering => Tool::Dithering,
            Tool::Centerline => Tool::Centerline,
            _ => Tool::Vectorize,
        };
        vec![
            ("GrayScaleConversion.RasterConversionTool", SVal::Tool(tool)),
            ("GrayScaleConversion.Line2LineOptions.Direction", SVal::Dir(self.direction)),
            ("GrayScaleConversion.Line2LineOptions.Quality", SVal::Dec(self.quality)),
            ("GrayScaleConversion.Line2LineOptions.Preview", SVal::Bool(self.line_preview)),
            ("GrayScaleConversion.VectorizeOptions.SpotRemoval.Enabled", SVal::Bool(self.use_spot_removal)),
            ("GrayScaleConversion.VectorizeOptions.SpotRemoval.Value", SVal::Dec(self.spot_removal)),
            ("GrayScaleConversion.VectorizeOptions.Smooting.Enabled", SVal::Bool(self.use_smoothing)),
            ("GrayScaleConversion.VectorizeOptions.Smooting.Value", SVal::Dec(self.smoothing)),
            ("GrayScaleConversion.VectorizeOptions.Optimize.Enabled", SVal::Bool(self.use_optimize)),
            ("GrayScaleConversion.VectorizeOptions.UseAdaptiveQuality.Enabled", SVal::Bool(self.adaptive_quality)),
            ("GrayScaleConversion.VectorizeOptions.Optimize.Value", SVal::Dec(self.optimize)),
            ("GrayScaleConversion.VectorizeOptions.DownSample.Enabled", SVal::Bool(self.use_down_sample)),
            ("GrayScaleConversion.VectorizeOptions.DownSample.Value", SVal::Dec(self.down_sample)),
            ("GrayScaleConversion.VectorizeOptions.FillingDirection", SVal::Dir(self.filling_direction)),
            ("GrayScaleConversion.VectorizeOptions.FillingQuality", SVal::Dec(self.filling_quality)),
            ("GrayScaleConversion.VectorizeOptions.OptimizeFast.Enabled", SVal::Bool(self.optimize_fast)),
            ("GrayScaleConversion.DitheringOptions.DitheringMode", SVal::Dither(self.dither)),
            ("GrayScaleConversion.Parameters.Interpolation", SVal::Interp(self.interpolation)),
            ("GrayScaleConversion.Parameters.Mode", SVal::Formula(self.formula)),
            ("GrayScaleConversion.Parameters.R", SVal::I32(self.red)),
            ("GrayScaleConversion.Parameters.G", SVal::I32(self.green)),
            ("GrayScaleConversion.Parameters.B", SVal::I32(self.blue)),
            ("GrayScaleConversion.Parameters.Brightness", SVal::I32(self.brightness)),
            ("GrayScaleConversion.Parameters.Contrast", SVal::I32(self.contrast)),
            ("GrayScaleConversion.Parameters.Threshold.Enabled", SVal::Bool(self.use_threshold)),
            ("GrayScaleConversion.Parameters.Threshold.Value", SVal::I32(self.threshold)),
            ("GrayScaleConversion.Parameters.WhiteClip", SVal::I32(self.white_clip)),
            ("GrayScaleConversion.VectorizeOptions.BorderSpeed", SVal::I32(g.border_speed)),
            ("GrayScaleConversion.Gcode.Speed.Mark", SVal::I32(g.mark_speed)),
            ("GrayScaleConversion.Gcode.LaserOptions.LaserOn", SVal::Str(g.laser_on.clone())),
            ("GrayScaleConversion.Gcode.LaserOptions.LaserOff", SVal::Str(g.laser_off.clone())),
            ("GrayScaleConversion.Gcode.LaserOptions.PowerMin", SVal::I32(g.min_power)),
            ("GrayScaleConversion.Gcode.LaserOptions.PowerMax", SVal::I32(g.max_power)),
            ("GrayScaleConversion.Gcode.Offset.X", SVal::F32(g.target_offset.0)),
            ("GrayScaleConversion.Gcode.Offset.Y", SVal::F32(g.target_offset.1)),
            ("GrayScaleConversion.Gcode.ImageSize.W", SVal::F32(g.target_size.0)),
            ("GrayScaleConversion.Gcode.ImageSize.H", SVal::F32(g.target_size.1)),
            ("GrayScaleConversion.VectorizeOptions.LineThreshold.Enabled", SVal::Bool(ip.use_line_threshold())),
            ("GrayScaleConversion.VectorizeOptions.LineThreshold.Value", SVal::I32(ip.line_threshold())),
            ("GrayScaleConversion.VectorizeOptions.CornerThreshold.Enabled", SVal::Bool(ip.use_corner_threshold())),
            ("GrayScaleConversion.VectorizeOptions.CornerThreshold.Value", SVal::I32(ip.corner_threshold())),
        ]
    }
}

/// Crop / fill / outline tool state (`Cropping`, `Filling`, `Outlining`, drag fields).
#[derive(Default)]
struct Tools {
    cropping: bool,
    /// `Filling`: -1 off, 127 tolerance when active.
    filling: i32,
    outlining: bool,
    /// Rubber band start / end in control coordinates while dragging.
    drag: Option<(Pos2, Pos2)>,
}

/// `RasterToLaserForm` + `CreateAndShowDialog` semantics.
pub struct RasterImport {
    uid: u64,
    ip: Option<ImageProcessor>,
    rx: Receiver<ImageProcessorEvent>,
    c: Ctl,
    tools: Tools,
    db: SharedMaterialDb,
    /// `Load` event done (processor resumed).
    started: bool,
    prevent_close: bool,
    /// 0 = Preview tab, 1 = Original tab.
    tab: usize,
    tex_conv: Option<TextureHandle>,
    tex_orig: Option<TextureHandle>,
    /// Waiting bar `WB` and its 50 ms start timer `WT`.
    wb: WaitingProgressBar,
    wb_visible: bool,
    wt_since: Option<f64>,
    create_enabled: bool,
    /// After "Create!": everything but the waiting bar is disabled.
    generating: bool,
    demo_down: bool,
    convert: Option<ConvertSizeForm>,
    helper: Option<(ResolutionHelperForm, QualityTarget)>,
    msgs: MessageBoxHost<Msg>,
    usage: UsageDelta,
    outcome: Option<RasterOutcome>,
    /// Outcome waits for a message box to be answered.
    finish_after_msgs: bool,
}

/// Action chosen in the form body.
enum Action {
    Cancel,
    Next,
}

impl RasterImport {
    /// `CreateAndShowDialog(core, filename, append)` -- constructor part.
    ///
    /// `file`: the program the generator fills (`core.LoadedFile`). `path`: image file.
    /// `append`: append to the program instead of replacing it. Returns the dialog state or the
    /// load error text. The caller must call `Project`-independent bookkeeping (`Core.LastOpenFile`)
    /// before, like `GrblCore.OpenFile` does.
    pub fn open(file: Arc<RwLock<GrblFile>>, path: impl AsRef<Path>, append: bool) -> Result<Self, String> {
        Self::open_with_db(file, path, append, default_db())
    }

    /// Like [`open`](Self::open) with an explicit material database for the P/S helper.
    ///
    /// `db`: shared material database (`GrblCore.MaterialDB`). Other arguments as `open`.
    pub fn open_with_db(file: Arc<RwLock<GrblFile>>, path: impl AsRef<Path>, append: bool, db: SharedMaterialDb) -> Result<Self, String> {
        let (tx, rx) = crossbeam_channel::unbounded();
        // GetImageSize() = PbConverted.Size - 20
        let mut ip = ImageProcessor::new(file, &path.as_ref().to_string_lossy(), (PB_W - 20, PB_H - 20), append, tx)?;
        let mut c = Ctl::new(ip.is_gray_scale());
        c.load_settings(&mut ip);
        Ok(Self {
            uid: NEXT_UID.fetch_add(1, Ordering::Relaxed),
            ip: Some(ip),
            rx,
            c,
            tools: Tools { filling: -1, ..Default::default() },
            db,
            started: false,
            prevent_close: false,
            tab: 0,
            tex_conv: None,
            tex_orig: None,
            wb: {
                let mut w = WaitingProgressBar::new(crate::ui::widgets::progress::BouncingMode::PingPong);
                w.base.size = Vec2::new(65.0, 16.0);
                w.base.bar_color = Color32::from_rgb(70, 130, 180);
                w.base.fill_color = Color32::WHITE;
                w.base.maximum = 20.0;
                w.base.step = 1.0;
                w.base.reverse = true;
                w.interval_ms = 25.0;
                w
            },
            wb_visible: false,
            wt_since: None,
            create_enabled: true,
            generating: false,
            demo_down: false,
            convert: None,
            helper: None,
            msgs: MessageBoxHost::default(),
            usage: UsageDelta::default(),
            outcome: None,
            finish_after_msgs: false,
        })
    }

    /// Drain `PreviewBegin` / `PreviewReady` / `GenerationComplete`.
    fn drain_events(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                ImageProcessorEvent::PreviewBegin => {
                    // OnPreviewBegin
                    self.prevent_close = true;
                    if self.wt_since.is_none() {
                        self.wt_since = Some(now);
                    }
                    self.create_enabled = false;
                }
                ImageProcessorEvent::PreviewReady(img) => {
                    // OnPreviewReady
                    if let Some(ip) = &self.ip {
                        self.tex_orig = Some(paper_texture(ctx, "raster_orig", ip.original()));
                    }
                    self.tex_conv = Some(paper_texture(ctx, "raster_conv", &img));
                    self.wt_since = None;
                    self.wb_visible = false;
                    self.wb.stop();
                    self.create_enabled = true;
                    self.prevent_close = false;
                }
                ImageProcessorEvent::GenerationComplete(res) => self.on_generation_complete(res),
            }
        }
        // WT_Tick: preview takes longer than 50 ms -> show the waiting bar
        if let Some(t) = self.wt_since {
            if now - t >= 0.05 {
                self.wt_since = None;
                self.wb_visible = true;
                self.wb.start();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
    }

    /// `OnGenerationComplete(ex)`.
    fn on_generation_complete(&mut self, res: Result<(), String>) {
        let Some(ip) = &self.ip else { return };
        // usage counters of the used tool
        match ip.selected_tool() {
            Tool::Dithering => self.usage.dithering += 1,
            Tool::Line2Line => self.usage.line2line += 1,
            Tool::Vectorize => self.usage.vectorization += 1,
            Tool::Centerline => self.usage.centerline += 1,
            Tool::NoProcessing => self.usage.passthrough += 1,
        }
        self.prevent_close = false;
        self.wt_since = None;
        if let Some(mut p) = self.ip.take() {
            p.dispose();
        }
        let mut out = RasterOutcome { generated: res.is_ok(), usage: self.usage, error: None };
        if let Err(e) = res {
            // MessageBox.Show(ex.Message) then Close()
            self.msgs.push(Msg::GenError, MessageBox::new(e.clone(), ""));
            out.error = Some(e);
        }
        out.usage.raster_file = 1;
        self.outcome = Some(out);
        self.finish_after_msgs = true;
    }

    /// `BtnCancelClick`: dispose the processor and close.
    fn cancel(&mut self) {
        if let Some(mut p) = self.ip.take() {
            p.dispose();
        }
        let mut out = RasterOutcome::default();
        out.usage.raster_file = 1;
        self.outcome = Some(out);
        self.finish_after_msgs = true;
    }

    /// `StoreSettings()`.
    fn store_settings(&self) {
        let Some(ip) = &self.ip else { return };
        for (k, v) in self.c.actual_settings(ip) {
            lg_base::settings::set(k, v.to_setting());
        }
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns None while open, then the outcome (generation done or cancelled).
    pub fn show(&mut self, ctx: &egui::Context) -> Option<RasterOutcome> {
        // RasterToLaserForm_Load
        if !self.started {
            self.started = true;
            if let Some(ip) = &mut self.ip {
                ip.resume();
            }
        }
        self.drain_events(ctx);
        if self.prevent_close || self.generating {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
        if self.generating {
            ctx.set_cursor_icon(egui::CursorIcon::Wait);
        }
        let top_free = self.convert.is_none() && self.helper.is_none() && !self.msgs.is_open();
        let esc = top_free && ctx.input(|i| i.key_pressed(Key::Escape)) && !self.generating;
        let uid = self.uid;
        let title = tr_plain(&format!("{P}.$this.Text"));
        let mut action = None;
        if self.outcome.is_none() {
            let (a, closed) = form_modal_x(ctx, ("raster_to_laser", uid), &title, |ui| self.body(ui));
            action = a;
            // close box = Cancel
            if closed && top_free && !self.generating {
                action = action.or(Some(Action::Cancel));
            }
        }
        if esc && action.is_none() {
            action = Some(Action::Cancel);
        }
        match action {
            Some(Action::Cancel) => self.cancel(),
            Some(Action::Next) => self.next_clicked(),
            None => {}
        }

        // resolution helper (BtnQualityInfo / BtnFillingQualityInfo)
        if let Some((h, target)) = &mut self.helper {
            let target = *target;
            if let Some(r) = h.show(ctx) {
                self.helper = None;
                if let (Some(v), Some(ip)) = (r, &mut self.ip) {
                    // UD.Value = Math.Min(UD.Maximum, (decimal)result)
                    let v = decimal_to_f64(f64_to_decimal(v)).min(self.c.max_quality);
                    match target {
                        QualityTarget::Line => {
                            self.c.quality = v;
                            ip.set_quality(f64_to_decimal(v));
                        }
                        QualityTarget::Filling => {
                            self.c.filling_quality = v;
                            ip.set_filling_quality(f64_to_decimal(v));
                        }
                    }
                }
            }
        }

        // second page
        if let (Some(cs), Some(ip)) = (&mut self.convert, &mut self.ip) {
            if let Some(res) = cs.show(ctx, ip) {
                self.convert = None;
                if res == DialogResult::Ok {
                    self.start_generation();
                }
            }
        }

        if let Some((tag, _)) = self.msgs.show(ctx) {
            match tag {
                Msg::TimeConsuming => {}
                Msg::GenError | Msg::ProjectError => {}
            }
        }
        // the C# Close() runs after the (blocking) error box
        if self.finish_after_msgs && !self.msgs.is_open() {
            return self.outcome.take();
        }
        None
    }

    /// Dev hook (gallery automation): step 1 = press "Next", step 2 = press "Create!".
    ///
    /// `step`: 1 or 2. Returns true when the step was applied.
    pub fn debug_step(&mut self, step: u8) -> bool {
        match step {
            1 if self.create_enabled && self.convert.is_none() && !self.msgs.is_open() => {
                self.next_clicked();
                true
            }
            2 if self.convert.is_some() => {
                self.convert = None;
                self.start_generation();
                true
            }
            _ => false,
        }
    }

    /// `BtnCreateClick` start: warn before a slow filling, then open the size page.
    fn next_clicked(&mut self) {
        let Some(ip) = &self.ip else { return };
        if ip.selected_tool() == Tool::Vectorize && lg_gcode::time_consuming_filling(ip.filling_direction()) && ip.filling_quality() > Decimal::from(2) {
            let text = format!("Using {} with quality > 2 line/mm could be very time consuming with big image. Continue?", dir_text(&ip.filling_direction()));
            self.msgs.push(Msg::TimeConsuming, MessageBox::new(text, "Warning").buttons(MessageBoxButtons::OkCancel).icon(MessageBoxIcon::Warning).default_button(1));
            return;
        }
        self.open_convert();
    }

    fn open_convert(&mut self) {
        if let Some(ip) = &mut self.ip {
            self.convert = Some(ConvertSizeForm::new(ip, self.db.clone()));
        }
    }

    /// "Create!" accepted on the size page: lock the form, store the settings, start the generator.
    fn start_generation(&mut self) {
        self.prevent_close = true;
        self.tab = 0;
        self.create_enabled = false;
        self.wb_visible = true;
        self.wb.start();
        self.generating = true;
        self.store_settings();
        // Project.AddSettings(GetActualSettings())
        let Some(ip) = &mut self.ip else { return };
        let mut dict = SettingsDict::new();
        let mut err = None;
        for (k, v) in self.c.actual_settings(ip) {
            if let Err(e) = dict.add(k, v.to_project()) {
                err = Some(e);
            }
        }
        if err.is_none() {
            err = project::add_settings(dict).err();
        }
        if let Some(e) = err {
            // C# lets the exception escape the click handler: report it and keep the form usable
            self.prevent_close = false;
            self.generating = false;
            self.create_enabled = true;
            self.wb_visible = false;
            self.wb.stop();
            self.msgs.push(Msg::ProjectError, MessageBox::new(e, "").icon(MessageBoxIcon::Error));
            return;
        }
        ip.generate_gcode(); // async, ends with GenerationComplete
    }

    /// Whole form body: left options, tab control with preview, tool strip and buttons.
    fn body(&mut self, ui: &mut Ui) -> Option<Action> {
        let uid = self.uid;
        let mut action = None;
        // processor is moved out while drawing so the closures can borrow `self` freely
        let mut ip = self.ip.take()?;
        let locked = self.generating;
        ui.horizontal_top(|ui| {
            // --- left column (TlpLeft) ---
            ui.vertical(|ui| {
                ui.set_width(246.0);
                ui.add_enabled_ui(!locked, |ui| {
                    egui::ScrollArea::vertical().id_salt(("rtl_left", uid)).max_height(PB_H as f32 + 70.0).auto_shrink([false, false]).show(ui, |ui| {
                        left_column(ui, uid, &mut self.c, &mut ip, &mut self.demo_down, &mut self.helper);
                    });
                });
            });
            ui.add_space(4.0);
            // --- right column ---
            ui.vertical(|ui| {
                // TCOriginalPreview tabs
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, 0, tr_plain(&format!("{P}.TpPreview.Text")));
                    ui.selectable_value(&mut self.tab, 1, tr_plain(&format!("{P}.TpOriginal.Text")));
                });
                self.picture(ui, locked, &mut ip);
                ui.add_space(4.0);
                // bottom row: FlipControl on the left, Cancel / Next on the right
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!locked, |ui| {
                        flip_strip(ui, &mut ip, &mut self.tools);
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // BtnCreate "Next"
                        let r = ui.add_enabled(self.create_enabled && !locked, GrblButton::new(tr_plain(&format!("{P}.BtnCreate.Text"))).icon("mdi-checkbox-marked").min_size(Vec2::new(74.0, 27.0)));
                        if r.clicked() {
                            action = Some(Action::Next);
                        }
                        // BtnCancel
                        let r = ui.add_enabled(!locked, GrblButton::new(tr_plain(&format!("{P}.BtnCancel.Text"))).icon("mdi-close-box").min_size(Vec2::new(74.0, 27.0)));
                        if r.clicked() {
                            action = Some(Action::Cancel);
                        }
                    });
                });
            });
        });
        self.ip = Some(ip);
        action
    }

    /// Picture box of the active tab (`PbConverted` with the mouse tools / `PbOriginal`).
    fn picture(&mut self, ui: &mut Ui, locked: bool, ip: &mut ImageProcessor) {
        let tex = if self.tab == 0 { self.tex_conv.clone() } else { self.tex_orig.clone() };
        let size = Vec2::new(PB_W as f32, PB_H as f32);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::WHITE);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Theme::current().controls_border), egui::StrokeKind::Inside);
        // CenterImage placement (integer maths like C#)
        let (iw, ih) = tex.as_ref().map(|t| (t.size()[0] as i32, t.size()[1] as i32)).unwrap_or((0, 0));
        let (left, top) = ((PB_W - iw) / 2, (PB_H - ih) / 2);
        if let Some(t) = &tex {
            let r = Rect::from_min_size(rect.min + Vec2::new(left as f32, top as f32), Vec2::new(iw as f32, ih as f32));
            painter.image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        // waiting bar WB at (447, 468) of the preview tab
        if self.tab == 0 && self.wb_visible {
            let r = Rect::from_min_size(rect.min + Vec2::new(447.0 - 3.0, 468.0 - 3.0), Vec2::new(65.0, 16.0));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
            self.wb.show(&mut child);
        }
        if self.tab != 0 || locked || tex.is_none() {
            return;
        }
        let local = |p: Pos2| (p - rect.min).to_pos2();
        // image rectangle limits of the rubber band
        let right = PB_W - left - 6;
        let bottom = PB_H - top - 6;
        // PbConvertedMouseDown
        if self.tools.cropping && resp.drag_started_by(PointerButton::Primary) {
            if let Some(p) = resp.interact_pointer_pos().map(local) {
                let (x, y) = (p.x as i32, p.y as i32);
                if x >= left && y >= top && x <= right && y <= bottom {
                    self.tools.drag = Some((Pos2::new(x as f32, y as f32), Pos2::new(x as f32, y as f32)));
                }
            }
        }
        // PbConvertedMouseMove
        if let Some((s, _)) = self.tools.drag {
            if let Some(p) = resp.interact_pointer_pos().map(local) {
                let e = Pos2::new((p.x as i32).clamp(left, right.max(left)) as f32, (p.y as i32).clamp(top, bottom.max(top)) as f32);
                self.tools.drag = Some((s, e));
            }
            // dashed reversible frame
            if let Some((s, e)) = self.tools.drag {
                let r = Rect::from_two_pos(s + rect.min.to_vec2(), e + rect.min.to_vec2());
                let stroke = Stroke::new(1.0, Color32::from_rgb(255, 0, 0));
                painter.extend(egui::Shape::dashed_line(&[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()], stroke, 4.0, 3.0));
            }
        }
        // PbConvertedMouseUp
        let up: Option<(PointerButton, Pos2)> = [PointerButton::Primary, PointerButton::Secondary, PointerButton::Middle]
            .into_iter()
            .find(|b| resp.drag_stopped_by(*b) || resp.clicked_by(*b))
            .and_then(|b| resp.interact_pointer_pos().or(ui.input(|i| i.pointer.latest_pos())).map(|p| (b, local(p))));
        if self.tools.drag.is_some() && self.tools.cropping && resp.drag_stopped_by(PointerButton::Primary) {
            let (s, e) = self.tools.drag.take().unwrap_or_default();
            let rect_c = IpRect { x: s.x.min(e.x) as i32 - left, y: s.y.min(e.y) as i32 - top, width: (e.x - s.x).abs() as i32, height: (e.y - s.y).abs() as i32 };
            // NOTE(upstream): the size passed is the paper image size (image + 6 px shadow)
            ip.crop_image(rect_c, (iw, ih));
            self.tools.cropping = false;
        } else if let Some((b, p)) = up {
            let loc = Point::new(p.x as i32 - left, p.y as i32 - top);
            if self.tools.filling >= 0 {
                let color = if b == PointerButton::Primary { Color { a: 255, r: 0, g: 0, b: 0 } } else { Color { a: 255, r: 255, g: 255, b: 255 } };
                ip.fill(loc, (iw, ih), color, self.tools.filling);
            }
            if self.tools.outlining {
                ip.outliner(loc, (iw, ih));
            }
        }
    }
}

/// `CreatePaper` + upload as texture.
///
/// `ctx`: egui context. `name`: texture name. `img`: preview bitmap. Returns the texture handle.
fn paper_texture(ctx: &egui::Context, name: &str, img: &RgbaImage) -> TextureHandle {
    let p = create_paper(img);
    let ci = ColorImage::from_rgba_unmultiplied([p.width() as usize, p.height() as usize], p.as_raw());
    ctx.load_texture(name, ci, TextureOptions::NEAREST)
}

/// `CreatePaper(img)`: transparent canvas with shadow, white page, light gray border and the picture.
///
/// `img`: source bitmap. Returns a bitmap 6 px larger in both directions.
fn create_paper(img: &RgbaImage) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let mut out = RgbaImage::from_pixel(w + 6, h + 6, Rgba([0, 0, 0, 0]));
    let fill = |out: &mut RgbaImage, x: u32, y: u32, fw: u32, fh: u32, c: [u8; 4]| {
        for yy in y..(y + fh).min(out.height()) {
            for xx in x..(x + fw).min(out.width()) {
                out.put_pixel(xx, yy, Rgba(c));
            }
        }
    };
    // shadow, page
    fill(&mut out, 6, 6, w + 2, h + 2, [128, 128, 128, 255]);
    fill(&mut out, 0, 0, w + 2, h + 2, [255, 255, 255, 255]);
    // border: DrawRectangle(LightGray, 0, 0, w + 1, h + 1)
    let lg = Rgba([211, 211, 211, 255]);
    for x in 0..=w + 1 {
        out.put_pixel(x, 0, lg);
        out.put_pixel(x, h + 1, lg);
    }
    for y in 0..=h + 1 {
        out.put_pixel(0, y, lg);
        out.put_pixel(w + 1, y, lg);
    }
    // picture at (1,1), source-over
    for y in 0..h {
        for x in 0..w {
            let s = img.get_pixel(x, y).0;
            let d = out.get_pixel(x + 1, y + 1).0;
            let a = s[3] as u32;
            let mix = |sc: u8, dc: u8| ((sc as u32 * a + dc as u32 * (255 - a) + 127) / 255) as u8;
            out.put_pixel(x + 1, y + 1, Rgba([mix(s[0], d[0]), mix(s[1], d[1]), mix(s[2], d[2]), 255]));
        }
    }
    out
}

/// Pressed-button back color (`ColorScheme.PressedButtons`) or transparent.
fn tool_back(active: bool) -> Color32 {
    if active { Theme::current().pressed_buttons } else { Color32::TRANSPARENT }
}

/// `FlipControl`: revert, rotate, flip, crop, auto trim, fill, outliner, invert.
///
/// `ui`: parent. `ip`: processor. `t`: crop / fill / outline mode state.
fn flip_strip(ui: &mut Ui, ip: &mut ImageProcessor, t: &mut Tools) {
    let sz = Vec2::splat(25.0);
    ui.spacing_mut().item_spacing.x = 6.0;
    // BtnRevert
    if tip(ui.add(ImageButton::new("mdi-arrow-u-left-top-bold").size(sz)), P, "BtnRevert").clicked() {
        ip.revert();
    }
    ui.add_space(9.0);
    // BtRotateCW / BtRotateCCW / BtFlipH / BtFlipV (icon keys exactly as the C# assigns them)
    if tip(ui.add(ImageButton::new("mdi-rotate-right").size(sz)), P, "BtRotateCW").clicked() {
        ip.rotate_cw();
    }
    if tip(ui.add(ImageButton::new("mdi-rotate-left").size(sz)), P, "BtRotateCCW").clicked() {
        ip.rotate_ccw();
    }
    if tip(ui.add(ImageButton::new("mdi-flip-vertical").size(sz)), P, "BtFlipH").clicked() {
        ip.flip_h();
    }
    if tip(ui.add(ImageButton::new("mdi-flip-horizontal").size(sz)), P, "BtFlipV").clicked() {
        ip.flip_v();
    }
    // BtnCrop
    if tip(ui.add(ImageButton::new("mdi-crop").size(sz).back_color(tool_back(t.cropping))), P, "BtnCrop").clicked() {
        if t.filling >= 0 || t.outlining {
            t.filling = -1;
            t.outlining = false;
        }
        t.cropping = !t.cropping;
    }
    // BtnAutoTrim
    if tip(ui.add(ImageButton::new("mdi-image-size-select-large").size(sz)), P, "BtnAutoTrim").clicked() {
        ip.auto_trim();
    }
    // BtnFill
    if tip(ui.add(ImageButton::new("mdi-format-color-fill").size(sz).back_color(tool_back(t.filling >= 0))), P, "BtnFill").clicked() {
        if t.cropping || t.outlining {
            t.cropping = false;
            t.outlining = false;
        }
        t.filling = if t.filling >= 0 { -1 } else { 127 };
    }
    // BtnOutliner
    if tip(ui.add(ImageButton::new("mdi-auto-fix").size(sz).back_color(tool_back(t.outlining))), P, "BtnOutliner").clicked() {
        if t.filling >= 0 || t.cropping {
            t.filling = -1;
            t.cropping = false;
        }
        t.outlining = !t.outlining;
    }
    // BtnReverse
    if tip(ui.add(ImageButton::new("mdi-invert-colors").size(sz)), P, "BtnReverse").clicked() {
        ip.invert();
    }
}

/// Slider colors of the designer: bar outer / inner, elapsed outer / inner.
fn colors(bo: Color32, bi: Color32, eo: Color32, ei: Color32) -> SliderColors {
    SliderColors { bar_outer: bo, bar_inner: bi, elapsed_outer: eo, elapsed_inner: ei, ..SliderColors::default() }
}

/// One designer ColorSlider (thumb 8, rounded 8/4, length 170). Returns (changed, response).
fn slider(ui: &mut Ui, v: &mut i32, range: std::ops::RangeInclusive<i32>, c: SliderColors, len: f32, reset: Option<i32>) -> (bool, egui::Response) {
    let before = *v;
    let r = ui.add(ColorSlider::new(v, range).thumb_size(8.0).thumb_round(4.0).border_round(8.0).large_change(5).small_change(1).colors(c).length(len));
    // DoubleClick handler: reset value
    if r.double_clicked() {
        if let Some(d) = reset {
            *v = d;
        }
    }
    (*v != before, r)
}

/// Gray track bar colors (brightness, contrast, threshold, white clip).
fn gray_colors() -> SliderColors {
    colors(Color32::BLACK, Color32::from_rgb(105, 105, 105), Color32::from_rgb(224, 224, 224), Color32::WHITE)
}

/// Yellow colors of the centerline thresholds.
fn gold_colors() -> SliderColors {
    colors(Color32::from_rgb(255, 215, 0), Color32::from_rgb(250, 250, 210), Color32::from_rgb(255, 215, 0), Color32::YELLOW)
}

/// UpDown helper (`NumericUpDown` with decimal places). Returns true when the value changed.
fn updown(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, v: &mut f64, min: f64, max: f64, inc: f64, dp: usize, enabled: bool, tip_ctl: &str) -> bool {
    let before = *v;
    let r = ui.add_enabled(enabled, NumericUpDown::new(v).range(min, max).increment(inc).decimal_places(dp).width(54.0).id(egui::Id::unique(id)));
    // WinForms ToolTip shows on disabled controls too
    let key = format!("{P}.{tip_ctl}.ToolTip");
    let text = tr(&key);
    if text != key && !text.is_empty() {
        r.on_hover_text(text.to_owned()).on_disabled_hover_text(text.to_owned());
    }
    *v != before
}

/// Left column: parameters, conversion tool and the option group of the selected tool.
fn left_column(ui: &mut Ui, uid: u64, c: &mut Ctl, ip: &mut ImageProcessor, demo_down: &mut bool, helper: &mut Option<(ResolutionHelperForm, QualityTarget)>) {
    let t = |ctl: &str| tr_plain(&format!("{P}.{ctl}.Text"));
    let no_proc = c.tool == Tool::NoProcessing;
    // ---- GbParameters ----
    if !no_proc {
        grbl_group_box(ui, &t("GbParameters"), |ui| {
            ui.set_min_width(226.0);
            egui::Grid::new(("rtl_params", uid)).num_columns(2).spacing([4.0, 3.0]).show(ui, |ui| {
                // Resize
                ui.label(t("label28"));
                let items = [InterpolationMode::HighQualityBicubic, InterpolationMode::NearestNeighbor];
                let before = c.interpolation;
                let r = enum_combo(ui, ("rtl_resize", uid), &mut c.interpolation, &items, |v| translate_enum(*v, "InterpolationMode"), false, 168.0);
                tip(r, P, "CbResize");
                if before != c.interpolation {
                    ip.set_interpolation(c.interpolation);
                }
                ui.end_row();
                // Grayscale formula (colour images only)
                if !c.is_gray {
                    ui.label(t("LblGrayscale"));
                    let items = [Formula::SimpleAverage, Formula::WeightAverage, Formula::OpticalCorrect, Formula::Custom];
                    let before = c.formula;
                    let r = enum_combo(ui, ("rtl_mode", uid), &mut c.formula, &items, |v| translate_enum(*v, "Formula"), false, 168.0);
                    tip(r, P, "CbMode");
                    if before != c.formula {
                        ip.set_formula(c.formula);
                    }
                    ui.end_row();
                    if c.formula == Formula::Custom {
                        let red = colors(Color32::from_rgb(139, 0, 0), Color32::from_rgb(178, 34, 34), Color32::from_rgb(139, 0, 0), Color32::RED);
                        let green = colors(Color32::from_rgb(0, 100, 0), Color32::from_rgb(0, 128, 0), Color32::from_rgb(0, 100, 0), Color32::from_rgb(127, 255, 0));
                        let blue = colors(Color32::from_rgb(0, 0, 139), Color32::from_rgb(0, 0, 205), Color32::from_rgb(70, 130, 180), Color32::from_rgb(30, 144, 255));
                        ui.label(t("LblRed"));
                        let (ch, _) = slider(ui, &mut c.red, 40..=160, red, 170.0, Some(100));
                        if ch {
                            ip.set_red(c.red);
                        }
                        ui.end_row();
                        ui.label(t("LblGreen"));
                        let (ch, _) = slider(ui, &mut c.green, 40..=160, green, 170.0, Some(100));
                        if ch {
                            ip.set_green(c.green);
                        }
                        ui.end_row();
                        ui.label(t("LblBlue"));
                        let (ch, _) = slider(ui, &mut c.blue, 40..=160, blue, 170.0, Some(100));
                        if ch {
                            ip.set_blue(c.blue);
                        }
                        ui.end_row();
                    }
                }
                // Brightness / Contrast
                ui.label(t("label2"));
                let (ch, _) = slider(ui, &mut c.brightness, 40..=160, gray_colors(), 170.0, Some(100));
                if ch {
                    ip.set_brightness(c.brightness);
                }
                ui.end_row();
                ui.label(t("label3"));
                let (ch, _) = slider(ui, &mut c.contrast, 40..=160, gray_colors(), 170.0, Some(100));
                if ch {
                    ip.set_contrast(c.contrast);
                }
                ui.end_row();
                // White clip: mouse down / up toggles the demo overlay
                ui.label(t("label4"));
                let (ch, r) = slider(ui, &mut c.white_clip, 0..=100, gray_colors(), 170.0, None);
                if ch {
                    ip.set_white_clip(c.white_clip);
                }
                let down = r.is_pointer_button_down_on();
                if down != *demo_down {
                    *demo_down = down;
                    ip.set_demo(down);
                }
                ui.end_row();
                // Threshold
                let dithering = c.tool == Tool::Dithering;
                if !dithering {
                    let before = c.use_threshold;
                    let r = ui.checkbox(&mut c.use_threshold, t("CbThreshold"));
                    tip(r, P, "CbThreshold");
                    if before != c.use_threshold {
                        ip.set_use_threshold(c.use_threshold);
                    }
                    if c.use_threshold {
                        let (ch, _) = slider(ui, &mut c.threshold, 0..=100, gray_colors(), 170.0, Some(50));
                        if ch {
                            ip.set_threshold(c.threshold);
                        }
                    }
                    ui.end_row();
                }
            });
        });
    }
    // ---- GbConversionTool ----
    grbl_group_box(ui, &t("GbConversionTool"), |ui| {
        ui.set_min_width(226.0);
        let before = c.tool;
        let mut radio = |ui: &mut Ui, tool: Tool, ctl: &str| {
            let r = ui.radio_value(&mut c.tool, tool, t(ctl));
            tip(r, P, ctl);
        };
        if c.support_pwm {
            radio(ui, Tool::Line2Line, "RbLineToLineTracing");
        }
        radio(ui, Tool::Dithering, "RbDithering");
        radio(ui, Tool::Vectorize, "RbVectorize");
        radio(ui, Tool::Centerline, "RbCenterline");
        radio(ui, Tool::NoProcessing, "RbNoProcessing");
        if before != c.tool {
            ip.set_selected_tool(c.tool);
        }
    });
    // ---- GbPassthrough ----
    if no_proc {
        grbl_group_box(ui, &t("GbPassthrough"), |ui| {
            ui.set_min_width(226.0);
            let mut text = tr(&format!("{P}.TbPassthroughInfo.Text")).to_string();
            ui.add(TextInput::new(&mut text).read_only(true).multiline(true).rows(5).width(220.0));
        });
    }
    // ---- GbCenterlineOptions ----
    if c.tool == Tool::Centerline {
        grbl_group_box(ui, &t("GbCenterlineOptions"), |ui| {
            ui.set_min_width(226.0);
            egui::Grid::new(("rtl_center", uid)).num_columns(3).spacing([4.0, 3.0]).show(ui, |ui| {
                ui.label(t("label7"));
                ui.add_enabled_ui(c.use_corner_threshold, |ui| {
                    let (ch, r) = slider(ui, &mut c.corner_threshold, 0..=360, gold_colors(), 119.0, Some(110));
                    tip(r, P, "TBCornerThreshold");
                    if ch {
                        ip.set_corner_threshold(c.corner_threshold);
                    }
                });
                let before = c.use_corner_threshold;
                ui.checkbox(&mut c.use_corner_threshold, "");
                if before != c.use_corner_threshold {
                    ip.set_use_corner_threshold(c.use_corner_threshold);
                }
                ui.end_row();
                ui.label(t("label6"));
                ui.add_enabled_ui(c.use_line_threshold, |ui| {
                    let (ch, r) = slider(ui, &mut c.line_threshold, 0..=100, gold_colors(), 119.0, Some(10));
                    tip(r, P, "TBLineThreshold");
                    if ch {
                        ip.set_line_threshold(c.line_threshold);
                    }
                });
                let before = c.use_line_threshold;
                ui.checkbox(&mut c.use_line_threshold, "");
                if before != c.use_line_threshold {
                    ip.set_use_line_threshold(c.use_line_threshold);
                }
                ui.end_row();
            });
        });
    }
    // ---- GbLineToLineOptions ----
    if matches!(c.tool, Tool::Line2Line | Tool::Dithering) {
        let title = if c.tool == Tool::Line2Line { tr("Strings.Line2LineOptions") } else { tr("Strings.DitheringOptions") };
        grbl_group_box(ui, title, |ui| {
            ui.set_min_width(226.0);
            egui::Grid::new(("rtl_l2l", uid)).num_columns(4).spacing([4.0, 3.0]).show(ui, |ui| {
                if c.tool == Tool::Dithering {
                    ui.label(t("LblDitherMode"));
                    let items = DitheringMode::table().iter().map(|(d, _)| *d).collect::<Vec<_>>();
                    let before = c.dither;
                    // FlatComboBox with raw enum names
                    let mut sel = items.iter().position(|d| *d == c.dither);
                    let names: Vec<String> = items.iter().map(|d| d.name().to_string()).collect();
                    let r = flat_combo(ui, ("rtl_dither", uid), &mut sel, &names, 150.0);
                    if r.changed() {
                        if let Some(i) = sel {
                            c.dither = items[i];
                        }
                    }
                    if before != c.dither {
                        ip.set_dithering_mode(c.dither);
                    }
                    ui.label("");
                    ui.label("");
                    ui.end_row();
                }
                // Direction
                ui.label(t("label27"));
                let before = c.direction;
                let items = c.line_dirs.clone();
                let r = enum_combo(ui, ("rtl_dir", uid), &mut c.direction, &items, dir_text, true, 150.0);
                tip(r, P, "CbDirections");
                if before != c.direction {
                    ip.set_line_direction(c.direction);
                }
                ui.label("");
                ui.label("");
                ui.end_row();
                // Quality
                ui.label(t("label5"));
                let max = c.max_quality;
                let r_before = c.quality;
                let r = ui.add(NumericUpDown::new(&mut c.quality).range(1.0, max).decimal_places(3).width(55.0).id(egui::Id::unique(("rtl_q", uid))));
                tip(r, P, "UDQuality");
                if r_before != c.quality {
                    ip.set_quality(f64_to_decimal(c.quality));
                }
                ui.label(t("label8"));
                let r = ui.add(ImageButton::new("mdi-information-slab-box").size(Vec2::splat(17.0)));
                if tip(r, P, "BtnQualityInfo").clicked() && helper.is_none() {
                    *helper = Some((ResolutionHelperForm::new(c.quality), QualityTarget::Line));
                }
                ui.end_row();
                // Line preview
                let before = c.line_preview;
                let r = ui.checkbox(&mut c.line_preview, t("CbLinePreview"));
                tip(r, P, "CbLinePreview");
                if before != c.line_preview {
                    ip.set_line_preview(c.line_preview);
                }
                ui.end_row();
            });
        });
    }
    // ---- GbVectorizeOptions ----
    if c.tool == Tool::Vectorize {
        grbl_group_box(ui, &t("GbVectorizeOptions"), |ui| {
            ui.set_min_width(226.0);
            egui::Grid::new(("rtl_vec", uid)).num_columns(4).spacing([4.0, 3.0]).show(ui, |ui| {
                // Spot removal
                ui.label(t("label22"));
                // (int) cast of the UpDown value in the C# handler: decimals = 0
                if updown(ui, ("rtl_spot", uid), &mut c.spot_removal, 2.0, 100.0, 1.0, 0, c.use_spot_removal, "UDSpotRemoval") {
                    ip.set_spot_removal(f64_to_decimal(c.spot_removal.trunc()));
                }
                let before = c.use_spot_removal;
                ui.checkbox(&mut c.use_spot_removal, "");
                if before != c.use_spot_removal {
                    ip.set_use_spot_removal(c.use_spot_removal);
                }
                ui.end_row();
                // Smoothing
                ui.label(t("label23"));
                if updown(ui, ("rtl_smooth", uid), &mut c.smoothing, 0.0, 100.0, 0.1, 1, c.use_smoothing, "UDSmoothing") {
                    ip.set_smoothing(f64_to_decimal(c.smoothing));
                }
                let before = c.use_smoothing;
                ui.checkbox(&mut c.use_smoothing, "");
                if before != c.use_smoothing {
                    ip.set_use_smoothing(c.use_smoothing);
                }
                ui.end_row();
                // Optimize
                ui.label(t("label24"));
                if updown(ui, ("rtl_opt", uid), &mut c.optimize, 0.2, 50.0, 0.1, 1, c.use_optimize, "UDOptimize") {
                    ip.set_optimize(f64_to_decimal(c.optimize));
                }
                let before = c.use_optimize;
                ui.checkbox(&mut c.use_optimize, "");
                if before != c.use_optimize {
                    ip.set_use_optimize(c.use_optimize);
                }
                ui.end_row();
                // Downsampling
                ui.label(t("label1"));
                if updown(ui, ("rtl_down", uid), &mut c.down_sample, 1.0, 10.0, 0.1, 1, c.use_down_sample, "UDDownSample") {
                    ip.set_down_sampling(f64_to_decimal(c.down_sample));
                }
                let before = c.use_down_sample;
                ui.checkbox(&mut c.use_down_sample, "");
                if before != c.use_down_sample {
                    ip.set_use_down_sampling(c.use_down_sample);
                }
                ui.end_row();
                // Adaptive quality
                ui.label(t("LAdaptiveQuality"));
                let before = c.adaptive_quality;
                ui.checkbox(&mut c.adaptive_quality, "");
                if before != c.adaptive_quality {
                    ip.set_use_adaptive_quality(c.adaptive_quality);
                }
                ui.label("");
                let r = ui.add(ImageButton::new("mdi-information-slab-box").size(Vec2::splat(17.0)));
                if tip(r, P, "BtnAdaptiveQualityInfo").clicked() {
                    lg_base::tools::utils::open_link("https://lasergrbl.com/usage/raster-image-import/vectorization-tool/#adaptive-quality");
                }
                ui.end_row();
                // Optimize travel
                ui.label(t("lOptimizeFast"));
                let before = c.optimize_fast;
                ui.checkbox(&mut c.optimize_fast, "");
                if before != c.optimize_fast {
                    ip.set_optimize_fast(c.optimize_fast);
                }
                ui.end_row();
                // Filling
                ui.label(t("label14"));
                let before = c.filling_direction;
                let items = c.fill_dirs.clone();
                let r = enum_combo(ui, ("rtl_fill", uid), &mut c.filling_direction, &items, dir_text, false, 113.0);
                tip(r, P, "CbFillingDirection");
                if before != c.filling_direction {
                    ip.set_filling_direction(c.filling_direction);
                }
                ui.end_row();
                if c.filling_direction != Direction::None {
                    ui.label(t("LblFillingQuality"));
                    let max = c.max_quality;
                    let before = c.filling_quality;
                    let r = ui.add(NumericUpDown::new(&mut c.filling_quality).range(0.1, max).decimal_places(3).width(55.0).id(egui::Id::unique(("rtl_fq", uid))));
                    tip(r, P, "UDFillingQuality");
                    if before != c.filling_quality {
                        ip.set_filling_quality(f64_to_decimal(c.filling_quality));
                    }
                    ui.label(t("LblFillingLineLbl"));
                    let r = ui.add(ImageButton::new("mdi-information-slab-box").size(Vec2::splat(17.0)));
                    if tip(r, P, "BtnFillingQualityInfo").clicked() && helper.is_none() {
                        *helper = Some((ResolutionHelperForm::new(c.filling_quality), QualityTarget::Filling));
                    }
                    ui.end_row();
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_has_shadow_and_border() {
        let img = RgbaImage::from_pixel(4, 3, Rgba([10, 20, 30, 255]));
        let p = create_paper(&img);
        assert_eq!((p.width(), p.height()), (10, 9));
        assert_eq!(p.get_pixel(0, 0).0, [211, 211, 211, 255]);
        assert_eq!(p.get_pixel(1, 1).0, [10, 20, 30, 255]);
        assert_eq!(p.get_pixel(8, 8).0, [128, 128, 128, 255]);
        assert_eq!(p.get_pixel(9, 0).0, [0, 0, 0, 0]);
    }

    #[test]
    fn icon_keys_exist() {
        let i = crate::ui::icons::Icons::new(false);
        for k in [
            "mdi-flip-vertical",
            "mdi-flip-horizontal",
            "mdi-rotate-right",
            "mdi-rotate-left",
            "mdi-crop",
            "mdi-arrow-u-left-top-bold",
            "mdi-image-size-select-large",
            "mdi-format-color-fill",
            "mdi-invert-colors",
            "mdi-auto-fix",
            "mdi-information-slab-box",
            "mdi-checkbox-marked",
            "mdi-close-box",
            "mdi-database",
            "mdi-target",
            "mdi-lock",
            "mdi-lock-open-outline",
            "custom-exif",
        ] {
            assert!(i.contains(k), "missing icon {k}");
        }
    }
}
