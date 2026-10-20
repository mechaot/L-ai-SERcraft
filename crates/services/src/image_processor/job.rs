//! Worker side of `ImageProcessor`: `CreatePreview`, `DoTrueWork` and the bitmap pipeline.
//! A [`Job`] is the C# `Clone()` of the processor that a thread works on.

use super::overlay::{image_to_pixmap, pixmap_to_image, preview_line_by_line, white_veil};
use super::{Direction, GcodeOptions, ImageProcessorEvent, Options, Tool};
use crate::autotrace;
use crossbeam_channel::Sender;
use image::RgbaImage;
use lg_dotnet::{decimal_to_f64, f64_to_i32, Decimal};
use lg_gcode::{Firmware, GrblFile, L2LConf};
use lg_potrace::{export_to_gdiplus, potrace_trace, PotraceParams};
use lg_raster::{dither_image, gray_scale, resize_image, threshold, whitenize, Formula, InterpolationMode};
use resvg::tiny_skia::{Color as SkColor, Paint, Pixmap, Stroke, Transform};
use rust_decimal::prelude::ToPrimitive;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

/// Snapshot of the processor state used by one worker thread.
pub(crate) struct Job {
    pub opts: Options,
    pub gcode: GcodeOptions,
    pub original: Arc<RgbaImage>,
    pub resized: RgbaImage,
    pub gray_scale: bool,
    pub file_dpi: i32,
    pub file_name: String,
    pub append: bool,
    pub file: Arc<RwLock<GrblFile>>,
    pub events: Sender<ImageProcessorEvent>,
    pub must_exit: Arc<AtomicBool>,
}

/// C# `GetVectorQuality`: quality factor from the output size.
///
/// `size`: target area in mm squared. `adaptive`: adaptive quality enabled.
/// Returns 10 when not adaptive, else `255 * side^-0.5` clamped to 4..=255.
pub fn get_vector_quality(size: f64, adaptive: bool) -> f64 {
    if !adaptive {
        return 10.0;
    }
    let lato = size.sqrt();
    let fqual = 255.0 * lato.powf(-0.5);
    fqual.clamp(4.0, 255.0)
}

/// C# `TargetSize.Width * TargetSize.Height`: float product (rounded to f32), then widened.
///
/// `w`,`h`: size in mm. Returns the area as double.
pub(crate) fn target_area(w: f32, h: f32) -> f64 {
    (w * h) as f64
}

/// C# `(int)decimal` (truncate toward zero).
fn dec_to_i32(d: Decimal) -> i32 {
    d.trunc().to_i32().unwrap_or(0)
}

impl Job {
    /// C# `MustExitTH`.
    fn exit(&self) -> bool {
        self.must_exit.load(Ordering::Relaxed)
    }

    /// C# `CreatePreview` (thread body): build the preview and send it unless aborted.
    pub fn create_preview(self) {
        // C# catches every exception of the thread
        let r = catch_unwind(AssertUnwindSafe(|| self.create_preview_inner()));
        if let Err(e) = r {
            log::debug!("preview thread panicked: {e:?}");
        }
    }

    fn create_preview_inner(&self) {
        if self.opts.demo {
            let bmp = self.produce_whitepoint_demo();
            if !self.exit() {
                let _ = self.events.send(ImageProcessorEvent::PreviewReady(bmp));
            }
            return;
        }
        let size = (self.resized.width() as i32, self.resized.height() as i32);
        let mut bmp = self.produce_bitmap(&self.resized, size);
        if !self.exit() {
            match self.opts.tool {
                Tool::Line2Line | Tool::Dithering | Tool::NoProcessing => self.preview_line_by_line(&mut bmp),
                Tool::Vectorize => {
                    if let Some(b) = self.preview_vector(&bmp) {
                        bmp = b;
                    }
                }
                Tool::Centerline => self.preview_centerline(&mut bmp),
            }
        }
        if !self.exit() {
            let _ = self.events.send(ImageProcessorEvent::PreviewReady(bmp));
        }
    }

    /// C# `PreviewLineByLine(bmp)`.
    fn preview_line_by_line(&self, bmp: &mut RgbaImage) {
        let (w, h) = (bmp.width() as i32, bmp.height() as i32);
        preview_line_by_line(bmp.as_mut(), w, h, self.opts.tool, self.opts.line_preview, self.opts.direction, self.opts.filling_direction, &self.must_exit);
    }

    /// C# `PreviewVector`: trace, draw fill, line pattern and red borders on white.
    ///
    /// `bmp`: black/white bitmap. Returns the preview image, None when aborted / empty.
    fn preview_vector(&self, bmp: &RgbaImage) -> Option<RgbaImage> {
        let o = &self.opts;
        let params = PotraceParams {
            turdsize: if o.use_spot_removal { dec_to_i32(o.spot_removal) } else { 2 },
            alphamax: if o.use_smoothing { decimal_to_f64(o.smoothing) } else { 0.0 },
            opttolerance: if o.use_optimize { decimal_to_f64(o.optimize) } else { 0.2 },
            curveoptimizing: o.use_optimize,
            ..PotraceParams::default()
        };
        if self.exit() {
            return None;
        }
        let plist = potrace_trace(bmp, &params);
        if self.exit() {
            return None;
        }
        // g.Clear(White)
        let mut pm = Pixmap::new(bmp.width(), bmp.height())?;
        pm.fill(SkColor::WHITE);
        // trace filling
        let alpha = if o.filling_direction != Direction::None { 255 } else { 30 };
        export_to_gdiplus(&plist, &mut pm, Some(SkColor::from_rgba8(0, 0, 0, alpha)), None, 1.0);
        if self.exit() {
            return None;
        }
        // the pixmap stays opaque, so premultiplied == straight
        let (w, h) = (pm.width() as i32, pm.height() as i32);
        preview_line_by_line(pm.data_mut(), w, h, o.tool, o.line_preview, o.direction, o.filling_direction, &self.must_exit);
        if self.exit() {
            return None;
        }
        // trace borders
        export_to_gdiplus(&plist, &mut pm, None, Some((SkColor::from_rgba8(255, 0, 0, 255), 1.0)), 0.0);
        if self.exit() {
            return None;
        }
        Some(pixmap_to_image(&pm))
    }

    /// C# `PreviewCenterline`: autotrace, grey veil, red centre lines; error text on failure.
    fn preview_centerline(&self, bmp: &mut RgbaImage) {
        if self.exit() {
            return;
        }
        let o = &self.opts;
        let svg = autotrace::bitmap_to_svg_string(bmp, o.use_corner_threshold, o.corner_threshold, o.use_line_threshold, o.line_threshold).map_err(|e| e.to_string());
        if self.exit() {
            return;
        }
        let drawn = svg.and_then(|s| {
            let tree = parse_svg(&s)?;
            white_veil(bmp.as_mut());
            if self.exit() {
                return Ok(());
            }
            let mut pm = image_to_pixmap(bmp).ok_or("empty bitmap")?;
            stroke_tree_red(tree.root(), &mut pm);
            *bmp = pixmap_to_image(&pm);
            Ok(())
        });
        if let Err(msg) = drawn {
            if self.exit() {
                return;
            }
            white_veil(bmp.as_mut());
            if let Some(mut pm) = image_to_pixmap(bmp) {
                draw_error_text(&mut pm, &msg);
                *bmp = pixmap_to_image(&pm);
            }
        }
    }

    /// C# `ProduceWhitepointDemo`.
    fn produce_whitepoint_demo(&self) -> RgbaImage {
        let o = &self.opts;
        let resized = resize_image(&self.resized, self.resized.width(), self.resized.height(), false, o.interpolation);
        let gs = self.gray(&resized);
        whitenize(&gs, o.white_point, true)
    }

    /// `GrayScale(img, R/100, G/100, B/100, -((100-B)/100), C/100, IsGrayScale ? SimpleAverage : Formula)`.
    ///
    /// `img`: source. Returns the grey image.
    fn gray(&self, img: &RgbaImage) -> RgbaImage {
        let o = &self.opts;
        let formula = if self.gray_scale { Formula::SimpleAverage } else { o.formula };
        gray_scale(img, o.red as f32 / 100.0, o.green as f32 / 100.0, o.blue as f32 / 100.0, -((100 - o.brightness) as f32 / 100.0), o.contrast as f32 / 100.0, formula)
    }

    /// C# `ProduceBitmap(img, size)`: optional down sampling for vectorize, then the main chain.
    pub fn produce_bitmap(&self, img: &RgbaImage, size: (i32, i32)) -> RgbaImage {
        let o = &self.opts;
        if o.tool == Tool::Vectorize && o.use_down_sampling && o.down_sampling > Decimal::ONE {
            // size * 1 / DownSampling (decimal math, truncated)
            let w = dec_to_i32(Decimal::from(size.0) / o.down_sampling);
            let h = dec_to_i32(Decimal::from(size.1) / o.down_sampling);
            let down = resize_image(img, w.max(0) as u32, h.max(0) as u32, false, InterpolationMode::HighQualityBicubic);
            self.produce_bitmap2(&down, size)
        } else {
            self.produce_bitmap2(img, size)
        }
    }

    /// C# `ProduceBitmap2`: the exact transform chain per tool.
    fn produce_bitmap2(&self, img: &RgbaImage, size: (i32, i32)) -> RgbaImage {
        let o = &self.opts;
        if o.tool == Tool::NoProcessing {
            return gray_scale(img, 0.0, 0.0, 0.0, 0.0, 1.0, Formula::SimpleAverage);
        }
        let resized = resize_image(img, size.0.max(0) as u32, size.1.max(0) as u32, false, o.interpolation);
        let grayscale = self.gray(&resized);
        let whiten = whitenize(&grayscale, o.white_point, false);
        match o.tool {
            Tool::Dithering => dither_image(&whiten, o.dithering),
            // variable threshold (if enabled), then a fixed 50% threshold
            Tool::Centerline => threshold(&threshold(&whiten, o.threshold as f32 / 100.0, o.use_threshold), 50.0 / 100.0, true),
            _ => threshold(&whiten, o.threshold as f32 / 100.0, o.use_threshold),
        }
    }

    /// C# `DoTrueWork` (thread body): build the target bitmap and fill the shared program.
    pub fn do_true_work(self) {
        let result = catch_unwind(AssertUnwindSafe(|| self.do_true_work_inner()));
        let outcome = match result {
            Ok(r) => r,
            Err(_) => Err("image processing panicked".to_string()),
        };
        let _ = self.events.send(ImageProcessorEvent::GenerationComplete(outcome));
    }

    fn do_true_work_inner(&self) -> Result<(), String> {
        let o = &self.opts;
        let g = &self.gcode;
        // memory limit: bigger on 64 bit
        let max_size: i32 = if cfg!(target_pointer_width = "64") { 22000 * 22000 } else { 6000 * 7000 };
        let filesize = target_area(g.target_size.0, g.target_size.1);
        let max_res = (max_size as f64 / filesize).sqrt();
        let mut fres = max_res.min(decimal_to_f64(o.filling_quality));
        let mut res = match o.tool {
            Tool::Line2Line | Tool::Dithering => max_res.min(decimal_to_f64(o.quality)),
            Tool::Centerline => 10.0,
            _ => max_res.min(get_vector_quality(filesize, o.use_adaptive_quality)),
        };
        let mut pixel_size = (f64_to_i32(g.target_size.0 as f64 * res), f64_to_i32(g.target_size.1 as f64 * res));
        if o.tool == Tool::NoProcessing {
            pixel_size = (self.original.width() as i32, self.original.height() as i32);
            res = self.file_dpi as f64 / 25.4;
            fres = res;
        }
        if res.is_nan() || res <= 0.0 {
            return Err("Target size too big for processing!".into());
        }

        // CreateTarget
        let mut bmp = self.produce_bitmap(&self.original, pixel_size);
        let mut conf = L2LConf {
            res,
            fres,
            mark_speed: g.mark_speed,
            min_power: g.min_power,
            max_power: g.max_power,
            l_on: g.laser_on.clone(),
            l_off: g.laser_off.clone(),
            dir: match o.tool {
                Tool::NoProcessing => Direction::Horizontal,
                Tool::Vectorize => o.filling_direction,
                _ => o.direction,
            },
            o_x: g.target_offset.0,
            o_y: g.target_offset.1,
            border_speed: g.border_speed,
            pwm: lg_base::settings::get("Support Hardware PWM", true),
            firmware_type: lg_base::settings::get("Firmware Type", Firmware::Grbl),
            ..L2LConf::default()
        };
        let mut file = self.file.write().unwrap_or_else(|e| e.into_inner());
        let loaded = match o.tool {
            Tool::Line2Line | Tool::Dithering | Tool::NoProcessing => file.load_image_l2l(&mut bmp, &self.file_name, &conf, self.append),
            Tool::Vectorize => file.load_image_potrace(
                &mut bmp,
                &self.file_name,
                o.use_spot_removal,
                dec_to_i32(o.spot_removal),
                o.use_smoothing,
                o.smoothing,
                o.use_optimize,
                o.optimize,
                o.optimize_fast,
                &mut conf,
                self.append,
            ),
            Tool::Centerline => {
                let (uct, ct, ult, lt) = (o.use_corner_threshold, o.corner_threshold, o.use_line_threshold, o.line_threshold);
                let b = &bmp;
                file.load_image_centerline(b, &self.file_name, &|| autotrace::bitmap_to_svg_string(b, uct, ct, ult, lt).map_err(|e| e.to_string()), self.append)
            }
        };
        // a busy program is silently skipped, like C#
        loaded.map(|_| ()).map_err(|e| e.to_string())
    }
}

/// `Svg.SvgDocument.FromSvg`: parse autotrace output.
///
/// `text`: SVG document. Returns the tree or the parser message.
fn parse_svg(text: &str) -> Result<resvg::usvg::Tree, String> {
    resvg::usvg::Tree::from_str(text, &resvg::usvg::Options::default()).map_err(|e| e.to_string())
}

/// `svg.Draw(path); g.DrawPath(Pens.Red, path)`: stroke every path 1 px red.
///
/// `g`: svg group. `pm`: target.
fn stroke_tree_red(g: &resvg::usvg::Group, pm: &mut Pixmap) {
    for n in g.children() {
        match n {
            resvg::usvg::Node::Group(c) => stroke_tree_red(c, pm),
            resvg::usvg::Node::Path(p) => {
                let mut paint = Paint::default();
                paint.set_color_rgba8(255, 0, 0, 255);
                paint.anti_alias = true;
                pm.stroke_path(p.data(), &paint, &Stroke { width: 1.0, ..Stroke::default() }, p.abs_transform(), None);
            }
            _ => {}
        }
    }
}

/// `g.DrawString(ex.Message, DefaultFont, Brushes.Red, rect, centered)`: wrapped red text,
/// rendered with resvg and the system fonts (nothing is drawn when no font exists).
///
/// `pm`: target. `msg`: text.
fn draw_error_text(pm: &mut Pixmap, msg: &str) {
    let (w, h) = (pm.width() as f32, pm.height() as f32);
    let per_line = ((w / 6.5) as usize).max(8);
    // greedy word wrap
    let mut lines: Vec<String> = Vec::new();
    for raw in msg.lines() {
        let mut cur = String::new();
        for word in raw.split_whitespace() {
            if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > per_line {
                lines.push(std::mem::take(&mut cur));
            }
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(word);
        }
        lines.push(cur);
    }
    let lh = 13.0f32;
    let top = (h - lines.len() as f32 * lh) / 2.0 + lh * 0.8;
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let mut svg = format!("<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}'>");
    for (i, l) in lines.iter().enumerate() {
        svg.push_str(&format!("<text x='{}' y='{}' font-size='11' font-family='sans-serif' fill='red' text-anchor='middle'>{}</text>", w / 2.0, top + i as f32 * lh, esc(l)));
    }
    svg.push_str("</svg>");
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    if let Ok(tree) = resvg::usvg::Tree::from_str(&svg, &opt) {
        resvg::render(&tree, Transform::identity(), &mut pm.as_mut());
    }
}

#[cfg(test)]
mod area_tests {
    use super::target_area;

    #[test]
    fn area_is_f32_product() {
        // 0.1f32 * 0.3f32 differs from the f64 product of the widened operands
        let (w, h) = (0.1f32, 0.3f32);
        assert_ne!(target_area(w, h), w as f64 * h as f64);
        assert_eq!(target_area(w, h), (w * h) as f64);
    }
}
