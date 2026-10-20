//! Modal-form framework: WinForms `ShowDialog`, `MessageBox.Show`, `OpenFileDialog/SaveFileDialog`.
//!
//! * [`show_modal`]: generic modal window around a closure returning `Option<T>` (the dialog result).
//! * [`MessageBox`] + [`MessageBoxHost`]: C# `MessageBox.Show` variants as immediate-mode state objects.
//! * [`PendingFile`]: rfd dialogs on a background thread; poll each frame, never blocks egui.
//! * [`filters`]: exact C# filter strings.

use crossbeam_channel::{Receiver, TryRecvError};
use eframe::egui::{self, Align, Color32, Key, Layout, RichText, Ui, Vec2};
use std::collections::VecDeque;
use std::path::PathBuf;

use super::icons::Tint;

/// `System.Windows.Forms.DialogResult`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogResult {
    None,
    Ok,
    Cancel,
    Abort,
    Retry,
    Ignore,
    Yes,
    No,
}

impl DialogResult {
    /// Default caption of the button (English fallback if i18n key missing).
    pub fn caption(self) -> String {
        let (key, en) = match self {
            DialogResult::Ok => ("Strings.Ok", "OK"),
            DialogResult::Cancel => ("Strings.Cancel", "Cancel"),
            DialogResult::Abort => ("Strings.Abort", "Abort"),
            DialogResult::Retry => ("Strings.Retry", "Retry"),
            DialogResult::Ignore => ("Strings.Ignore", "Ignore"),
            DialogResult::Yes => ("Strings.Yes", "Yes"),
            DialogResult::No => ("Strings.No", "No"),
            DialogResult::None => ("", ""),
        };
        let t = super::i18n::tr(key);
        if t == key { en.to_owned() } else { t.to_owned() }
    }
}

/// `MessageBoxButtons`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageBoxButtons {
    Ok,
    OkCancel,
    AbortRetryIgnore,
    YesNoCancel,
    YesNo,
    RetryCancel,
}

impl MessageBoxButtons {
    /// Buttons left to right.
    pub fn list(self) -> &'static [DialogResult] {
        use DialogResult::*;
        match self {
            MessageBoxButtons::Ok => &[Ok],
            MessageBoxButtons::OkCancel => &[Ok, Cancel],
            MessageBoxButtons::AbortRetryIgnore => &[Abort, Retry, Ignore],
            MessageBoxButtons::YesNoCancel => &[Yes, No, Cancel],
            MessageBoxButtons::YesNo => &[Yes, No],
            MessageBoxButtons::RetryCancel => &[Retry, Cancel],
        }
    }

    /// Result produced by Escape / close box (WinForms rules), None if the box cannot be dismissed.
    pub fn escape_result(self) -> Option<DialogResult> {
        match self {
            MessageBoxButtons::Ok => Some(DialogResult::Ok),
            MessageBoxButtons::OkCancel | MessageBoxButtons::YesNoCancel | MessageBoxButtons::RetryCancel => {
                Some(DialogResult::Cancel)
            }
            _ => Option::None,
        }
    }
}

/// `MessageBoxIcon` (Asterisk=Information, Hand/Stop=Error, Exclamation=Warning).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageBoxIcon {
    None,
    Information,
    Warning,
    Error,
    Question,
}

impl MessageBoxIcon {
    /// (icon key, color) used for the glyph.
    fn glyph(self) -> Option<(&'static str, Color32)> {
        match self {
            MessageBoxIcon::None => None,
            MessageBoxIcon::Information => Some(("mdi-information", Color32::from_rgb(0, 122, 217))),
            MessageBoxIcon::Warning => Some(("mdi-alert", Color32::from_rgb(242, 165, 1))),
            MessageBoxIcon::Error => Some(("mdi-close-circle", Color32::from_rgb(219, 20, 43))),
            MessageBoxIcon::Question => Some(("mdi-help-circle", Color32::from_rgb(0, 122, 217))),
        }
    }
}

/// Show a modal window; returns the dialog result once `content` yields `Some`.
///
/// `ctx`: egui context. `id`: unique dialog id. `title`: window title. `content`: draws body, returns
/// `Some(result)` to close. Returns `Some(result)` on the closing frame, else None. Blocks input behind it.
pub fn show_modal<T>(
    ctx: &egui::Context,
    id: impl std::hash::Hash + std::fmt::Debug,
    title: &str,
    content: impl FnOnce(&mut Ui) -> Option<T>,
) -> Option<T> {
    let modal = egui::Modal::new(egui::Id::unique(id));
    let r = modal.show(ctx, |ui| {
        ui.set_min_width(260.0);
        ui.heading(title);
        ui.add_space(6.0);
        content(ui)
    });
    r.inner
}

/// A `MessageBox.Show(text, caption, buttons, icon, defaultButton)` call.
#[derive(Clone, Debug)]
pub struct MessageBox {
    pub title: String,
    pub text: String,
    pub buttons: MessageBoxButtons,
    pub icon: MessageBoxIcon,
    /// 0-based default button index (`MessageBoxDefaultButton`).
    pub default_button: usize,
    /// `MessageBox.Show(.., helpLink)`: adds a Help button that opens this URL (box stays open).
    help_url: Option<String>,
    id: u64,
    focused: bool,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl MessageBox {
    /// `MessageBox.Show(text, title)` (OK, no icon).
    ///
    /// `text`: body. `title`: caption.
    pub fn new(text: impl Into<String>, title: impl Into<String>) -> Self {
        MessageBox {
            title: title.into(),
            text: text.into(),
            buttons: MessageBoxButtons::Ok,
            icon: MessageBoxIcon::None,
            default_button: 0,
            help_url: None,
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            focused: false,
        }
    }
    /// Set buttons.
    pub fn buttons(mut self, b: MessageBoxButtons) -> Self {
        self.buttons = b;
        self
    }
    /// Set icon.
    pub fn icon(mut self, i: MessageBoxIcon) -> Self {
        self.icon = i;
        self
    }
    /// Set default button (0-based, clamped).
    pub fn default_button(mut self, i: usize) -> Self {
        self.default_button = i;
        self
    }

    /// Add a Help button (C# `MessageBox.Show(.., 0, url)`).
    ///
    /// `url`: link opened with `open_link` when Help is pressed.
    pub fn help_url(mut self, url: impl Into<String>) -> Self {
        self.help_url = Some(url.into());
        self
    }

    /// Draw the box; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the pressed result on the closing frame, else None.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<DialogResult> {
        let list = self.buttons.list();
        let def = self.default_button.min(list.len() - 1);
        let esc = self.buttons.escape_result();
        let text = self.text.clone();
        let glyph = self.icon.glyph();
        let first = !self.focused;
        self.focused = true;
        let mid = self.id;
        let help = self.help_url.clone();
        let mut res = show_modal(ctx, ("msgbox", mid), &self.title.clone(), |ui| {
            ui.horizontal(|ui| {
                if let Some((key, col)) = glyph {
                    let img = super::icons::icons().image_tinted(ui.ctx(), key, 32.0, Tint::Color(col));
                    ui.add(img);
                }
                ui.add(egui::Label::new(text).wrap());
            });
            ui.add_space(10.0);
            let mut out = None;
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Help sits right of the answer buttons and keeps the box open
                if let Some(url) = &help {
                    if ui.add_sized(Vec2::new(80.0, 24.0), egui::Button::new("Help")).clicked() {
                        lg_base::tools::utils::open_link(url, "");
                    }
                }
                for (i, b) in list.iter().enumerate().rev() {
                    let r = ui.add_sized(Vec2::new(80.0, 24.0), egui::Button::new(b.caption()));
                    if i == def && first {
                        r.request_focus();
                    }
                    if r.clicked() {
                        out = Some(*b);
                    }
                }
            });
            out
        });
        if res.is_none() && ctx.input(|i| i.key_pressed(Key::Escape)) {
            res = esc;
        }
        res
    }
}

/// Queue of message boxes with a caller tag, so continuations can be matched when the user answers.
pub struct MessageBoxHost<T> {
    queue: VecDeque<(T, MessageBox)>,
}

impl<T> Default for MessageBoxHost<T> {
    fn default() -> Self {
        MessageBoxHost { queue: VecDeque::new() }
    }
}

impl<T: Clone> MessageBoxHost<T> {
    /// Enqueue a box.
    ///
    /// `tag`: caller id returned with the answer. `mb`: the message box.
    pub fn push(&mut self, tag: T, mb: MessageBox) {
        self.queue.push_back((tag, mb));
    }

    /// True while a box is open.
    pub fn is_open(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Show front box; returns `(tag, result)` when answered.
    ///
    /// `ctx`: egui context.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<(T, DialogResult)> {
        let (tag, mb) = self.queue.front_mut()?;
        let r = mb.show(ctx)?;
        let tag = tag.clone();
        self.queue.pop_front();
        Some((tag, r))
    }
}

/// `string.Format`-free helper for shortcut message boxes.
///
/// `text`,`title`: content. Returns an OK/Information box.
pub fn info_box(text: impl Into<String>, title: impl Into<String>) -> MessageBox {
    MessageBox::new(text, title).icon(MessageBoxIcon::Information)
}

/// Exact C# `FileDialog.Filter` strings.
pub mod filters {
    pub const ZIPPED_BUTTON_SAVE: &str = "ZippedButton|*.zbn";
    pub const ZIPPED_BUTTON_OPEN: &str = "ZippedButton|*.zbn;*.gz";
    pub const COMM_LOG: &str = "Communication log|*.txt";
    pub const GCODE_SAVE_NC: &str = "GCODE Files|*.nc";
    pub const GCODE_OPEN_CONFIG: &str = "GCODE Files|*.nc;*.gcode";
    pub const IMAGE_FILE: &str = "Image file|*.bmp;*.png;*.jpg;*.jpeg;*.gif";
    pub const HEX_FIRMWARE: &str = "Precompiled .hex firmware|*.hex";
    pub const OPEN_ANY: &str = "Any supported file|*.nc;*.cnc;*.tap;*.gcode;*.ngc;*.bmp;*.png;*.jpg;*.jpeg;*.gif;*.svg;*.lps|GCODE Files|*.nc;*.cnc;*.tap;*.gcode;*.ngc|Raster Image|*.bmp;*.png;*.jpg;*.jpeg;*.gif|Vector Image (experimental)|*.svg|LaserGRBL Project|*.lps";
    pub const PROJECT_SAVE: &str = "LaserGRBL Project|*.lps";
}

/// Parse a C# filter string into `(name, extensions-without-dot)`.
///
/// `filter`: "Name|*.a;*.b|Name2|*.c". Returns list; odd leftover segments ignored.
pub fn parse_filter(filter: &str) -> Vec<(String, Vec<String>)> {
    let parts: Vec<&str> = filter.split('|').collect();
    parts
        .chunks_exact(2)
        .map(|p| {
            let exts = p[1].split(';').map(|e| e.trim().trim_start_matches("*.").to_owned()).filter(|e| !e.is_empty() && e != "*").collect();
            (p[0].to_owned(), exts)
        })
        .collect()
}

/// A file dialog running on a worker thread.
pub struct PendingFile {
    rx: Receiver<Option<PathBuf>>,
}

/// Dialog kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Open,
    Save,
    Folder,
}

/// Builder for [`PendingFile`] (`OpenFileDialog` / `SaveFileDialog` / folder browser).
#[derive(Clone, Debug, Default)]
pub struct FileDialogOptions {
    pub title: Option<String>,
    pub filter: Option<String>,
    pub directory: Option<PathBuf>,
    pub file_name: Option<String>,
}

impl PendingFile {
    /// Spawn dialog thread.
    fn spawn(ctx: &egui::Context, kind: Kind, o: FileDialogOptions) -> PendingFile {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut d = rfd::FileDialog::new();
            if let Some(t) = &o.title {
                d = d.set_title(t);
            }
            if let Some(f) = &o.filter {
                for (name, exts) in parse_filter(f) {
                    let e: Vec<&str> = exts.iter().map(String::as_str).collect();
                    d = d.add_filter(name, &e);
                }
            }
            if let Some(dir) = &o.directory {
                d = d.set_directory(dir);
            }
            if let Some(n) = &o.file_name {
                d = d.set_file_name(n);
            }
            let r = match kind {
                Kind::Open => d.pick_file(),
                Kind::Save => d.save_file(),
                Kind::Folder => d.pick_folder(),
            };
            let _ = tx.send(r);
            ctx.request_repaint();
        });
        PendingFile { rx }
    }

    /// `OpenFileDialog.ShowDialog`.
    ///
    /// `ctx`: egui context (repaint on finish). `o`: options. Returns pending handle.
    pub fn open(ctx: &egui::Context, o: FileDialogOptions) -> PendingFile {
        Self::spawn(ctx, Kind::Open, o)
    }

    /// `SaveFileDialog.ShowDialog`.
    pub fn save(ctx: &egui::Context, o: FileDialogOptions) -> PendingFile {
        Self::spawn(ctx, Kind::Save, o)
    }

    /// Folder picker.
    pub fn folder(ctx: &egui::Context, o: FileDialogOptions) -> PendingFile {
        Self::spawn(ctx, Kind::Folder, o)
    }

    /// Non-blocking poll. Returns None while open; `Some(None)` if cancelled; `Some(Some(path))` if chosen.
    pub fn poll(&mut self) -> Option<Option<PathBuf>> {
        match self.rx.try_recv() {
            Ok(v) => Some(v),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(None),
        }
    }
}

/// Convenience label (bold) for dialogs.
pub fn bold(s: &str) -> RichText {
    crate::ui::fonts::bold(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_parse() {
        let f = parse_filter(filters::OPEN_ANY);
        assert_eq!(f.len(), 5);
        assert_eq!(f[1].0, "GCODE Files");
        assert!(f[0].1.contains(&"lps".to_string()));
        assert_eq!(parse_filter(filters::GCODE_SAVE_NC)[0].1, vec!["nc".to_string()]);
    }

    #[test]
    fn escape_rules() {
        assert_eq!(MessageBoxButtons::YesNo.escape_result(), None);
        assert_eq!(MessageBoxButtons::OkCancel.escape_result(), Some(DialogResult::Cancel));
    }

    #[test]
    fn icon_keys_exist() {
        let i = super::super::icons::Icons::new(false);
        for ic in [MessageBoxIcon::Information, MessageBoxIcon::Warning, MessageBoxIcon::Error, MessageBoxIcon::Question] {
            assert!(i.contains(ic.glyph().unwrap().0));
        }
    }
}
