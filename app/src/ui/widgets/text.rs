//! Ports `PlaceholderTextbox.cs`, `TextInput.cs`, `GrblTextBox.cs` and `ColoredBorderControl.cs`.

use eframe::egui::{self, text_edit::TextEditState, text::{CCursor, CCursorRange}, Color32, Frame, Id, Key, Margin, Modifiers, Response, RichText, Stroke, TextEdit, Ui, Widget};

use crate::ui::theme::{named, Theme};

/// `ColoredBorderControl`: frame with colored inset border around `add_contents`.
///
/// `ui`: parent. `border`: border color. `size`: border width px (C# `BorderSize`, default 1).
/// `fill`: background. `add_contents`: body. Returns body result.
pub fn colored_border<R>(ui: &mut Ui, border: Color32, size: f32, fill: Color32, add_contents: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    Frame::new()
        .stroke(Stroke::new(size, border))
        .fill(fill)
        .inner_margin(Margin::same(1))
        .show(ui, add_contents)
}

/// `PlaceholderTextBox`: text edit with watermark (themed). Shows hint color by focus state.
pub struct PlaceholderTextBox<'a> {
    text: &'a mut String,
    watermark: String,
    id: Option<Id>,
    width: Option<f32>,
}

impl<'a> PlaceholderTextBox<'a> {
    /// New box.
    ///
    /// `text`: bound text. `watermark`: placeholder shown when empty.
    pub fn new(text: &'a mut String, watermark: impl Into<String>) -> Self {
        PlaceholderTextBox { text, watermark: watermark.into(), id: None, width: None }
    }
    /// Explicit widget id (needed to find focus from outside).
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }
    /// Desired width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
}

impl Widget for PlaceholderTextBox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let th = Theme::current();
        let id = self.id.unwrap_or_else(|| ui.next_auto_id());
        let focused = ui.memory(|m| m.has_focus(id));
        // C#: LightGray inactive, Gray when focused
        let hint = if focused { named::GRAY } else { named::LIGHT_GRAY };
        let mut te = TextEdit::singleline(self.text)
            .id(id)
            .hint_text(RichText::new(self.watermark).color(hint))
            .text_color(th.form_fore_color)
            .background_color(th.log_back_color)
            .frame(egui::Frame::new().inner_margin(Margin::symmetric(2, 1)));
        if let Some(w) = self.width {
            te = te.desired_width(w);
        }
        ui.add(te)
    }
}

/// `TextInput`: bordered themed text box (optionally multiline / read-only).
pub struct TextInput<'a> {
    text: &'a mut String,
    read_only: bool,
    multiline: bool,
    width: Option<f32>,
    rows: usize,
}

impl<'a> TextInput<'a> {
    /// New input.
    ///
    /// `text`: bound text.
    pub fn new(text: &'a mut String) -> Self {
        TextInput { text, read_only: false, multiline: false, width: None, rows: 3 }
    }
    /// `ReadOnly`.
    pub fn read_only(mut self, v: bool) -> Self {
        self.read_only = v;
        self
    }
    /// `Multiline`.
    pub fn multiline(mut self, v: bool) -> Self {
        self.multiline = v;
        self
    }
    /// Rows when multiline.
    pub fn rows(mut self, r: usize) -> Self {
        self.rows = r;
        self
    }
    /// Desired width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
}

impl Widget for TextInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let th = Theme::current();
        colored_border(ui, th.controls_border, 1.0, th.log_back_color, |ui| {
            let mut te = if self.multiline { TextEdit::multiline(self.text).desired_rows(self.rows) } else { TextEdit::singleline(self.text) };
            te = te
                .text_color(th.form_fore_color)
                .background_color(th.log_back_color)
                .interactive(!self.read_only)
                .frame(egui::Frame::NONE);
            if let Some(w) = self.width {
                te = te.desired_width(w);
            }
            ui.add(te)
        })
        .inner
    }
}

/// `GrblTextBox`: command entry with history (Up/Down), Enter submits, Escape clears.
#[derive(Default)]
pub struct GrblTextBox {
    pub text: String,
    history: Vec<String>,
    index: usize,
    id: Option<Id>,
}

impl GrblTextBox {
    /// Empty box.
    pub fn new() -> Self {
        Self::default()
    }

    /// Command history (oldest first).
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Draw the box.
    ///
    /// `ui`: parent. `watermark`: placeholder text. `width`: desired width. Returns `(response, entered)`
    /// where `entered` is the trimmed command if Enter was pressed on a non-empty line.
    pub fn show(&mut self, ui: &mut Ui, watermark: &str, width: f32) -> (Response, Option<String>) {
        let id = *self.id.get_or_insert_with(|| ui.next_auto_id());
        let mut entered = None;
        let mut set_cursor_end = false;
        if ui.memory(|m| m.has_focus(id)) {
            // consume keys before TextEdit sees them
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                let cmd = self.text.trim().to_owned();
                if !cmd.is_empty() {
                    self.history.push(cmd.clone());
                    self.text.clear();
                    self.index = self.history.len();
                    entered = Some(cmd);
                }
            }
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
                if self.index > 0 {
                    self.index -= 1;
                }
                if self.index < self.history.len() {
                    self.text = self.history[self.index].clone();
                    set_cursor_end = true;
                }
            }
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
                if self.index < self.history.len() {
                    self.index += 1;
                }
                if self.index < self.history.len() {
                    self.text = self.history[self.index].clone();
                    set_cursor_end = true;
                }
            }
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                self.text.clear();
            }
        }
        if set_cursor_end {
            let mut st = TextEditState::load(ui.ctx(), id).unwrap_or_default();
            st.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(self.text.chars().count()))));
            st.store(ui.ctx(), id);
        }
        let r = colored_border(ui, Theme::current().controls_border, 1.0, Theme::current().log_back_color, |ui| {
            ui.add(PlaceholderTextBox::new(&mut self.text, watermark).id(id).width(width))
        })
        .inner;
        (r, entered)
    }
}
