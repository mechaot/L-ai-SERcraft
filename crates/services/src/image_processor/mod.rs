// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `RasterConverter/ImageProcessor.cs`.
//!
//! Same life cycle as C#: the processor holds the original bitmaps and all options; every
//! property change calls `Refresh` (unless suspended) which aborts the running preview worker,
//! clones the state and starts a new preview thread; [`ImageProcessor::generate_gcode`] does the
//! same for the G-code worker (`DoTrueWork`). The static C# events become
//! [`ImageProcessorEvent`] values sent through a `crossbeam_channel::Sender`.
//!
//! `GrblCore` is replaced by an `Arc<RwLock<GrblFile>>` (`mCore.LoadedFile`); the loaders read
//! "Support Hardware PWM" / "Firmware Type" from the settings exactly like C#.
//! GDI+ drawing in the previews is reproduced with direct pixel writes / tiny-skia, see
//! [`overlay`].

mod file_dpi;
mod job;
pub mod overlay;

pub use file_dpi::{file_dpi, horizontal_resolution};
pub use lg_potrace::Direction;
pub use lg_raster::{DitheringMode, Formula, InterpolationMode, Point, Rect};
pub use lg_raster::Color;

use crossbeam_channel::Sender;
use image::RgbaImage;
use lg_dotnet::{f64_to_i32, round_half_even, Decimal};
use lg_gcode::GrblFile;
use lg_raster::{clone_rect, resize_image, rotate_flip, RotateFlipType};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// C# `ImageProcessor.Tool` (same order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Line2Line,
    Dithering,
    Vectorize,
    Centerline,
    NoProcessing,
}

/// C# events `PreviewBegin` / `PreviewReady` / `GenerationComplete`.
#[derive(Debug, Clone)]
pub enum ImageProcessorEvent {
    /// A preview thread is about to start.
    PreviewBegin,
    /// A preview image is ready (never sent for an aborted worker).
    PreviewReady(RgbaImage),
    /// G-code generation ended; `Err(message)` is the C# exception text.
    GenerationComplete(Result<(), String>),
}

/// All options that feed the image pipeline (C# `m*` fields).
#[derive(Debug, Clone)]
pub(crate) struct Options {
    pub interpolation: InterpolationMode,
    pub tool: Tool,
    pub formula: Formula,
    pub red: i32,
    pub green: i32,
    pub blue: i32,
    pub contrast: i32,
    pub white_point: i32,
    pub brightness: i32,
    pub threshold: i32,
    pub use_threshold: bool,
    pub quality: Decimal,
    pub line_preview: bool,
    pub spot_removal: Decimal,
    pub use_spot_removal: bool,
    pub optimize: Decimal,
    pub use_optimize: bool,
    pub use_adaptive_quality: bool,
    pub smoothing: Decimal,
    pub use_smoothing: bool,
    pub down_sampling: Decimal,
    pub use_down_sampling: bool,
    pub optimize_fast: bool,
    pub direction: Direction,
    pub filling_direction: Direction,
    pub dithering: DitheringMode,
    pub filling_quality: Decimal,
    pub use_line_threshold: bool,
    pub line_threshold: i32,
    pub use_corner_threshold: bool,
    pub corner_threshold: i32,
    pub demo: bool,
}

impl Default for Options {
    /// C# field initialisers: everything zero / false except `Interpolation = HighQualityBicubic`.
    fn default() -> Self {
        Self {
            interpolation: InterpolationMode::HighQualityBicubic,
            tool: Tool::Line2Line,
            formula: Formula::SimpleAverage,
            red: 0,
            green: 0,
            blue: 0,
            contrast: 0,
            white_point: 0,
            brightness: 0,
            threshold: 0,
            use_threshold: false,
            quality: Decimal::ZERO,
            line_preview: false,
            spot_removal: Decimal::ZERO,
            use_spot_removal: false,
            optimize: Decimal::ZERO,
            use_optimize: false,
            use_adaptive_quality: false,
            smoothing: Decimal::ZERO,
            use_smoothing: false,
            down_sampling: Decimal::ZERO,
            use_down_sampling: false,
            optimize_fast: false,
            direction: Direction::None,
            filling_direction: Direction::None,
            dithering: DitheringMode::Atkinson,
            filling_quality: Decimal::ZERO,
            use_line_threshold: false,
            line_threshold: 0,
            use_corner_threshold: false,
            corner_threshold: 0,
            demo: false,
        }
    }
}

/// Options for the G-code generator (C# public fields `TargetSize`... `MaxPower`).
#[derive(Debug, Clone, Default)]
pub struct GcodeOptions {
    /// `TargetSize` (mm): width, height.
    pub target_size: (f32, f32),
    /// `TargetOffset` (mm): x, y.
    pub target_offset: (f32, f32),
    /// `LaserOn` (e.g. `M3`).
    pub laser_on: String,
    /// `LaserOff` (e.g. `M5`).
    pub laser_off: String,
    pub border_speed: i32,
    pub mark_speed: i32,
    pub min_power: i32,
    pub max_power: i32,
}

/// Running worker (C# `Current` + its `TH` / `MustExit`).
struct Worker {
    must_exit: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// C# `AbortThread`: raise the exit flag and wait up to 100 ms (a Rust thread cannot be killed;
    /// it ends by itself at its next exit check).
    fn abort(mut self) {
        self.must_exit.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let end = Instant::now() + Duration::from_millis(100);
            while !h.is_finished() && Instant::now() < end {
                std::thread::sleep(Duration::from_millis(2));
            }
            // detach when still running
        }
    }
}

/// C# `ImageProcessor`.
pub struct ImageProcessor {
    file: Arc<RwLock<GrblFile>>,
    events: Sender<ImageProcessorEvent>,
    /// `mTrueOriginal`: image as loaded.
    true_original: Arc<RgbaImage>,
    /// `mOriginal`: cropped / rotated image.
    original: Arc<RgbaImage>,
    /// `mResized`: image fitted into the preview box.
    resized: RgbaImage,
    file_dpi: i32,
    file_resolution: (u32, u32),
    gray_scale: bool,
    suspended: bool,
    box_size: (i32, i32),
    opts: Options,
    /// Generator options (public fields in C#).
    pub gcode: GcodeOptions,
    file_name: String,
    append: bool,
    current: Option<Worker>,
}

/// Expands to the getter/setter pair of a plain property. `refresh` = C# calls `Refresh()`.
macro_rules! prop {
    ($(#[$m:meta])* $get:ident, $set:ident, $field:ident, $t:ty, refresh) => {
        $(#[$m])*
        pub fn $get(&self) -> $t {
            self.opts.$field
        }
        $(#[$m])*
        /// Setter: on change stores the value and refreshes the preview.
        pub fn $set(&mut self, value: $t) {
            if value != self.opts.$field {
                self.opts.$field = value;
                self.refresh();
            }
        }
    };
    ($(#[$m:meta])* $get:ident, $set:ident, $field:ident, $t:ty, quiet) => {
        $(#[$m])*
        pub fn $get(&self) -> $t {
            self.opts.$field
        }
        $(#[$m])*
        /// Setter: stores the value, no refresh (C# has that call commented out).
        pub fn $set(&mut self, value: $t) {
            self.opts.$field = value;
        }
    };
}

impl ImageProcessor {
    /// C# ctor: load the file, normalise to 32 bpp, read the DPI, fit into the box. Starts suspended.
    ///
    /// `file`: shared program the generator fills (`core.LoadedFile`). `file_name`: image path.
    /// `box_size`: preview box (width, height). `append`: append to the program instead of replacing.
    /// `events`: channel for the C# events. Returns the processor or the load error.
    pub fn new(file: Arc<RwLock<GrblFile>>, file_name: &str, box_size: (i32, i32), append: bool, events: Sender<ImageProcessorEvent>) -> Result<Self, String> {
        let bytes = std::fs::read(file_name).map_err(|e| e.to_string())?;
        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?.to_rgba8();
        let dpi = file_dpi(&bytes);
        Ok(Self::from_image(file, file_name, img, dpi, box_size, append, events))
    }

    /// Ctor from an already decoded image (tests, clipboard images).
    ///
    /// `img`: pixels. `dpi`: value for `FileDPI`. Other arguments as [`new`](Self::new).
    pub fn from_image(file: Arc<RwLock<GrblFile>>, file_name: &str, img: RgbaImage, dpi: i32, box_size: (i32, i32), append: bool, events: Sender<ImageProcessorEvent>) -> Self {
        let res = (img.width(), img.height());
        let original = Arc::new(img);
        let mut p = Self {
            file,
            events,
            true_original: original.clone(),
            gray_scale: false,
            resized: RgbaImage::new(1, 1),
            original,
            file_dpi: dpi,
            file_resolution: res,
            suspended: true,
            box_size,
            opts: Options::default(),
            gcode: GcodeOptions::default(),
            file_name: file_name.to_string(),
            append,
            current: None,
        };
        p.resize_recalc();
        p.gray_scale = test_gray_scale(&p.original);
        p
    }

    /// Test helper: change the preview box without refresh.
    #[cfg(test)]
    pub(crate) fn set_box_for_test(&mut self, size: (i32, i32)) {
        self.box_size = size;
        self.resize_recalc();
    }

    /// C# `FormResize`: new preview box size.
    ///
    /// `size`: box (width, height).
    pub fn form_resize(&mut self, size: (i32, i32)) {
        self.box_size = size;
        self.resize_recalc();
        self.refresh();
    }

    /// C# `IsGrayScale`.
    pub fn is_gray_scale(&self) -> bool {
        self.gray_scale
    }

    /// C# `Dispose`: suspend and abort the worker.
    pub fn dispose(&mut self) {
        self.suspend();
        if let Some(w) = self.current.take() {
            w.abort();
        }
    }

    /// C# `Suspend`: batch property changes without previews.
    pub fn suspend(&mut self) {
        self.suspended = true;
    }

    /// C# `Resume`: leave suspension and refresh once.
    pub fn resume(&mut self) {
        if self.suspended {
            self.suspended = false;
            self.refresh();
        }
    }

    prop!(/// C# `SelectedTool`.
        selected_tool, set_selected_tool, tool, Tool, refresh);
    prop!(/// C# `Formula`.
        formula, set_formula, formula, Formula, refresh);
    prop!(/// C# `DitheringMode`.
        dithering_mode, set_dithering_mode, dithering, DitheringMode, refresh);
    prop!(/// C# `Red` (percent).
        red, set_red, red, i32, refresh);
    prop!(/// C# `Green` (percent).
        green, set_green, green, i32, refresh);
    prop!(/// C# `Blue` (percent).
        blue, set_blue, blue, i32, refresh);
    prop!(/// C# `Contrast` (percent).
        contrast, set_contrast, contrast, i32, refresh);
    prop!(/// C# `Brightness` (percent).
        brightness, set_brightness, brightness, i32, refresh);
    prop!(/// C# `WhiteClip`.
        white_clip, set_white_clip, white_point, i32, refresh);
    prop!(/// C# `Threshold` (percent).
        threshold, set_threshold, threshold, i32, refresh);
    prop!(/// C# `UseThreshold`.
        use_threshold, set_use_threshold, use_threshold, bool, refresh);
    prop!(/// C# `Quality` (lines/mm).
        quality, set_quality, quality, Decimal, quiet);
    prop!(/// C# `LinePreview`.
        line_preview, set_line_preview, line_preview, bool, refresh);
    prop!(/// C# `SpotRemoval`.
        spot_removal, set_spot_removal, spot_removal, Decimal, refresh);
    prop!(/// C# `UseSpotRemoval`.
        use_spot_removal, set_use_spot_removal, use_spot_removal, bool, refresh);
    prop!(/// C# `Optimize`.
        optimize, set_optimize, optimize, Decimal, refresh);
    prop!(/// C# `UseOptimize`.
        use_optimize, set_use_optimize, use_optimize, bool, refresh);
    prop!(/// C# `UseAdaptiveQuality` (never refreshes).
        use_adaptive_quality, set_use_adaptive_quality, use_adaptive_quality, bool, quiet);
    prop!(/// C# `Smoothing`.
        smoothing, set_smoothing, smoothing, Decimal, refresh);
    prop!(/// C# `UseSmoothing`.
        use_smoothing, set_use_smoothing, use_smoothing, bool, refresh);
    prop!(/// C# `UseDownSampling`.
        use_down_sampling, set_use_down_sampling, use_down_sampling, bool, refresh);
    prop!(/// C# `DownSampling`.
        down_sampling, set_down_sampling, down_sampling, Decimal, refresh);
    prop!(/// C# `OptimizeFast`.
        optimize_fast, set_optimize_fast, optimize_fast, bool, quiet);
    prop!(/// C# `LineDirection`.
        line_direction, set_line_direction, direction, Direction, refresh);
    prop!(/// C# `FillingDirection`.
        filling_direction, set_filling_direction, filling_direction, Direction, refresh);
    prop!(/// C# `FillingQuality`.
        filling_quality, set_filling_quality, filling_quality, Decimal, quiet);
    prop!(/// C# `LineThreshold`.
        line_threshold, set_line_threshold, line_threshold, i32, refresh);
    prop!(/// C# `UseLineThreshold`.
        use_line_threshold, set_use_line_threshold, use_line_threshold, bool, refresh);
    prop!(/// C# `CornerThreshold`.
        corner_threshold, set_corner_threshold, corner_threshold, i32, refresh);
    prop!(/// C# `UseCornerThreshold`.
        use_corner_threshold, set_use_corner_threshold, use_corner_threshold, bool, refresh);
    prop!(/// C# `Demo` (white point preview).
        demo, set_demo, demo, bool, refresh);

    /// C# `Interpolation` getter.
    pub fn interpolation(&self) -> InterpolationMode {
        self.opts.interpolation
    }

    /// C# `Interpolation` setter: also recomputes the preview image.
    ///
    /// `value`: new mode.
    pub fn set_interpolation(&mut self, value: InterpolationMode) {
        if value != self.opts.interpolation {
            self.opts.interpolation = value;
            self.resize_recalc();
            self.refresh();
        }
    }

    /// C# `Original`: the image fitted into the box (preview source).
    pub fn original(&self) -> &RgbaImage {
        &self.resized
    }

    /// C# `TrueOriginal`: the original after crop / rotate / invert (names swapped in C# too).
    #[allow(clippy::misnamed_getters)]
    pub fn true_original(&self) -> &RgbaImage {
        &self.original
    }

    /// C# `FileDPI`.
    pub fn file_dpi(&self) -> i32 {
        self.file_dpi
    }

    /// C# `mFileResolution` (pixel size of the file).
    pub fn file_resolution(&self) -> (u32, u32) {
        self.file_resolution
    }

    /// C# `WidthToHeight`: height for a width keeping the aspect ratio.
    ///
    /// `width`: width. Returns `width * H / W` (float math like C#).
    pub fn width_to_height(&self, width: f32) -> f32 {
        width * self.original.height() as f32 / self.original.width() as f32
    }

    /// C# `HeightToWidht` (sic): width for a height keeping the aspect ratio.
    ///
    /// `height`: height. Returns `height * W / H`.
    pub fn height_to_widht(&self, height: f32) -> f32 {
        height * self.original.width() as f32 / self.original.height() as f32
    }

    /// C# `AutoTrim`: cut homogeneous borders.
    pub fn auto_trim(&mut self) {
        let Some(bg) = self.guess_trim_color() else { return };
        let mut trim = [0i32; 4];
        for (i, t) in trim.iter_mut().enumerate() {
            *t = self.find_limit(bg, i as i32);
        }
        let (w, h) = (self.original.width() as i32, self.original.height() as i32);
        let scaled = Rect { x: trim[2], y: trim[0], width: w - trim[2] - trim[3], height: h - trim[0] - trim[1] };
        if scaled.width <= 0 || scaled.height <= 0 {
            return;
        }
        if let Some(n) = clone_rect(&self.original, scaled) {
            self.original = Arc::new(n);
            self.resize_recalc();
            self.refresh();
        }
    }

    /// C# `FindLimit`. `mode`: 0 top, 1 bottom, 2 left, 3 right.
    /// NOTE(upstream): the `check` colour of `GetLineColor` is ignored, any homogeneous line counts.
    fn find_limit(&self, bgcolor: Color, mode: i32) -> i32 {
        let limit = if mode == 0 || mode == 1 { self.original.height() } else { self.original.width() } as i32;
        let mut i = 0;
        while i < limit && self.get_line_color(mode, 1, i, Some(bgcolor)).is_some() {
            i += 1;
        }
        i
    }

    /// C# `GuessTrimColor`: common colour of the four borders, None when they differ.
    fn guess_trim_color(&self) -> Option<Color> {
        let colors: Vec<Option<Color>> = (0..4).map(|i| self.get_line_color(i, 1, 0, None)).collect();
        let mut rv: Option<Color> = None;
        for c in colors.into_iter().flatten() {
            match rv {
                None => rv = Some(c),
                Some(r) if is_similar_color(r, c, 20) => rv = Some(color_avg(r, c)),
                Some(_) => return None,
            }
        }
        rv
    }

    /// C# `GetLineColor`. `mode`: 0 top, 1 bottom, 2 left, 3 right. `step`: pixel step.
    /// `line`: distance from the border. `_check`: unused in C# too.
    /// Returns the average colour when the line is homogeneous, else None (`Color.Empty`).
    fn get_line_color(&self, mode: i32, step: i32, line: i32, _check: Option<Color>) -> Option<Color> {
        let img = &*self.original;
        let (w, h) = (img.width() as i32, img.height() as i32);
        let (limit, limit2) = if mode == 0 || mode == 1 { (w, h) } else { (h, w) };
        let mut primo: Option<Color> = None;
        let mut rv: Option<Color> = None;
        let mut i = 0;
        while i < limit {
            let (x, y) = match mode {
                0 => (i, line),
                1 => (i, limit2 - 1 - line),
                2 => (line, i),
                _ => (limit2 - 1 - line, i),
            };
            // out of range reads would throw in GDI+
            if x < 0 || y < 0 || x >= w || y >= h {
                return None;
            }
            let p = img.get_pixel(x as u32, y as u32);
            let pixel = Color { a: p[3], r: p[0], g: p[1], b: p[2] };
            let primo_c = *primo.get_or_insert(pixel);
            match rv {
                None => rv = Some(pixel),
                Some(r) if is_similar_color(pixel, primo_c, 20) => rv = Some(color_avg(r, pixel)),
                Some(_) => return None,
            }
            i += step;
        }
        rv
    }

    /// C# `CropImage`: crop to a rectangle given in preview coordinates.
    ///
    /// `rect`: selection in preview pixels. `rsize`: size of the preview it refers to.
    /// Returns true when the image was cropped.
    pub fn crop_image(&mut self, rect: Rect, rsize: (i32, i32)) -> bool {
        if rect.width <= 0 || rect.height <= 0 || rsize.0 == 0 || rsize.1 == 0 {
            return false;
        }
        let (ow, oh) = (self.original.width() as i32, self.original.height() as i32);
        let scaled = Rect {
            x: rect.x.wrapping_mul(ow) / rsize.0,
            y: rect.y.wrapping_mul(oh) / rsize.1,
            width: rect.width.wrapping_mul(ow) / rsize.0,
            height: rect.height.wrapping_mul(oh) / rsize.1,
        };
        if scaled.width <= 0 || scaled.height <= 0 {
            return false;
        }
        let Some(n) = clone_rect(&self.original, scaled) else { return false };
        self.original = Arc::new(n);
        self.resize_recalc();
        self.refresh();
        true
    }

    /// Preview point to original pixel (C# integer math).
    fn scale_point(&self, location: Point, rsize: (i32, i32)) -> Point {
        let (ow, oh) = (self.original.width() as i32, self.original.height() as i32);
        Point::new(location.x.wrapping_mul(ow) / rsize.0.max(1), location.y.wrapping_mul(oh) / rsize.1.max(1))
    }

    /// C# `Fill`: flood fill at a preview position.
    ///
    /// `location`: click in preview pixels. `rsize`: preview size. `color`: fill colour. `v`: tolerance.
    pub fn fill(&mut self, location: Point, rsize: (i32, i32), color: Color, v: i32) {
        let p = self.scale_point(location, rsize);
        if let Some(n) = lg_raster::fill(&self.original, p, color, v) {
            self.original = Arc::new(n);
        }
        self.resize_recalc();
        self.refresh();
    }

    /// C# `Outliner`: outline the region at a preview position.
    ///
    /// `location`: click in preview pixels. `rsize`: preview size.
    pub fn outliner(&mut self, location: Point, rsize: (i32, i32)) {
        let p = self.scale_point(location, rsize);
        if let Some(n) = lg_raster::outliner(&self.original, p) {
            self.original = Arc::new(n);
        }
        self.resize_recalc();
        self.refresh();
    }

    /// C# `Invert`.
    pub fn invert(&mut self) {
        self.original = Arc::new(lg_raster::inverting_image(&self.original));
        self.resize_recalc();
        self.refresh();
    }

    /// Shared body of the rotate / flip commands.
    fn rotate_flip_original(&mut self, t: RotateFlipType) {
        self.original = Arc::new(rotate_flip(&self.original, t));
        self.resize_recalc();
        self.refresh();
    }

    /// C# `RotateCW`.
    pub fn rotate_cw(&mut self) {
        self.rotate_flip_original(RotateFlipType::Rotate90FlipNone);
    }

    /// C# `RotateCCW`.
    pub fn rotate_ccw(&mut self) {
        self.rotate_flip_original(RotateFlipType::Rotate270FlipNone);
    }

    /// C# `FlipH` (`RotateNoneFlipY`).
    pub fn flip_h(&mut self) {
        self.rotate_flip_original(RotateFlipType::RotateNoneFlipY);
    }

    /// C# `FlipV` (`RotateNoneFlipX`).
    pub fn flip_v(&mut self) {
        self.rotate_flip_original(RotateFlipType::RotateNoneFlipX);
    }

    /// C# `Revert`: back to the file as loaded.
    pub fn revert(&mut self) {
        self.original = self.true_original.clone();
        self.resize_recalc();
        self.refresh();
    }

    /// C# `ResizeRecalc`: rebuild the preview-sized image.
    fn resize_recalc(&mut self) {
        let (w, h) = calculate_resize_to_fit((self.original.width() as i32, self.original.height() as i32), self.box_size);
        self.resized = resize_image(&self.original, w.max(1) as u32, h.max(1) as u32, false, self.opts.interpolation);
    }

    /// C# `Refresh`: abort the running preview and start a new one (no-op while suspended).
    fn refresh(&mut self) {
        if self.suspended {
            return;
        }
        if let Some(w) = self.current.take() {
            w.abort();
        }
        let must_exit = Arc::new(AtomicBool::new(false));
        let job = self.snapshot(must_exit.clone());
        // PreviewBegin is raised before the thread starts
        let _ = self.events.send(ImageProcessorEvent::PreviewBegin);
        let handle = std::thread::Builder::new().name("Image Processor".into()).spawn(move || job.create_preview()).ok();
        self.current = Some(Worker { must_exit, handle });
    }

    /// C# `GenerateGCode`: abort the preview and run `DoTrueWork` on a worker thread.
    /// Does nothing while suspended. Completion arrives as [`ImageProcessorEvent::GenerationComplete`].
    pub fn generate_gcode(&mut self) {
        if self.suspended {
            return;
        }
        if let Some(w) = self.current.take() {
            w.abort();
        }
        let must_exit = Arc::new(AtomicBool::new(false));
        let job = self.snapshot(must_exit.clone());
        let handle = std::thread::Builder::new().name("GCode Generator".into()).spawn(move || job.do_true_work()).ok();
        self.current = Some(Worker { must_exit, handle });
    }

    /// Wait for the running worker (tests / shutdown). Returns true when it finished in time.
    ///
    /// `timeout`: maximum wait.
    pub fn wait_worker(&mut self, timeout: Duration) -> bool {
        let end = Instant::now() + timeout;
        loop {
            match &self.current {
                Some(Worker { handle: Some(h), .. }) if !h.is_finished() => {}
                _ => return true,
            }
            if Instant::now() >= end {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// C# `Clone()`: copy of everything a worker needs.
    fn snapshot(&self, must_exit: Arc<AtomicBool>) -> job::Job {
        job::Job {
            opts: self.opts.clone(),
            gcode: self.gcode.clone(),
            original: self.original.clone(),
            resized: self.resized.clone(),
            gray_scale: self.gray_scale,
            file_dpi: self.file_dpi,
            file_name: self.file_name.clone(),
            append: self.append,
            file: self.file.clone(),
            events: self.events.clone(),
            must_exit,
        }
    }
}

impl Drop for ImageProcessor {
    fn drop(&mut self) {
        if let Some(w) = self.current.take() {
            w.must_exit.store(true, Ordering::SeqCst);
        }
    }
}

/// C# `TestGrayScale`: sampled every 10 px, colour spread below 20 means "grey".
///
/// `img`: image. Returns true for a (nearly) grey image.
pub fn test_gray_scale(img: &RgbaImage) -> bool {
    let mut maxdiff = 0i32;
    let (w, h) = (img.width(), img.height());
    for x in (0..w).step_by(10) {
        for y in (0..h).step_by(10) {
            let p = img.get_pixel(x, y);
            let (r, g, b) = (p[0] as i32, p[1] as i32, p[2] as i32);
            maxdiff = maxdiff.max((r - g).abs()).max((g - b).abs()).max((r - b).abs());
        }
    }
    maxdiff < 20
}

/// C# `CalculateResizeToFit`.
///
/// `image_size`: image (w, h). `box_size`: box (w, h). Returns the size that fits the box keeping
/// the aspect ratio (banker's rounding like `Math.Round`).
pub fn calculate_resize_to_fit(image_size: (i32, i32), box_size: (i32, i32)) -> (i32, i32) {
    let width_scale = box_size.0 as f64 / image_size.0 as f64;
    let height_scale = box_size.1 as f64 / image_size.1 as f64;
    let scale = width_scale.min(height_scale);
    (f64_to_i32(round_half_even(image_size.0 as f64 * scale)), f64_to_i32(round_half_even(image_size.1 as f64 * scale)))
}

/// C# `IsSimilarColor`.
fn is_similar_color(c1: Color, c2: Color, tolerance: i32) -> bool {
    (c1.a as i32 - c2.a as i32).abs() < tolerance
        && (c1.r as i32 - c2.r as i32).abs() < tolerance
        && (c1.g as i32 - c2.g as i32).abs() < tolerance
        && (c1.b as i32 - c2.b as i32).abs() < tolerance
}

/// C# `ColorAVG` (integer halves).
fn color_avg(c1: Color, c2: Color) -> Color {
    let h = |a: u8, b: u8| ((a as i32 + b as i32) / 2) as u8;
    Color { a: h(c1.a, c2.a), r: h(c1.r, c2.r), g: h(c1.g, c2.g), b: h(c1.b, c2.b) }
}

/// Convenience: path helper for UIs showing the file name.
///
/// `p`: path. Returns the file name text.
pub fn display_name(p: &str) -> String {
    Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

#[cfg(test)]
mod tests;
