//! Port of `UserControls/GrblPanel.cs`: CPU (GDI fallback) preview.
//!
//! `GrblFile::draw_on_graphics` gives a [`DrawList`]; lines and arcs are rasterised with
//! `resvg::tiny_skia` on a worker thread into an egui texture (re-rendered on resize, file change or color
//! change, like the C# background bitmap thread). Text primitives and the cursor / position overlays are
//! painted with egui on top of the texture.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, RwLock};

use eframe::egui::{self, Align, Align2, Color32, ColorImage, PointerButton, Pos2, Rect, Sense, TextureHandle, TextureOptions, Ui, Vec2};
use lg_dotnet::fmt_float_custom;
use lg_gcode::{CartesianQuadrant, DrawList, DrawPrim, GrblFile, PreviewColor, Transform2D};
use resvg::tiny_skia::{self, FillRule, LineCap, Paint, PathBuilder, Pixmap, Stroke, StrokeDash};

use super::{LayerView, PreviewEvent, PreviewInput};
use crate::ui::Theme;

/// Preview palette for the CPU job (C# `ColorScheme.Preview*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuColors {
    pub first_movement: [u8; 3],
    pub laser_power: [u8; 3],
    pub other_movement: [u8; 3],
    pub text: [u8; 3],
    pub job_range: [u8; 3],
    pub ruler: [u8; 3],
}

impl CpuColors {
    /// Palette of a theme.
    ///
    /// `t`: theme. Returns colors.
    pub fn of(t: &Theme) -> Self {
        use super::rgb;
        Self {
            first_movement: rgb(t.preview_first_movement),
            laser_power: rgb(t.preview_laser_power),
            other_movement: rgb(t.preview_other_movement),
            text: rgb(t.preview_text),
            job_range: rgb(t.preview_job_range),
            ruler: rgb(t.preview_ruler),
        }
    }

    /// Map a semantic color.
    ///
    /// `c`: semantic color. Returns RGB.
    fn map(&self, c: PreviewColor) -> [u8; 3] {
        match c {
            PreviewColor::FirstMovement => self.first_movement,
            PreviewColor::LaserPower => self.laser_power,
            PreviewColor::OtherMovement => self.other_movement,
            PreviewColor::Text => self.text,
            PreviewColor::JobRange => self.job_range,
            PreviewColor::Ruler => self.ruler,
        }
    }
}

/// A text primitive left for egui to draw.
#[derive(Debug, Clone)]
pub struct TextPrim {
    pub text: String,
    /// Device px of the text top-left.
    pub x: f32,
    pub y: f32,
    pub rotation_deg: f32,
    /// Font size in device px.
    pub em_px: f32,
    pub color: [u8; 3],
}

/// Result of one render.
pub struct Rendered {
    pub size: (i32, i32),
    pub transform: Option<Transform2D>,
    pub image: ColorImage,
    pub texts: Vec<TextPrim>,
}

/// `Graphics.MeasureString` stand-in for a monospace font: 0.6 em wide, 1.2 em high.
///
/// `text`: string. `em`: font size. Returns `(w, h)`.
fn measure_mono(text: &str, em: f32) -> (f32, f32) {
    (text.chars().count() as f32 * 0.6 * em, 1.2 * em)
}

/// Rasterise a [`DrawList`] (C# `DrawOnGraphics` onto a bitmap).
///
/// `dl`: draw list. `size`: bitmap size in px. `colors`: palette. `view`: layer overlay (hidden prims are
/// skipped, highlighted laser-on prims get a second, wide red stroke). Returns pixmap plus text prims.
pub fn rasterize(dl: &DrawList, size: (i32, i32), colors: &CpuColors, view: &LayerView) -> Option<(Pixmap, Vec<TextPrim>)> {
    let mut pm = Pixmap::new(size.0.max(1) as u32, size.1.max(1) as u32)?;
    let t = dl.transform;
    let ts = tiny_skia::Transform::from_row(t.a, t.b, t.c, t.d, t.e, t.f);
    let mut texts = Vec::new();
    // group consecutive lines of one pen into one path (fast for big jobs)
    let mut run: Option<(PenKey, PathBuilder, usize)> = None;
    let flush = |pm: &mut Pixmap, run: &mut Option<(PenKey, PathBuilder, usize)>| {
        if let Some((k, pb, _)) = run.take() {
            if let Some(path) = pb.finish() {
                stroke(pm, &path, &k, colors, ts);
            }
        }
    };
    // red overlay path and its base pen width
    let mut hl = PathBuilder::new();
    let mut hl_width: Option<f32> = None;
    for prim in &dl.prims {
        match prim {
            DrawPrim::Line { cmd, .. } | DrawPrim::Arc { cmd, .. } if view.is_hidden(*cmd) => {}
            DrawPrim::Line { x1, y1, x2, y2, pen, cmd } => {
                if view.is_highlighted(*cmd) && pen.dash.is_none() {
                    hl.move_to(*x1, *y1);
                    hl.line_to(*x2, *y2);
                    hl_width.get_or_insert(pen.width);
                }
                let k = PenKey::of(pen);
                let same = run.as_ref().is_some_and(|(rk, _, n)| *rk == k && *n < 4096);
                if !same {
                    flush(&mut pm, &mut run);
                    run = Some((k, PathBuilder::new(), 0));
                }
                if let Some((_, pb, n)) = run.as_mut() {
                    pb.move_to(*x1, *y1);
                    pb.line_to(*x2, *y2);
                    *n += 1;
                }
            }
            DrawPrim::Arc { x, y, w, h, start_deg, sweep_deg, pen, cmd } => {
                flush(&mut pm, &mut run);
                let mut pb = PathBuilder::new();
                arc_points(&mut pb, *x, *y, *w, *h, *start_deg, *sweep_deg);
                if view.is_highlighted(*cmd) && pen.dash.is_none() {
                    arc_points(&mut hl, *x, *y, *w, *h, *start_deg, *sweep_deg);
                    hl_width.get_or_insert(pen.width);
                }
                if let Some(path) = pb.finish() {
                    stroke(&mut pm, &path, &PenKey::of(pen), colors, ts);
                }
            }
            DrawPrim::Text { text, x, y, rotation_deg, em_size, color } => {
                // text lives in the y-flipped world: device = T(x, -y)
                let (px, py) = t.apply(*x, -*y);
                texts.push(TextPrim { text: text.clone(), x: px, y: py, rotation_deg: *rotation_deg, em_px: em_size * dl.zoom, color: colors.map(*color) });
            }
        }
    }
    flush(&mut pm, &mut run);
    // selected layer: wide red stroke on top
    if let (Some(path), Some(w)) = (hl.finish(), hl_width) {
        let mut paint = Paint::default();
        paint.set_color_rgba8(255, 0, 0, 255);
        paint.anti_alias = true;
        let st = Stroke { width: (w * 3.0).max(1e-6), line_cap: LineCap::Round, ..Default::default() };
        pm.stroke_path(&path, &paint, &st, ts, None);
    }
    Some((pm, texts))
}

/// Add an elliptical arc as a polyline sub-path.
///
/// `pb`: target path. `x`,`y`,`w`,`h`: bounding rect of the ellipse. `start_deg`,`sweep_deg`: GDI angles.
fn arc_points(pb: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32, start_deg: f32, sweep_deg: f32) {
    let n = ((sweep_deg.abs() / 2.0).ceil() as usize).clamp(2, 720);
    let (cx, cy, rx, ry) = (x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0);
    for i in 0..=n {
        let a = (start_deg + sweep_deg * i as f32 / n as f32).to_radians();
        let (px, py) = (cx + rx * a.cos(), cy + ry * a.sin());
        if i == 0 { pb.move_to(px, py) } else { pb.line_to(px, py) }
    }
}

/// Pen identity for run grouping.
#[derive(Debug, Clone, PartialEq)]
struct PenKey {
    color: PreviewColor,
    alpha: u8,
    width: f32,
    dash: Option<Vec<f32>>,
}

impl PenKey {
    /// From a draw list pen.
    ///
    /// `p`: pen. Returns key.
    fn of(p: &lg_gcode::Pen) -> Self {
        Self { color: p.color, alpha: p.alpha, width: p.width, dash: p.dash.clone() }
    }
}

/// Stroke a path with a pen.
///
/// `pm`: target. `path`: path in world coordinates. `k`: pen. `colors`: palette. `ts`: world to device.
fn stroke(pm: &mut Pixmap, path: &tiny_skia::Path, k: &PenKey, colors: &CpuColors, ts: tiny_skia::Transform) {
    let c = colors.map(k.color);
    let mut paint = Paint::default();
    paint.set_color_rgba8(c[0], c[1], c[2], k.alpha);
    paint.anti_alias = true;
    let mut st = Stroke { width: k.width.max(1e-6), line_cap: LineCap::Butt, ..Default::default() };
    if let Some(d) = &k.dash {
        // GDI: pattern lengths are multiples of the pen width
        let v: Vec<f32> = d.iter().map(|x| (x * k.width).max(1e-6)).collect();
        st.dash = StrokeDash::new(v, 0.0);
    }
    let _ = FillRule::Winding;
    pm.stroke_path(path, &paint, &st, ts, None);
}

/// Render the whole preview bitmap (C# `DoTheWork`).
///
/// `file`: program. `size`: bitmap px. `colors`: palette. `view`: layer overlay. Returns the bitmap + overlay data.
pub fn render(file: &GrblFile, size: (i32, i32), colors: &CpuColors, view: &LayerView) -> Option<Rendered> {
    if size.0 < 1 || size.1 < 1 {
        return None;
    }
    let dl = match file.draw_on_graphics(size, &measure_mono) {
        Ok(Some(dl)) => dl,
        Ok(None) => {
            return Some(Rendered { size, transform: None, image: ColorImage::filled([size.0 as usize, size.1 as usize], Color32::TRANSPARENT), texts: Vec::new() });
        }
        Err(e) => {
            log::warn!("Drawing Preview: {e}");
            return Some(Rendered { size, transform: None, image: ColorImage::filled([size.0 as usize, size.1 as usize], Color32::TRANSPARENT), texts: Vec::new() });
        }
    };
    let (pm, texts) = rasterize(&dl, size, colors, view)?;
    // premultiplied RGBA -> egui
    let px = pm.data();
    let pixels: Vec<Color32> = px.chunks_exact(4).map(|p| Color32::from_rgba_premultiplied(p[0], p[1], p[2], p[3])).collect();
    Some(Rendered { size, transform: Some(dl.transform), image: ColorImage::new([size.0 as usize, size.1 as usize], pixels), texts })
}

/// What a render depends on.
#[derive(Debug, Clone, PartialEq)]
struct JobKey {
    ptr: usize,
    revision: u64,
    size: (i32, i32),
    colors: CpuColors,
    /// Layer overlay (highlight suppressed while a job runs).
    view: LayerView,
}

/// Worker mailbox.
#[derive(Default)]
struct Mailbox {
    running: bool,
    pending: Option<(JobKey, Arc<RwLock<GrblFile>>)>,
    done: Option<(JobKey, Rendered)>,
}

/// CPU preview state.
pub struct CpuPanel {
    tex: Option<TextureHandle>,
    mail: Arc<Mutex<Mailbox>>,
    requested: Option<JobKey>,
    shown: Option<(JobKey, Option<Transform2D>, Vec<TextPrim>)>,
    fs_trig: bool,
    force: bool,
}

impl Default for CpuPanel {
    /// Same as `new()`.
    fn default() -> Self {
        Self::new()
    }
}

impl CpuPanel {
    /// New panel.
    pub fn new() -> Self {
        Self { tex: None, mail: Arc::new(Mutex::new(Mailbox::default())), requested: None, shown: None, fs_trig: false, force: false }
    }

    /// Force a re-render (C# `RecreateBMP`).
    pub fn invalidate(&mut self) {
        self.force = true;
    }

    /// Queue a render; starts the worker when idle.
    fn request(&self, ctx: &egui::Context, key: JobKey, file: Arc<RwLock<GrblFile>>) {
        let mut m = self.mail.lock().unwrap_or_else(|e| e.into_inner());
        m.pending = Some((key, file));
        if m.running {
            return;
        }
        m.running = true;
        let mail = self.mail.clone();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name("GrblPanel Drawing Thread".into()).spawn(move || loop {
            let job = {
                let mut m = mail.lock().unwrap_or_else(|e| e.into_inner());
                match m.pending.take() {
                    Some(j) => j,
                    None => {
                        m.running = false;
                        return;
                    }
                }
            };
            let (key, file) = job;
            // wait for loads in progress instead of blocking on the lock
            let out = match file.read() {
                Ok(g) if !g.loading_flag().load(Ordering::Relaxed) => render(&g, key.size, &key.colors, &key.view),
                Ok(_) | Err(_) => None,
            };
            if let Some(r) = out {
                mail.lock().unwrap_or_else(|e| e.into_inner()).done = Some((key, r));
                ctx.request_repaint();
            }
        });
        if spawned.is_err() {
            self.mail.lock().unwrap_or_else(|e| e.into_inner()).running = false;
        }
    }

    /// Draw the panel.
    ///
    /// `ui`: target. `input`: state. `events`: output list.
    pub fn show(&mut self, ui: &mut Ui, input: &PreviewInput, events: &mut Vec<PreviewEvent>) {
        let theme = Theme::current();
        let ppp = ui.ctx().pixels_per_point();
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, theme.preview_back_color);
        let size = (((rect.width() * ppp).round() as i32).clamp(0, 8192), ((rect.height() * ppp).round() as i32).clamp(0, 8192));

        // request a render when anything it depends on changed
        if let Some(file) = &input.file {
            let mut view = input.layer_view.clone().unwrap_or_default();
            if input.in_program {
                view.highlight = None;
            }
            let key = JobKey { ptr: Arc::as_ptr(file) as *const () as usize, revision: input.file_revision, size, colors: CpuColors::of(&theme), view };
            if size.0 > 0 && size.1 > 0 && (self.requested.as_ref() != Some(&key) || self.force) {
                self.force = false;
                self.requested = Some(key.clone());
                self.request(ui.ctx(), key, file.clone());
            }
        } else {
            self.shown = None;
            self.requested = None;
            self.tex = None;
        }

        // take a finished bitmap
        let done = self.mail.lock().unwrap_or_else(|e| e.into_inner()).done.take();
        if let Some((key, r)) = done {
            // drop stale results (resize raced the render)
            if self.requested.as_ref() == Some(&key) {
                match &mut self.tex {
                    Some(t) => t.set(r.image, TextureOptions::LINEAR),
                    None => self.tex = Some(ui.ctx().load_texture("preview-cpu", r.image, TextureOptions::LINEAR)),
                }
                self.shown = Some((key, r.transform, r.texts));
            }
        }
        let busy = self.mail.lock().unwrap_or_else(|e| e.into_inner()).running;
        if busy {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        }

        let transform = self.shown.as_ref().and_then(|s| s.1);
        if let Some(tex) = &self.tex {
            painter.image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        // texts of the draw list (rulers, bounds)
        if let Some((_, _, texts)) = &self.shown {
            for t in texts {
                let pos = rect.min + Vec2::new(t.x, t.y) / ppp;
                let galley = painter.layout_no_wrap(t.text.clone(), egui::FontId::monospace((t.em_px / ppp).max(1.0)), Color32::from_rgb(t.color[0], t.color[1], t.color[2]));
                let shape = egui::epaint::TextShape::new(pos, galley, Color32::from_rgb(t.color[0], t.color[1], t.color[2])).with_angle(t.rotation_deg.to_radians());
                painter.add(shape);
            }
        }

        // laser cross (C# OnPaint)
        let to_screen = |p: (f32, f32)| rect.min + Vec2::new(p.0, p.1) / ppp;
        let work = input.work_position;
        let mach = input.machine_position;
        let cross_at = match transform {
            Some(t) => to_screen(t.apply(work.x, work.y)),
            None => rect.min + Vec2::new(work.x, work.y),
        };
        let s = egui::Stroke::new(2.0, theme.preview_cross);
        let (cx, cy) = (cross_at.x.trunc(), cross_at.y.trunc());
        painter.line_segment([Pos2::new(cx, cy - 5.0), Pos2::new(cx, cy + 5.0)], s);
        painter.line_segment([Pos2::new(cx - 5.0, cy), Pos2::new(cx + 5.0, cy)], s);

        // position text, aligned by quadrant
        let quad = input.file.as_ref().and_then(|f| f.try_read().ok().map(|g| g.quadrant())).unwrap_or(CartesianQuadrant::Unknown);
        let f3 = |v: f32| fmt_float_custom(v, "0.000");
        let use_z = work.z != 0.0 || mach.z != 0.0 || input.enable_z_jog;
        let mut text = format!("X: {} Y: {}", f3(mach.x), f3(mach.y));
        if use_z {
            text.push_str(&format!(" Z: {}", f3(mach.z)));
        }
        let has_wco = input.working_offset != lg_gcode::GPoint::ZERO;
        if has_wco {
            text.push_str(&format!("\nX: {} Y: {}", f3(work.x), f3(work.y)));
            if use_z {
                text.push_str(&format!(" Z: {}", f3(work.z)));
            }
        }
        if input.current_f != 0.0 || input.current_s != 0.0 || self.fs_trig {
            self.fs_trig = true;
            text.push_str(&format!("\nF: {} S: {}", fmt_float_custom(input.current_f, "00000.##"), fmt_float_custom(input.current_s, "000.##")));
        }
        let inner = rect.shrink(5.0);
        let right_align = !matches!(quad, CartesianQuadrant::II | CartesianQuadrant::III);
        let bottom_align = matches!(quad, CartesianQuadrant::III | CartesianQuadrant::IV);
        let mut job = egui::text::LayoutJob::single_section(text, egui::TextFormat::simple(egui::FontId::proportional(12.0), theme.preview_text));
        // egui halign anchors the text at pos: right = text extends to the left of pos
        job.halign = if right_align { Align::RIGHT } else { Align::LEFT };
        let galley = painter.layout_job(job);
        let y = if bottom_align { inner.bottom() - galley.size().y } else { inner.top() };
        painter.galley(Pos2::new(if right_align { inner.right() } else { inner.left() }, y), galley, theme.preview_text);

        // double click jog: device point to machine point
        for (btn, fast) in [(PointerButton::Primary, false), (PointerButton::Secondary, true)] {
            if resp.double_clicked_by(btn) && input.click_n_jog && input.jog_enabled {
                if let (Some(pos), Some(t)) = (resp.hover_pos(), transform) {
                    let d = (pos - rect.min) * ppp;
                    if let Some((x, y)) = invert(&t, d.x, d.y) {
                        events.push(PreviewEvent::JogToPosition { x, y, fast });
                    }
                }
            }
        }
        let _ = Align2::LEFT_TOP;
    }
}

/// C# `DrawToMachine`: invert the affine transform.
///
/// `t`: world to device. `x`,`y`: device point. Returns world point, None if not invertible.
pub fn invert(t: &Transform2D, x: f32, y: f32) -> Option<(f32, f32)> {
    let det = t.a * t.d - t.b * t.c;
    if det.abs() < 1e-12 {
        return None;
    }
    let (dx, dy) = (x - t.e, y - t.f);
    Some(((t.d * dx - t.c * dy) / det, (-t.b * dx + t.a * dy) / det))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lg_gcode::GrblCommand;

    fn file() -> GrblFile {
        let mut f = GrblFile::new();
        for l in ["G90", "M4", "G0 X0 Y0", "G1 X50 Y0 S500 F1000", "G1 X50 Y30", "G2 X20 Y30 I-15 J0", "M5"] {
            f.commands_mut().push(GrblCommand::new(l));
        }
        f.analyze().unwrap();
        f
    }

    #[test]
    fn renders_pixels_and_inverts() {
        let th = Theme::of(crate::ui::Scheme::CADDark);
        let r = render(&file(), (400, 300), &CpuColors::of(&th), &LayerView::default()).unwrap();
        assert!(r.image.pixels.iter().any(|p| p.a() > 0), "nothing drawn");
        let t = r.transform.unwrap();
        let (dx, dy) = t.apply(12.5, 7.5);
        let (x, y) = invert(&t, dx, dy).unwrap();
        assert!((x - 12.5).abs() < 1e-3 && (y - 7.5).abs() < 1e-3);
        assert!(!r.texts.is_empty());
    }

    #[test]
    fn empty_file_is_transparent() {
        let th = Theme::of(crate::ui::Scheme::CADDark);
        let r = render(&GrblFile::new(), (50, 50), &CpuColors::of(&th), &LayerView::default()).unwrap();
        assert!(r.transform.is_none());
    }

    /// Count of strongly red pixels. `r`: render. Returns count.
    fn red(r: &Rendered) -> usize {
        r.image.pixels.iter().filter(|p| p.a() > 200 && p.r() > 200 && p.g() < 60 && p.b() < 60).count()
    }

    #[test]
    #[allow(clippy::single_range_in_vec_init)]
    fn highlight_is_red_and_hidden_vanishes() {
        let th = Theme::of(crate::ui::Scheme::CADDark);
        let c = CpuColors::of(&th);
        let f = file();
        let plain = render(&f, (400, 300), &c, &LayerView::default()).unwrap();
        // highlight the laser-on commands 3..6
        let hl = LayerView { revision: 1, highlight: Some(3..6), hidden: vec![] };
        let r = render(&f, (400, 300), &c, &hl).unwrap();
        assert!(red(&r) > red(&plain) + 100, "no red stroke: {} vs {}", red(&r), red(&plain));
        // hide them: fewer pixels, no red
        let hid = LayerView { revision: 2, highlight: None, hidden: vec![3..6] };
        let r = render(&f, (400, 300), &c, &hid).unwrap();
        let count = |r: &Rendered| r.image.pixels.iter().filter(|p| p.a() > 0).count();
        assert!(count(&r) < count(&plain));
        assert_eq!(red(&r), red(&plain));
    }
}
