//! Ports `PSHelper/PSEditorForm.cs` (+ Designer): editable table of the laser/material database.
//!
//! The WinForms `DataGridView` bound to the typed DataSet becomes an `egui_extras` table. Each
//! cell commits on focus loss (`update_row` / `add_row` run the DataSet column checks); errors go
//! through the `DG_DataError` box. The last (empty) line is the "new row" of the grid.

use eframe::egui::{self, Id, Key, RichText, Sense, TextEdit, Ui};
use egui_extras::{Column, TableBuilder};
use lg_services::material_db::MaterialRow;
use std::collections::HashMap;

use super::SharedMaterialDb;
use crate::ui::dialog::{DialogResult, MessageBox, MessageBoxButtons, MessageBoxHost, MessageBoxIcon};
use crate::ui::i18n::{tr, tr_plain};
use crate::ui::theme::Theme;

/// Resource prefix of the form.
const P: &str = "PSHelper.PSEditorForm";

/// Grid columns (without the hidden `id`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum Col {
    Model,
    Material,
    Action,
    Thick,
    Power,
    Speed,
    Cycles,
    Remarks,
}

const COLS: [Col; 8] = [Col::Model, Col::Material, Col::Action, Col::Thick, Col::Power, Col::Speed, Col::Cycles, Col::Remarks];

impl Col {
    /// Header resource name.
    fn res(self) -> &'static str {
        match self {
            Col::Model => "ColModel",
            Col::Material => "ColMaterial",
            Col::Action => "ColAction",
            Col::Thick => "ColThick",
            Col::Power => "ColPower",
            Col::Speed => "ColSpeed",
            Col::Cycles => "ColCycles",
            Col::Remarks => "ColRemarks",
        }
    }
}

/// Continuation tag of the message boxes.
#[derive(Clone, Debug)]
enum Msg {
    /// `DG_DataError`: OK keeps editing (restore the cell text), Cancel reverts.
    DataError(Option<(usize, Col, String)>),
    /// Info box of the paste import.
    Info,
}

/// `PSEditorForm`.
pub struct MaterialEditor {
    uid: u64,
    db: SharedMaterialDb,
    /// Cell edit buffers while a cell has focus.
    bufs: HashMap<(usize, Col), String>,
    /// Cell to re-focus (after "continue editing").
    restore: Option<(usize, Col, String)>,
    selected: Option<usize>,
    msgs: MessageBoxHost<Msg>,
}

/// `StripNonNumber`: keep digits only.
fn strip_non_number(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// `GetInt`: `Int32.TryParse(StripNonNumber(v))`, 0 on failure.
fn get_int(v: &str) -> i32 {
    strip_non_number(v).parse().unwrap_or(0)
}

/// Text of a cell as the grid shows it (`DefaultCellStyle.Format`).
fn cell_text(r: &MaterialRow, c: Col) -> String {
    match c {
        Col::Model => r.model.clone(),
        Col::Material => r.material.clone(),
        Col::Action => r.action.clone(),
        Col::Thick => r.thickness.clone(),
        Col::Power => format!("{}%", r.power),
        Col::Speed => format!("{} mm/min", r.speed),
        Col::Cycles => r.cycles.to_string(),
        Col::Remarks => r.remarks().to_string(),
    }
}

/// `DG_CellParsing` + default parsing: apply edited text to a copy of the row.
///
/// `row`: row to change. `c`: column. `text`: edited text. Returns Err with the parse exception text.
fn apply_text(row: &mut MaterialRow, c: Col, text: &str) -> Result<(), String> {
    let bad = || Err("Input string was not in a correct format.".to_string());
    match c {
        Col::Model => row.model = text.to_string(),
        Col::Material => row.material = text.to_string(),
        Col::Action => row.action = text.to_string(),
        Col::Thick => row.thickness = text.to_string(),
        Col::Power | Col::Speed => match strip_non_number(text).parse::<i32>() {
            Ok(v) => {
                if c == Col::Power { row.power = v } else { row.speed = v }
            }
            Err(_) => return bad(),
        },
        Col::Cycles => match text.trim().parse::<i32>() {
            Ok(v) => row.cycles = v,
            Err(_) => return bad(),
        },
        Col::Remarks => row.remarks = if text.is_empty() { None } else { Some(text.to_string()) },
    }
    Ok(())
}

impl MaterialEditor {
    /// Ctor (`source.DataSource = GrblCore.MaterialDB.Materials`).
    ///
    /// `db`: material database. Returns the editor.
    pub fn new(db: SharedMaterialDb) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            uid: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            db,
            bufs: HashMap::new(),
            restore: None,
            selected: None,
            msgs: MessageBoxHost::default(),
        }
    }

    /// Draw the editor; call every frame while open.
    ///
    /// `ctx`: egui context. Returns true on the frame the form closed (`FormClosing` saved the DB).
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let uid = self.uid;
        let title = tr_plain(&format!("{P}.$this.Text"));
        let mut closed = false;
        egui::Modal::new(Id::unique(("ps_editor", uid))).show(ctx, |ui| {
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
        // message boxes sit above the editor
        if let Some((tag, res)) = self.msgs.show(ctx) {
            if let Msg::DataError(Some((row, col, text))) = tag {
                // e.Cancel = (result != Cancel): OK -> keep editing
                if res != DialogResult::Cancel {
                    self.restore = Some((row, col, text));
                }
            }
        }
        if closed {
            // PSEditorForm_FormClosing
            self.db.lock().unwrap_or_else(|e| e.into_inner()).save_changes();
        }
        closed
    }

    /// Show a `DG_DataError` box.
    fn data_error(&mut self, msg: &str, restore: Option<(usize, Col, String)>) {
        self.msgs.push(Msg::DataError(restore), MessageBox::new(msg, "Error").buttons(MessageBoxButtons::OkCancel).icon(MessageBoxIcon::Error));
    }

    /// Form body: grid + status strip.
    fn body(&mut self, ui: &mut Ui) {
        ui.set_width(891.0f32.min(ui.ctx().content_rect().width() - 80.0));
        let th = Theme::current();
        let uid = self.uid;
        let nrows = self.db.lock().unwrap_or_else(|e| e.into_inner()).materials.rows().len();
        let mut commits: Vec<(usize, Col, String)> = Vec::new();
        let mut visible_toggle: Option<(usize, bool)> = None;
        let text_focused = ui.memory(|m| m.focused()).is_some_and(|id| self.is_cell_id(id, nrows + 1));

        // paste import when no cell is being edited (Ctrl+V on the grid)
        if !text_focused {
            let pasted: Option<String> = ui.input(|i| i.events.iter().find_map(|e| if let egui::Event::Paste(t) = e { Some(t.clone()) } else { None }));
            if let Some(t) = pasted {
                self.paste(&t);
            }
            // Delete key removes the selected row
            if ui.input(|i| i.key_pressed(Key::Delete))
                && let Some(s) = self.selected.take()
            {
                self.db.lock().unwrap_or_else(|e| e.into_inner()).materials.delete_row(s);
            }
        }

        let rows: Vec<MaterialRow> = self.db.lock().unwrap_or_else(|e| e.into_inner()).materials.rows().to_vec();
        let height = (ui.ctx().content_rect().height() - 220.0).clamp(160.0, 480.0);
        TableBuilder::new(ui)
            .id_salt(("ps_editor_table", uid))
            .striped(true)
            .max_scroll_height(height)
            .min_scrolled_height(height)
            .column(Column::exact(18.0)) // row selector
            .column(Column::exact(43.0)) // visible
            .column(Column::initial(130.0).resizable(true)) // model
            .column(Column::initial(130.0).resizable(true)) // material
            .column(Column::initial(110.0).resizable(true)) // action
            .column(Column::initial(81.0).resizable(true)) // thickness
            .column(Column::initial(62.0).resizable(true)) // power
            .column(Column::initial(80.0).resizable(true)) // speed
            .column(Column::initial(63.0).resizable(true)) // cycles
            .column(Column::remainder().at_least(80.0)) // remarks
            .header(20.0, |mut h| {
                h.col(|_| {});
                h.col(|ui| {
                    ui.strong(tr(&format!("{P}.ColVisible.HeaderText")));
                });
                for c in COLS {
                    h.col(|ui| {
                        ui.strong(tr(&format!("{P}.{}.HeaderText", c.res())));
                    });
                }
            })
            .body(|body| {
                body.rows(22.0, rows.len() + 1, |mut row| {
                    let ri = row.index();
                    let is_new = ri >= rows.len();
                    let data = rows.get(ri);
                    row.set_selected(self.selected == Some(ri));
                    row.col(|ui| {
                        let (text, col) = if is_new { ("*", th.form_fore_color) } else if self.selected == Some(ri) { ("\u{25B6}", th.form_fore_color) } else { (" ", th.form_fore_color) };
                        if ui.add(egui::Label::new(RichText::new(text).color(col)).sense(Sense::click())).clicked() && !is_new {
                            self.selected = Some(ri);
                        }
                    });
                    row.col(|ui| {
                        let mut v = data.map(|d| d.visible).unwrap_or(true);
                        if !is_new && ui.checkbox(&mut v, "").changed() {
                            visible_toggle = Some((ri, v));
                        }
                    });
                    for c in COLS {
                        row.col(|ui| {
                            let display = data.map(|d| cell_text(d, c)).unwrap_or_default();
                            if let Some(t) = self.text_cell(ui, ri, c, display) {
                                commits.push((ri, c, t));
                            }
                        });
                    }
                });
            });

        // statusStrip1 items (New elements / import) are Visible=false in the designer and never shown by the code
        if let Some((i, v)) = visible_toggle {
            let mut r = rows[i].clone();
            r.visible = v;
            let res = self.db.lock().unwrap_or_else(|e| e.into_inner()).materials.update_row(i, r);
            if let Err(e) = res {
                self.data_error(&e.0, None);
            }
        }
        for (ri, c, t) in commits {
            self.commit(ri, c, t, &rows);
        }
    }

    /// True when `id` is the id of one of the cell editors.
    fn is_cell_id(&self, id: Id, rows: usize) -> bool {
        (0..rows).any(|r| COLS.iter().any(|c| Id::unique(("psed", self.uid, r, *c)) == id))
    }

    /// One editable text cell. Returns the new text when the edit ended with a changed value.
    ///
    /// `ui`: cell ui. `ri`: row. `c`: column. `display`: formatted text of the model.
    fn text_cell(&mut self, ui: &mut Ui, ri: usize, c: Col, display: String) -> Option<String> {
        let id = Id::unique(("psed", self.uid, ri, c));
        let focused = ui.memory(|m| m.has_focus(id));
        let mut buf = if focused { self.bufs.get(&(ri, c)).cloned().unwrap_or_else(|| display.clone()) } else { display.clone() };
        // "continue editing" after an error box
        let restoring = matches!(&self.restore, Some((r, cc, _)) if *r == ri && *cc == c);
        if restoring && let Some((_, _, t)) = self.restore.take() {
            buf = t;
        }
        let esc = focused && ui.input(|i| i.key_pressed(Key::Escape));
        let r = ui.add(TextEdit::singleline(&mut buf).id(id).frame(egui::Frame::NONE).desired_width(f32::INFINITY));
        if restoring {
            r.request_focus();
        }
        if esc {
            // Esc cancels the edit
            self.bufs.remove(&(ri, c));
            ui.memory_mut(|m| m.surrender_focus(id));
            return None;
        }
        if r.has_focus() || restoring {
            self.bufs.insert((ri, c), buf.clone());
        }
        if r.lost_focus() {
            let edited = self.bufs.remove(&(ri, c)).unwrap_or(buf);
            if edited != display {
                return Some(edited);
            }
        }
        None
    }

    /// Commit one edited cell (`CellParsing` + `EndEdit`).
    fn commit(&mut self, ri: usize, c: Col, text: String, rows: &[MaterialRow]) {
        let mut g = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let is_new = ri >= rows.len();
        let mut row = if is_new { g.materials.new_row() } else { rows[ri].clone() };
        let res = apply_text(&mut row, c, &text).and_then(|_| {
            if is_new { g.materials.add_row(row.clone()).map_err(|e| e.0) } else { g.materials.update_row(ri, row.clone()).map_err(|e| e.0) }
        });
        drop(g);
        if let Err(e) = res {
            self.data_error(&e, Some((ri, c, text)));
        }
    }

    /// `Paste()`: tab separated lines -> new rows.
    ///
    /// `text`: clipboard text. Each line: model, material, action, thickness, power, speed, cycles, remarks.
    fn paste(&mut self, text: &str) {
        let trimmed = text.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            return;
        }
        // NOTE: C# splits on "\r\n" only; also accept a bare "\n" (X11/Wayland clipboards)
        for line in trimmed.split('\n') {
            let line = line.trim_end_matches('\r');
            let cells: Vec<&str> = line.split('\t').collect();
            let res: Result<(), String> = (|| {
                // cells[7] is read last in C#: any missing cell throws IndexOutOfRange
                if cells.len() < 8 {
                    return Err("Index was outside the bounds of the array.".to_string());
                }
                let mut g = self.db.lock().unwrap_or_else(|e| e.into_inner());
                let mut row = g.materials.new_row();
                row.model = cells[0].to_string();
                row.material = cells[1].to_string();
                row.thickness = cells[3].to_string();
                row.action = cells[2].to_string();
                row.power = get_int(cells[4]);
                row.speed = get_int(cells[5]);
                row.cycles = get_int(cells[6]);
                row.remarks = Some(cells[7].to_string());
                row.visible = true;
                g.materials.add_row(row).map_err(|e| e.0)
            })();
            if let Err(e) = res {
                self.msgs.push(Msg::Info, MessageBox::new(format!("Error importing line: [{line}]\r\n{e}"), ""));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lg_services::material_db::MaterialDb;
    use std::sync::{Arc, Mutex};

    fn editor() -> MaterialEditor {
        MaterialEditor::new(Arc::new(Mutex::new(MaterialDb::default())))
    }

    #[test]
    fn paste_adds_rows() {
        let mut e = editor();
        e.paste("Laser A\tWood\tCut\t3mm\t90%\t200 mm/min\t2\tnote\r\n");
        let g = e.db.lock().unwrap();
        let r = &g.materials.rows()[0];
        assert_eq!((r.model.as_str(), r.action.as_str(), r.thickness.as_str(), r.power, r.speed, r.cycles), ("Laser A", "Cut", "3mm", 90, 200, 2));
    }

    #[test]
    fn paste_short_line_errors() {
        let mut e = editor();
        e.paste("a\tb");
        assert!(e.msgs.is_open());
        assert!(e.db.lock().unwrap().materials.rows().is_empty());
    }

    #[test]
    fn parse_power_strips_text() {
        let mut r = MaterialDb::default().materials.new_row();
        apply_text(&mut r, Col::Power, "55 %").unwrap();
        assert_eq!(r.power, 55);
        assert!(apply_text(&mut r, Col::Speed, "abc").is_err());
    }
}
