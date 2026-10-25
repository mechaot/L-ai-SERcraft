// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Gallery "Preview" tab: both renderers side by side with sample programs, a fake moving cursor
//! and a fake "program run" that marks commands as executed (exercises the invalidation path).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use eframe::egui::{self, Ui};
use lg_gcode::{GPoint, GrblCommand, GrblFile};

use super::{Backend, PreviewEvent, PreviewInput, PreviewPanel};

/// Sample program choices.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sample {
    Svg,
    Arcs,
    Raster,
    Large,
}

/// Generated raster program (long strokes, S modulated), `lines` G1 moves.
///
/// `lines`: number of moves. Returns the analysed file.
pub fn generate_raster(lines: usize) -> GrblFile {
    let mut f = GrblFile::new();
    {
        let l = f.commands_mut();
        l.push(GrblCommand::new("G90"));
        l.push(GrblCommand::new("M4"));
        l.push(GrblCommand::new("G0 X0 Y0"));
        let (mut x, mut y, mut dir) = (0.0f64, 0.0f64, 1.0f64);
        for i in 0..lines {
            if i % 400 == 399 {
                y += 0.1;
                l.push(GrblCommand::new(&format!("G0 X{x:.3} Y{y:.3}")));
                dir = -dir;
            } else {
                x += dir * 0.1;
                l.push(GrblCommand::new(&format!("G1 X{x:.3} Y{y:.3} S{}", (i * 37) % 1000)));
            }
        }
        l.push(GrblCommand::new("M5"));
    }
    let _ = f.analyze();
    f
}

/// Demo state.
pub struct PreviewDemo {
    gl: PreviewPanel,
    cpu: PreviewPanel,
    file: Arc<RwLock<GrblFile>>,
    revision: Arc<AtomicU64>,
    sample: Option<Sample>,
    input: PreviewInput,
    run_pos: Option<usize>,
    last_event: String,
    started: std::time::Instant,
    frames: u32,
}

impl Default for PreviewDemo {
    /// Empty demo (nothing loaded).
    fn default() -> Self {
        Self {
            gl: PreviewPanel::new(Backend::Gl),
            cpu: PreviewPanel::new(Backend::Cpu),
            file: Arc::new(RwLock::new(GrblFile::new())),
            revision: Arc::new(AtomicU64::new(0)),
            sample: None,
            input: PreviewInput::default(),
            run_pos: None,
            last_event: String::new(),
            started: std::time::Instant::now(),
            frames: 0,
        }
    }
}

impl PreviewDemo {
    /// Load a sample on a thread (replaces the program, bumps the revision).
    ///
    /// `ctx`: for repaint. `s`: sample.
    fn load(&mut self, ctx: &egui::Context, s: Sample) {
        self.sample = Some(s);
        self.run_pos = None;
        let (file, rev, ctx) = (self.file.clone(), self.revision.clone(), ctx.clone());
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
        std::thread::spawn(move || {
            let mut f = GrblFile::new();
            let res = match s {
                Sample::Svg => f.load_imported_svg(&format!("{root}/resources/generator/LaserGRBL-accuracy-test-file.svg"), false, lg_svg2gcode::ColorFilter::All),
                Sample::Arcs => f.load_file(&format!("{root}/tools/golden/harness/inputs/gcode_analyze/arcs_quadrants.nc"), false),
                Sample::Raster => f.load_file(&format!("{root}/tools/golden/harness/inputs/gcode_analyze/raster_like.nc"), false),
                Sample::Large => {
                    f = generate_raster(500_000);
                    Ok(())
                }
            };
            if let Err(e) = res {
                log::warn!("preview demo load: {e}");
            }
            *file.write().unwrap_or_else(|e| e.into_inner()) = f;
            rev.fetch_add(1, Ordering::SeqCst);
            ctx.request_repaint();
        });
    }

    /// Dev helper: with env `LASERGRBL_PREVIEW_SHOT=file.png` save a screenshot after ~2 s and exit.
    ///
    /// `ctx`: egui context.
    fn debug_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = std::env::var_os("LASERGRBL_PREVIEW_SHOT") else { return };
        self.frames += 1;
        let at = std::env::var("LASERGRBL_PREVIEW_SHOT_FRAME").ok().and_then(|v| v.parse().ok()).unwrap_or(150);
        if self.frames == at {
            let c2 = ctx.clone();
            ctx.request_screenshot(move |img| {
                let raw: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Some(buf) = image::RgbaImage::from_raw(img.width() as u32, img.height() as u32, raw) {
                    let _ = buf.save(&path);
                }
                c2.send_viewport_cmd(egui::ViewportCommand::Close);
            });
        }
    }

    /// Draw the tab.
    ///
    /// `ui`: target ui.
    pub fn ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if self.sample.is_none() {
            // dev: LASERGRBL_PREVIEW_SAMPLE=svg|arcs|raster|large, LASERGRBL_PREVIEW_PERF=1
            let first = match std::env::var("LASERGRBL_PREVIEW_SAMPLE").as_deref() {
                Ok("arcs") => Sample::Arcs,
                Ok("raster") => Sample::Raster,
                Ok("large") => Sample::Large,
                _ => Sample::Svg,
            };
            self.input.options.show_performance_diagnostic = std::env::var_os("LASERGRBL_PREVIEW_PERF").is_some();
            self.load(&ctx, first);
            self.input.file = Some(self.file.clone());
            self.input.jog_enabled = true;
            self.input.connected = true;
            self.input.machine_area = Some((300.0, 200.0));
        }
        self.input.file_revision = self.revision.load(Ordering::SeqCst);
        ui.horizontal_wrapped(|ui| {
            let cur = self.sample;
            for (s, n) in [(Sample::Svg, "SVG accuracy test"), (Sample::Arcs, "arcs_quadrants.nc"), (Sample::Raster, "raster_like.nc"), (Sample::Large, "500k raster")] {
                if ui.selectable_label(cur == Some(s), n).clicked() {
                    self.load(&ctx, s);
                }
            }
            ui.separator();
            let o = &mut self.input.options;
            ui.checkbox(&mut o.show_laser_off_movements, "laser off");
            ui.checkbox(&mut o.show_executed_commands, "executed");
            ui.checkbox(&mut o.show_bounding_box, "bbox");
            ui.checkbox(&mut o.cross_cursor, "cross cursor");
            ui.checkbox(&mut o.show_performance_diagnostic, "perf");
            ui.checkbox(&mut o.auto_size_on_drawing, "autosize drawing");
            ui.add(egui::Slider::new(&mut o.preview_line_size, 0.5..=6.0).show_value(false)).on_hover_text("line size");
            if ui.button("Zoom auto").clicked() {
                self.gl.auto_size_drawing();
            }
            if ui.button("Zoom +").clicked() {
                self.gl.zoom_in();
            }
            if ui.button("Zoom -").clicked() {
                self.gl.zoom_out();
            }
            if ui.button(if self.run_pos.is_some() { "Stop run" } else { "Fake run" }).clicked() {
                if self.run_pos.take().is_none() {
                    PreviewPanel::on_program_ended(&self.file);
                    self.run_pos = Some(0);
                }
            }
        });
        if !self.last_event.is_empty() {
            ui.label(&self.last_event);
        }

        // fake program run: mark commands executed so the invalidation path runs
        if let Some(pos) = self.run_pos {
            let f = self.file.read().unwrap_or_else(|e| e.into_inner());
            let n = f.commands().len();
            let step = (n / 400).max(20);
            for c in f.commands().iter().skip(pos).take(step) {
                c.set_result("OK", false);
            }
            drop(f);
            self.run_pos = if pos + step >= n { None } else { Some(pos + step) };
            ctx.request_repaint();
        }

        // fake cursor on a circle around the drawing center
        let t = self.started.elapsed().as_secs_f32();
        let (cx, cy, r) = {
            let g = self.file.read().unwrap_or_else(|e| e.into_inner());
            let dr = g.range().drawing_range;
            if dr.valid_range() {
                let c = dr.center();
                let rr = lg_dotnet::decimal_to_f32(dr.width().unwrap_or_default()).max(10.0) / 2.5;
                (c.0, c.1, rr)
            } else {
                (0.0, 0.0, 50.0)
            }
        };
        self.input.work_position = GPoint::new(cx + r * t.cos(), cy + r * t.sin(), 0.0);
        self.input.machine_position = self.input.work_position + GPoint::new(10.0, 5.0, 0.0);
        self.input.working_offset = GPoint::new(10.0, 5.0, 0.0);
        self.input.current_f = 1000.0;
        self.input.current_s = (500.0 + 500.0 * t.sin()).round();
        ctx.request_repaint();

        self.debug_screenshot(&ctx);
        let mut events = Vec::new();
        // two equal halves, each with a caption and the panel filling the rest
        // the toolbar above may overflow the width: bound by the visible area
        let mut total = ui.available_size();
        total.x = total.x.min(ui.ctx().content_rect().right() - ui.cursor().left());
        let half = ((total.x - ui.spacing().item_spacing.x) / 2.0).floor();
        ui.horizontal_top(|ui| {
            for (i, caption) in ["GL (GrblPanel3D)", "CPU (GrblPanel)"].into_iter().enumerate() {
                ui.allocate_ui_with_layout(egui::vec2(half, total.y), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(half);
                    ui.label(caption);
                    let ev = if i == 0 { self.gl.show(ui, &self.input) } else { self.cpu.show(ui, &self.input) };
                    events.extend(ev);
                });
            }
        });
        for e in events {
            if let PreviewEvent::JogToPosition { x, y, fast } = e {
                self.last_event = format!("jog to X{x:.2} Y{y:.2} fast={fast}");
            }
        }
    }
}
