// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Test-pattern generator forms (`Generator/CuttingTest.cs`, `PowerVsSpeedForm.cs`, `ShakeTest.cs`).

pub mod cutting_test;
pub mod power_vs_speed;
pub mod shake_test;

/// How a generator form ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratorOutcome {
    /// Create pressed: the program in the shared file was replaced (or the file was busy, message shown).
    Created,
    /// Cancel / Esc / close.
    Cancelled,
}

use crate::forms::form_util::{already_loading_box, run_generator, GenStatus, SharedFile};
use crate::ui::dialog::{MessageBox, MessageBoxHost, MessageBoxIcon};
use lg_gcode::{GResult, GrblFile};

/// Shared "Create!" flow of the generator forms: run, show a message box on busy/error, then close.
#[derive(Default)]
pub struct GenFlow {
    msgs: MessageBoxHost<()>,
    pending: Option<GeneratorOutcome>,
}

impl GenFlow {
    /// C# `BtnCreate_Click` body: run the generator on the shared file; the form closes afterwards.
    ///
    /// `file`: shared program. `f`: closure calling the `lg_gcode` generator.
    pub fn run(&mut self, file: &SharedFile, f: impl FnOnce(&mut GrblFile) -> GResult<bool>) {
        match run_generator(file, f) {
            GenStatus::Done => {}
            // C# CheckInUse(true) message
            GenStatus::Busy => self.msgs.push((), already_loading_box()),
            GenStatus::Failed(e) => self.msgs.push((), MessageBox::new(e, "").icon(MessageBoxIcon::Error)),
        }
        self.pending = Some(GeneratorOutcome::Created);
    }

    /// Per-frame step before drawing the form.
    ///
    /// `ctx`: egui context. Returns `Some(Some(outcome))` when the form must close now,
    /// `Some(None)` while a message box blocks the form, `None` to draw the form normally.
    pub fn poll(&mut self, ctx: &eframe::egui::Context) -> Option<Option<GeneratorOutcome>> {
        if self.msgs.is_open() {
            self.msgs.show(ctx);
            return Some(None);
        }
        self.pending.take().map(Some)
    }
}
