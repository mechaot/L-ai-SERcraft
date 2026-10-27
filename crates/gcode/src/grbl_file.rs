// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `GrblFile.cs` (core part): command list, load/save, analysis, quadrant, events.
//!
//! The program can be edited as independent blocks, one per loaded file: [`GrblFile::replace_block`],
//! [`GrblFile::remove_block`].
//!
//! Sibling modules add the generators (`generators.rs`), raster line-to-line (`l2l.rs`),
//! vector loaders (`vector.rs`), path optimizer (`optimize.rs`) and preview drawing (`draw.rs`).
//!
//! Threading: C# `SafeLoadFile` spawns a thread per load. Here every `load_*` method runs
//! synchronously on the caller thread; [`GrblFile::spawn_load`] gives the threaded variant for an
//! `Arc<RwLock<GrblFile>>`. The "loading thread" flag is the shared `loading` atomic.

use crate::command::{Element, GrblCommand};
use crate::config::{configuration, GCODE_STD_FOOTER, GCODE_STD_HEADER, GCODE_STD_PASSES};
use crate::program_range::ProgramRange;
use crate::state_builder::StatePositionBuilder;
use crate::{GResult, GcodeError};
use lg_dotnet::{Decimal, TimeSpan};
use std::io::Write;
use std::ops::Range;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Quadrant of the drawing (C# `CartesianQuadrant`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartesianQuadrant {
    I,
    II,
    III,
    IV,
    Mix,
    Unknown,
}

impl std::fmt::Display for CartesianQuadrant {
    /// Variant name (as C# `ToString`).
    ///
    /// `f`: formatter. Returns fmt result.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::I => "I",
            Self::II => "II",
            Self::III => "III",
            Self::IV => "IV",
            Self::Mix => "Mix",
            Self::Unknown => "Unknown",
        };
        f.write_str(s)
    }
}

/// Callback for `OnFileLoading` / `OnFileLoaded`: `(elapsed_ms, filename)`.
pub type FileEvent = Box<dyn Fn(i64, &str) + Send + Sync>;

/// A G-code program (C# `GrblFile`).
#[derive(Default)]
pub struct GrblFile {
    pub(crate) list: Vec<GrblCommand>,
    pub(crate) range: ProgramRange,
    pub(crate) estimated_total_time: TimeSpan,
    pub(crate) loading: Arc<AtomicBool>,
    /// Bumped by every producer that raises `OnFileLoaded` (see [`GrblFile::load_serial`]).
    load_serial: AtomicU64,
    pub(crate) skipcmd: String,
    on_file_loading: Vec<FileEvent>,
    on_file_loaded: Vec<FileEvent>,
    /// C# `InUse` (set by the core while a job runs).
    pub in_use: bool,
}

impl GrblFile {
    /// Empty file.
    pub fn new() -> Self {
        Self { skipcmd: "G0".into(), ..Default::default() }
    }

    /// C# `GrblFile(x, y, x1, y1)`: empty program with a preset range (for the preview of a box).
    ///
    /// `x`,`y`,`x1`,`y1`: two corner points.
    pub fn with_range(x: Decimal, y: Decimal, x1: Decimal, y1: Decimal) -> Self {
        let mut f = Self::new();
        f.range.update_xy_range(Some(&Element::new('X', x)), Some(&Element::new('Y', y)), false);
        f.range.update_xy_range(Some(&Element::new('X', x1)), Some(&Element::new('Y', y1)), false);
        f
    }

    /// Register an `OnFileLoading` handler.
    ///
    /// `f`: callback `(elapsed_ms (always 0), filename)`.
    pub fn add_on_file_loading(&mut self, f: FileEvent) {
        self.on_file_loading.push(f);
    }

    /// Register an `OnFileLoaded` handler.
    ///
    /// `f`: callback `(elapsed_ms, filename)`.
    pub fn add_on_file_loaded(&mut self, f: FileEvent) {
        self.on_file_loaded.push(f);
    }

    pub(crate) fn rise_on_file_loaded(&self, filename: &str, elapsed: i64) {
        self.load_serial.fetch_add(1, Ordering::SeqCst);
        for f in &self.on_file_loaded {
            f(elapsed, filename);
        }
    }

    pub(crate) fn rise_on_file_loading(&self, filename: &str) {
        for f in &self.on_file_loading {
            f(0, filename);
        }
    }

    /// Commands (C# `Commands`).
    pub fn commands(&self) -> &[GrblCommand] {
        &self.list
    }

    /// Mutable commands (core marks results through shared wrappers; rarely needed).
    pub fn commands_mut(&mut self) -> &mut Vec<GrblCommand> {
        &mut self.list
    }

    /// Command at `index` (C# indexer).
    ///
    /// `index`: 0-based line. Returns None when out of range.
    pub fn get(&self, index: usize) -> Option<&GrblCommand> {
        self.list.get(index)
    }

    /// Number of commands.
    pub fn count(&self) -> usize {
        self.list.len()
    }

    /// Iterate commands.
    pub fn iter(&self) -> std::slice::Iter<'_, GrblCommand> {
        self.list.iter()
    }

    /// Estimated total time.
    pub fn estimated_time(&self) -> TimeSpan {
        self.estimated_total_time
    }

    /// Program ranges.
    pub fn range(&self) -> &ProgramRange {
        &self.range
    }

    /// Shared flag that is true while a threaded load runs (C# `mLoadingThread != null`).
    pub fn loading_flag(&self) -> Arc<AtomicBool> {
        self.loading.clone()
    }

    /// Counter of finished loads: every loader, generator and [`GrblFile::raise_file_loaded`] bumps it.
    /// Lets an owner of command blocks notice that someone else rewrote the program (even to the same length).
    ///
    /// Returns the current value.
    pub fn load_serial(&self) -> u64 {
        self.load_serial.load(Ordering::SeqCst)
    }

    /// Raise `OnFileLoaded` by hand (the preview rebuilds). For edits that no loader reports, like
    /// [`GrblFile::remove_block`].
    ///
    /// `filename`: name shown as the loaded file (may be empty).
    pub fn raise_file_loaded(&self, filename: &str) {
        self.rise_on_file_loaded(filename, 0);
    }

    /// C# `ClearList()`: dispose and drop all commands.
    pub(crate) fn clear_list(&mut self) {
        for c in &mut self.list {
            c.dispose();
        }
        self.list.clear();
    }

    /// Dispose and drop commands at index >= `len` (re-apply of an appended import).
    ///
    /// `len`: number of leading commands to keep; no-op when the list is not longer.
    pub fn truncate_list(&mut self, len: usize) {
        // dispose only the tail
        for c in self.list.iter_mut().skip(len) {
            c.dispose();
        }
        self.list.truncate(len);
    }

    /// Replace one block of commands inside the program, keep the commands before and after it.
    ///
    /// The program is treated as a concatenation of independent blocks (one per loaded file). The commands
    /// after `range` are parked, the block is disposed, `produce` appends the new block at the end of what is left
    /// (call a loader with `append = true`), the parked commands are put back and the whole program is analyzed.
    /// A range beyond the list is clamped (an empty range at/after the end appends).
    ///
    /// `range`: command indices of the old block. `produce`: producer of the new block; indices it sees (and
    /// returns) are absolute, so the first new command sits at `range.start`. Returns `produce`'s value and the
    /// range of the new block. When `produce` fails, whatever it appended is dropped again: the block stays
    /// removed, the commands around it are intact and the error is returned. A failing analysis of the
    /// joined program is only logged (the list is complete, ranges / times may be partial), like a G-code
    /// file that cannot be analyzed. A panic in `produce` also restores the parked commands before it continues.
    pub fn replace_block<T>(&mut self, range: Range<usize>, produce: impl FnOnce(&mut GrblFile) -> GResult<T>) -> GResult<(T, Range<usize>)> {
        // clamp to the list
        let end = range.end.min(self.list.len());
        let start = range.start.min(end);
        // park the commands behind the block, drop the block itself
        let suffix = self.list.split_off(end);
        self.truncate_list(start);
        let res = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| produce(self))) {
            Ok(r) => r,
            Err(p) => {
                // never lose the parked commands, even when the producer panics
                self.truncate_list(start);
                self.list.extend(suffix);
                let _ = self.analyze();
                std::panic::resume_unwind(p);
            }
        };
        // failed producer: drop the partial block
        if res.is_err() {
            self.truncate_list(start);
        }
        let new = start..self.list.len();
        self.list.extend(suffix);
        // ranges and times of the joined program
        if let Err(e) = self.analyze() {
            lg_base::logger::log_exception("replace_block", &e);
        }
        res.map(|t| (t, new))
    }

    /// Remove one block of commands, keep the commands before and after it.
    ///
    /// `range`: command indices of the block (clamped to the list). Returns Ok(()) when done. Raises no
    /// `OnFileLoaded` event: call [`GrblFile::raise_file_loaded`] when the preview must follow.
    pub fn remove_block(&mut self, range: Range<usize>) -> GResult<()> {
        self.replace_block(range, |_| Ok(())).map(|_| ())
    }

    /// C# `CheckInUse(showMessageBox)`. The message box is the caller's job (`Strings.AlreadyLoading`).
    ///
    /// Returns true when a load runs or the file is in use.
    pub fn check_in_use(&self) -> bool {
        self.loading.load(Ordering::SeqCst) || self.in_use
    }

    /// C# `SafeLoadFile` for a shared file: run `action` on a new thread, errors are logged
    /// (`Logger.LogException("SafeLoadFile", ex)`), the loading flag is cleared at the end.
    ///
    /// `file`: shared file. `action`: load body (gets the file under the write lock).
    /// Returns the join handle, or None when the file was busy (nothing started).
    pub fn spawn_load<F>(file: &Arc<RwLock<GrblFile>>, action: F) -> Option<std::thread::JoinHandle<()>>
    where
        F: FnOnce(&mut GrblFile) -> GResult<()> + Send + 'static,
    {
        let flag = {
            let g = file.read().unwrap_or_else(|e| e.into_inner());
            if g.check_in_use() {
                return None;
            }
            g.loading.store(true, Ordering::SeqCst);
            g.loading.clone()
        };
        let file = file.clone();
        Some(std::thread::spawn(move || {
            let r = {
                let mut g = file.write().unwrap_or_else(|e| e.into_inner());
                // body sees the flag set; panics must not leave it set
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| action(&mut g)))
            };
            match r {
                Ok(Err(e)) => lg_base::logger::log_exception("SafeLoadFile", &e),
                Err(_) => lg_base::logger::log_exception("SafeLoadFile", &"panic in load thread"),
                Ok(Ok(())) => {}
            }
            flag.store(false, Ordering::SeqCst);
        }))
    }

    /// Write the program as a text file (C# `SaveGCODE`). C# swallows every error; here the
    /// io error is returned and the caller may ignore it.
    ///
    /// `filename`: target. `header`/`footer`: add custom header/footer. `between`: add custom
    /// "passes" code between cycles. `cycles`: repeat count. `use_lf_line_endings`: `\n`, else the
    /// platform newline (`\r\n` on Windows, `\n` elsewhere like Mono). `evaluate`: expression
    /// evaluator (C# `core.EvaluateExpression`); empty result drops the line.
    ///
    /// Custom code is split on `\n` with a trailing `\r` removed per line (C# split on
    /// `Environment.NewLine`, and the stored text comes from WinForms boxes with `\r\n`).
    #[allow(clippy::too_many_arguments)]
    pub fn save_gcode(
        &self,
        filename: &Path,
        header: bool,
        footer: bool,
        between: bool,
        cycles: i32,
        use_lf_line_endings: bool,
        evaluate: &dyn Fn(&str) -> String,
    ) -> std::io::Result<()> {
        let nl: &str = if use_lf_line_endings || !cfg!(windows) { "\n" } else { "\r\n" };
        let mut out = String::new();
        if header {
            evaluate_add_lines(&mut out, nl, evaluate, &lg_base::settings::get("GCode.CustomHeader", GCODE_STD_HEADER.to_string()));
        }
        for i in 0..cycles {
            for cmd in &self.list {
                out.push_str(cmd.command());
                out.push_str(nl);
            }
            if between && i < cycles - 1 {
                evaluate_add_lines(&mut out, nl, evaluate, &lg_base::settings::get("GCode.CustomPasses", GCODE_STD_PASSES.to_string()));
            }
        }
        if footer {
            evaluate_add_lines(&mut out, nl, evaluate, &lg_base::settings::get("GCode.CustomFooter", GCODE_STD_FOOTER.to_string()));
        }
        let mut f = std::fs::File::create(filename)?;
        f.write_all(out.as_bytes())
    }

    /// C# `LoadFile(filename, append)`, synchronous body (see [`GrblFile::spawn_load`]).
    ///
    /// `filename`: .nc/.gcode text file (missing file = empty program). `append`: keep old commands.
    pub fn load_file(&mut self, filename: &str, append: bool) -> GResult<()> {
        self.rise_on_file_loading(filename);
        let start = lg_base::tools::hires_timer::total_milliseconds();
        if !append {
            self.clear_list();
        }
        self.range.reset_range();
        let p = Path::new(filename);
        if p.exists() {
            let bytes = std::fs::read(p).map_err(|e| GcodeError::Other(e.to_string()))?;
            // StreamReader: UTF-8 default, BOM detection for UTF-8/16/32
            let text = crate::text::decode_stream_reader(&bytes);
            for line in read_lines(&text) {
                let line = crate::text::dn_trim(line);
                if !line.is_empty() {
                    let cmd = GrblCommand::new(line);
                    if !cmd.is_empty() {
                        self.list.push(cmd);
                    }
                }
            }
        }
        self.analyze()?;
        let elapsed = lg_base::tools::hires_timer::total_milliseconds() - start;
        self.rise_on_file_loaded(filename, elapsed);
        Ok(())
    }

    /// Append G-code text lines (split on CR/LF, empty entries dropped) as commands.
    ///
    /// `gcode`: converter output.
    pub(crate) fn add_text_lines(&mut self, gcode: &str) {
        for l in gcode.split(['\r', '\n']).filter(|s| !s.is_empty()) {
            let line = crate::text::dn_trim(l);
            if !line.is_empty() {
                let cmd = GrblCommand::new(line);
                if !cmd.is_empty() {
                    self.list.push(cmd);
                }
            }
        }
    }

    /// C# `Analyze()`: ranges and cumulative time for each command.
    ///
    /// NOTE: the grbl configuration is read once, C# re-reads it for every command.
    pub fn analyze(&mut self) -> GResult<()> {
        let mut spb = StatePositionBuilder::new();
        self.range.reset_range();
        self.range.update_xy_range(Some(&Element::lit("X0")), Some(&Element::lit("Y0")), false);
        self.estimated_total_time = TimeSpan::ZERO;
        let conf = configuration::configuration();

        for cmd in self.list.iter_mut() {
            let r = (|| -> GResult<()> {
                let delay = spb.analyze_command(cmd, true, Some(&conf))?;
                self.range.update_s_range(Some(&**spb.s()));
                if let Some(h) = &spb.last_arc_helper_result {
                    self.range.update_xy_range_rect(h.b_box.x, h.b_box.y, h.b_box.width, h.b_box.height, spb.laser_burning());
                } else {
                    self.range.update_xy_range(Some(&**spb.x()), Some(&**spb.y()), spb.laser_burning());
                }
                self.estimated_total_time = self.estimated_total_time + delay;
                cmd.set_offset(self.estimated_total_time);
                Ok(())
            })();
            // finally { cmd.DeleteHelper(); }
            cmd.delete_helper();
            r?;
        }
        Ok(())
    }

    /// C# `Quadrant`: where the drawing lives.
    pub fn quadrant(&self) -> CartesianQuadrant {
        let r = &self.range.drawing_range;
        if !r.valid_range() {
            CartesianQuadrant::Unknown
        } else if r.x.min >= Decimal::ZERO && r.y.min >= Decimal::ZERO {
            CartesianQuadrant::I
        } else if r.x.max <= Decimal::ZERO && r.y.min >= Decimal::ZERO {
            CartesianQuadrant::II
        } else if r.x.max <= Decimal::ZERO && r.y.max <= Decimal::ZERO {
            CartesianQuadrant::III
        } else if r.x.min >= Decimal::ZERO && r.y.max <= Decimal::ZERO {
            CartesianQuadrant::IV
        } else {
            CartesianQuadrant::Mix
        }
    }

    /// C# `formatnumber(double)`: `ToString("0.###", Invariant)`.
    ///
    /// `number`: value. Returns text.
    pub fn formatnumber(number: f64) -> String {
        lg_dotnet::fmt_double_custom(number, "0.###")
    }
}

/// Port of `EvaluateAddLines`.
///
/// `out`: text sink. `nl`: newline to write. `evaluate`: expression evaluator. `lines`: custom code.
fn evaluate_add_lines(out: &mut String, nl: &str, evaluate: &dyn Fn(&str) -> String, lines: &str) {
    for line in lines.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if !crate::text::dn_trim(line).is_empty() {
            let command = evaluate(line);
            if !command.is_empty() {
                out.push_str(&command);
                out.push_str(nl);
            }
        }
    }
}

/// `StreamReader.ReadLine` splitting: CR, LF or CRLF; no empty final line.
///
/// `text`: whole file. Returns lines.
fn read_lines(text: &str) -> Vec<&str> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\n' || b[i] == b'\r' {
            out.push(&text[start..i]);
            if b[i] == b'\r' && i + 1 < b.len() && b[i + 1] == b'\n' {
                i += 1;
            }
            start = i + 1;
        }
        i += 1;
    }
    if start < b.len() {
        out.push(&text[start..]);
    }
    out
}

impl<'a> IntoIterator for &'a GrblFile {
    type Item = &'a GrblCommand;
    type IntoIter = std::slice::Iter<'a, GrblCommand>;
    /// Iterate commands.
    fn into_iter(self) -> Self::IntoIter {
        self.list.iter()
    }
}
