// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `Tools/InputBox.cs`: modal "prompt + text + OK/Cancel" dialog with optional validator.

use eframe::egui::{self, Align, Key, Layout, Ui};

use crate::ui::dialog::show_modal;

/// `InputBoxResult`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputBoxResult {
    pub ok: bool,
    pub text: String,
}

/// Validator: gets text, returns `Err(message)` to block OK (`InputBoxValidatingArgs.Cancel/Message`).
pub type Validator = Box<dyn Fn(&str) -> Result<(), String>>;

/// Input dialog state. Keep in your form struct; call [`InputBox::show`] each frame while open.
pub struct InputBox {
    pub prompt: String,
    pub title: String,
    pub text: String,
    validator: Option<Validator>,
    error: Option<String>,
    first: bool,
    id: u64,
}

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl InputBox {
    /// `InputBox.Show(parent, prompt, title, defaultResponse, validator)`.
    ///
    /// `prompt`: label. `title`: window title. `default_response`: initial text.
    /// `validator`: optional check run on OK.
    pub fn new(prompt: impl Into<String>, title: impl Into<String>, default_response: impl Into<String>, validator: Option<Validator>) -> Self {
        InputBox {
            prompt: prompt.into(),
            title: title.into(),
            text: default_response.into(),
            validator,
            error: None,
            first: true,
            id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Draw; returns the result once OK (valid) or Cancel/Escape.
    ///
    /// `ctx`: egui context.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<InputBoxResult> {
        let title = self.title.clone();
        let id = ("inputbox", self.id);
        let first = std::mem::replace(&mut self.first, false);
        let r = show_modal(ctx, id, &title, |ui| self.body(ui, first));
        if r.is_none() && ctx.input(|i| i.key_pressed(Key::Escape)) {
            return Some(InputBoxResult { ok: false, text: String::new() });
        }
        r
    }

    /// Dialog body.
    fn body(&mut self, ui: &mut Ui, first: bool) -> Option<InputBoxResult> {
        let mut out = None;
        ui.label(&self.prompt);
        let te = ui.add(egui::TextEdit::singleline(&mut self.text).desired_width(416.0));
        if first {
            te.request_focus();
        }
        if te.changed() {
            self.error = None; // textBoxText_TextChanged clears error
        }
        if let Some(e) = &self.error {
            ui.colored_label(egui::Color32::RED, e);
        }
        ui.add_space(8.0);
        let enter = te.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button(crate::ui::dialog::DialogResult::Cancel.caption()).clicked() {
                out = Some(InputBoxResult { ok: false, text: String::new() });
            }
            if ui.button(crate::ui::dialog::DialogResult::Ok.caption()).clicked() || enter {
                match self.validator.as_ref().map(|v| v(&self.text)) {
                    Some(Err(m)) => self.error = Some(m),
                    _ => out = Some(InputBoxResult { ok: true, text: self.text.clone() }),
                }
            }
        });
        out
    }
}
