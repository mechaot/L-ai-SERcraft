// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `CustomButtonForm.cs` + Designer: modal editor to create / edit one custom button.

use std::io::Cursor;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions, Ui, Vec2};
use lg_raster::resize::{resize_image, InterpolationMode};
use lg_services::custom_button::{ButtonTypes, CustomButton, CustomButtons, EnableStyles};

use super::custom_button_ib::PngButton;
use crate::ui::dialog::{filters, FileDialogOptions, MessageBox, MessageBoxHost, MessageBoxIcon, PendingFile};
use crate::ui::icons::icons;
use crate::ui::theme::Theme;
use crate::ui::widgets::{enum_combo, GrblButton, TextInput};
use crate::ui::{tr, tr_plain};

/// Page of the custom button help (Designer `LblDescription` link).
const HELP_URL: &str = "https://lasergrbl.com/usage/custom-buttons/";
/// Page of the MDI icon library (`LblCaption_Click`).
const MDI_URL: &str = "https://pictogrammers.com/library/mdi/";

/// Field a validation box points at (C# `Focus()` after the box).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    GCode,
    GCode2,
    ToolTip,
}

/// Editor state (C# `CustomButtonForm` with `inedit`).
pub struct CustomButtonForm {
    /// Guid of the edited button (`inedit`), None = create.
    edit_guid: Option<String>,
    gcode: String,
    gcode2: String,
    caption: String,
    tooltip: String,
    enable_style: EnableStyles,
    button_type: ButtonTypes,
    /// `BTOpenImage.Image` (PNG bytes).
    image: Option<Vec<u8>>,
    tex: Option<(usize, Option<TextureHandle>)>,
    msgs: MessageBoxHost<Field>,
    focus: Option<Field>,
    picker: Option<PendingFile>,
    id: egui::Id,
}

/// Result of one frame of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormResult {
    /// Still open.
    Open,
    /// Closed by Create/Save (buttons were changed and saved).
    Saved,
    /// Closed by Cancel / window close.
    Cancelled,
}

impl CustomButtonForm {
    /// `CreateAndShowDialog(parent)`: new empty button.
    ///
    /// Returns the dialog state (defaults: type Button, enabled Always).
    pub fn new_create() -> Self {
        Self {
            edit_guid: None,
            gcode: String::new(),
            gcode2: String::new(),
            caption: String::new(),
            tooltip: String::new(),
            // NOTE(upstream): ctor sets CbEStyles.SelectedItem twice, 2nd time with a ButtonTypes value (no-op)
            enable_style: EnableStyles::Always,
            button_type: ButtonTypes::Button,
            image: None,
            tex: None,
            msgs: MessageBoxHost::default(),
            focus: None,
            picker: None,
            id: egui::Id::unique(("custom_button_form", crate::forms::preview_form::next_form_id())),
        }
    }

    /// `CreateAndShowDialog(parent, cb)`: edit an existing button.
    ///
    /// `cb`: button to edit (fields are copied; saving writes them back by guid).
    pub fn new_edit(cb: &CustomButton) -> Self {
        let mut f = Self::new_create();
        f.edit_guid = Some(cb.guid.clone());
        f.gcode = cb.gcode.clone();
        f.gcode2 = cb.gcode2.clone();
        f.image = cb.image.clone();
        f.caption = cb.caption.clone();
        f.tooltip = cb.tooltip.clone();
        // undefined raw styles fall back to Always (combo shows nothing in C#)
        f.enable_style = cb.enable_style().unwrap_or(EnableStyles::Always);
        f.button_type = cb.button_type;
        f
    }

    /// True when `TBGCode2` / `LblGCode2` are shown (`SelectedItem != Button`).
    pub fn gcode2_visible(&self) -> bool {
        self.button_type != ButtonTypes::Button
    }

    /// `BtnCreate_Click` checks.
    ///
    /// Returns the first invalid field (GCode empty, GCode2 empty when visible, ToolTip empty), None when ok.
    pub fn validate(&self) -> Option<Field> {
        if self.gcode.trim().is_empty() {
            return Some(Field::GCode);
        }
        if self.gcode2_visible() && self.gcode2.trim().is_empty() {
            return Some(Field::GCode2);
        }
        if self.tooltip.trim().is_empty() {
            return Some(Field::ToolTip);
        }
        None
    }

    /// Write the fields into `buttons` (new button appended, or the edited one replaced in place) and save.
    ///
    /// `buttons`: the list. Returns false when the edited button no longer exists.
    pub fn apply(&self, buttons: &mut CustomButtons) -> bool {
        let fill = |cb: &mut CustomButton| {
            cb.gcode = self.gcode.clone();
            cb.gcode2 = self.gcode2.clone();
            cb.image = self.image.clone();
            cb.caption = self.caption.clone();
            cb.tooltip = self.tooltip.clone();
            cb.set_enable_style(self.enable_style);
            cb.button_type = self.button_type;
        };
        match &self.edit_guid {
            None => {
                let mut cb = CustomButton::default();
                fill(&mut cb);
                buttons.add(cb);
            }
            Some(g) => match buttons.buttons_mut().iter_mut().find(|b| &b.guid == g) {
                Some(cb) => fill(cb),
                None => return false,
            },
        }
        buttons.save_file();
        true
    }

    /// `BTOpenImage_Click` tail: load the file, resize to 48x48 (HighQualityBicubic), store as PNG.
    ///
    /// `path`: picked image. Returns false when it cannot be decoded.
    pub fn set_image_from_file(&mut self, path: &std::path::Path) -> bool {
        let Ok(img) = image::open(path) else { return false };
        let small = resize_image(&img.to_rgba8(), 48, 48, false, InterpolationMode::HighQualityBicubic);
        let mut png = Vec::new();
        if small.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png).is_err() {
            return false;
        }
        self.image = Some(png);
        self.tex = None;
        true
    }

    /// Texture of the current image.
    fn texture(&mut self, ctx: &egui::Context) -> Option<TextureHandle> {
        let len = self.image.as_ref().map_or(0, Vec::len);
        if self.tex.as_ref().is_none_or(|(l, _)| *l != len) {
            let t = self.image.as_deref().and_then(|b| image::load_from_memory(b).ok()).map(|i| {
                let i = i.to_rgba8();
                ctx.load_texture("custombutton:form", ColorImage::from_rgba_unmultiplied([i.width() as usize, i.height() as usize], i.as_raw()), TextureOptions::LINEAR)
            });
            self.tex = Some((len, t));
        }
        self.tex.as_ref().and_then(|(_, t)| t.clone())
    }

    /// Draw the dialog (call every frame while open).
    ///
    /// `ctx`: egui context. `buttons`: list to modify on Create/Save. Returns the dialog state.
    pub fn show(&mut self, ctx: &egui::Context, buttons: &mut CustomButtons) -> FormResult {
        // validation box answered: focus the field
        if let Some((field, _)) = self.msgs.show(ctx) {
            self.focus = Some(field);
        }
        // image picker finished
        if let Some(p) = self.picker.as_mut()
            && let Some(r) = p.poll()
        {
            self.picker = None;
            if let Some(path) = r {
                self.set_image_from_file(&path);
            }
        }
        let legacy = icons().is_legacy();
        let tex = self.texture(ctx);
        let mut result = FormResult::Open;
        let modal = egui::Modal::new(self.id).show(ctx, |ui| {
            ui.set_width(397.0);
            ui.heading(tr("CustomButtonForm.$this.Text"));
            ui.add_space(6.0);
            // dialog open behind a message box: keep it inert
            ui.add_enabled_ui(!self.msgs.is_open(), |ui| self.body(ui, tex.as_ref(), legacy, &mut result, buttons));
        });
        if modal.should_close() && result == FormResult::Open && !self.msgs.is_open() {
            result = FormResult::Cancelled;
        }
        result
    }

    /// Table content of the Designer.
    fn body(&mut self, ui: &mut Ui, tex: Option<&TextureHandle>, legacy: bool, result: &mut FormResult, buttons: &mut CustomButtons) {
        let th = Theme::current();
        egui::Grid::new(self.id.with("grid")).num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            // row 0: type
            ui.label(tr("CustomButtonForm.LblType.Text"));
            let types = [ButtonTypes::Button, ButtonTypes::TwoStateButton, ButtonTypes::PushButton];
            enum_combo(ui, (self.id, "type"), &mut self.button_type, &types, |t| t.name().to_string(), false, 160.0);
            ui.end_row();
            // rows 1-2: image (legacy only) and gcode boxes
            ui.vertical(|ui| {
                if legacy {
                    ui.label(tr("CustomButtonForm.LblImage.Text"));
                    let r = PngButton { tex, caption: "", disabled: false, back: None, sense: egui::Sense::click() }.show(ui);
                    if r.clicked() && self.picker.is_none() {
                        self.picker = Some(PendingFile::open(ui.ctx(), FileDialogOptions { filter: Some(filters::IMAGE_FILE.into()), ..Default::default() }));
                    }
                }
            });
            ui.vertical(|ui| {
                let two = self.gcode2_visible();
                let w = if two { 162.0 } else { 330.0 };
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.label(tr("CustomButtonForm.LblGCode.Text"));
                        let r = ui.add(TextInput::new(&mut self.gcode).multiline(true).rows(6).width(w));
                        if self.focus == Some(Field::GCode) {
                            r.request_focus();
                            self.focus = None;
                        }
                    });
                    if two {
                        ui.vertical(|ui| {
                            ui.label(tr("CustomButtonForm.LblGCode2.Text"));
                            let r = ui.add(TextInput::new(&mut self.gcode2).multiline(true).rows(6).width(w));
                            if self.focus == Some(Field::GCode2) {
                                r.request_focus();
                                self.focus = None;
                            }
                        });
                    }
                });
            });
            ui.end_row();
            // row 4: caption (non-legacy: link to the MDI icon list)
            if legacy {
                ui.label(tr("CustomButtonForm.LblCaption.Text"));
            } else if ui.add(egui::Label::new(egui::RichText::new(tr("Strings.MdiIconCaption")).color(th.link_color)).sense(egui::Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                lg_base::tools::utils::open_link(MDI_URL);
            }
            ui.add(TextInput::new(&mut self.caption).width(330.0));
            ui.end_row();
            // row 5: tooltip
            ui.label(tr("CustomButtonForm.LblToolTip.Text"));
            let r = ui.add(TextInput::new(&mut self.tooltip).width(330.0));
            if self.focus == Some(Field::ToolTip) {
                r.request_focus();
                self.focus = None;
            }
            ui.end_row();
            // row 6: enable style
            ui.label(tr("CustomButtonForm.LblEnabled.Text"));
            let styles = [EnableStyles::Always, EnableStyles::Connected, EnableStyles::Idle, EnableStyles::Run, EnableStyles::IdleProgram];
            enum_combo(ui, (self.id, "enable"), &mut self.enable_style, &styles, |s| s.name().to_string(), false, 160.0);
            ui.end_row();
        });
        // row 7: description with link
        ui.add_space(6.0);
        ui.add(egui::Label::new(tr("CustomButtonForm.LblDescription.Text").replace(HELP_URL, "")).wrap());
        if ui.link(HELP_URL).clicked() {
            lg_base::tools::utils::open_link(HELP_URL);
        }
        ui.add_space(6.0);
        // buttons: Cancel, Create/Save (right aligned)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let create_text = if self.edit_guid.is_some() { "Save".to_string() } else { tr_plain("CustomButtonForm.BtnCreate.Text") };
            if ui.add(GrblButton::new(create_text).icon("mdi-checkbox-marked").min_size(Vec2::new(90.0, 28.0))).clicked() {
                match self.validate() {
                    Some(f) => {
                        let text = if f == Field::ToolTip { tr("Strings.BoxCustomButtonPleaseToolTipText") } else { tr("Strings.BoxCustomButtonPleaseGCodeText") };
                        self.msgs.push(f, MessageBox::new(text, tr("Strings.BoxCustomButtonPleaseToolTipTitle")).icon(MessageBoxIcon::Error));
                    }
                    None => {
                        self.apply(buttons);
                        *result = FormResult::Saved;
                    }
                }
            }
            if ui.add(GrblButton::new(tr_plain("CustomButtonForm.BtnCancel.Text")).icon("mdi-close-box").min_size(Vec2::new(90.0, 28.0))).clicked() {
                *result = FormResult::Cancelled;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        lg_base::paths::test_dir(&format!("cbform_{name}"))
    }

    fn list(name: &str) -> CustomButtons {
        let d = scratch(name);
        CustomButtons::with_paths(d.join("u.json"), d.join("none.json"))
    }

    #[test]
    fn validation_order() {
        let mut f = CustomButtonForm::new_create();
        assert_eq!(f.validate(), Some(Field::GCode));
        f.gcode = "G0 X0".into();
        assert_eq!(f.validate(), Some(Field::ToolTip));
        f.tooltip = "  ".into();
        assert_eq!(f.validate(), Some(Field::ToolTip));
        f.tooltip = "tip".into();
        assert_eq!(f.validate(), None);
        // second box only for non-plain types
        f.button_type = ButtonTypes::PushButton;
        assert_eq!(f.validate(), Some(Field::GCode2));
        f.gcode2 = "M5".into();
        assert_eq!(f.validate(), None);
    }

    #[test]
    fn gcode2_visibility() {
        let mut f = CustomButtonForm::new_create();
        assert!(!f.gcode2_visible());
        f.button_type = ButtonTypes::TwoStateButton;
        assert!(f.gcode2_visible());
    }

    #[test]
    fn create_then_edit() {
        let mut l = list("create");
        let mut f = CustomButtonForm::new_create();
        f.gcode = "G0 X1".into();
        f.caption = "cap".into();
        f.tooltip = "tip".into();
        f.enable_style = EnableStyles::Idle;
        assert!(f.apply(&mut l));
        assert_eq!(l.count(), 1);
        let cb = l.get_button(0).unwrap().clone();
        assert_eq!(cb.enable_style(), Some(EnableStyles::Idle));
        // edit keeps guid and position
        let mut e = CustomButtonForm::new_edit(&cb);
        assert_eq!(e.gcode, "G0 X1");
        e.gcode = "G0 X2".into();
        e.button_type = ButtonTypes::TwoStateButton;
        assert!(e.apply(&mut l));
        assert_eq!(l.count(), 1);
        let after = l.get_button(0).unwrap();
        assert_eq!(after.guid, cb.guid);
        assert_eq!(after.gcode, "G0 X2");
        assert_eq!(after.button_type, ButtonTypes::TwoStateButton);
        // edited button vanished
        l.remove(&cb.guid);
        assert!(!e.apply(&mut l));
    }

    #[test]
    fn image_is_resized_to_48() {
        let d = scratch("img");
        let p = d.join("a.png");
        image::RgbaImage::from_pixel(100, 60, image::Rgba([255, 0, 0, 255])).save(&p).unwrap();
        let mut f = CustomButtonForm::new_create();
        assert!(f.set_image_from_file(&p));
        let img = image::load_from_memory(f.image.as_ref().unwrap()).unwrap();
        assert_eq!((img.width(), img.height()), (48, 48));
        assert!(!f.set_image_from_file(&d.join("missing.png")));
    }
}
