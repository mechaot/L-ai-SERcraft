// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Gallery tab "PreviewForm": the form with a core on the built-in emulator and the standard buttons.

use std::sync::Arc;

use crossbeam_channel::Receiver;
use eframe::egui::Ui;
use lasergrbl_core::com::{ComConfig, WrapperType};
use lasergrbl_core::{CoreEvent, GrblCore};
use lg_gcode::config::Firmware;
use lg_services::custom_button::CustomButtons;
use lg_services::hotkeys::HotKeysManager;

use super::PreviewForm;
use crate::ui::preview::Backend;

/// Small job for the demo: square with a diagonal.
const SAMPLE_GCODE: &str = "G21\nG90\nM4\nG0 X10 Y10\nG1 X90 Y10 F1000 S500\nG1 X90 Y70\nG1 X10 Y70\nG1 X10 Y10\nG1 X90 Y70\nM5\nG0 X0 Y0\n";

/// Demo state.
pub struct PreviewFormDemo {
    form: PreviewForm,
    core: Arc<GrblCore>,
    events: Receiver<CoreEvent>,
    buttons: CustomButtons,
    hotkeys: HotKeysManager,
    log: String,
}

impl Default for PreviewFormDemo {
    /// Core on the emulator (not connected yet); button list in a scratch file seeded from the standard buttons.
    fn default() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        let core = GrblCore::new(Firmware::Grbl, tx);
        core.configure(WrapperType::Emulator, &ComConfig::None);
        let dir = std::env::temp_dir().join(format!("lasergrbl_gallery_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let mut buttons = CustomButtons::with_paths(dir.join("CustomButtons.json"), lg_base::paths::resources_dir().join("misc").join("standard_buttons.json"));
        buttons.load_file();
        Self { form: PreviewForm::with_backend(Backend::Auto), core, events: rx, buttons, hotkeys: HotKeysManager::with_defaults(), log: String::new() }
    }
}

impl PreviewFormDemo {
    /// Draw the tab.
    ///
    /// `ui`: target ui (full remaining area).
    pub fn ui(&mut self, ui: &mut Ui) {
        // drain core events like the main window does
        while let Ok(ev) = self.events.try_recv() {
            self.form.on_core_event(&ev, &self.core);
        }
        ui.horizontal(|ui| {
            if self.core.is_connected() {
                if ui.button("Disconnect").clicked() {
                    let _ = self.core.close_com(true);
                }
            } else if ui.button("Connect (emulator)").clicked()
                && let Err(e) = self.core.open_com()
            {
                self.log = e;
            }
            if ui.button("Load sample G-code").clicked() {
                let p = std::env::temp_dir().join("lasergrbl_gallery_sample.nc");
                if std::fs::write(&p, SAMPLE_GCODE).is_ok() {
                    self.log = format!("{:?}", self.core.open_file(&p, false));
                }
            }
            if ui.button("Hotkey: button 1 down/up").clicked() {
                self.form.custom_button_down(0);
                self.form.custom_button_up(0);
            }
            ui.label(format!("status: {:?}  {}", self.core.machine_status(), self.log));
        });
        let actions = self.form.show(ui, &self.core, &mut self.buttons, &self.hotkeys, None);
        if !actions.is_empty() {
            self.log = format!("{actions:?}");
        }
        // keep ticking: status polling like the 200 ms UpdateTimer
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
}
