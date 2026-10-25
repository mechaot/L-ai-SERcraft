// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of the vector loaders of `GrblFile.cs`: `LoadImagePotrace`, `LoadImportedSVG`,
//! `LoadImageCenterline`.

use crate::command::GrblCommand;
use crate::config::{configuration, Firmware};
use crate::grbl_file::GrblFile;
use crate::l2l::{raster_filling, vector_filling, L2LConf};
use crate::optimize::{optimize_paths, parallel_optimize_paths};
use crate::{GResult, GcodeError};
use image::RgbaImage;
use lg_dotnet::{decimal_to_f64, f64_to_i32, fmt_decimal, Decimal};
use lg_potrace::{export_to_gcode, export_to_gdiplus, potrace_trace, FillConf, PotraceParams};
use lg_raster::{resize_image, rotate_flip, InterpolationMode, RotateFlipType};
use lg_svg2gcode::{ColorFilter, GCodeFromSvg, PointF, SvgConvOptions};
pub use lg_svg2gcode::{LayerInfo, LayerParams, list_layers_from_file};
use std::ops::Range;
use std::sync::OnceLock;

/// C# `gcodeRelated.firmwareType`: static initialiser, read once per process.
fn svg_firmware() -> lg_svg2gcode::Firmware {
    static FW: OnceLock<Firmware> = OnceLock::new();
    match *FW.get_or_init(|| lg_base::settings::get("Firmware Type", Firmware::Grbl)) {
        Firmware::Grbl => lg_svg2gcode::Firmware::Grbl,
        Firmware::Smoothie => lg_svg2gcode::Firmware::Smoothie,
        Firmware::Marlin => lg_svg2gcode::Firmware::Marlin,
        Firmware::VigoWork => lg_svg2gcode::Firmware::VigoWork,
    }
}

/// Snapshot of the settings `gcodeRelated.setup` reads.
pub fn svg_conv_options() -> SvgConvOptions {
    use lg_base::settings::get;
    SvgConvOptions {
        support_hardware_pwm: get("Support Hardware PWM", true),
        disable_g0_fast_skip: get("Disable G0 fast skip", false),
        border_speed: get("GrayScaleConversion.VectorizeOptions.BorderSpeed", 1000),
        power_max: get("GrayScaleConversion.Gcode.LaserOptions.PowerMax", 255),
        laser_on: get("GrayScaleConversion.Gcode.LaserOptions.LaserOn", "M3".to_string()),
        laser_off: get("GrayScaleConversion.Gcode.LaserOptions.LaserOff", "M5".to_string()),
        max_pwm: configuration::configuration().max_pwm(),
        firmware: svg_firmware(),
    }
}

/// `Bitmap` -> straight RGBA from a premultiplied tiny-skia pixmap.
///
/// `p`: pixmap. Returns image.
fn pixmap_to_image(p: &resvg::tiny_skia::Pixmap) -> RgbaImage {
    let mut img = RgbaImage::new(p.width(), p.height());
    for (i, px) in p.pixels().iter().enumerate() {
        let c = px.demultiply();
        let (x, y) = (i as u32 % p.width(), i as u32 / p.width());
        img.put_pixel(x, y, image::Rgba([c.red(), c.green(), c.blue(), c.alpha()]));
    }
    img
}

impl GrblFile {
    /// C# `LoadImageCenterline`.
    ///
    /// `bmp`: source (only its size is used here). `filename`: name for events.
    /// `autotrace`: runs autotrace on the bitmap (C# `Autotrace.BitmapToSvgString`); on `Err` the
    /// error is logged ("Centerline") and an empty SVG text is converted (which fails in the XML
    /// parser exactly like C#). `append`: keep old commands.
    /// Returns Ok(false) when busy.
    pub fn load_image_centerline(&mut self, bmp: &RgbaImage, filename: &str, autotrace: &dyn Fn() -> Result<String, String>, append: bool) -> GResult<bool> {
        if self.check_in_use() {
            return Ok(false);
        }
        self.rise_on_file_loading(filename);
        let start = lg_base::tools::hires_timer::total_milliseconds();
        if !append {
            self.clear_list();
        }
        self.range.reset_range();

        let content = match autotrace() {
            Ok(s) => s,
            Err(e) => {
                lg_base::logger::log_exception("Centerline", &e);
                String::new()
            }
        };

        use lg_base::settings::get;
        let mut converter = GCodeFromSvg::new();
        converter.gcode_xy_feed = get("GrayScaleConversion.VectorizeOptions.BorderSpeed", 1000i32) as f32;
        converter.svg_scale_apply = true;
        converter.svg_max_size = (bmp.width() as f64 / 10.0).max(bmp.height() as f64 / 10.0) as f32;
        converter.user_offset = PointF { x: get("GrayScaleConversion.Gcode.Offset.X", 0f32), y: get("GrayScaleConversion.Gcode.Offset.Y", 0f32) };
        converter.use_legacy_bezier = !get("Vector.UseSmartBezier", true);

        let gcode = converter
            .convert_from_text(&content, &svg_conv_options(), false, ColorFilter::All)
            .map_err(|e| GcodeError::Other(e.to_string()))?;
        self.add_text_lines(&gcode);

        self.analyze()?;
        let elapsed = lg_base::tools::hires_timer::total_milliseconds() - start;
        self.rise_on_file_loaded(filename, elapsed);
        Ok(true)
    }

    /// C# `LoadImportedSVG` (synchronous body, see [`GrblFile::spawn_load`]).
    ///
    /// `filename`: SVG file. `append`: keep old commands. `filter`: colour filter.
    pub fn load_imported_svg(&mut self, filename: &str, append: bool, filter: ColorFilter) -> GResult<()> {
        self.rise_on_file_loading(filename);
        let start = lg_base::tools::hires_timer::total_milliseconds();
        if !append {
            self.clear_list();
        }
        self.range.reset_range();

        use lg_base::settings::get;
        let mut converter = GCodeFromSvg::new();
        converter.gcode_xy_feed = get("GrayScaleConversion.VectorizeOptions.BorderSpeed", 1000i32) as f32;
        converter.use_legacy_bezier = !get("Vector.UseSmartBezier", true);
        let gcode = converter
            .convert_from_file(std::path::Path::new(filename), &svg_conv_options(), filter)
            .map_err(|e| GcodeError::Other(e.to_string()))?;
        self.add_text_lines(&gcode);

        self.analyze()?;
        let elapsed = lg_base::tools::hires_timer::total_milliseconds() - start;
        self.rise_on_file_loaded(filename, elapsed);
        Ok(())
    }

    /// Layered variant of [`Self::load_imported_svg`]: per-layer feed, power, passes, enable and filter.
    ///
    /// `filename`: SVG file. `append`: keep old commands. `layers`: params aligned with
    /// `list_layers_from_file` order (empty = plain conversion). Returns, per layer, the command index
    /// range it produced in the list (absolute, `None` = disabled layer).
    pub fn load_imported_svg_layered(&mut self, filename: &str, append: bool, layers: &[LayerParams]) -> GResult<Vec<Option<Range<u32>>>> {
        self.rise_on_file_loading(filename);
        let start = lg_base::tools::hires_timer::total_milliseconds();
        if !append {
            self.clear_list();
        }
        self.range.reset_range();

        use lg_base::settings::get;
        let mut converter = GCodeFromSvg::new();
        converter.gcode_xy_feed = get("GrayScaleConversion.VectorizeOptions.BorderSpeed", 1000i32) as f32;
        converter.use_legacy_bezier = !get("Vector.UseSmartBezier", true);
        let (gcode, byte_ranges) = converter
            .convert_from_file_layered(std::path::Path::new(filename), &svg_conv_options(), layers)
            .map_err(|e| GcodeError::Other(e.to_string()))?;

        // feed text slice by slice, so each layer gets an exact command range
        let mut out = Vec::with_capacity(byte_ranges.len());
        let mut cursor = 0usize;
        for br in &byte_ranges {
            match br {
                Some(r) if r.start >= cursor && r.end <= gcode.len() => {
                    self.add_text_lines(&gcode[cursor..r.start]);
                    let a = self.list.len() as u32;
                    self.add_text_lines(&gcode[r.clone()]);
                    out.push(Some(a..self.list.len() as u32));
                    cursor = r.end;
                }
                _ => out.push(None),
            }
        }
        self.add_text_lines(&gcode[cursor..]);

        self.analyze()?;
        let elapsed = lg_base::tools::hires_timer::total_milliseconds() - start;
        self.rise_on_file_loaded(filename, elapsed);
        Ok(out)
    }

    /// C# `LoadImagePotrace`. Flips `bmp` vertically in place; may set `c.vectorfilling`.
    ///
    /// `bmp`: black/white source. `filename`: events name. `use_spot_removal`/`spot_removal`:
    /// turd size (2 when off). `use_smoothing`/`smoothing`: alphamax (0 when off).
    /// `use_optimize`/`optimize`: curve optimisation and tolerance (0.2 when off).
    /// `use_optimize_fast`: reorder border paths. `c`: conf. `append`: keep old commands.
    /// Returns Ok(false) when busy.
    #[allow(clippy::too_many_arguments)]
    pub fn load_image_potrace(
        &mut self,
        bmp: &mut RgbaImage,
        filename: &str,
        use_spot_removal: bool,
        spot_removal: i32,
        use_smoothing: bool,
        smoothing: Decimal,
        use_optimize: bool,
        optimize: Decimal,
        use_optimize_fast: bool,
        c: &mut L2LConf,
        append: bool,
    ) -> GResult<bool> {
        if self.check_in_use() {
            return Ok(false);
        }
        self.skipcmd = if lg_base::settings::get("Disable G0 fast skip", false) { "G1".into() } else { "G0".into() };
        self.rise_on_file_loading(filename);
        *bmp = rotate_flip(bmp, RotateFlipType::RotateNoneFlipY);
        let start = lg_base::tools::hires_timer::total_milliseconds();
        if !append {
            self.clear_list();
        }
        self.range.reset_range();

        let params = PotraceParams {
            turdsize: if use_spot_removal { spot_removal } else { 2 },
            alphamax: if use_smoothing { decimal_to_f64(smoothing) } else { 0.0 },
            opttolerance: if use_optimize { decimal_to_f64(optimize) } else { 0.2 },
            curveoptimizing: use_optimize,
            ..PotraceParams::default()
        };
        let mut plist = potrace_trace(bmp, &params);
        let mut flist: Option<Vec<Vec<lg_potrace::Curve>>> = None;
        let (bw, bh) = (bmp.width(), bmp.height());

        if vector_filling(c.dir) {
            let fc = FillConf { dir: c.dir, res: c.res, fres: c.fres };
            let f = lg_potrace::build_filling(&plist, bw as f64, bh as f64, &fc).map_err(|e| GcodeError::Other(format!("{e:?}")))?;
            flist = parallel_optimize_paths(f, 0.0)?;
        }
        if raster_filling(c.dir) {
            let mut ptb = resvg::tiny_skia::Pixmap::new(bw, bh).ok_or_else(|| GcodeError::Argument("empty bitmap".into()))?;
            // small border to end a bit before the edge
            let ins = c.res / c.fres;
            let inset = if ins.is_nan() || ins > 1.0 { ins } else { 1.0 };
            export_to_gdiplus(&plist, &mut ptb, Some(resvg::tiny_skia::Color::BLACK), None, inset);
            let ptb_img = pixmap_to_image(&ptb);
            let rw = f64_to_i32(bw as f64 * c.fres / c.res) + 1;
            let rh = f64_to_i32(bh as f64 * c.fres / c.res) + 1;
            let resampled = resize_image(&ptb_img, rw as u32, rh as u32, true, InterpolationMode::HighQualityBicubic);

            if c.pwm {
                self.list.push(GrblCommand::new(&format!("{} S0", c.l_on)));
            } else {
                self.list.push(GrblCommand::new(&format!("{} S{}", c.l_off, fmt_decimal(configuration::configuration().max_pwm()))));
            }
            // set speed to markspeed
            self.list.push(GrblCommand::new(&format!("F{}", c.mark_speed)));
            c.vectorfilling = true;
            self.image_line2line(&resampled, c);
            // laser off
            self.list.push(GrblCommand::new(&c.l_off));
        }

        let support_pwm: bool = lg_base::settings::get("Support Hardware PWM", true);
        if support_pwm {
            self.list.push(GrblCommand::new(&format!("{} S0", c.l_on)));
        } else {
            self.list.push(GrblCommand::new(&format!("{} S{}", c.l_off, fmt_decimal(configuration::configuration().max_pwm()))));
        }

        // trace raster filling
        if let Some(fl) = &flist {
            let gc = if support_pwm {
                export_to_gcode(fl, c.o_x, c.o_y, c.res, &format!("S{}", c.max_power), "S0", (bw, bh), &self.skipcmd)
            } else {
                export_to_gcode(fl, c.o_x, c.o_y, c.res, &c.l_on, &c.l_off, (bw, bh), &self.skipcmd)
            };
            self.list.push(GrblCommand::new(&format!("F{}", c.mark_speed)));
            self.list.extend(gc.iter().map(|s| GrblCommand::new(s)));
        }

        // trace borders
        if use_optimize_fast {
            plist = optimize_paths(plist, 0.0)?;
        } else {
            // potrace gives outer first then inner paths; cutting is better the other way round
            plist.reverse();
        }
        let gc = if support_pwm {
            export_to_gcode(&plist, c.o_x, c.o_y, c.res, &format!("S{}", c.max_power), "S0", (bw, bh), &self.skipcmd)
        } else {
            export_to_gcode(&plist, c.o_x, c.o_y, c.res, &c.l_on, &c.l_off, (bw, bh), &self.skipcmd)
        };
        self.list.push(GrblCommand::new(&format!("F{}", c.border_speed)));
        self.list.extend(gc.iter().map(|s| GrblCommand::new(s)));

        // laser off (needed: the trace ends with S0 only)
        if support_pwm {
            self.list.push(GrblCommand::new(&c.l_off));
        }

        self.analyze()?;
        let elapsed = lg_base::tools::hires_timer::total_milliseconds() - start;
        self.rise_on_file_loaded(filename, elapsed);
        Ok(true)
    }
}
