//! Port of the GDI preview drawing of `GrblFile.cs` (`DrawOnGraphics`, `DrawJobPreview`,
//! `DrawJobRange`, `ScaleAndPosition`, `DrawString`).
//!
//! Instead of painting on a `Graphics`, the code returns a [`DrawList`]: an affine transform
//! (world mm to device pixels, y flipped) and abstract primitives in world coordinates. The CPU
//! renderer paints them with its own library. Text needs font metrics (`Graphics.MeasureString`):
//! the caller passes a measuring callback.

use crate::grbl_file::{CartesianQuadrant, GrblFile};
use crate::program_range::XYRange;
use crate::state_builder::StatePositionBuilder;
use crate::GResult;
use lg_base::tools::math_helper::RulerStepCalculator;
use lg_dotnet::{decimal_to_f32, f32_to_decimal, f32_to_i32, fmt_decimal_custom, fmt_float_custom, Decimal};

/// Semantic preview colors (C# `ColorScheme.Preview*`); the UI maps them to its scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewColor {
    FirstMovement,
    LaserPower,
    OtherMovement,
    Text,
    JobRange,
    Ruler,
}

/// Pen description. `width` is in world units (C# pen width 1 scaled by `1/zoom`, i.e. 1 device
/// pixel). `dash` lengths are in units of the pen width (GDI semantics).
#[derive(Debug, Clone, PartialEq)]
pub struct Pen {
    pub color: PreviewColor,
    /// Alpha 0..255.
    pub alpha: u8,
    pub width: f32,
    pub dash: Option<Vec<f32>>,
}

/// Affine transform `x' = a*x + c*y + e`, `y' = b*x + d*y + f` (GDI Matrix m11,m12,m21,m22,dx,dy).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Transform2D {
    /// Identity (after `ResetTransform`).
    pub fn identity() -> Self {
        Self { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 }
    }

    /// `ScaleTransform(sx, sy, MatrixOrder.Append)`.
    ///
    /// `sx`,`sy`: factors.
    fn scale_append(&mut self, sx: f32, sy: f32) {
        self.a *= sx;
        self.c *= sx;
        self.e *= sx;
        self.b *= sy;
        self.d *= sy;
        self.f *= sy;
    }

    /// `TranslateTransform(tx, ty, MatrixOrder.Append)`.
    ///
    /// `tx`,`ty`: shift.
    fn translate_append(&mut self, tx: f32, ty: f32) {
        self.e += tx;
        self.f += ty;
    }

    /// Map a point.
    ///
    /// `x`,`y`: world point. Returns device point.
    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }
}

/// One drawing primitive, in world coordinates (y up, mm).
#[derive(Debug, Clone, PartialEq)]
pub enum DrawPrim {
    /// `g.DrawLine`.
    Line { x1: f32, y1: f32, x2: f32, y2: f32, pen: Pen },
    /// `g.DrawArc` (angles in degrees, GDI convention: clockwise positive in device space).
    Arc { x: f32, y: f32, w: f32, h: f32, start_deg: f32, sweep_deg: f32, pen: Pen },
    /// `DrawString` through `DrawRotatedTextAt`: the text is drawn in the y-flipped world
    /// (`ScaleTransform(1,-1)` applied before), at `(x, y)` rotated by `rotation_deg`, with a
    /// monospace font of `em_size` world units (8 / zoom).
    Text { text: String, x: f32, y: f32, rotation_deg: f32, em_size: f32, color: PreviewColor },
}

/// Result of [`GrblFile::draw_on_graphics`].
#[derive(Debug, Clone, PartialEq)]
pub struct DrawList {
    pub transform: Transform2D,
    pub zoom: f32,
    pub prims: Vec<DrawPrim>,
}

/// Text metrics callback: `(text, em_size_world) -> (width, height)` of `Graphics.MeasureString`
/// in the same units as `em_size`.
pub type MeasureString<'a> = &'a dyn Fn(&str, f32) -> (f32, f32);

impl GrblFile {
    /// C# `DrawOnGraphics(g, size)`.
    ///
    /// `size`: target size in pixels `(width, height)`. `measure`: text measuring callback.
    /// Returns None when the moving range is not valid (C# draws nothing).
    pub fn draw_on_graphics(&self, size: (i32, i32), measure: MeasureString) -> GResult<Option<DrawList>> {
        if !self.range.moving_range.valid_range() {
            return Ok(None);
        }
        let mut spb = StatePositionBuilder::new();
        let scale_range = self.range.moving_range;
        let (w, h) = (scale_range.width(), scale_range.height());
        let zero = Decimal::ZERO;
        let zoom = match (w, h) {
            (Some(w), Some(h)) if w > zero && h > zero => {
                let a = size.0 as f32 / decimal_to_f32(w);
                let b = size.1 as f32 / decimal_to_f32(h);
                dn_min_f32(a, b) * 0.95f32
            }
            _ => 1.0f32,
        };

        let transform = self.scale_and_position(size, &scale_range, zoom);
        let mut prims = Vec::new();
        self.draw_job_preview(&mut prims, &mut spb, zoom)?;
        self.draw_job_range(&mut prims, size, zoom, measure);
        Ok(Some(DrawList { transform, zoom, prims }))
    }

    /// C# `DrawJobPreview`: the program as lines and arcs.
    fn draw_job_preview(&self, prims: &mut Vec<DrawPrim>, spb: &mut StatePositionBuilder, zoom: f32) -> GResult<()> {
        let mut firstline = true;
        for orig in &self.list {
            // work on a copy: the C# build/delete dance leaves the command unchanged anyway
            let mut cmd = orig.clone();
            cmd.build_helper();
            spb.analyze_command(&mut cmd, false, None)?;

            if spb.true_movement() {
                let color = if firstline {
                    PreviewColor::FirstMovement
                } else if spb.laser_burning() {
                    PreviewColor::LaserPower
                } else {
                    PreviewColor::OtherMovement
                };
                let alpha = spb.get_current_alpha(&self.range.spindle_range)?;
                let alpha = alpha.clamp(0, 255) as u8; // Color.FromArgb throws outside 0..255
                let pen = Pen {
                    color,
                    alpha,
                    width: 1.0 / zoom,
                    dash: if !spb.laser_burning() { Some(vec![1.0, 1.0]) } else { None },
                };

                if spb.g0g1() && cmd.is_linear_movement() && pen.alpha > 0 {
                    prims.push(DrawPrim::Line {
                        x1: decimal_to_f32(spb.x().previous()),
                        y1: decimal_to_f32(spb.y().previous()),
                        x2: decimal_to_f32(spb.x().number()),
                        y2: decimal_to_f32(spb.y().number()),
                        pen,
                    });
                } else if spb.g2g3() && cmd.is_arc_movement() && pen.alpha > 0 {
                    let ah = spb.get_arc_helper(&cmd)?;
                    if ah.rect_w > 0.0 && ah.rect_h > 0.0 {
                        prims.push(DrawPrim::Arc {
                            x: ah.rect_x as f32,
                            y: ah.rect_y as f32,
                            w: ah.rect_w as f32,
                            h: ah.rect_h as f32,
                            start_deg: (ah.start_angle * 180.0 / std::f64::consts::PI) as f32,
                            sweep_deg: (ah.angular_width * 180.0 / std::f64::consts::PI) as f32,
                            pen,
                        });
                    }
                }
                firstline = false;
            }
        }
        Ok(())
    }

    /// C# `ScaleAndPosition`: world to device transform for the quadrant.
    fn scale_and_position(&self, s: (i32, i32), scale_range: &XYRange, zoom: f32) -> Transform2D {
        let mut t = Transform2D::identity();
        let margin = 10.0f32;
        let (sw, sh) = (s.0 as f32, s.1 as f32);
        match self.quadrant() {
            CartesianQuadrant::Unknown | CartesianQuadrant::I => {
                t.scale_append(zoom, -zoom);
                t.translate_append(margin, sh - margin);
            }
            CartesianQuadrant::II => {
                t.scale_append(zoom, -zoom);
                t.translate_append(sw - margin, sh - margin);
            }
            CartesianQuadrant::III => {
                t.scale_append(zoom, -zoom);
                t.translate_append(sw - margin, margin);
            }
            CartesianQuadrant::IV => {
                t.scale_append(zoom, -zoom);
                t.translate_append(margin, margin);
            }
            CartesianQuadrant::Mix => {
                // center of gravity of the drawing to the center of the area
                let (cx, cy) = scale_range.center();
                t.translate_append(-cx, -cy);
                t.scale_append(zoom, -zoom);
                // C# int division: s.Width / 2
                t.translate_append((s.0 / 2) as f32, (s.1 / 2) as f32);
            }
        }
        t
    }

    /// C# `DrawJobRange`: axes, job bounds and rulers.
    fn draw_job_range(&self, prims: &mut Vec<DrawPrim>, s: (i32, i32), zoom: f32, measure: MeasureString) {
        let w_size = (s.0 as f32 / zoom, s.1 as f32 / zoom);
        let pw = 1.0 / zoom;
        let pen = |color: PreviewColor, dash: Option<Vec<f32>>| Pen { color, alpha: 255, width: pw, dash };
        let dr = &self.range.drawing_range;

        // cartesian plane
        let p = pen(PreviewColor::Text, None);
        prims.push(DrawPrim::Line { x1: -w_size.0, y1: 0.0, x2: w_size.0, y2: 0.0, pen: p.clone() });
        prims.push(DrawPrim::Line { x1: 0.0, y1: -w_size.1, x2: 0.0, y2: w_size.1, pen: p });

        // job range
        if dr.valid_range() {
            let p = pen(PreviewColor::JobRange, Some(vec![1.0 / zoom, 2.0 / zoom]));
            let (xmin, xmax) = (decimal_to_f32(dr.x.min), decimal_to_f32(dr.x.max));
            let (ymin, ymax) = (decimal_to_f32(dr.y.min), decimal_to_f32(dr.y.max));
            prims.push(DrawPrim::Line { x1: -w_size.0, y1: ymin, x2: w_size.0, y2: ymin, pen: p.clone() });
            prims.push(DrawPrim::Line { x1: -w_size.0, y1: ymax, x2: w_size.0, y2: ymax, pen: p.clone() });
            prims.push(DrawPrim::Line { x1: xmin, y1: -w_size.1, x2: xmin, y2: w_size.1, pen: p.clone() });
            prims.push(DrawPrim::Line { x1: xmax, y1: -w_size.1, x2: xmax, y2: w_size.1, pen: p });

            let q = self.quadrant();
            let right = q == CartesianQuadrant::I || q == CartesianQuadrant::IV;
            let top = q == CartesianQuadrant::I || q == CartesianQuadrant::II;
            let small = dr.width().is_some_and(|w| w < Decimal::from(50)) && dr.height().is_some_and(|h| h < Decimal::from(50));
            let format = if small { "0.0" } else { "0" };
            let dec = |d: Decimal| fmt_decimal_custom(d, format);
            let txt = |prims: &mut Vec<DrawPrim>, x: Decimal, y: Decimal, t: String, cx, cy, sx, sy| {
                draw_string(prims, measure, zoom, x, y, &t, cx, cy, sx, sy, PreviewColor::Text, 0.0);
            };
            txt(prims, Decimal::ZERO, dr.y.min, dec(dr.y.min), false, true, !right, false);
            txt(prims, Decimal::ZERO, dr.y.max, dec(dr.y.max), false, true, !right, false);
            txt(prims, dr.x.min, Decimal::ZERO, dec(dr.x.min), true, false, false, top);
            txt(prims, dr.x.max, Decimal::ZERO, dec(dr.x.max), true, false, false, top);
        }

        // ruler
        let p = pen(PreviewColor::Ruler, None);
        let q = self.quadrant();
        let right = q == CartesianQuadrant::Unknown || q == CartesianQuadrant::I || q == CartesianQuadrant::IV;
        let top = q == CartesianQuadrant::Unknown || q == CartesianQuadrant::I || q == CartesianQuadrant::II;
        let format = if dr.valid_range() && dr.width().is_some_and(|w| w < Decimal::from(50)) && dr.height().is_some_and(|h| h < Decimal::from(50)) { "0.0" } else { "0" };

        // horizontal scale
        let hscale = RulerStepCalculator::new(-(w_size.0 as f64), w_size.0 as f64, f32_to_i32(2.0 * s.0 as f32 / 100.0));
        let h1 = (if top { -4.0 } else { 4.0 }) / zoom as f64;
        let h2 = 1.8 * h1;
        let h3 = (if top { 1.0 } else { -1.0 }) / zoom as f64;
        let mut d = hscale.first_small() as f32;
        while d < w_size.0 {
            prims.push(DrawPrim::Line { x1: d, y1: 0.0, x2: d, y2: h1 as f32, pen: p.clone() });
            d += hscale.small_step() as f32;
        }
        let mut d = hscale.first_big() as f32;
        while d < w_size.0 {
            prims.push(DrawPrim::Line { x1: d, y1: 0.0, x2: d, y2: h2 as f32, pen: p.clone() });
            d += hscale.big_step() as f32;
        }
        let mut d = hscale.first_big() as f32;
        while d < w_size.0 {
            draw_string(prims, measure, zoom, f32_to_decimal(d), lg_dotnet::f64_to_decimal(h3), &fmt_float_custom(d, format), false, false, !right, !top, PreviewColor::Text, 0.0);
            d += hscale.big_step() as f32;
        }

        // vertical scale
        let vscale = RulerStepCalculator::new(-(w_size.1 as f64), w_size.1 as f64, f32_to_i32(2.0 * s.1 as f32 / 100.0));
        let v1 = (if right { -4.0 } else { 4.0 }) / zoom as f64;
        let v2 = 1.8 * v1;
        let v3 = (if right { 2.5 } else { 0.0 }) / zoom as f64;
        let mut d = vscale.first_small() as f32;
        while d < w_size.1 {
            prims.push(DrawPrim::Line { x1: 0.0, y1: d, x2: v1 as f32, y2: d, pen: p.clone() });
            d += vscale.small_step() as f32;
        }
        let mut d = vscale.first_big() as f32;
        while d < w_size.1 {
            prims.push(DrawPrim::Line { x1: 0.0, y1: d, x2: v2 as f32, y2: d, pen: p.clone() });
            d += vscale.big_step() as f32;
        }
        let mut d = vscale.first_big() as f32;
        while d < w_size.1 {
            draw_string(prims, measure, zoom, lg_dotnet::f64_to_decimal(v3), f32_to_decimal(d), &fmt_float_custom(d, format), false, false, right, !top, PreviewColor::Text, -90.0);
            d += vscale.big_step() as f32;
        }
    }
}

/// `Math.Min(float, float)` with NaN propagation.
fn dn_min_f32(a: f32, b: f32) -> f32 {
    if a < b || a.is_nan() { a } else { b }
}

/// C# `DrawString` + `DrawRotatedTextAt`.
///
/// `prims`: sink. `measure`: text metrics. `zoom`: zoom (font = 8/zoom). `cur_x`,`cur_y`: anchor
/// in world units. `text`: text. `center_x`/`center_y`: center the text on the anchor.
/// `subtract_x`/`subtract_y`: shift by the text size. `color`: color. `rotation`: degrees.
#[allow(clippy::too_many_arguments)]
fn draw_string(prims: &mut Vec<DrawPrim>, measure: MeasureString, zoom: f32, cur_x: Decimal, cur_y: Decimal, text: &str, center_x: bool, center_y: bool, subtract_x: bool, subtract_y: bool, color: PreviewColor, rotation: f32) {
    let em = 8.0f32 * 1.0 / zoom;
    let (mw, mh) = measure(text, em);
    let mut offset_x = 0.0f32;
    let mut offset_y = 0.0f32;
    if center_x {
        offset_x = mw / 2.0;
    }
    if center_y {
        offset_y = mh / 2.0;
    }
    if subtract_x {
        offset_x += if rotation == 0.0 { mw } else { mh };
    }
    if subtract_y {
        offset_y += if rotation == 0.0 { mh } else { -mw };
    }
    prims.push(DrawPrim::Text {
        text: text.to_string(),
        x: decimal_to_f32(cur_x) - offset_x,
        y: -decimal_to_f32(cur_y) - offset_y,
        rotation_deg: rotation,
        em_size: em,
        color,
    });
}
