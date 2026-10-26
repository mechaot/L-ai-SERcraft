// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `UserControls/GrblPanel3D.cs`: OpenGL job preview as an egui widget.
//!
//! Geometry/GL live in [`super::scene`] and [`super::gl`]; this file holds the camera, input handling,
//! grid/ruler/pointer construction and the egui overlays (rulers, info boxes, cross cursor).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Align2, Color32, CursorIcon, PointerButton, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use lg_dotnet::decimal_to_f64;
use lg_gcode::GrblFile;

use super::camera::{human_readable_length, ruler_step, ruler_ticks, Bounds, Camera, PADDING};
use super::gl::{self, Batch, GlShared, Item};
use super::overlay;
use super::scene::{self, recolor_chunk, recolor_off_chunk, BuildResult, JobColors, SceneShared, Segment};
use super::{rgb, PreviewEvent, PreviewInput};
use crate::ui::widgets::graph::wheel_notches;
use crate::ui::{tr, Theme};

/// Unique scene ids (GL buffers keyed by them).
static SCENE_ID: AtomicU64 = AtomicU64::new(1);

/// Cache key of the highlight segments: (overlay revision, scene id, scene done, segment count).
type HighlightKey = (u64, u64, bool, usize);

/// Color of the selected layer overlay (pure red).
const HIGHLIGHT_RGBA: [u8; 4] = [255, 0, 0, 255];

/// Moving average of the last `n` samples (C# `MobileDAverageCalculator`).
#[derive(Debug, Clone)]
struct MovingAvg {
    n: usize,
    v: Vec<f64>,
}

impl MovingAvg {
    /// New average over `n` samples.
    fn new(n: usize) -> Self {
        Self { n, v: Vec::new() }
    }

    /// Add a sample.
    fn push(&mut self, x: f64) {
        if self.v.len() >= self.n {
            self.v.remove(0);
        }
        self.v.push(x);
    }

    /// Current average (0 when empty).
    fn avg(&self) -> f64 {
        if self.v.is_empty() { 0.0 } else { self.v.iter().sum::<f64>() / self.v.len() as f64 }
    }
}

/// Identity of the loaded file as seen by the builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileKey {
    ptr: usize,
    revision: u64,
}

/// OpenGL preview state.
pub struct GlPanel {
    cam: Camera,
    shared: Arc<Mutex<GlShared>>,
    scene: Option<Arc<SceneShared>>,
    key: Option<FileKey>,
    pending_auto_size: bool,
    last_size: Vec2,
    colors: Option<(JobColors, bool)>,
    /// Layer overlay state last applied to the chunks: (revision, scene id, segment count when hiding).
    layer_sig: Option<(u64, u64, usize)>,
    /// Red segments of the selected layer, with their cache key.
    highlight: Option<(HighlightKey, Vec<Segment>)>,
    mouse_world: Option<(f64, f64)>,
    fs_trig: bool,
    frame_time: MovingAvg,
    refresh: MovingAvg,
    last_paint: Option<Instant>,
    last_view: [f64; 4],
    /// Message shown instead of the load percentage (C# `mMessage`).
    message: Option<String>,
}

impl Default for GlPanel {
    /// Same as `new()`.
    fn default() -> Self {
        Self::new()
    }
}

impl GlPanel {
    /// New panel; GL is created lazily on the first paint.
    pub fn new() -> Self {
        Self {
            cam: Camera::default(),
            shared: Arc::new(Mutex::new(GlShared::default())),
            scene: None,
            key: None,
            pending_auto_size: true,
            last_size: Vec2::ZERO,
            colors: None,
            layer_sig: None,
            highlight: None,
            mouse_world: None,
            fs_trig: false,
            frame_time: MovingAvg::new(10),
            refresh: MovingAvg::new(3),
            last_paint: None,
            last_view: [0.0; 4],
            message: None,
        }
    }

    /// GL init failure text, if any.
    pub fn failure(&self) -> Option<String> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner()).failed.clone()
    }

    /// GL context is up.
    pub fn is_ready(&self) -> bool {
        self.shared.lock().unwrap_or_else(|e| e.into_inner()).ready
    }

    /// Vendor / renderer / version text.
    pub fn info(&self) -> String {
        self.shared.lock().unwrap_or_else(|e| e.into_inner()).info.clone()
    }

    /// Camera (read only).
    pub fn camera(&self) -> &Camera {
        &self.cam
    }

    /// Ask for auto size on the next frame (C# `AutoSizeDrawing`).
    pub fn request_auto_size(&mut self) {
        self.pending_auto_size = true;
    }

    /// C# `CenterZoom`.
    ///
    /// `k`: factor on the half sizes.
    pub fn zoom(&mut self, k: f64) {
        self.cam.center_zoom(k);
    }

    /// Stop a running build and drop the scene.
    fn drop_scene(&mut self) {
        if let Some(s) = self.scene.take() {
            s.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Start (re)building the scene of `file` on a thread (C# `mReload`).
    fn start_build(&mut self, ctx: &egui::Context, file: &Arc<std::sync::RwLock<GrblFile>>, colors: JobColors, show_executed: bool) {
        self.drop_scene();
        let shared = SceneShared::new(SCENE_ID.fetch_add(1, Ordering::Relaxed));
        self.scene = Some(shared.clone());
        let file = file.clone();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name("GrblPanel3D build".into()).spawn(move || {
            if scene::build_scene(&file, &colors, show_executed, &shared) == BuildResult::Done {
                ctx.request_repaint();
            }
        });
        if let Err(e) = spawned {
            log::error!("preview build thread: {e}");
        }
    }

    /// Draw the panel.
    ///
    /// `ui`: target. `input`: state. `events`: output list.
    pub fn show(&mut self, ui: &mut Ui, input: &PreviewInput, events: &mut Vec<PreviewEvent>) {
        let t0 = Instant::now();
        if let Some(prev) = self.last_paint {
            self.refresh.push(prev.elapsed().as_secs_f64() * 1000.0);
        }
        self.last_paint = Some(t0);

        let theme = Theme::current();
        let opts = input.options;
        let size = ui.available_size();
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let (w, h) = (rect.width() as f64, rect.height() as f64);
        let colors = JobColors {
            background: rgb(theme.preview_back_color),
            laser_power: rgb(theme.preview_laser_power),
            other_movement: rgb(theme.preview_other_movement),
            command_ok: rgb(theme.preview_command_ok),
            command_ko: rgb(theme.preview_command_ko),
            command_wait: rgb(theme.preview_command_wait),
        };

        // follow control size (C# Resize)
        if rect.size() != self.last_size {
            self.cam.on_resize(w, h);
            self.last_size = rect.size();
        }

        // file change -> rebuild + auto size (C# OnFileLoaded)
        self.sync_file(ui.ctx(), input, colors, opts.show_executed_commands);
        if self.pending_auto_size && w > 1.0 && h > 1.0 {
            self.pending_auto_size = false;
            let bounds = input.file.as_ref().and_then(|f| f.try_read().ok().and_then(|g| file_bounds(&g, opts.auto_size_on_drawing)));
            self.cam.auto_size(w, h, bounds);
        }

        // recolor on theme / option change and for invalidated chunks (C# mInvalidateAll / Invalidate)
        self.update_colors(input, colors, opts.show_executed_commands);

        // input
        self.handle_input(ui, &resp, rect, input, events);

        // paint
        let painter = ui.painter_at(rect);
        let items = self.build_items(input, &theme, w, h);
        let frame = gl::Frame { view: [self.cam.left, self.cam.right, self.cam.bottom, self.cam.top], background: colors.background, items: Arc::new(items), scene: self.scene.clone() };
        painter.add(egui::PaintCallback { rect, callback: Arc::new(gl::callback(self.shared.clone(), frame)) });
        self.draw_rulers(&painter, rect, input, &theme);
        self.draw_overlays(&painter, rect, input, &theme, &resp);

        // view changed event
        let view = [self.cam.left, self.cam.right, self.cam.bottom, self.cam.top];
        if view != self.last_view {
            self.last_view = view;
            events.push(PreviewEvent::ViewChanged { left: view[0], right: view[1], bottom: view[2], top: view[3] });
        }

        self.frame_time.push(t0.elapsed().as_secs_f64() * 1000.0);
        // keep animating while GL is not up or the scene is loading
        let loading = self.scene.as_ref().is_some_and(|s| !s.scene.lock().map(|g| g.done).unwrap_or(true));
        if loading || !self.is_ready() {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }
    }

    /// Detect file replacement and (re)start the builder.
    fn sync_file(&mut self, ctx: &egui::Context, input: &PreviewInput, colors: JobColors, show_executed: bool) {
        // revision 0 = nothing loaded yet: C# `mGrbl3D` stays null until the first `OnFileLoaded` (tips shown)
        let Some(file) = input.file.as_ref().filter(|_| input.file_revision > 0) else {
            if self.key.take().is_some() {
                self.drop_scene();
            }
            self.message = None;
            return;
        };
        // file still loading? wait (C# OnFileLoading message)
        let loading = match file.try_read() {
            Ok(g) => g.loading_flag().load(Ordering::Relaxed),
            Err(_) => true,
        };
        if loading {
            self.message = Some(tr("Strings.Loading").to_owned());
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }
        self.message = None;
        let key = FileKey { ptr: Arc::as_ptr(file) as *const () as usize, revision: input.file_revision };
        if self.key != Some(key) {
            self.key = Some(key);
            self.start_build(ctx, file, colors, show_executed);
            self.pending_auto_size = true;
        }
    }

    /// Recolor chunks when the palette changed or commands got results.
    fn update_colors(&mut self, input: &PreviewInput, colors: JobColors, show_executed: bool) {
        let Some(sc) = self.scene.clone() else {
            self.highlight = None;
            return;
        };
        let view = input.layer_view.clone().unwrap_or_default();
        let hidden = &view.hidden;
        let file = input.file.as_ref();
        let mut s = sc.scene.lock().unwrap_or_else(|e| e.into_inner());
        let total = s.on.segment_count() + s.off.segment_count();
        // hidden layers need a recolor on every geometry growth; plain changes only on revision
        let sig = (view.revision, sc.id, if hidden.is_empty() { 0 } else { total });
        let layer_changed = self.layer_sig != Some(sig);
        self.layer_sig = Some(sig);
        let all = self.colors != Some((colors, show_executed)) || layer_changed;
        self.colors = Some((colors, show_executed));
        if all {
            for c in &mut s.on.chunks {
                c.seen_gen = u64::MAX;
            }
            for c in &mut s.off.chunks {
                recolor_off_chunk(c, &colors, hidden);
            }
        }
        // selected layer in red, cached per overlay revision and scene growth
        let hl_key = (view.revision, sc.id, s.done, total);
        match (&view.highlight, input.in_program) {
            (Some(range), false) => {
                if self.highlight.as_ref().map(|h| h.0) != Some(hl_key) {
                    let mut segs = Vec::new();
                    for c in &s.on.chunks {
                        for (i, seg) in c.segs.iter().enumerate() {
                            if range.contains(&c.cmd[i]) {
                                segs.push(Segment { rgba: HIGHLIGHT_RGBA, ..*seg });
                            }
                        }
                    }
                    self.highlight = Some((hl_key, segs));
                }
            }
            _ => self.highlight = None,
        }
        let Some(file) = file else { return };
        let Ok(f) = file.try_read() else { return };
        // cap per frame so a big job never stalls the UI
        let mut budget = 64;
        for c in &mut s.on.chunks {
            if c.link.generation() != c.seen_gen {
                recolor_chunk(c, &f, &colors, show_executed, hidden);
                budget -= 1;
                if budget == 0 {
                    break;
                }
            }
        }
    }

    /// Mouse: pan, wheel zoom, double click jog (C# GrblSceneControl_* and DoubleClick).
    fn handle_input(&mut self, ui: &Ui, resp: &egui::Response, rect: Rect, input: &PreviewInput, events: &mut Vec<PreviewEvent>) {
        let (w, h) = (rect.width() as f64, rect.height() as f64);
        let hover = resp.hover_pos().map(|p| p - rect.min);
        self.mouse_world = hover.map(|p| self.cam.screen_to_world(p.x as f64, p.y as f64, w, h));
        // pan with left drag
        if resp.dragged_by(PointerButton::Primary) {
            let d = resp.drag_delta();
            self.cam.pan(d.x as f64, d.y as f64, w);
        }
        // wheel zoom to the mouse position: k = Delta / 1000, one notch = 120
        if resp.hovered() {
            let notches = wheel_notches(ui);
            if notches != 0 {
                if let Some((mx, my)) = self.mouse_world {
                    self.cam.wheel_zoom(mx, my, notches as f64 * 0.12);
                }
            }
        }
        // double click: jog (left = normal, right = fast)
        for (btn, fast) in [(PointerButton::Primary, false), (PointerButton::Secondary, true)] {
            if resp.double_clicked_by(btn) && input.click_n_jog && input.jog_enabled {
                if let Some((x, y)) = self.mouse_world {
                    events.push(PreviewEvent::JogToPosition { x: x as f32, y: y as f32, fast });
                }
            }
        }
    }

    /// Dynamic geometry in paint order (C# `DrawScene`).
    fn build_items(&self, input: &PreviewInput, theme: &Theme, w: f64, _h: f64) -> Vec<Item> {
        let opts = input.options;
        let mut items = Vec::new();
        for b in grid_batches(&self.cam, w, theme) {
            items.push(Item::Batch(b));
        }
        // machine table (dashed, ruler color)
        if let Some((tw, th)) = input.machine_area {
            let c = rgba(theme.preview_ruler);
            let (tw, th) = (tw, th);
            let seg = |x0: f32, y0: f32, x1: f32, y1: f32| Segment { x0, y0, x1, y1, rgba: c };
            items.push(Item::Batch(Batch { segs: vec![seg(0.0, 0.0, tw, 0.0), seg(tw, 0.0, tw, th), seg(tw, th, 0.0, th), seg(0.0, th, 0.0, 0.0)], width: 1.0, dash: 6.0 }));
        }
        let has_scene = self.scene.is_some();
        if has_scene && opts.show_laser_off_movements {
            items.push(Item::Off);
        }
        if has_scene && opts.show_bounding_box {
            if let Some(b) = input.file.as_ref().and_then(|f| f.try_read().ok().and_then(|g| drawing_bounds(&g))) {
                items.push(Item::Batch(bbox_batch(&self.cam, b, rgba(theme.preview_job_range))));
            }
        }
        if has_scene {
            items.push(Item::On(opts.preview_line_size.max(0.5)));
            // selected layer on top, wide red stroke
            if let Some((_, segs)) = self.highlight.as_ref().filter(|(_, s)| !s.is_empty()) {
                items.push(Item::Batch(Batch { segs: segs.clone(), width: (3.0 * opts.preview_line_size).max(3.0), dash: 0.0 }));
            }
        }
        items.push(Item::Batch(pointer_batch(&self.cam, w, input, rgba(theme.preview_cross))));
        items
    }

    /// Ruler strips + labels (C# `DrawRulers`).
    fn draw_rulers(&self, p: &egui::Painter, rect: Rect, input: &PreviewInput, theme: &Theme) {
        let pad = PADDING;
        let back = theme.preview_back_color;
        // strips cover the geometry like the GL scissor clears
        p.rect_filled(Rect::from_min_max(rect.min, Pos2::new(rect.left() + pad.left, rect.bottom())), 0.0, back);
        p.rect_filled(Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - pad.bottom), rect.max), 0.0, back);
        if pad.right > 0.0 {
            p.rect_filled(Rect::from_min_max(Pos2::new(rect.right() - pad.right, rect.top()), rect.max), 0.0, back);
        }
        if pad.top > 0.0 {
            p.rect_filled(Rect::from_min_max(rect.min, Pos2::new(rect.right(), rect.top() + pad.top)), 0.0, back);
        }
        let cam = &self.cam;
        let (w, h) = (rect.width() as f64, rect.height() as f64);
        if cam.width() <= 0.0 || cam.height() <= 0.0 || w <= 0.0 || h <= 0.0 {
            return;
        }
        let world_w = cam.width();
        let step = ruler_step(world_w);
        let w_ratio = w / world_w;
        let h_ratio = h / cam.height();
        let (_, uom) = human_readable_length(0, world_w);
        let bbox = if input.options.show_bounding_box {
            input.file.as_ref().and_then(|f| f.try_read().ok().and_then(|g| drawing_bounds(&g)))
        } else {
            None
        };
        let font = egui::FontId::monospace(10.0);
        let text_color = theme.preview_text;
        let edge_color = theme.preview_job_range;
        let tick_x = bbox.map(|b| (b.0 as i64, b.1 as i64));
        let tick_y = bbox.map(|b| (b.2 as i64, b.3 as i64));
        // horizontal
        let start = cam.left as i64 + (pad.left as f64 / w_ratio) as i64;
        let end = cam.right as i64 - (pad.right as f64 / w_ratio) as i64;
        for (i, edge) in ruler_ticks(start, end, step, tick_x) {
            let (text, _) = human_readable_length(i, world_w);
            let x = i as f64 * w_ratio - cam.left * w_ratio;
            let pos = Pos2::new(rect.left() + x as f32 - 3.0, rect.bottom() - pad.bottom + 3.0);
            p.text(pos, Align2::LEFT_TOP, text, font.clone(), if edge { edge_color } else { text_color });
        }
        // vertical
        let start = cam.bottom as i64 + (pad.bottom as f64 / h_ratio) as i64;
        let end = cam.top as i64 - (pad.top as f64 / h_ratio) as i64;
        for (i, edge) in ruler_ticks(start, end, step, tick_y) {
            let (text, _) = human_readable_length(i, world_w);
            let y = i as f64 * h_ratio - cam.bottom * h_ratio;
            let pos = Pos2::new(rect.left() + pad.left - 3.0, rect.bottom() - y as f32);
            p.text(pos, Align2::RIGHT_CENTER, text, font.clone(), if edge { edge_color } else { text_color });
        }
        // unit of measure corner
        p.rect_filled(Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - pad.bottom), Pos2::new(rect.left() + pad.left, rect.bottom())), 0.0, back);
        p.text(Pos2::new(rect.left() + pad.left - 20.0, rect.bottom() - pad.bottom + 3.0), Align2::RIGHT_TOP, uom, font, edge_color);
    }

    /// HUD boxes and cross cursor (C# `DoGDIDraw`).
    fn draw_overlays(&mut self, p: &egui::Painter, rect: Rect, input: &PreviewInput, theme: &Theme, resp: &egui::Response) {
        let pad = PADDING;
        let fore = theme.preview_text;
        let back = theme.preview_ruler;
        let use_z = input.work_position.z != 0.0 || input.machine_position.z != 0.0 || input.enable_z_jog;
        let right_edge = rect.right() - pad.right;
        let (mp, wp) = (input.machine_position, input.work_position);

        // position box
        let mut text = String::from("         X          Y");
        if use_z {
            text.push_str("          Z    ");
        }
        text.push_str(&format!("\nMCO {} {}", overlay::format_coord(mp.x), overlay::format_coord(mp.y)));
        if use_z {
            text.push_str(&format!(" {}", overlay::format_coord(mp.z)));
        }
        if input.working_offset != lg_gcode::GPoint::ZERO {
            text.push_str(&format!("\nWCO {} {}", overlay::format_coord(wp.x), overlay::format_coord(wp.y)));
            if use_z {
                text.push_str(&format!(" {}", overlay::format_coord(wp.z)));
            }
        }
        if let Some((mx, my)) = self.mouse_world {
            text.push_str(&format!("\nPTR {} {}", overlay::format_coord(mx as f32), overlay::format_coord(my as f32)));
            if use_z {
                text.push_str("          ");
            }
        }
        let size = overlay::measure(p, &text);
        let mut point = Pos2::new(right_edge - size.x, rect.top() + 12.0);
        let mut last_size = overlay::label(p, point, &text, back, 100, fore);

        // feed / spindle
        if input.current_f != 0.0 || input.current_s != 0.0 || self.fs_trig {
            self.fs_trig = true;
            let y = point.y + last_size.y + 5.0;
            let text = format!("F {}\nS {}", overlay::format_feed(input.current_f), overlay::format_spindle(input.current_s));
            let size = overlay::measure(p, &text);
            point = Pos2::new(right_edge - size.x, y);
            last_size = overlay::label(p, point, &text, back, 100, fore);
        }

        // performance
        if input.options.show_performance_diagnostic {
            let y = point.y + last_size.y + 5.0;
            let vc = self.scene.as_ref().map_or(0, |s| s.vertex_count());
            let vs = if vc < 1000 {
                format!("{vc:>6}")
            } else if vc < 1_000_000 {
                format!("{:>6.1} K", vc as f64 / 1000.0)
            } else if vc < 1_000_000_000 {
                format!("{:>6.1} M", vc as f64 / 1_000_000.0)
            } else {
                format!("{:>6.1} B", vc as f64 / 1_000_000_000.0)
            };
            let rps = (1000.0 / self.refresh.avg().max(0.001) - 1.0).floor().max(0.0);
            let text = format!("VER   {vs}\nAFT   {:>6.0} ms\nAST   {:>6.0} ms\nRPS   {:>6.0}", self.frame_time.avg(), 0.0, rps);
            let size = overlay::measure(p, &text);
            point = Pos2::new(right_edge - size.x, y);
            last_size = overlay::label(p, point, &text, back, 100, fore);
        }

        // loading
        let perc = self.scene.as_ref().map(|s| (s.percentage(), s.scene.lock().map(|g| g.done).unwrap_or(true)));
        if self.message.is_some() || perc.is_some_and(|(_, done)| !done) {
            let y = point.y + last_size.y + 5.0;
            let text = match &self.message {
                Some(m) => m.clone(),
                None => format!("{} {:.1}%", tr("Strings.Loading"), perc.map_or(0.0, |x| x.0)),
            };
            let size = overlay::measure(p, &text);
            point = Pos2::new(right_edge - size.x, y);
            overlay::label(p, point, &text, back, 100, fore);
        }

        // usage tips while nothing is loaded
        if self.scene.is_none() {
            let mut text = format!("{}\n{}\n{}\n{}\n\n", tr("Strings.TipsBasicUsage"), tr("Strings.TipsZoom"), tr("Strings.TipsPan"), tr("Strings.TipsJog"));
            let mut sc = String::new();
            for (k, label) in [(&input.hotkeys.auto_size, "Strings.TipsZoomAuto"), (&input.hotkeys.zoom_in, "Strings.TipsZoomIn"), (&input.hotkeys.zoom_out, "Strings.TipsZoomOut")] {
                if !k.is_empty() {
                    sc.push_str(&format!("{}: {}\n", k, tr(label)));
                }
            }
            if !sc.is_empty() {
                text.push_str(&format!("{}\n{}", tr("Strings.TipsKeyboardShortcuts"), sc));
            }
            let text = text.trim_matches(['\r', '\n']).to_owned();
            let size = overlay::measure(p, &text);
            let point = Pos2::new(right_edge - size.x, rect.bottom() - size.y - 35.0);
            overlay::label(p, point, &text, back, 60, fore);
        }

        // cross cursor
        let inner = Rect::from_min_max(
            Pos2::new(rect.left() + pad.left, rect.top() + pad.top),
            Pos2::new(rect.right() - pad.right, rect.bottom() - pad.bottom),
        );
        let hover = resp.hover_pos();
        let pressed = resp.is_pointer_button_down_on();
        if resp.dragged_by(PointerButton::Primary) {
            resp.ctx.set_cursor_icon(CursorIcon::AllScroll);
        } else if pressed {
            resp.ctx.set_cursor_icon(CursorIcon::Crosshair);
        } else if let (true, Some(pos)) = (input.options.cross_cursor, hover) {
            if inner.contains(pos) {
                let shadow = Color32::from_rgba_unmultiplied(theme.preview_back_color.r(), theme.preview_back_color.g(), theme.preview_back_color.b(), 128);
                draw_cross(p, inner, pos + Vec2::splat(1.0), shadow);
                draw_cross(p, inner, pos, theme.preview_cross_cursor);
                resp.ctx.set_cursor_icon(CursorIcon::None);
            }
        }
    }
}

/// C# `DrawCross`: lines to the inner borders and a small box around the point.
///
/// `p`: painter. `inner`: area without rulers. `pt`: cursor point. `color`: stroke color.
fn draw_cross(p: &egui::Painter, inner: Rect, pt: Pos2, color: Color32) {
    const HALF: f32 = 4.0;
    let s = Stroke::new(1.0, color);
    p.line_segment([Pos2::new(inner.left(), pt.y), Pos2::new(pt.x - 5.0, pt.y)], s);
    p.line_segment([Pos2::new(pt.x + HALF, pt.y), Pos2::new(inner.right(), pt.y)], s);
    p.line_segment([Pos2::new(pt.x, inner.top()), Pos2::new(pt.x, pt.y - HALF)], s);
    p.line_segment([Pos2::new(pt.x, pt.y + HALF), Pos2::new(pt.x, inner.bottom())], s);
    p.rect_stroke(Rect::from_min_size(pt - Vec2::splat(HALF), Vec2::splat(HALF * 2.0)), 0.0, s, egui::StrokeKind::Middle);
}

/// egui color to opaque RGBA bytes.
///
/// `c`: color. Returns `[r, g, b, 255]`.
fn rgba(c: Color32) -> [u8; 4] {
    [c.r(), c.g(), c.b(), 255]
}

/// Bounds of a range as f64.
///
/// `r`: XY range. Returns `(xmin, xmax, ymin, ymax)` when valid.
fn range_bounds(r: &lg_gcode::XYRange) -> Option<Bounds> {
    if !r.valid_range() {
        return None;
    }
    Some((decimal_to_f64(r.x.min), decimal_to_f64(r.x.max), decimal_to_f64(r.y.min), decimal_to_f64(r.y.max)))
}

/// Drawing range of the file.
///
/// `f`: file. Returns bounds when valid.
fn drawing_bounds(f: &GrblFile) -> Option<Bounds> {
    range_bounds(&f.range().drawing_range)
}

/// Bounds for auto size (C# `AutoSizeDrawing`): drawing range or, if the option is off, the moving range.
///
/// `f`: file. `on_drawing`: `AutoSizeOnDrawing`. Returns bounds; None = default viewport.
pub fn file_bounds(f: &GrblFile, on_drawing: bool) -> Option<Bounds> {
    let r = f.range();
    if !r.drawing_range.valid_range() {
        return None;
    }
    range_bounds(if on_drawing { &r.drawing_range } else { &r.moving_range })
}

/// Grid: minor lines (when zoomed in), major ticks and origin axes (C# `Grid3D.Draw`).
///
/// `cam`: camera. `w`: control width in points. `theme`: colors. Returns batches in paint order.
pub fn grid_batches(cam: &Camera, w: f64, theme: &Theme) -> Vec<Batch> {
    let mut out = Vec::new();
    if w <= 0.0 || cam.width() <= 0.0 {
        return out;
    }
    let grid = cam.grid_size(w);
    let (l, r, b, t) = (cam.left, cam.right, cam.bottom, cam.top);
    let vline = |x: f64, c: [u8; 4]| Segment { x0: x as f32, y0: b as f32, x1: x as f32, y1: t as f32, rgba: c };
    let hline = |y: f64, c: [u8; 4]| Segment { x0: l as f32, y0: y as f32, x1: r as f32, y1: y as f32, rgba: c };
    // minor every mm when at least 5 px per mm
    if grid <= 2.0 {
        let c = rgba(theme.preview_grid_minor);
        let mut segs = Vec::new();
        for i in (l.ceil() as i64)..=(r.floor() as i64) {
            segs.push(vline(i as f64, c));
        }
        for i in (b.ceil() as i64)..=(t.floor() as i64) {
            segs.push(hline(i as f64, c));
        }
        out.push(Batch { segs, width: 1.0, dash: 0.0 });
    }
    // major ticks: C# `Math.Max(10, (int)GridSize * 2)`
    let step = ((grid as i64) * 2).max(10);
    let c = rgba(theme.preview_grid);
    let mut segs = Vec::new();
    let first = (l / step as f64).ceil() as i64;
    let last = (r / step as f64).floor() as i64;
    for m in first..=last {
        segs.push(vline((m * step) as f64, c));
    }
    let first = (b / step as f64).ceil() as i64;
    let last = (t / step as f64).floor() as i64;
    for m in first..=last {
        segs.push(hline((m * step) as f64, c));
    }
    out.push(Batch { segs, width: 1.0, dash: 0.0 });
    // axes
    let c = rgba(theme.preview_ruler);
    out.push(Batch { segs: vec![vline(0.0, c), hline(0.0, c)], width: 1.0, dash: 0.0 });
    out
}

/// Bounding box lines (C# `Grbl3D` first display list: infinite dashed lines).
///
/// `cam`: camera (lines are clamped to the window +margin). `b`: drawing range. `color`: line color.
pub fn bbox_batch(cam: &Camera, b: Bounds, color: [u8; 4]) -> Batch {
    let m = cam.width().max(cam.height());
    let (l, r, bo, t) = ((cam.left - m) as f32, (cam.right + m) as f32, (cam.bottom - m) as f32, (cam.top + m) as f32);
    let (xmin, xmax, ymin, ymax) = (b.0 as f32, b.1 as f32, b.2 as f32, b.3 as f32);
    let s = |x0, y0, x1, y1| Segment { x0, y0, x1, y1, rgba: color };
    Batch { segs: vec![s(l, ymin, r, ymin), s(xmax, bo, xmax, t), s(l, ymax, r, ymax), s(xmin, bo, xmin, t)], width: 1.0, dash: 10.0 }
}

/// Laser cross at the work position (C# `DrawPointer`: size = GridSize * 1.5, width 3).
///
/// `cam`: camera. `w`: control width. `input`: positions. `color`: cross color.
pub fn pointer_batch(cam: &Camera, w: f64, input: &PreviewInput, color: [u8; 4]) -> Batch {
    let size = (cam.grid_size(w) * 1.5) as f32;
    let (x, y) = (input.work_position.x, input.work_position.y);
    Batch {
        segs: vec![Segment { x0: x - size, y0: y, x1: x + size, y1: y, rgba: color }, Segment { x0: x, y0: y - size, x1: x, y1: y + size, rgba: color }],
        width: 3.0,
        dash: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Scheme;

    #[test]
    fn grid_minor_only_when_zoomed_in() {
        let th = Theme::of(Scheme::CADDark);
        let mut cam = Camera::default();
        cam.auto_size(800.0, 600.0, Some((0.0, 100.0, 0.0, 50.0)));
        assert_eq!(grid_batches(&cam, 800.0, &th).len(), 3);
        cam.set_world_position(-5000.0, 5000.0, -4000.0, 4000.0);
        let g = grid_batches(&cam, 800.0, &th);
        assert_eq!(g.len(), 2);
        // spacing at least 10, tick count bounded
        assert!(g[0].segs.len() < 300);
    }

    #[test]
    fn pointer_is_cross() {
        let cam = Camera::default();
        let mut i = PreviewInput::default();
        i.work_position = lg_gcode::GPoint::new(10.0, 20.0, 0.0);
        let b = pointer_batch(&cam, 400.0, &i, [1, 2, 3, 255]);
        assert_eq!(b.segs.len(), 2);
        assert!((b.segs[0].y0 - 20.0).abs() < 1e-6);
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;
    use crate::ui::preview::{Backend, PreviewPanel};

    /// One frame with the given events.
    fn frame(ctx: &egui::Context, panel: &mut PreviewPanel, input: &PreviewInput, t: f64, events: Vec<egui::Event>) -> Vec<PreviewEvent> {
        let mut raw = egui::RawInput::default();
        raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 400.0)));
        raw.time = Some(t);
        raw.events = events;
        let mut out = Vec::new();
        let mut o = ctx.run_ui(raw, |ui| {
            out = panel.show(ui, input);
        });
        o.textures_delta.clear();
        out
    }

    fn click(pos: Pos2, btn: PointerButton) -> Vec<egui::Event> {
        let m = egui::Modifiers::NONE;
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton { pos, button: btn, pressed: true, modifiers: m },
            egui::Event::PointerButton { pos, button: btn, pressed: false, modifiers: m },
        ]
    }

    /// Width of the last reported view in `events`.
    fn view_width(events: &[PreviewEvent]) -> Option<f64> {
        events.iter().rev().find_map(|e| match e {
            PreviewEvent::ViewChanged { left, right, .. } => Some(right - left),
            _ => None,
        })
    }

    #[test]
    fn double_click_jogs_and_wheel_zooms() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let ctx = egui::Context::default();
        let mut panel = PreviewPanel::new(Backend::Gl);
        let mut input = PreviewInput::default();
        input.jog_enabled = true;
        let w0 = view_width(&frame(&ctx, &mut panel, &input, 0.0, vec![])).expect("initial view");
        let pos = Pos2::new(300.0, 200.0);
        let mut ev = Vec::new();
        for t in [0.1, 0.15] {
            ev = frame(&ctx, &mut panel, &input, t, click(pos, PointerButton::Primary));
        }
        assert!(ev.iter().any(|e| matches!(e, PreviewEvent::JogToPosition { fast: false, .. })), "events: {ev:?}");
        for t in [0.6, 0.65] {
            ev = frame(&ctx, &mut panel, &input, t, click(pos, PointerButton::Secondary));
        }
        assert!(ev.iter().any(|e| matches!(e, PreviewEvent::JogToPosition { fast: true, .. })), "events: {ev:?}");

        // jog disabled -> no event
        input.jog_enabled = false;
        for t in [1.0, 1.05] {
            ev = frame(&ctx, &mut panel, &input, t, click(pos, PointerButton::Primary));
        }
        assert!(!ev.iter().any(|e| matches!(e, PreviewEvent::JogToPosition { .. })));

        // wheel up zooms in
        let evs = vec![
            egui::Event::PointerMoved(pos),
            egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: Vec2::new(0.0, 1.0), phase: egui::TouchPhase::Move, modifiers: egui::Modifiers::NONE, source: egui::MouseWheelSource::Wheel },
        ];
        let w1 = view_width(&frame(&ctx, &mut panel, &input, 2.0, evs)).expect("view after wheel");
        assert!(w1 < w0, "{w0} -> {w1}");
    }
}
