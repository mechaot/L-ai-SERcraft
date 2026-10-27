// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Andreas Pösch

//! Test support (compiled for tests only): a headless egui driver around the job dock, shared by the tests of the
//! job dock and of the two file editors. Clicks, typing and waiting go through real egui input events and the real
//! job coordinator, so an edit reaches the G-code the same way as in the app.

use eframe::egui;
use lasergrbl_core::{CoreEvent, GrblCore};
use lg_gcode::Firmware;
use lg_services::layer_presets::PresetDb;
use std::ops::Range;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::forms::form_util::{replace_block_rel, GenRequest, LayerRanges, SharedFile};
use crate::forms::image_dock::ImageDock;
use crate::forms::job_dock::{JobAction, JobDock};
use crate::forms::params_dock::ParamsDock;

/// Collect text shapes recursively. `s`: shape. `out`: sink of (text, screen rect).
fn collect(s: &egui::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match s {
        egui::Shape::Text(t) => out.push((t.galley.text().to_owned(), t.galley.rect.translate(t.pos.to_vec2()))),
        egui::Shape::Vec(v) => v.iter().for_each(|c| collect(c, out)),
        _ => {}
    }
}

/// One file's block of a program, run by hand: what the job dock does for a single editor, without threads.
pub(crate) struct Rig {
    /// The program.
    pub file: SharedFile,
    /// Block of the editor under test.
    pub block: Range<usize>,
}

impl Rig {
    /// Program with the given head lines; the block starts behind them.
    ///
    /// `head`: command lines that stay in front of the block. Returns the rig with an empty block.
    pub fn new(head: &[&str]) -> Rig {
        let mut f = lg_gcode::GrblFile::new();
        for l in head {
            f.commands_mut().push(lg_gcode::GrblCommand::new(l));
        }
        let n = f.count();
        Rig { file: Arc::new(RwLock::new(f)), block: n..n }
    }

    /// Run a request on the block (replacing what is there). `req`: work order of an editor.
    /// Returns its `seq` and the layer ranges relative to the block (Err: the block is gone).
    pub fn run(&mut self, req: GenRequest) -> (u64, Result<LayerRanges, String>) {
        let mut g = self.file.write().unwrap();
        match replace_block_rel(&mut g, self.block.clone(), req.produce) {
            Ok((rel, new)) => {
                self.block = new;
                (req.seq, Ok(rel))
            }
            Err(e) => {
                let start = self.block.start.min(g.count());
                self.block = start..start;
                (req.seq, Err(e.to_string()))
            }
        }
    }

    /// Program lines. Returns the command texts.
    pub fn lines(&self) -> Vec<String> {
        self.file.read().unwrap().commands().iter().map(|c| c.command().to_owned()).collect()
    }

    /// Number of commands in the program.
    pub fn count(&self) -> usize {
        self.file.read().unwrap().count()
    }
}

/// Headless driver around one [`JobDock`] on a real core.
pub(crate) struct Harness {
    pub ctx: egui::Context,
    pub core: Arc<GrblCore>,
    pub job: JobDock,
    /// Text shapes of the last frame: (text, screen rect).
    texts: Vec<(String, egui::Rect)>,
    /// Core events (the preview bumps its rebuild counter on `FileLoaded`).
    rx: crossbeam_channel::Receiver<CoreEvent>,
    /// Actions the dock returned since the harness was built.
    pub actions: Vec<JobAction>,
}

impl Harness {
    /// Build core + dock on a throw-away preset file (never the real config).
    ///
    /// Returns the harness after one frame.
    pub fn new() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        let core = GrblCore::new(Firmware::Grbl, tx);
        // temp-file DB with only the example: never the real config file
        let f = lg_base::paths::test_dir("job_presets").join("LayerPresets.json");
        let db = Arc::new(Mutex::new(PresetDb::load_from(&f)));
        let job = JobDock::with_defaults(core.clone(), ParamsDock::with_presets(db));
        let mut h = Harness { ctx: egui::Context::default(), core, job, texts: Vec::new(), rx, actions: Vec::new() };
        h.frame(vec![]);
        h
    }

    /// Run one frame. `events`: input events.
    pub fn frame(&mut self, events: Vec<egui::Event>) {
        let ri = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 1500.0))), events, ..Default::default() };
        let job = &mut self.job;
        let mut acts = Vec::new();
        let mut out = self.ctx.run_ui(ri, |ui| {
            acts = job.show(ui);
        });
        self.actions.extend(acts);
        out.textures_delta.clear();
        self.texts.clear();
        for cs in &out.shapes {
            collect(&cs.shape, &mut self.texts);
        }
    }

    /// Click at `p` (press frame + release frame + settle frame). `p`: screen position.
    pub fn click_at(&mut self, p: egui::Pos2) {
        let btn = |pressed| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        self.frame(vec![egui::Event::PointerMoved(p), btn(true)]);
        self.frame(vec![btn(false)]);
        self.frame(vec![]);
    }

    /// Two quiet frames: grids settle their column widths before the next rect is read.
    pub fn stabilize(&mut self) {
        self.frame(vec![]);
        self.frame(vec![]);
    }

    /// Click the last drawn text equal to `t` (popup items are drawn last). Panics when absent.
    pub fn click_text(&mut self, t: &str) {
        self.stabilize();
        let r = self.texts.iter().rev().find(|(s, _)| s == t).map(|(_, r)| *r).unwrap_or_else(|| panic!("text {t:?} not on screen: {:?}", self.texts.iter().map(|x| &x.0).collect::<Vec<_>>()));
        self.click_at(r.center());
    }

    /// Click the first drawn text equal to `t` (the file list sits above the editor). Panics when absent.
    pub fn click_first_text(&mut self, t: &str) {
        self.stabilize();
        let r = self.texts.iter().find(|(s, _)| s == t).map(|(_, r)| *r).unwrap_or_else(|| panic!("text {t:?} not on screen: {:?}", self.texts.iter().map(|x| &x.0).collect::<Vec<_>>()));
        self.click_at(r.center());
    }

    /// Pick `item` in the combo currently showing `current`.
    pub fn pick(&mut self, current: &str, item: &str) {
        self.click_text(current);
        self.click_text(item);
    }

    /// Type `v` into the numeric field `id` and press Enter.
    pub fn type_into(&mut self, id: &str, v: &str) {
        self.stabilize();
        let r = self.ctx.read_response(egui::Id::unique(id)).unwrap_or_else(|| panic!("field {id} not shown")).rect;
        self.click_at(r.center());
        let all = egui::Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND };
        self.frame(vec![all, egui::Event::Text(v.into())]);
        let enter = |p| egui::Event::Key { key: egui::Key::Enter, physical_key: None, pressed: p, repeat: false, modifiers: egui::Modifiers::NONE };
        self.frame(vec![enter(true), enter(false)]);
        self.frame(vec![]);
    }

    /// True when a text is on screen. `t`: text (compared trimmed).
    pub fn shows(&self, t: &str) -> bool {
        self.texts.iter().any(|(s, _)| s.trim() == t.trim())
    }

    /// Returns how often a text is on screen. `t`: text (compared trimmed).
    pub fn count(&self, t: &str) -> usize {
        self.texts.iter().filter(|(s, _)| s.trim() == t.trim()).count()
    }

    /// Click the aspect lock of the image editor (right of the width input, one grid gap away).
    pub fn click_lock(&mut self) {
        self.stabilize();
        let w = self.ctx.read_response(egui::Id::unique("image-w")).expect("width input not shown").rect;
        self.click_at(egui::pos2(w.max.x + 6.0 + 8.5, w.center().y));
    }

    /// Poll until a worker finished (10 s limit), frames keep going. Returns whether it succeeded.
    pub fn wait(&mut self) -> bool {
        let t = Instant::now();
        loop {
            if let Some(ok) = self.job.poll() {
                self.frame(vec![]);
                return ok;
            }
            assert!(t.elapsed() < Duration::from_secs(10), "generation timeout");
            std::thread::sleep(Duration::from_millis(5));
            self.frame(vec![]);
        }
    }

    /// Wait until every file is generated: no worker runs and nothing waits. Panics on a failed generation.
    pub fn wait_idle(&mut self) {
        let t = Instant::now();
        while self.job.job_running() || self.job.pending_auto() || self.job.is_dirty_idle() {
            if let Some(ok) = self.job.poll() {
                assert!(ok, "generation failed");
            }
            assert!(t.elapsed() < Duration::from_secs(10), "generation timeout");
            std::thread::sleep(Duration::from_millis(5));
            self.frame(vec![]);
        }
        self.frame(vec![]);
    }

    /// Wait until auto regen caught up with all edits. Panics when no regen was triggered or one failed.
    pub fn settle(&mut self) {
        assert!(self.job.needs_apply(), "edit did not trigger regen");
        self.wait_idle();
        assert!(!self.job.needs_apply(), "something is still dirty");
    }

    /// Number of `FileLoaded` events since the last call. Each one makes `PreviewForm` rebuild its scene.
    pub fn file_loaded_events(&self) -> usize {
        self.rx.try_iter().filter(|e| matches!(e, CoreEvent::FileLoaded { .. })).count()
    }

    /// Whole program text lines.
    pub fn all_lines(&self) -> Vec<String> {
        let f = self.core.loaded_file();
        let g = f.read().unwrap();
        g.commands().iter().map(|c| c.command().to_owned()).collect()
    }

    /// Program text lines of a command range. `r`: command range.
    pub fn lines_of(&self, r: std::ops::Range<usize>) -> Vec<String> {
        self.all_lines()[r].to_vec()
    }

    /// Screen rectangle of the remove button of list row `idx` (last frame). `idx`: row.
    pub fn row_remove_rect(&self, idx: usize) -> egui::Rect {
        self.job.test_remove_rect(idx)
    }

    /// Program lines of layer `i` of the selected SVG (None = layer not generated).
    ///
    /// `i`: layer index.
    pub fn layer_lines(&self, i: usize) -> Option<Vec<String>> {
        let r = self.svg().generated_ranges().get(i)?.clone()?;
        let base = self.job.selected_svg_block()?.start;
        Some(self.lines_of(base + r.start as usize..base + r.end as usize))
    }

    /// The SVG editor of the selected file (else the first SVG). Panics without an SVG.
    pub fn svg(&self) -> &ParamsDock {
        self.job.selected_svg().expect("no SVG editor")
    }

    /// The image editor of the selected file (else the first image). Panics without an image.
    pub fn image(&self) -> &ImageDock {
        self.job.selected_image().expect("no image editor")
    }
}
