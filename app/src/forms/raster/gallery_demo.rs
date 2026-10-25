//! Gallery tab "Raster import": opens [`RasterImport`] on a bundled test image with a throwaway
//! `GrblFile` and prints the resulting G-code line count (dev tool, no C# counterpart).

use eframe::egui::{self, Ui};
use lg_gcode::GrblFile;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use super::{RasterImport, RasterOutcome};
use crate::forms::material::PresetEditor;

/// State of the gallery tab.
pub struct RasterDemo {
    file: Arc<RwLock<GrblFile>>,
    path: PathBuf,
    dialog: Option<RasterImport>,
    editor: Option<PresetEditor>,
    status: String,
    auto_open: bool,
    frames: u32,
    step: u8,
}

impl Default for RasterDemo {
    fn default() -> Self {
        // LASERGRBL_RASTER_SAMPLE=<image> overrides the sample, LASERGRBL_RASTER_AUTOOPEN=1 opens it at once
        let path = std::env::var_os("LASERGRBL_RASTER_SAMPLE")
            .map(PathBuf::from)
            .unwrap_or_else(|| lg_base::paths::resources_dir().join("grafica").join("powervsspeed.png"));
        Self {
            file: Arc::new(RwLock::new(GrblFile::new())),
            path,
            dialog: None,
            editor: None,
            status: String::new(),
            auto_open: std::env::var_os("LASERGRBL_RASTER_AUTOOPEN").is_some(),
            frames: 0,
            step: 0,
        }
    }
}

impl RasterDemo {
    /// Draw the tab; the dialogs are modal windows on top of it.
    ///
    /// `ui`: tab ui.
    pub fn ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        ui.label(format!("Sample image: {}", self.path.display()));
        ui.horizontal(|ui| {
            if ui.button("Open raster import").clicked() || std::mem::take(&mut self.auto_open) {
                self.open();
            }
            if ui.button("Open preset editor").clicked() || (self.frames == 2 && std::env::var_os("LASERGRBL_RASTER_EDITOR").is_some()) {
                self.editor = Some(PresetEditor::new(crate::forms::material::default_presets()));
            }
        });
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
        self.frames += 1;
        self.dev_automation(&ctx);
        if let Some(d) = &mut self.dialog {
            if let Some(out) = d.show(&ctx) {
                self.dialog = None;
                self.report(&out);
            }
        }
        if let Some(e) = &mut self.editor {
            if e.show(&ctx) {
                self.editor = None;
            }
        }
    }

    /// Dev helper: `LASERGRBL_RASTER_STEPS=1|2` presses Next (and Create!) once the preview is
    /// ready, `LASERGRBL_RASTER_SHOT=file.png` saves a screenshot at frame `LASERGRBL_RASTER_SHOT_FRAME`
    /// (default 120) and closes the app.
    fn dev_automation(&mut self, ctx: &egui::Context) {
        let steps: u8 = std::env::var("LASERGRBL_RASTER_STEPS").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        if let Some(d) = &mut self.dialog {
            if steps >= 1 && self.frames > 60 && self.step == 0 && d.debug_step(1) {
                self.step = 1;
            } else if steps >= 2 && self.frames > 90 && self.step == 1 && d.debug_step(2) {
                self.step = 2;
            }
        }
        if let Some(path) = std::env::var_os("LASERGRBL_RASTER_SHOT") {
            let at = std::env::var("LASERGRBL_RASTER_SHOT_FRAME").ok().and_then(|v| v.parse().ok()).unwrap_or(120);
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
        if self.dialog.is_some() || steps > 0 || std::env::var_os("LASERGRBL_RASTER_SHOT").is_some() {
            ctx.request_repaint();
        }
    }

    /// Start the dialog. The core sets `Core.LastOpenFile` before it (needed by `Project.AddSettings`).
    fn open(&mut self) {
        lg_base::settings::set("Core.LastOpenFile", self.path.to_string_lossy().into_owned());
        match RasterImport::open(self.file.clone(), &self.path, false) {
            Ok(d) => self.dialog = Some(d),
            Err(e) => self.status = format!("open failed: {e}"),
        }
    }

    /// Print the result line.
    fn report(&mut self, out: &RasterOutcome) {
        let n = self.file.read().map(|f| f.count()).unwrap_or(0);
        self.status = format!("generated={} lines={} usage={:?} error={:?}", out.generated, n, out.usage, out.error);
        println!("raster import: {}", self.status);
    }
}
