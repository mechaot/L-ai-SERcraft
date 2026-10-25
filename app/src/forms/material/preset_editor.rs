//! List editor of the layer presets: read-only table, delete per row.
//!
//! Not part of the C# app (replaces `PSHelper/PSEditorForm.cs`). Delete is immediate and saved.

use eframe::egui::{self, Id};
use egui_extras::{Column, TableBuilder};
use lg_services::layer_presets::LayerPreset;

use super::SharedPresetDb;
use crate::ui::i18n::{tr, tr_fmt, tr_plain};

/// Preset list editor.
pub struct PresetEditor {
    uid: u64,
    db: SharedPresetDb,
    /// Last delete/save error, shown in red.
    error: Option<String>,
}

impl PresetEditor {
    /// Ctor.
    ///
    /// `db`: preset database. Returns the editor.
    pub fn new(db: SharedPresetDb) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self { uid: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed), db, error: None }
    }

    /// Draw the editor; call every frame while open.
    ///
    /// `ctx`: egui context. Returns true on the frame the form closed.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let uid = self.uid;
        let title = tr_plain("Presets.EditorTitle");
        let mut closed = false;
        egui::Modal::new(Id::unique(("preset_editor", uid))).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(crate::ui::fonts::bold(&title));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(crate::ui::widgets::ImageButton::new("mdi-close").size(egui::Vec2::splat(16.0))).clicked() {
                        closed = true;
                    }
                });
            });
            ui.separator();
            self.body(ui);
        });
        closed
    }

    /// Form body: table, error line, file path.
    ///
    /// `ui`: parent.
    fn body(&mut self, ui: &mut egui::Ui) {
        ui.set_width(700.0f32.min(ui.ctx().content_rect().width() - 80.0));
        // snapshot so the lock is not held while drawing
        let (rows, file): (Vec<LayerPreset>, Option<std::path::PathBuf>) = {
            let g = self.db.lock().unwrap_or_else(|e| e.into_inner());
            (g.presets().to_vec(), g.file().map(|p| p.to_path_buf()))
        };
        if rows.is_empty() {
            ui.label(egui::RichText::new(tr_plain("Presets.Empty")).italics());
        } else {
            let height = (ui.ctx().content_rect().height() - 220.0).clamp(160.0, 480.0);
            let mut delete: Option<String> = None;
            TableBuilder::new(ui)
                .id_salt(("preset_table", self.uid))
                .striped(true)
                .max_scroll_height(height)
                .min_scrolled_height(height.min(22.0 * rows.len() as f32 + 24.0))
                .column(Column::initial(170.0).resizable(true)) // name
                .column(Column::initial(55.0)) // passes
                .column(Column::initial(105.0)) // speed
                .column(Column::initial(80.0)) // power
                .column(Column::initial(50.0)) // mode
                .column(Column::initial(60.0)) // filter
                .column(Column::initial(130.0)) // fill
                .column(Column::remainder().at_least(24.0)) // delete
                .header(20.0, |mut h| {
                    for k in ["ColName", "ColPasses", "ColSpeed", "ColPower", "ColMode", "ColFilter", "ColFill"] {
                        h.col(|ui| {
                            ui.strong(tr(&format!("Presets.{k}")));
                        });
                    }
                    h.col(|_| {});
                })
                .body(|body| {
                    body.rows(22.0, rows.len(), |mut row| {
                        let p = &rows[row.index()];
                        row.col(|ui| {
                            ui.label(&p.name);
                        });
                        row.col(|ui| {
                            ui.label(p.passes.to_string());
                        });
                        row.col(|ui| {
                            ui.label(p.speed.to_string());
                        });
                        row.col(|ui| {
                            ui.label(format!("{:.1}", p.power_pct));
                        });
                        row.col(|ui| {
                            ui.label(&p.laser_on);
                        });
                        row.col(|ui| {
                            ui.label(&p.filter);
                        });
                        row.col(|ui| {
                            // "x" marks cross hatch
                            let t = if p.fill { format!("{:.2} mm · {}°{}", p.fill_distance, p.fill_angle.round(), if p.fill_cross { " · x" } else { "" }) } else { "—".to_owned() };
                            ui.label(t);
                        });
                        row.col(|ui| {
                            if ui.add(crate::ui::widgets::ImageButton::new("mdi-delete").size(egui::Vec2::splat(16.0))).on_hover_text(tr("Presets.Delete")).clicked() {
                                delete = Some(p.name.clone());
                            }
                        });
                    });
                });
            // delete at once, no confirmation
            if let Some(name) = delete {
                self.error = self.db.lock().unwrap_or_else(|e| e.into_inner()).delete(&name).err();
            }
        }
        if let Some(e) = &self.error {
            ui.colored_label(egui::Color32::from_rgb(200, 40, 40), e);
        }
        if let Some(f) = file {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(tr_fmt("Presets.File", &[&f.display()])).small());
        }
    }
}
