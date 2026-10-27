// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Andreas Pösch

//! Job dock (right side of the main window): the loaded program as a list of per-file blocks and the editor of the
//! selected file. Several files can be loaded ("Append file"); each SVG or raster image keeps its own parameter
//! editor ([`ParamsDock`], [`ImageDock`]) and owns one contiguous block of the shared program. An edit regenerates
//! only that file's block ([`GrblFile::replace_block`]), the blocks before and after stay as they are. G-code files and
//! classic-dialog results are `Static` items (no parameters). One job at a time runs on the shared program.
//! Not part of the C# app (intentional deviation).

use eframe::egui::{self, RichText, Ui, Vec2};
use lasergrbl_core::GrblCore;
use lg_gcode::GrblFile;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::forms::form_util::*;
use crate::forms::image_dock::{ImageDock, ImageDockAction};
use crate::forms::params_dock::{ParamsAction, ParamsDock};
use crate::ui::preview::LayerView;
use crate::ui::widgets::{grbl_group_box_wide, ImageButton};
use crate::ui::{tr_fmt, tr_plain};

/// Rows of the file list shown before it scrolls.
const MAX_ROWS: usize = 6;

/// Width of the type tag column in px.
const TAG_W: f32 = 40.0;

/// Width of the state column (generating / error) in px.
const STATE_W: f32 = 14.0;

/// What the dock asks the main window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobAction {
    /// Open the layer preset list editor.
    OpenPresetEditor,
    /// Open the image of item `id` in the classic raster import dialog (see [`JobDock::begin_classic`]).
    OpenClassic { id: u64 },
}

/// What a file of the job is.
enum JobKind {
    /// SVG with its parameter editor.
    Svg(Box<ParamsDock>),
    /// Raster image with its parameter editor.
    Image(Box<ImageDock>),
    /// G-code that cannot be edited (G-code file, classic dialog result, unknown program).
    Static,
}

/// One file of the job, owner of one block of the program.
pub struct JobItem {
    /// Stable id (list positions change).
    id: u64,
    /// Name shown in the list.
    name: String,
    /// Source file, if any.
    path: Option<PathBuf>,
    kind: JobKind,
    /// Command range of this file in the shared program. Blocks of all items are contiguous and cover the program.
    block: Range<usize>,
}

impl JobItem {
    /// Returns true when the G-code of this file is out of date or being generated.
    fn needs_apply(&self) -> bool {
        match &self.kind {
            JobKind::Svg(d) => d.needs_apply(),
            JobKind::Image(d) => d.needs_apply(),
            JobKind::Static => false,
        }
    }

    /// Returns true when edits wait and a request could start now.
    fn is_dirty_idle(&self) -> bool {
        match &self.kind {
            JobKind::Svg(d) => d.is_dirty_idle(),
            JobKind::Image(d) => d.is_dirty_idle(),
            JobKind::Static => false,
        }
    }

    /// Returns true when a request should start by itself (see `ParamsDock::wants_auto`).
    fn wants_auto(&self) -> bool {
        match &self.kind {
            JobKind::Svg(d) => d.wants_auto(),
            JobKind::Image(d) => d.wants_auto(),
            JobKind::Static => false,
        }
    }

    /// Returns true when the file is stale and cannot be regenerated (invalid values).
    fn is_blocked(&self) -> bool {
        match &self.kind {
            JobKind::Svg(d) => d.is_blocked(),
            JobKind::Image(d) => d.is_blocked(),
            JobKind::Static => false,
        }
    }

    /// Returns true while a request of this file runs.
    fn in_flight(&self) -> bool {
        match &self.kind {
            JobKind::Svg(d) => d.in_flight(),
            JobKind::Image(d) => d.in_flight(),
            JobKind::Static => false,
        }
    }

    /// Returns the problem text of this file, None when fine.
    fn problem(&self) -> Option<String> {
        match &self.kind {
            JobKind::Svg(d) => d.problem(),
            JobKind::Image(d) => d.problem(),
            JobKind::Static => None,
        }
    }

    /// Returns the editor's change counter for the preview overlay (0 without overlay data).
    fn revision(&self) -> u64 {
        match &self.kind {
            JobKind::Svg(d) => d.revision(),
            _ => 0,
        }
    }

    /// Ask the editor for its work order. Returns None when it has nothing to do.
    fn take_request(&mut self) -> Option<GenRequest> {
        match &mut self.kind {
            JobKind::Svg(d) => d.take_request(),
            JobKind::Image(d) => d.take_request(),
            JobKind::Static => None,
        }
    }

    /// Give a taken request back unrun. `seq`: the request's `seq`.
    fn cancel_request(&mut self, seq: u64) {
        match &mut self.kind {
            JobKind::Svg(d) => d.cancel_request(seq),
            JobKind::Image(d) => d.cancel_request(seq),
            JobKind::Static => {}
        }
    }

    /// Hand the result of a request to the editor.
    ///
    /// `seq`: the request's `seq`. `result`: layer ranges relative to the block start, or the error text.
    fn complete(&mut self, seq: u64, result: Result<LayerRanges, String>) {
        match &mut self.kind {
            JobKind::Svg(d) => d.complete(seq, result),
            JobKind::Image(d) => d.complete(seq, result.map(|_| ())),
            JobKind::Static => {}
        }
    }

    /// Returns the short type tag of the list row.
    fn tag(&self) -> String {
        match &self.kind {
            JobKind::Svg(_) => tr_plain("JobDock.TagSvg"),
            JobKind::Image(_) => tr_plain("JobDock.TagImage"),
            JobKind::Static => tr_plain("JobDock.TagGCode"),
        }
    }
}

/// What a finished worker leaves behind.
struct Done {
    /// Ok: layer ranges relative to the new block and the new block. Err: the (empty) range the block shrank to.
    result: Result<(LayerRanges, Range<usize>), Range<usize>>,
    /// `GrblFile::load_serial` after the job.
    serial: u64,
}

/// What the running worker does.
#[derive(Clone, Copy)]
enum RunKind {
    /// Regenerate the block of item `item` for the editor request `seq`.
    Generate { item: u64, seq: u64 },
    /// Cut the block of item `item` out of the program.
    Remove { item: u64 },
}

/// The one worker on the shared program.
struct RunningJob {
    kind: RunKind,
    handle: JoinHandle<()>,
    /// Written by the worker just before it ends.
    slot: Arc<Mutex<Option<Done>>>,
}

/// Right dock: file list, editor of the selected file and the job coordinator.
pub struct JobDock {
    core: Arc<GrblCore>,
    /// "(defaults)" editor: shown without a file, seeds the next SVG.
    defaults: ParamsDock,
    /// Files in program order.
    items: Vec<JobItem>,
    /// Id of the edited file; always valid while `items` is not empty.
    selected: Option<u64>,
    next_id: u64,
    running: Option<RunningJob>,
    /// Name of an appended G-code file the core loads on its own thread.
    pending_static: Option<String>,
    /// `GrblFile::load_serial` the blocks match; a different value means somebody else rewrote the program.
    seen_serial: u64,
    /// A modal form is open: finish a running job but start nothing and leave the program alone.
    paused: bool,
    /// Bumped on every change the preview overlay depends on.
    revision: u64,
    /// Sum of the editors' revisions at the last check (detects their changes).
    item_rev_sum: u64,
    /// Test only: remove-button rectangle of every list row of the last frame (item id, rect).
    #[cfg(test)]
    remove_rects: std::cell::RefCell<Vec<(u64, egui::Rect)>>,
}

impl JobDock {
    /// Build the dock with the process wide preset database.
    ///
    /// `core`: engine; its loaded file is the program the blocks live in. Returns a dock without files.
    pub fn new(core: Arc<GrblCore>) -> Self {
        Self::with_defaults(core, ParamsDock::new())
    }

    /// Build the dock around a ready defaults editor (tests use an in-memory preset database).
    ///
    /// `core`: engine. `defaults`: editor for the no-file state. Returns a dock without files.
    pub fn with_defaults(core: Arc<GrblCore>, defaults: ParamsDock) -> Self {
        let seen_serial = core.loaded_file().read().map(|f| f.load_serial()).unwrap_or(0);
        JobDock { core, defaults, items: Vec::new(), selected: None, next_id: 1, running: None, pending_static: None, seen_serial, paused: false, revision: 1, item_rev_sum: 0, #[cfg(test)] remove_rects: Default::default() }
    }

    // ------------------------------------------------------------------ items

    /// Returns the number of files.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true without files.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the number of commands the blocks cover (end of the last block).
    pub fn covered_len(&self) -> usize {
        self.items.last().map_or(0, |i| i.block.end)
    }

    /// Returns the index of the item with `id`. `id`: item id. Returns None when it is gone.
    fn index_of(&self, id: u64) -> Option<usize> {
        self.items.iter().position(|i| i.id == id)
    }

    /// Returns the index of the edited item, None without files.
    fn selected_index(&self) -> Option<usize> {
        self.selected.and_then(|id| self.index_of(id))
    }

    /// Returns the SVG editor whose values seed the next SVG: the selected SVG, else the defaults.
    fn seed_dock(&self) -> &ParamsDock {
        match self.selected_index().map(|i| &self.items[i].kind) {
            Some(JobKind::Svg(d)) => d,
            _ => &self.defaults,
        }
    }

    /// Length of the shared program in commands. Returns the count (blocks while the file is locked elsewhere).
    fn file_len(&self) -> usize {
        self.core.loaded_file().read().map_or_else(|e| e.into_inner().count(), |f| f.count())
    }

    /// Length and load serial of the shared program without waiting. Returns None while a worker holds the file.
    fn snapshot(&self) -> Option<(usize, u64)> {
        let file = self.core.loaded_file();
        let g = file.try_read().ok()?;
        Some((g.count(), g.load_serial()))
    }

    /// Remember the program's current load serial as "ours" (the blocks match it).
    fn mark_synced(&mut self) {
        if let Some((_, serial)) = self.snapshot() {
            self.seen_serial = self.seen_serial.max(serial);
        }
    }

    /// Commands no item covers yet: empty at the end when the blocks cover the program, the whole old program
    /// after [`JobDock::reset`] (the first generation of the next item replaces it).
    fn uncovered(&self) -> Range<usize> {
        let end = self.covered_len();
        end..self.file_len().max(end)
    }

    /// Append an item.
    ///
    /// `name`: list text. `path`: source file. `kind`: editor. `block`: its commands; must start at the end of
    /// the last block. Returns the new item's id (it is selected now).
    fn push_item(&mut self, name: String, path: Option<PathBuf>, kind: JobKind, block: Range<usize>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(JobItem { id, name, path, kind, block });
        self.selected = Some(id);
        self.mark_synced();
        self.revision += 1;
        id
    }

    /// Drop all files (a plain open replaces the program). The edited values of the selected SVG become the
    /// defaults, like the old dock kept them. The next added item replaces the old program.
    pub fn reset(&mut self) {
        if let Some(i) = self.selected_index()
            && let JobKind::Svg(d) = &self.items[i].kind
        {
            self.defaults.adopt_defaults(d);
        }
        self.items.clear();
        self.selected = None;
        self.pending_static = None;
        self.revision += 1;
    }

    /// Add an SVG; it is generated by the scheduler as soon as the program is free.
    ///
    /// `path`: SVG file. `from_project`: the settings were just restored from a `.lps` entry (values come from them,
    /// not from the per-file store). `append`: keep the files already loaded; otherwise they are dropped and the
    /// program is replaced. Returns the item id (selected).
    pub fn add_svg(&mut self, path: &Path, from_project: bool, append: bool) -> u64 {
        if append {
            self.sync_before_add();
        }
        let dock = ParamsDock::for_svg(self.seed_dock(), &path.to_string_lossy(), from_project);
        if !append {
            self.reset();
        }
        let block = self.uncovered();
        self.push_item(file_name_of(path), Some(path.to_path_buf()), JobKind::Svg(Box::new(dock)), block)
    }

    /// Add a raster image; it is generated by the scheduler as soon as the program is free.
    ///
    /// `path`: image file. `append`: keep the files already loaded; otherwise they are dropped and the program is
    /// replaced (not before the image was read: an unreadable image changes nothing).
    /// Returns the item id (selected), or the read / decode error text.
    pub fn add_image(&mut self, path: &Path, append: bool) -> Result<u64, String> {
        let mut dock = ImageDock::new();
        dock.load_image(path)?;
        if append {
            self.sync_before_add();
        } else {
            self.reset();
        }
        let block = self.uncovered();
        Ok(self.push_item(file_name_of(path), Some(path.to_path_buf()), JobKind::Image(Box::new(dock)), block))
    }

    /// Add G-code that is already in the program as a file that cannot be edited.
    ///
    /// `name`: list text. `block`: its commands; the start is moved to the end of the last block, so blocks
    /// stay contiguous. Returns the item id (selected).
    pub fn add_static(&mut self, name: &str, block: Range<usize>) -> u64 {
        let end = self.covered_len();
        self.push_item(name.to_owned(), None, JobKind::Static, end..block.end.max(end))
    }

    /// Add every command that no item covers yet as one static file (result of the classic dialog).
    ///
    /// `name`: list text. Returns the item id (selected).
    pub fn add_static_tail(&mut self, name: &str) -> u64 {
        let block = self.uncovered();
        self.add_static(name, block)
    }

    /// The core is loading an appended (or, after [`JobDock::reset`], a plain) G-code file on its own thread: when
    /// the load is done, the new commands become a static item with this name.
    ///
    /// `name`: list text.
    pub fn expect_static(&mut self, name: &str) {
        self.pending_static = Some(name.to_owned());
    }

    /// Before an append: make sure the blocks describe the program (somebody else may have changed it).
    fn sync_before_add(&mut self) {
        if !self.items.is_empty() {
            self.reconcile();
        }
    }

    /// Cut a file's block out of the program and drop the item.
    ///
    /// `id`: item id. Returns false when nothing was started (unknown id, a job runs, the program is busy).
    /// An item without commands is dropped at once; otherwise the item goes when the worker is done.
    pub fn remove(&mut self, id: u64) -> bool {
        if self.running.is_some() {
            return false;
        }
        let Some(idx) = self.index_of(id) else { return false };
        let block = self.items[idx].block.clone();
        if block.is_empty() {
            self.drop_item(idx);
            return true;
        }
        if self.paused || !self.core.can_load_new_file() {
            return false;
        }
        // the preview follows under the name of the file that stays selected
        let neighbour = if idx > 0 { idx - 1 } else { idx + 1 };
        let shown = self.items.get(neighbour).map(|i| i.path.as_ref().map_or_else(|| i.name.clone(), |p| p.to_string_lossy().into_owned())).unwrap_or_default();
        let slot: Arc<Mutex<Option<Done>>> = Arc::new(Mutex::new(None));
        let slot_w = slot.clone();
        let handle = GrblFile::spawn_load(&self.core.loaded_file(), move |f| {
            let start = block.start.min(f.count());
            let r = f.remove_block(block);
            f.raise_file_loaded(&shown);
            let done = match &r {
                Ok(()) => Done { result: Ok((Vec::new(), start..start)), serial: f.load_serial() },
                Err(_) => Done { result: Err(start..start), serial: f.load_serial() },
            };
            *slot_w.lock().unwrap_or_else(|e| e.into_inner()) = Some(done);
            r
        });
        match handle {
            Some(handle) => {
                self.running = Some(RunningJob { kind: RunKind::Remove { item: id }, handle, slot });
                true
            }
            None => false,
        }
    }

    /// Start taking an image out of the job for the classic import dialog.
    ///
    /// `id`: item id of an image. Returns `(image path, append)` when the block removal was started (or not needed):
    /// open the dialog with that `append` once [`JobDock::job_running`] is false. None for other items or when
    /// nothing could be started.
    pub fn begin_classic(&mut self, id: u64) -> Option<(PathBuf, bool)> {
        let idx = self.index_of(id)?;
        let path = match (&self.items[idx].kind, &self.items[idx].path) {
            (JobKind::Image(_), Some(p)) => p.clone(),
            _ => return None,
        };
        let append = self.items.len() > 1;
        self.remove(id).then_some((path, append))
    }

    /// Drop item `idx` after its block left the program: later blocks move up, the selection moves to a neighbour.
    ///
    /// `idx`: list position.
    fn drop_item(&mut self, idx: usize) {
        let gone = self.items.remove(idx);
        self.shift_from(idx, -(gone.block.len() as isize));
        // the edited values stay available for the next SVG
        if self.selected == Some(gone.id) {
            if let JobKind::Svg(d) = &gone.kind {
                self.defaults.adopt_defaults(d);
            }
            let next = if idx > 0 { idx - 1 } else { 0 };
            self.selected = self.items.get(next).map(|i| i.id);
        }
        self.revision += 1;
    }

    /// Move the blocks of the items from list position `from` on.
    ///
    /// `from`: first item to move. `delta`: commands added (negative: removed) in front of it.
    fn shift_from(&mut self, from: usize, delta: isize) {
        if delta == 0 {
            return;
        }
        for it in self.items.iter_mut().skip(from) {
            it.block = it.block.start.saturating_add_signed(delta)..it.block.end.saturating_add_signed(delta);
        }
    }

    // ------------------------------------------------------------------ scheduler

    /// A modal form is open: running work is collected, nothing new starts and the program is not inspected.
    ///
    /// `paused`: true while a modal form is open.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Returns true while the worker runs.
    pub fn job_running(&self) -> bool {
        self.running.is_some()
    }

    /// Returns true when some file is waiting for its automatic regeneration.
    pub fn pending_auto(&self) -> bool {
        self.items.iter().any(|i| i.wants_auto())
    }

    /// Returns true when some file's G-code is out of date or being generated.
    pub fn needs_apply(&self) -> bool {
        self.running.is_some() || self.items.iter().any(|i| i.needs_apply())
    }

    /// Returns true when edits wait and no worker runs (an [`JobDock::apply`] call would start one).
    pub fn is_dirty_idle(&self) -> bool {
        self.running.is_none() && self.items.iter().any(|i| i.is_dirty_idle())
    }

    /// Returns true when the program is stale and cannot be regenerated because a file's values are invalid.
    pub fn is_blocked(&self) -> bool {
        self.running.is_none() && self.items.iter().any(|i| i.is_blocked())
    }

    /// Start the next regeneration now, also for a file whose automatic attempt failed (Run retries).
    ///
    /// Returns false when nothing was started (a worker runs, the program is busy or paused, nothing is dirty).
    pub fn apply(&mut self) -> bool {
        self.reconcile();
        self.schedule(false)
    }

    /// Check the worker, repair the blocks after outside changes, start the next automatic regeneration.
    /// Call every frame.
    ///
    /// Returns `Some(success)` once when a worker finished (generation or removal).
    pub fn poll(&mut self) -> Option<bool> {
        let done = self.collect();
        self.reconcile();
        self.schedule(true);
        self.sync_revision();
        done
    }

    /// Take the result of a finished worker.
    ///
    /// Returns None while it runs, else whether it succeeded.
    fn collect(&mut self) -> Option<bool> {
        if !self.running.as_ref().is_some_and(|j| j.handle.is_finished()) {
            return None;
        }
        let job = self.running.take()?;
        let _ = job.handle.join();
        let done = job.slot.lock().unwrap_or_else(|e| e.into_inner()).take();
        // a worker that panicked left no result: its block is gone
        let (item, serial) = match job.kind {
            RunKind::Generate { item, .. } | RunKind::Remove { item } => (item, done.as_ref().map(|d| d.serial)),
        };
        let Some(idx) = self.index_of(item) else {
            // the list was reset meanwhile: the result belongs to a forgotten file
            return Some(done.is_some_and(|d| d.result.is_ok()));
        };
        let old = self.items[idx].block.clone();
        let result = done.map_or_else(|| Err(old.start..old.start), |d| d.result);
        let ok = result.is_ok();
        match job.kind {
            RunKind::Generate { seq, .. } => {
                let (block, outcome) = match result {
                    Ok((ranges, new)) => (new, Ok(ranges)),
                    Err(empty) => (empty, Err(tr_plain("ParamsDock.ApplyFailed"))),
                };
                self.shift_from(idx + 1, block.len() as isize - old.len() as isize);
                self.items[idx].block = block;
                self.items[idx].complete(seq, outcome);
            }
            RunKind::Remove { .. } => {
                if ok {
                    self.drop_item(idx);
                }
            }
        }
        if let Some(s) = serial {
            self.seen_serial = self.seen_serial.max(s);
        }
        self.revision += 1;
        Some(ok)
    }

    /// Repair the blocks when the program was changed outside this dock (generator form, anything else): the
    /// files are replaced by one static item that covers the whole program. An announced G-code load
    /// ([`JobDock::expect_static`]) that is done becomes a static item for its new commands.
    ///
    /// Does nothing while a worker runs, the dock is paused or the program is busy.
    fn reconcile(&mut self) {
        if self.running.is_some() || self.paused || !self.core.can_load_new_file() {
            return;
        }
        let Some((len, serial)) = self.snapshot() else { return };
        let end = self.covered_len();
        if let Some(name) = self.pending_static.take()
            && len >= end
        {
            if len > end {
                self.add_static(&name, end..len);
            }
            self.seen_serial = self.seen_serial.max(serial);
            return;
        }
        if len == end && serial == self.seen_serial {
            return;
        }
        lg_base::logger::log_message("JobDock", &format!("program changed outside the job list ({end} -> {len} commands): files replaced by one static block"));
        self.items.clear();
        self.selected = None;
        if len > 0 {
            self.add_static(&tr_plain("JobDock.Program"), 0..len);
        }
        self.seen_serial = self.seen_serial.max(serial);
        self.revision += 1;
    }

    /// Start the next generation: the edited file first, then program order.
    ///
    /// `auto`: only files that want an automatic attempt (an edit not tried yet); false also retries failed ones.
    /// Returns true when a worker was started.
    fn schedule(&mut self, auto: bool) -> bool {
        if self.running.is_some() || self.paused || self.pending_static.is_some() || !self.core.can_load_new_file() {
            return false;
        }
        let mut order: Vec<usize> = (0..self.items.len()).collect();
        if let Some(s) = self.selected_index() {
            order.retain(|i| *i != s);
            order.insert(0, s);
        }
        for idx in order {
            let ready = if auto { self.items[idx].wants_auto() } else { self.items[idx].is_dirty_idle() };
            if ready {
                return self.start_generate(idx);
            }
        }
        false
    }

    /// Run the request of item `idx` on the shared program.
    ///
    /// `idx`: list position. Returns true when the worker started; false hands the request back.
    fn start_generate(&mut self, idx: usize) -> bool {
        let Some(req) = self.items[idx].take_request() else { return false };
        let (id, block, seq) = (self.items[idx].id, self.items[idx].block.clone(), req.seq);
        let slot: Arc<Mutex<Option<Done>>> = Arc::new(Mutex::new(None));
        let slot_w = slot.clone();
        let produce = req.produce;
        let handle = GrblFile::spawn_load(&self.core.loaded_file(), move |f| {
            let start = block.start.min(f.count());
            let r = replace_block_rel(f, block, produce);
            let serial = f.load_serial();
            let (done, out) = match r {
                Ok(v) => (Done { result: Ok(v), serial }, Ok(())),
                // the block is gone: it shrank to nothing at its start
                Err(e) => (Done { result: Err(start..start), serial }, Err(e)),
            };
            *slot_w.lock().unwrap_or_else(|e| e.into_inner()) = Some(done);
            out
        });
        match handle {
            Some(handle) => {
                self.running = Some(RunningJob { kind: RunKind::Generate { item: id, seq }, handle, slot });
                true
            }
            None => {
                self.items[idx].cancel_request(seq);
                false
            }
        }
    }

    /// Bump the overlay revision when an editor changed one of its overlay inputs (layer pick, enable toggle).
    fn sync_revision(&mut self) {
        let sum: u64 = self.items.iter().map(|i| i.revision()).sum();
        if sum != self.item_rev_sum {
            self.item_rev_sum = sum;
            self.revision += 1;
        }
    }

    // ------------------------------------------------------------------ preview

    /// Overlay for the preview: layers hidden by the user (all SVG files) and what to highlight.
    ///
    /// Returns None without files. Highlight: the selected layer of the selected SVG when it has several
    /// layers; otherwise, with several files, the whole block of the selected file.
    pub fn layer_view(&self) -> Option<LayerView> {
        if self.items.is_empty() {
            return None;
        }
        let abs = |base: usize, r: &Range<u32>| (base as u32 + r.start)..(base as u32 + r.end);
        let mut hidden = Vec::new();
        for it in &self.items {
            if let JobKind::Svg(d) = &it.kind
                && let Some(v) = d.view()
            {
                hidden.extend(v.hidden.iter().map(|r| abs(it.block.start, r)));
            }
        }
        let highlight = self.selected_index().and_then(|i| {
            let it = &self.items[i];
            match &it.kind {
                JobKind::Svg(d) if d.layer_count() > 1 => d.view().and_then(|v| v.highlight).map(|r| abs(it.block.start, &r)),
                _ if self.items.len() > 1 && !it.block.is_empty() => Some(it.block.start as u32..it.block.end as u32),
                _ => None,
            }
        });
        Some(LayerView { revision: self.revision, highlight, hidden })
    }

    /// Rebuild the preset dropdowns (after the preset editor closed).
    pub fn refresh_presets(&mut self) {
        self.defaults.refresh_presets();
        for it in &mut self.items {
            if let JobKind::Svg(d) = &mut it.kind {
                d.refresh_presets();
            }
        }
    }

    /// The SVG editor the dev hooks act on: the selected SVG, else the first one.
    fn dev_svg(&mut self) -> Option<&mut ParamsDock> {
        let sel = self.selected_index().filter(|i| matches!(self.items[*i].kind, JobKind::Svg(_)));
        let idx = sel.or_else(|| self.items.iter().position(|i| matches!(i.kind, JobKind::Svg(_))))?;
        match &mut self.items[idx].kind {
            JobKind::Svg(d) => Some(d),
            _ => None,
        }
    }

    /// Dev hook (`LASERGRBL_DEV_SEQ` `layer=N` / `layeroff=N`): select layer `i` of the SVG, or disable it.
    ///
    /// `i`: layer index. `select`: true = select it, false = disable it (selection unchanged).
    pub fn dev_select_layer(&mut self, i: usize, select: bool) {
        if let Some(d) = self.dev_svg() {
            d.dev_select_layer(i, select);
        }
        self.revision += 1;
    }

    /// Dev hook (`LASERGRBL_DEV_SEQ` `fill=N` / `nofill=N` / `cross=N` / `nocross=N` / `pershape=N` / `nopershape=N`).
    ///
    /// `i`: layer index. `fill`/`cross`/`per_shape`: new flags (None = keep).
    pub fn dev_set_fill(&mut self, i: usize, fill: Option<bool>, cross: Option<bool>, per_shape: Option<bool>) {
        if let Some(d) = self.dev_svg() {
            d.dev_set_fill(i, fill, cross, per_shape);
        }
        self.revision += 1;
    }

    // ------------------------------------------------------------------ UI

    /// Draw the message boxes of all editors; call every frame.
    ///
    /// `ctx`: egui context.
    pub fn show_dialogs(&mut self, ctx: &egui::Context) {
        self.defaults.show_dialogs(ctx);
        for it in &mut self.items {
            match &mut it.kind {
                JobKind::Svg(d) => d.show_dialogs(ctx),
                JobKind::Image(d) => d.show_dialogs(ctx),
                JobKind::Static => {}
            }
        }
    }

    /// Dock body: title, file list, editor of the selected file. Starts the regeneration of edits at once.
    ///
    /// `ui`: panel ui. Returns requests for the main window.
    pub fn show(&mut self, ui: &mut Ui) -> Vec<JobAction> {
        let mut actions = Vec::new();
        ui.label(RichText::new(tr_plain("ParamsDock.Title")).strong());
        ui.add_space(2.0);
        let (mut select, mut remove) = (None, None);
        if !self.items.is_empty() {
            (select, remove) = self.files_group(ui);
            ui.add_space(4.0);
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.editor(ui, &mut actions));
        // a click on a row takes effect after the old editor had its frame (a text edit commits into the right file)
        if let Some(id) = select.filter(|id| self.index_of(*id).is_some()) {
            self.selected = Some(id);
            self.revision += 1;
        }
        if let Some(id) = remove {
            self.remove(id);
        }
        // edits of this frame start their generation now
        self.schedule(true);
        self.sync_revision();
        actions
    }

    /// Editor of the selected file (defaults without files).
    ///
    /// `ui`: parent. `actions`: collects requests for the main window.
    fn editor(&mut self, ui: &mut Ui, actions: &mut Vec<JobAction>) {
        let Some(idx) = self.selected_index() else {
            for a in self.defaults.show(ui) {
                match a {
                    ParamsAction::OpenPresetEditor => actions.push(JobAction::OpenPresetEditor),
                }
            }
            return;
        };
        let it = &mut self.items[idx];
        let id = it.id;
        ui.push_id(id, |ui| match &mut it.kind {
            JobKind::Svg(d) => {
                for a in d.show(ui) {
                    match a {
                        ParamsAction::OpenPresetEditor => actions.push(JobAction::OpenPresetEditor),
                    }
                }
            }
            JobKind::Image(d) => {
                for a in d.show(ui) {
                    match a {
                        ImageDockAction::OpenClassic => actions.push(JobAction::OpenClassic { id }),
                    }
                }
            }
            JobKind::Static => {
                ui.label(tr_plain("JobDock.StaticNote"));
                ui.label(RichText::new(tr_fmt("JobDock.Lines", &[&it.block.len()])).small());
            }
        });
    }

    /// The "Files" group: one row per file.
    ///
    /// `ui`: parent. Returns the id of a clicked row and the id of a row whose remove button was clicked.
    fn files_group(&self, ui: &mut Ui) -> (Option<u64>, Option<u64>) {
        let (mut select, mut remove) = (None, None);
        let selected = self.selected;
        let can_remove = self.running.is_none();
        let items = &self.items;
        #[cfg(test)]
        self.remove_rects.borrow_mut().clear();
        grbl_group_box_wide(ui, &tr_plain("JobDock.Files"), |ui| {
            let row_h = ui.spacing().interact_size.y;
            let max_h = (row_h + ui.spacing().item_spacing.y) * MAX_ROWS as f32;
            egui::ScrollArea::vertical().id_salt("job-files").max_height(max_h).auto_shrink([false, true]).show(ui, |ui| {
                for it in items {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.add_sized([TAG_W, row_h], egui::Label::new(RichText::new(it.tag()).small().weak()));
                        let w = (ui.available_width() - STATE_W - 17.0 - 2.0 * 4.0).max(40.0);
                        let r = ui.add_sized([w, row_h], egui::Button::selectable(selected == Some(it.id), (RichText::new(&it.name), egui::Atom::grow())).truncate());
                        let tip = it.path.as_ref().map_or_else(|| it.name.clone(), |p| p.to_string_lossy().into_owned());
                        if r.on_hover_text(tip).clicked() {
                            select = Some(it.id);
                        }
                        // state: error mark (tooltip = text), dots while generating
                        if let Some(p) = it.problem() {
                            ui.add_sized([STATE_W, row_h], egui::Label::new(RichText::new("!").strong().color(ui.visuals().error_fg_color))).on_hover_text(p);
                        } else if it.in_flight() {
                            ui.add_sized([STATE_W, row_h], egui::Label::new("…"));
                        } else {
                            ui.add_space(STATE_W);
                        }
                        let b = ui.add_enabled(can_remove, ImageButton::new("mdi-close").size(Vec2::splat(17.0)));
                        #[cfg(test)]
                        self.remove_rects.borrow_mut().push((it.id, b.rect));
                        if b.on_hover_text(tr_plain("JobDock.Remove")).clicked() {
                            remove = Some(it.id);
                        }
                    });
                }
            });
        });
        (select, remove)
    }
}

#[cfg(test)]
impl JobDock {
    /// Test access: index of the selected SVG item, else of the first SVG item.
    fn svg_index(&self) -> Option<usize> {
        let sel = self.selected_index().filter(|i| matches!(self.items[*i].kind, JobKind::Svg(_)));
        sel.or_else(|| self.items.iter().position(|i| matches!(i.kind, JobKind::Svg(_))))
    }

    /// Test access: index of the selected image item, else of the first image item.
    fn image_index(&self) -> Option<usize> {
        let sel = self.selected_index().filter(|i| matches!(self.items[*i].kind, JobKind::Image(_)));
        sel.or_else(|| self.items.iter().position(|i| matches!(i.kind, JobKind::Image(_))))
    }

    /// Test access: screen rectangle of the remove button of list row `idx` in the last frame.
    ///
    /// `idx`: row (list position). Panics when the row was not drawn.
    pub(crate) fn test_remove_rect(&self, idx: usize) -> egui::Rect {
        let id = self.items[idx].id;
        self.remove_rects.borrow().iter().find(|(i, _)| *i == id).map(|(_, r)| *r).expect("row not drawn")
    }

    /// Test access: id of the edited file. Returns None without files.
    pub(crate) fn selected_id(&self) -> Option<u64> {
        self.selected
    }

    /// Test access: the SVG editor of the selected file, else of the first SVG. Returns None without an SVG.
    pub(crate) fn selected_svg(&self) -> Option<&ParamsDock> {
        match &self.items[self.svg_index()?].kind {
            JobKind::Svg(d) => Some(d),
            _ => None,
        }
    }

    /// Test access: like [`JobDock::selected_svg`], mutable.
    pub(crate) fn selected_svg_mut(&mut self) -> Option<&mut ParamsDock> {
        let i = self.svg_index()?;
        match &mut self.items[i].kind {
            JobKind::Svg(d) => Some(d),
            _ => None,
        }
    }

    /// Test access: block of the SVG that [`JobDock::selected_svg`] returns.
    pub(crate) fn selected_svg_block(&self) -> Option<Range<usize>> {
        Some(self.items[self.svg_index()?].block.clone())
    }

    /// Test access: the image editor of the selected file, else of the first image. Returns None without an image.
    pub(crate) fn selected_image(&self) -> Option<&ImageDock> {
        match &self.items[self.image_index()?].kind {
            JobKind::Image(d) => Some(d),
            _ => None,
        }
    }

    /// Test access: like [`JobDock::selected_image`], mutable.
    pub(crate) fn selected_image_mut(&mut self) -> Option<&mut ImageDock> {
        let i = self.image_index()?;
        match &mut self.items[i].kind {
            JobKind::Image(d) => Some(d),
            _ => None,
        }
    }
}

/// File name part of a path. `p`: path. Returns the last component, or the whole path when it has none.
fn file_name_of(p: &Path) -> String {
    p.file_name().map_or_else(|| p.to_string_lossy().into_owned(), |n| n.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::job_test_harness::Harness;
    use lg_services::raster_dock;
    use std::time::{Duration, Instant};

    /// Two layers, closed shapes (hatch fill has something to fill): rect in "Engrave", circle in "Cut".
    const SVG_TWO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="40mm" height="40mm" viewBox="0 0 40 40">
<g id="a" inkscape:label="Engrave"><rect x="2" y="2" width="10" height="10"/></g>
<g id="b" inkscape:label="Cut"><circle cx="25" cy="25" r="6"/></g></svg>"#;

    /// One path, no groups.
    const SVG_ONE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><path d="M1 1 L19 1 L19 19 Z" stroke="black" fill="none"/></svg>"#;

    /// Write an SVG into a scratch dir. `tag`: dir tag. `name`: file name. `body`: SVG text. Returns the path.
    fn write_svg(tag: &str, name: &str, body: &str) -> PathBuf {
        let p = lg_base::paths::test_dir(&format!("job_{tag}")).join(name);
        std::fs::write(&p, body).unwrap();
        p
    }

    /// Write a black PNG (40 x 20 px) into a scratch dir. `tag`: dir tag. `name`: file name. Returns the path.
    fn write_png(tag: &str, name: &str) -> PathBuf {
        let p = lg_base::paths::test_dir(&format!("job_{tag}")).join(name);
        image::RgbaImage::from_fn(40, 20, |_, _| image::Rgba([0, 0, 0, 255])).save(&p).unwrap();
        p
    }

    /// Write a G-code file. `tag`: dir tag. `name`: file name. `lines`: content. Returns the path.
    fn write_nc(tag: &str, name: &str, lines: &[&str]) -> PathBuf {
        let p = lg_base::paths::test_dir(&format!("job_{tag}")).join(name);
        std::fs::write(&p, lines.join("\n")).unwrap();
        p
    }

    /// Run frames and polls until `cond` holds (10 s limit). `what`: text of the timeout panic.
    fn pump_until(h: &mut Harness, what: &str, cond: impl Fn(&Harness) -> bool) {
        let t = Instant::now();
        while !cond(h) {
            h.job.poll();
            h.frame(vec![]);
            assert!(t.elapsed() < Duration::from_secs(10), "timeout: {what}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Blocks of all items. Returns them in list order.
    fn blocks(h: &Harness) -> Vec<Range<usize>> {
        h.job.items.iter().map(|i| i.block.clone()).collect()
    }

    /// Assert that the blocks are contiguous and cover the program. `h`: harness.
    fn assert_covers(h: &Harness) {
        let mut at = 0;
        for b in blocks(h) {
            assert_eq!(b.start, at, "blocks must be contiguous: {:?}", blocks(h));
            at = b.end;
        }
        assert_eq!(at, h.all_lines().len(), "blocks must cover the program: {:?}", blocks(h));
    }

    #[test]
    fn svg_then_png_is_the_old_append_flow_byte_for_byte() {
        let _g = settings_guard();
        let svg = write_svg("flow", "a.svg", SVG_TWO);
        let png = write_png("flow", "b.png");
        let mut h = Harness::new();
        h.job.add_svg(&svg, false, false);
        h.job.add_image(&png, true).unwrap();
        h.wait_idle();
        // two items, the PNG (last added) is selected, blocks contiguous
        assert_eq!(h.job.len(), 2);
        assert_eq!(h.job.selected_id(), Some(h.job.items[1].id));
        let (a, b) = (h.job.items[0].block.clone(), h.job.items[1].block.clone());
        assert!(!a.is_empty() && !b.is_empty());
        assert_covers(&h);
        // reference: load the SVG, then append the image, the way the single-file docks did it
        let mut f = GrblFile::new();
        f.load_imported_svg_layered(svg.to_str().unwrap(), false, &h.svg().layer_params()).unwrap();
        let (img, _) = raster_dock::load_image(&png).unwrap();
        let gray = raster_dock::gray_image(&img);
        raster_dock::generate(&mut f, &gray, png.to_str().unwrap(), &h.image().params(), true).unwrap();
        let want: Vec<String> = f.iter().map(|c| c.command().to_owned()).collect();
        assert_eq!(h.all_lines(), want);
        assert_eq!(h.lines_of(a), want[..blocks(&h)[0].end].to_vec());
    }

    #[test]
    fn editing_the_svg_changes_only_its_block_and_moves_the_next() {
        let _g = settings_guard();
        let svg = write_svg("editsvg", "a.svg", SVG_TWO);
        let png = write_png("editsvg", "b.png");
        let mut h = Harness::new();
        h.job.add_svg(&svg, false, false);
        h.job.add_image(&png, true).unwrap();
        h.wait_idle();
        let (a0, b0) = (blocks(&h)[0].clone(), blocks(&h)[1].clone());
        let (svg_before, png_before) = (h.lines_of(a0.clone()), h.lines_of(b0.clone()));
        // power only: same length, only the SVG text differs
        h.job.selected_svg_mut().unwrap().test_set_power(0, 400);
        h.settle();
        let (a1, b1) = (blocks(&h)[0].clone(), blocks(&h)[1].clone());
        assert_eq!((a1.clone(), b1.clone()), (a0.clone(), b0.clone()));
        assert_ne!(h.lines_of(a1), svg_before);
        assert_eq!(h.lines_of(b1), png_before);
        // fill on: the SVG block grows, the PNG block moves behind it with identical text
        h.job.dev_set_fill(0, Some(true), None, None);
        h.settle();
        let (a2, b2) = (blocks(&h)[0].clone(), blocks(&h)[1].clone());
        assert!(a2.len() > a0.len(), "fill adds lines");
        assert_eq!((b2.start, b2.len()), (a2.end, b0.len()));
        assert_eq!(h.lines_of(b2), png_before);
        assert_covers(&h);
        // the preview was told: one FileLoaded per regeneration
        assert!(h.file_loaded_events() >= 3);
    }

    #[test]
    fn editing_the_png_changes_only_its_block() {
        let _g = settings_guard();
        let svg = write_svg("editpng", "a.svg", SVG_TWO);
        let png = write_png("editpng", "b.png");
        let mut h = Harness::new();
        h.job.add_svg(&svg, false, false);
        h.job.add_image(&png, true).unwrap();
        h.wait_idle();
        let (a0, b0) = (blocks(&h)[0].clone(), blocks(&h)[1].clone());
        let (svg_before, png_before) = (h.lines_of(a0.clone()), h.lines_of(b0));
        h.job.selected_image_mut().unwrap().test_set_box(10.0, 12.0, 70.0, 42.0);
        h.settle();
        let (a1, b1) = (blocks(&h)[0].clone(), blocks(&h)[1].clone());
        assert_eq!(a1, a0);
        assert_eq!(h.lines_of(a1), svg_before);
        assert_eq!(b1.start, a0.end);
        let png_after = h.lines_of(b1);
        assert_ne!(png_after, png_before);
        assert!(png_after[0].starts_with("G0 X10 Y12 "), "{}", png_after[0]);
        assert_covers(&h);
    }

    #[test]
    fn three_files_edit_the_middle_then_remove_it() {
        let _g = settings_guard();
        let a = write_svg("three_a", "a.svg", SVG_TWO);
        let b = write_png("three_b", "b.png");
        let c = write_svg("three_c", "c.svg", SVG_ONE);
        let mut h = Harness::new();
        h.job.add_svg(&a, false, false);
        h.job.add_image(&b, true).unwrap();
        h.job.add_svg(&c, false, true);
        h.wait_idle();
        assert_eq!(h.job.len(), 3);
        assert_covers(&h);
        let (a0, b0, c0) = (blocks(&h)[0].clone(), blocks(&h)[1].clone(), blocks(&h)[2].clone());
        let (ta, tc) = (h.lines_of(a0.clone()), h.lines_of(c0.clone()));
        // edit the middle one (select it first): A and C keep their text, C moves with the size of B
        let id_b = h.job.items[1].id;
        h.job.selected = Some(id_b);
        h.job.selected_image_mut().unwrap().test_set_box(0.0, 0.0, 10.0, 5.0);
        h.settle();
        let (a1, b1, c1) = (blocks(&h)[0].clone(), blocks(&h)[1].clone(), blocks(&h)[2].clone());
        assert_eq!(a1, a0);
        assert_ne!(b1.len(), b0.len());
        assert_eq!((c1.start, c1.len()), (b1.end, c0.len()));
        assert_eq!((h.lines_of(a1.clone()), h.lines_of(c1.clone())), (ta.clone(), tc.clone()));
        assert_covers(&h);
        // remove the middle one: the worker cuts it out, neighbours stay intact, ranges close up
        assert!(h.job.remove(id_b));
        assert!(h.job.job_running() && h.job.needs_apply());
        assert!(h.wait());
        assert_eq!(h.job.len(), 2);
        let (a2, c2) = (blocks(&h)[0].clone(), blocks(&h)[1].clone());
        assert_eq!((a2.clone(), c2.start, c2.len()), (a1, a0.end, c0.len()));
        assert_eq!((h.lines_of(a2), h.lines_of(c2)), (ta, tc));
        assert_covers(&h);
        // the selection moved to a neighbour
        assert_eq!(h.job.selected_id(), Some(h.job.items[0].id));
        // removing an item without commands needs no worker
        let id_c = h.job.items[1].id;
        assert!(h.job.remove(id_c));
        assert!(h.wait());
        assert_eq!(h.job.len(), 1);
        assert_covers(&h);
    }

    #[test]
    fn appended_gcode_becomes_a_static_item_and_a_plain_open_resets() {
        let _g = settings_guard();
        let svg = write_svg("gcode", "a.svg", SVG_TWO);
        let nc = write_nc("gcode", "b.nc", &["G0 X5 Y5", "G1 X6 Y6 F100", "G1 X7 Y7"]);
        let nc2 = write_nc("gcode", "c.nc", &["G0 X1", "G1 X2"]);
        let mut h = Harness::new();
        h.job.add_svg(&svg, false, false);
        h.wait_idle();
        let a0 = blocks(&h)[0].clone();
        let svg_text = h.lines_of(a0.clone());
        // append like main_form does: the core loads on its own thread, the dock is told what to expect
        assert_eq!(h.core.open_file(&nc, true), lasergrbl_core::OpenFileOutcome::GCodeLoading);
        h.job.expect_static("b.nc");
        pump_until(&mut h, "static item", |h| h.job.len() == 2);
        let b = blocks(&h)[1].clone();
        assert_eq!((b.start, b.len()), (a0.end, 3));
        assert_eq!(h.lines_of(b.clone()), ["G0 X5 Y5", "G1 X6 Y6 F100", "G1 X7 Y7"]);
        assert!(matches!(h.job.items[1].kind, JobKind::Static) && h.job.items[1].name == "b.nc");
        assert_eq!(h.job.selected_id(), Some(h.job.items[1].id));
        assert_covers(&h);
        assert!(!h.job.needs_apply());
        // an edit of the SVG moves the static block with its text intact
        h.job.dev_set_fill(0, Some(true), None, None);
        h.job.selected = Some(h.job.items[0].id);
        h.settle();
        let (a1, b1) = (blocks(&h)[0].clone(), blocks(&h)[1].clone());
        assert!(a1.len() > a0.len(), "fill adds lines");
        assert_ne!(h.lines_of(a1.clone()), svg_text);
        assert_eq!((b1.start, b1.len()), (a1.end, 3));
        assert_eq!(h.lines_of(b1), ["G0 X5 Y5", "G1 X6 Y6 F100", "G1 X7 Y7"]);
        // a plain open: reset, then the new file is the only item and covers everything
        assert_eq!(h.core.open_file(&nc2, false), lasergrbl_core::OpenFileOutcome::GCodeLoading);
        h.job.reset();
        h.job.expect_static("c.nc");
        pump_until(&mut h, "reset + static", |h| h.job.len() == 1 && h.job.items[0].block.len() == 2);
        assert_eq!(blocks(&h), vec![0..2]);
        assert_eq!(h.all_lines(), ["G0 X1", "G1 X2"]);
    }

    #[test]
    fn empty_gcode_load_adds_nothing() {
        let _g = settings_guard();
        let svg = write_svg("emptync", "a.svg", SVG_ONE);
        let nc = write_nc("emptync", "e.nc", &[]);
        let mut h = Harness::new();
        h.job.add_svg(&svg, false, false);
        h.wait_idle();
        assert_eq!(h.core.open_file(&nc, true), lasergrbl_core::OpenFileOutcome::GCodeLoading);
        h.job.expect_static("e.nc");
        pump_until(&mut h, "pending static dropped", |h| h.job.pending_static.is_none());
        assert_eq!(h.job.len(), 1);
        assert_covers(&h);
    }

    #[test]
    fn a_plain_open_replaces_the_old_program() {
        let _g = settings_guard();
        let a = write_svg("replace", "a.svg", SVG_ONE);
        let b = write_png("replace", "b.png");
        // what the image alone makes
        let mut alone = Harness::new();
        alone.job.add_image(&b, false).unwrap();
        alone.wait_idle();
        let want = alone.all_lines();
        let mut h = Harness::new();
        h.job.add_svg(&a, false, false);
        h.wait_idle();
        assert_ne!(h.all_lines(), want);
        // plain open of another file: the old file is gone from the list and from the program
        h.job.add_image(&b, false).unwrap();
        assert_eq!(h.job.len(), 1);
        h.wait_idle();
        assert_eq!(h.all_lines(), want);
        assert_eq!(blocks(&h), vec![0..want.len()]);
        // an unreadable image changes nothing
        let before = h.all_lines();
        assert!(h.job.add_image(Path::new("/nonexistent/x.png"), false).is_err());
        assert_eq!((h.job.len(), h.all_lines()), (1, before));
    }

    #[test]
    fn a_generator_replacing_the_program_collapses_to_one_static_item() {
        let _g = settings_guard();
        let svg = write_svg("gen", "a.svg", SVG_ONE);
        let png = write_png("gen", "b.png");
        let mut h = Harness::new();
        h.job.add_svg(&svg, false, false);
        h.job.add_image(&png, true).unwrap();
        h.wait_idle();
        assert_eq!(h.job.len(), 2);
        // shake test: the real generator, run the way the generator forms run it
        let st = run_generator(&h.core.loaded_file(), |f| f.generate_shake_test("X", 4000, 300, 100, 1000));
        assert_eq!(st, GenStatus::Done);
        h.job.poll();
        assert_eq!(h.job.len(), 1);
        assert!(matches!(h.job.items[0].kind, JobKind::Static));
        assert_eq!(h.job.items[0].name, tr_plain("JobDock.Program"));
        assert_eq!(blocks(&h), vec![0..h.all_lines().len()]);
        assert!(!h.job.needs_apply());
        // a second look changes nothing
        h.job.poll();
        assert_eq!(h.job.len(), 1);
        // the same length is noticed too (the generator ran again)
        let before = h.all_lines();
        let st = run_generator(&h.core.loaded_file(), |f| f.generate_shake_test("X", 4000, 300, 100, 1000));
        assert_eq!(st, GenStatus::Done);
        let id = h.job.items[0].id;
        h.job.poll();
        assert_eq!(h.all_lines(), before);
        assert_ne!(h.job.items[0].id, id, "rewritten program gets a new static item");
        // a generator on a program without files adopts it, an emptied program leaves no item
        let nc = write_nc("gen", "empty.nc", &[]);
        h.core.loaded_file().write().unwrap().load_file(nc.to_str().unwrap(), false).unwrap();
        h.job.poll();
        assert!(h.job.is_empty());
    }

    #[test]
    fn modal_forms_pause_the_dock() {
        let _g = settings_guard();
        let svg = write_svg("pause", "a.svg", SVG_ONE);
        let mut h = Harness::new();
        h.job.set_paused(true);
        h.job.add_svg(&svg, false, false);
        h.job.poll();
        h.frame(vec![]);
        assert!(!h.job.job_running() && h.job.needs_apply(), "nothing starts while a modal form is open");
        assert!(!h.job.apply());
        // an outside change is not touched while paused, repaired afterwards
        let nc = write_nc("pause", "x.nc", &["G0 X1"]);
        h.core.loaded_file().write().unwrap().load_file(nc.to_str().unwrap(), false).unwrap();
        h.job.poll();
        assert_eq!(h.job.len(), 1);
        assert!(matches!(h.job.items[0].kind, JobKind::Svg(_)));
        h.job.set_paused(false);
        h.job.poll();
        assert_eq!(h.job.len(), 1);
        assert!(matches!(h.job.items[0].kind, JobKind::Static));
    }

    #[test]
    fn classic_dialog_result_becomes_a_static_tail() {
        let _g = settings_guard();
        let svg = write_svg("classic", "a.svg", SVG_ONE);
        let png = write_png("classic", "b.png");
        let mut h = Harness::new();
        h.job.add_svg(&svg, false, false);
        h.job.add_image(&png, true).unwrap();
        h.wait_idle();
        let (a0, b0) = (blocks(&h)[0].clone(), blocks(&h)[1].clone());
        let (ta, id_b) = (h.lines_of(a0.clone()), h.job.items[1].id);
        // a non-image has no classic hand-over
        assert!(h.job.begin_classic(h.job.items[0].id).is_none());
        // the image goes out of the program: dialog opens with append (the SVG stays)
        let (path, append) = h.job.begin_classic(id_b).expect("image item");
        assert_eq!((path, append), (png.clone(), true));
        assert!(h.wait());
        assert_eq!((h.job.len(), h.all_lines().len()), (1, a0.end));
        assert_eq!(h.lines_of(0..a0.end), ta);
        // the dialog appends its output: the dock adopts the new commands as one static file
        {
            let file = h.core.loaded_file();
            let mut g = file.write().unwrap();
            for l in ["G0 X1 Y1", "G1 X2 Y2 F100"] {
                g.commands_mut().push(lg_gcode::GrblCommand::new(l));
            }
            g.analyze().unwrap();
            g.raise_file_loaded("b.png");
        }
        h.job.add_static_tail("b.png");
        h.job.poll();
        assert_eq!(h.job.len(), 2, "no safety net after an adopted result");
        assert_eq!(blocks(&h)[1], a0.end..a0.end + 2);
        assert!(matches!(h.job.items[1].kind, JobKind::Static));
        let _ = b0;
        // cancelled dialog: the image comes back at the end
        h.job.add_image(&png, true).unwrap();
        h.wait_idle();
        assert_eq!(h.job.len(), 3);
        assert_covers(&h);
        // a lone image: nothing stays, the dialog replaces the program
        let mut h2 = Harness::new();
        h2.job.add_image(&png, false).unwrap();
        h2.wait_idle();
        let id = h2.job.items[0].id;
        assert_eq!(h2.job.begin_classic(id).map(|(_, a)| a), Some(false));
        assert!(h2.wait());
        assert!(h2.job.is_empty() && h2.all_lines().is_empty());
    }

    #[test]
    fn layer_view_offsets_of_an_svg_that_is_the_second_item() {
        let _g = settings_guard();
        let png = write_png("view", "b.png");
        let svg = write_svg("view", "a.svg", SVG_TWO);
        let mut h = Harness::new();
        assert!(h.job.layer_view().is_none());
        h.job.add_image(&png, false).unwrap();
        h.wait_idle();
        // one file: nothing to tell apart, no highlight
        let v = h.job.layer_view().unwrap();
        assert!(v.highlight.is_none() && v.hidden.is_empty());
        h.job.add_svg(&svg, false, true);
        h.wait_idle();
        let start = blocks(&h)[1].start as u32;
        assert!(start > 0);
        // selected layer 0 of the SVG, in program coordinates
        let rel = h.svg().generated_ranges()[0].clone().unwrap();
        let v = h.job.layer_view().unwrap();
        assert_eq!(v.highlight, Some(start + rel.start..start + rel.end));
        assert!(v.hidden.is_empty());
        assert!(h.lines_of((start + rel.start) as usize..(start + rel.end) as usize).iter().any(|l| l.starts_with("G1")));
        // layer 1
        let rev = v.revision;
        h.job.dev_select_layer(1, true);
        let rel1 = h.svg().generated_ranges()[1].clone().unwrap();
        let v1 = h.job.layer_view().unwrap();
        assert_eq!(v1.highlight, Some(start + rel1.start..start + rel1.end));
        assert!(v1.revision != rev);
        // layer 0 disabled but not regenerated yet: hidden in program coordinates
        h.job.dev_select_layer(0, false);
        let v2 = h.job.layer_view().unwrap();
        assert_eq!(v2.hidden, vec![start + rel.start..start + rel.end]);
        assert!(v2.revision != v1.revision);
        // the image row: its whole block is the highlight while there are several files
        let id_png = h.job.items[0].id;
        h.job.selected = Some(id_png);
        let v3 = h.job.layer_view().unwrap();
        assert_eq!(v3.highlight, Some(0..start));
        // the editors' own changes (layer pick) move the revision without a dock call
        h.job.selected = Some(h.job.items[1].id);
        h.wait_idle();
        let before = h.job.layer_view().unwrap().revision;
        h.job.selected_svg_mut().unwrap().dev_select_layer(0, true);
        h.job.poll();
        assert!(h.job.layer_view().unwrap().revision != before);
    }

    #[test]
    fn run_gating_follows_dirty_running_and_invalid_files() {
        let _g = settings_guard();
        let svg = write_svg("gate", "a.svg", SVG_ONE);
        let png = write_png("gate", "b.png");
        let mut h = Harness::new();
        assert!(!h.job.needs_apply() && !h.job.is_dirty_idle() && !h.job.is_blocked());
        h.job.add_svg(&svg, false, false);
        h.job.add_image(&png, true).unwrap();
        // dirty before the first generation
        assert!(h.job.needs_apply() && h.job.is_dirty_idle() && h.job.pending_auto());
        // started by hand (Run): a worker runs, nothing else is dirty-idle
        assert!(h.job.apply());
        assert!(h.job.job_running() && h.job.needs_apply() && !h.job.is_dirty_idle());
        assert!(h.wait());
        // the poll that collected it started the next file already
        assert!(h.job.needs_apply());
        h.wait_idle();
        assert!(!h.job.needs_apply() && !h.job.is_dirty_idle() && !h.job.pending_auto());
        // invalid layout of one image: the program is stale and cannot be regenerated
        let good = h.all_lines();
        h.job.selected_image_mut().unwrap().test_set_box(10.0, 10.0, 5.0, 5.0);
        assert!(h.job.needs_apply() && h.job.is_blocked() && !h.job.is_dirty_idle() && !h.job.pending_auto());
        assert!(!h.job.apply());
        h.job.poll();
        assert!(!h.job.job_running());
        assert_eq!(h.all_lines(), good);
        // the row shows the problem
        assert!(h.job.items[1].problem().is_some());
        // fixed: generated again
        h.job.selected_image_mut().unwrap().test_set_box(0.0, 0.0, 50.0, 25.0);
        assert!(!h.job.is_blocked());
        h.settle();
        assert_ne!(h.all_lines(), good);
        // an SVG with every layer off is blocked as well
        h.job.dev_select_layer(0, false);
        assert!(h.job.is_blocked());
    }

    #[test]
    fn failed_generation_removes_only_that_block() {
        let _g = settings_guard();
        let svg = write_svg("fail", "a.svg", SVG_ONE);
        let png = write_png("fail", "b.png");
        let mut h = Harness::new();
        h.job.add_svg(&svg, false, false);
        h.job.add_image(&png, true).unwrap();
        h.wait_idle();
        let (a0, b0) = (blocks(&h)[0].clone(), blocks(&h)[1].clone());
        let tb = h.lines_of(b0.clone());
        // the file vanishes, then an edit asks for a new block
        std::fs::remove_file(&svg).unwrap();
        h.job.selected_svg_mut().unwrap().test_set_power(0, 300);
        let t = Instant::now();
        while !h.job.items[0].problem().is_some() {
            if let Some(ok) = h.job.poll() {
                assert!(!ok);
            }
            h.frame(vec![]);
            assert!(t.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_millis(5));
        }
        // the failed block is empty, the image behind it moved up and kept its text
        let (a1, b1) = (blocks(&h)[0].clone(), blocks(&h)[1].clone());
        assert!(a1.is_empty());
        assert_eq!((b1.start, b1.len()), (0, b0.len()));
        assert_eq!(h.lines_of(b1), tb);
        assert_covers(&h);
        let _ = a0;
        // not retried without a new edit, Run retries
        h.job.poll();
        assert!(!h.job.job_running());
        std::fs::write(&svg, SVG_ONE).unwrap();
        assert!(h.job.apply());
        assert!(h.wait());
        assert!(h.job.items[0].problem().is_none());
        assert!(!blocks(&h)[0].is_empty());
        assert_covers(&h);
    }

    #[test]
    fn removing_a_file_that_was_never_generated_needs_no_worker() {
        let _g = settings_guard();
        let svg = write_svg("never", "a.svg", SVG_ONE);
        let png = write_png("never", "b.png");
        let mut h = Harness::new();
        h.job.add_svg(&svg, false, false);
        h.wait_idle();
        let id = h.job.add_image(&png, true).unwrap();
        // nothing generated for the image yet: its block is empty
        assert!(h.job.items[1].block.is_empty());
        assert!(h.job.remove(id));
        assert!(!h.job.job_running());
        assert_eq!(h.job.len(), 1);
        assert_eq!(h.job.selected_id(), Some(h.job.items[0].id));
        // unknown id
        assert!(!h.job.remove(9999));
    }

    #[test]
    fn selection_seeds_the_next_svg_like_the_old_dock() {
        let _g = settings_guard();
        let a = write_svg("seed_a", "a.svg", SVG_ONE);
        let b = write_svg("seed_b", "b.svg", SVG_ONE);
        let mut h = Harness::new();
        h.job.add_svg(&a, false, false);
        h.job.selected_svg_mut().unwrap().test_set_power(0, 321);
        // an appended SVG starts from the values of the selected one
        h.job.add_svg(&b, false, true);
        h.wait_idle();
        assert_eq!(h.job.selected_svg().unwrap().path(), b.to_str());
        assert_eq!(h.svg().test_power_of_current(), 321);
        // after a plain open the edited values stay the defaults of the next file
        h.job.reset();
        assert_eq!(h.job.defaults.test_power_of_current(), 321);
        assert!(h.job.layer_view().is_none() && h.job.is_empty());
    }

    #[test]
    fn project_restore_appends_svg_items_in_order() {
        let _g = settings_guard();
        let a = write_svg("proj_a", "a.svg", SVG_ONE);
        let b = write_svg("proj_b", "b.svg", SVG_TWO);
        let mut h = Harness::new();
        // like pump_project: first entry plain, later ones appended, values from the settings
        h.job.add_svg(&a, true, false);
        h.job.add_svg(&b, true, true);
        h.wait_idle();
        assert_eq!(h.job.len(), 2);
        assert_covers(&h);
        assert!(h.job.items.iter().all(|i| !i.block.is_empty()));
    }

    #[test]
    fn list_shows_both_files_clicks_switch_the_editor_and_edits_stay_in_their_block() {
        let _g = settings_guard();
        let svg = write_svg("e2e", "plan.svg", SVG_TWO);
        let png = write_png("e2e", "photo.png");
        let mut h = Harness::new();
        // nothing loaded: the defaults editor
        assert!(h.shows(&tr_plain("ParamsDock.NoSource")) && !h.shows(&tr_plain("JobDock.Files")));
        h.job.add_svg(&svg, false, false);
        h.job.add_image(&png, true).unwrap();
        h.wait_idle();
        h.frame(vec![]);
        // the list: group title, both names with their tags, the PNG editor is up (last added)
        assert!(h.shows(&tr_plain("JobDock.Files")));
        assert!(h.shows("plan.svg") && h.shows("photo.png"));
        assert!(h.shows(&tr_plain("JobDock.TagSvg")) && h.shows(&tr_plain("JobDock.TagImage")));
        assert!(h.shows(&tr_plain("ImageDock.GroupMap")) && !h.shows(&tr_plain("ParamsDock.Layer")));
        let (a0, b0) = (blocks(&h)[0].clone(), blocks(&h)[1].clone());
        let (ta, tb) = (h.lines_of(a0.clone()), h.lines_of(b0.clone()));
        // a typed edit in the image regenerates only the image block
        h.type_into("image-w", "60");
        h.settle();
        assert_eq!(h.lines_of(blocks(&h)[0].clone()), ta);
        assert_ne!(h.lines_of(blocks(&h)[1].clone()), tb);
        assert_eq!(blocks(&h)[0], a0);
        let tb2 = h.lines_of(blocks(&h)[1].clone());
        // click the SVG row: the SVG editor replaces the image editor
        h.click_first_text("plan.svg");
        assert_eq!(h.job.selected_id(), Some(h.job.items[0].id));
        assert!(h.shows(&tr_plain("ParamsDock.Layer")) && !h.shows(&tr_plain("ImageDock.GroupMap")));
        // a typed edit in the SVG regenerates only the SVG block
        h.type_into("params-border", "700");
        h.settle();
        assert_eq!(h.lines_of(blocks(&h)[1].clone()), tb2);
        assert_ne!(h.lines_of(blocks(&h)[0].clone()), ta);
        assert!(h.layer_lines(0).unwrap().iter().any(|l| l.contains("F700")), "speed reached the SVG block");
        // and back
        h.click_first_text("photo.png");
        assert_eq!(h.job.selected_id(), Some(h.job.items[1].id));
        assert!(h.shows(&tr_plain("ImageDock.GroupMap")));
        assert_covers(&h);
    }

    #[test]
    fn remove_button_takes_a_file_out_of_the_job() {
        let _g = settings_guard();
        let svg = write_svg("btn", "plan.svg", SVG_ONE);
        let png = write_png("btn", "photo.png");
        let mut h = Harness::new();
        h.job.add_svg(&svg, false, false);
        h.job.add_image(&png, true).unwrap();
        h.wait_idle();
        h.frame(vec![]);
        let ta = h.lines_of(blocks(&h)[0].clone());
        // the close buttons sit at the right edge of the rows: click the one of the second row
        let rect = h.row_remove_rect(1);
        h.click_at(rect.center());
        pump_until(&mut h, "image removed", |h| h.job.len() == 1);
        assert_eq!(h.all_lines(), ta);
        assert!(!h.shows("photo.png") && h.shows("plan.svg"));
    }
}
