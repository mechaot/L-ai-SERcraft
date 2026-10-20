//! Ports `UserControls/NumericInput/*`: `DecimalInputRanged`, `IntegerInputRanged`, `NumericUpDown`.
//!
//! Text is edited in a buffer kept in egui memory; value committed on focus loss / Enter (C#
//! `OnEndEdit`), Escape reverts. Border: orange while editing, red when out of range, else themed.

use eframe::egui::{self, Color32, Id, Key, Response, Sense, Stroke, TextEdit, Ui, Vec2, Widget};

use crate::ui::theme::{named, Theme};

/// Orange `InEditBorderColor` default.
pub const IN_EDIT_BORDER: Color32 = Color32::from_rgb(255, 165, 0);

/// Result of one numeric input frame.
pub struct NumericOutput<T> {
    pub response: Response,
    /// `(old, new)` when the user committed a new value (`CurrentValueChanged(.., ByUser=true)`).
    pub committed: Option<(T, T)>,
    /// Parsed value while typing (`OnTheFlyValueChanged`).
    pub on_the_fly: Option<T>,
    /// Edit began this frame (`BeginEdit`).
    pub began_edit: bool,
    /// Edit ended this frame (`EndEdit`).
    pub ended_edit: bool,
    /// Value is outside [min, max] with validation on (`InError`).
    pub in_error: bool,
}

/// Per-widget edit state stored in egui memory.
#[derive(Clone, Default)]
struct EditState {
    text: String,
    old: String,
    /// Had focus at the end of the last frame (detects focus taken by a widget drawn later).
    editing: bool,
}

/// Filter typed characters (`AcceptKeys`).
fn filter_text(s: &mut String, accept: impl Fn(char) -> bool) {
    s.retain(accept);
}

/// Shared single-line editor.
///
/// Returns `(response, began, ended_text)`; `ended_text` = buffer when focus left (None = no end).
#[allow(clippy::too_many_arguments)]
fn edit_box(
    ui: &mut Ui,
    id: Id,
    display: &str,
    width: f32,
    enabled: bool,
    border: Color32,
    accept: &dyn Fn(char) -> bool,
) -> (Response, bool, Option<String>, Option<String>) {
    let th = Theme::current();
    let mut st: EditState = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
    let had_focus = ui.memory(|m| m.has_focus(id));
    let mut began = false;
    let mut ended = None;
    let mut typing = None;
    // focus moved to a later widget last frame: commit the buffer before it is reset
    if st.editing && !had_focus {
        st.editing = false;
        if enabled {
            ended = Some(st.text.clone());
        }
    }
    let resp = egui::Frame::new()
        .stroke(Stroke::new(1.0, border))
        .fill(th.log_back_color)
        .show(ui, |ui| {
            if !enabled {
                // disabled: label (C# LB), left aligned
                let (r, resp) = ui.allocate_exact_size(Vec2::new(width, 16.0), Sense::hover());
                ui.painter().text(r.left_center() + Vec2::new(2.0, 0.0), egui::Align2::LEFT_CENTER, display, egui::FontId::proportional(13.0), th.disabled_buttons);
                return resp;
            }
            if !had_focus {
                st.text = display.to_owned();
            }
            ui.add(
                TextEdit::singleline(&mut st.text)
                    .id(id)
                    .desired_width(width)
                    .text_color(th.form_fore_color)
                    .background_color(th.log_back_color)
                    .frame(egui::Frame::NONE),
            )
        })
        .inner;
    if enabled {
        let has_focus = ui.memory(|m| m.has_focus(id));
        if has_focus && !had_focus {
            began = true;
            st.old = display.to_owned();
        }
        if has_focus {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
                st.text.clear();
                ui.memory_mut(|m| m.surrender_focus(id));
            }
            let before = st.text.clone();
            filter_text(&mut st.text, accept);
            if st.text != before || resp.changed() {
                typing = Some(st.text.clone());
            }
            if resp.lost_focus() || ui.input(|i| i.key_pressed(Key::Enter)) {
                ui.memory_mut(|m| m.surrender_focus(id));
            }
        }
        let still = ui.memory(|m| m.has_focus(id));
        if had_focus && !still {
            ended = Some(st.text.clone());
        }
        st.editing = still;
    }
    ui.data_mut(|d| d.insert_temp(id, st));
    (resp, began, ended, typing)
}

/// `DecimalInputRanged`: float with fixed decimals and min/max validation.
pub struct DecimalInputRanged<'a> {
    value: &'a mut f32,
    decimals: usize,
    min: f32,
    max: f32,
    validation: bool,
    force_min_max: bool,
    width: f32,
    id: Option<Id>,
}

impl<'a> DecimalInputRanged<'a> {
    /// New input (C# defaults: 3 decimals, range -100..100).
    ///
    /// `value`: bound value.
    pub fn new(value: &'a mut f32) -> Self {
        DecimalInputRanged { value, decimals: 3, min: -100.0, max: 100.0, validation: true, force_min_max: false, width: 78.0, id: None }
    }
    /// `MinValue`/`MaxValue`.
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }
    /// `DecimalPositions`.
    pub fn decimals(mut self, d: usize) -> Self {
        self.decimals = d;
        self
    }
    /// `ValidationEnabled`.
    pub fn validation(mut self, v: bool) -> Self {
        self.validation = v;
        self
    }
    /// `ForceMinMax`: reject out-of-range commits.
    pub fn force_min_max(mut self, v: bool) -> Self {
        self.force_min_max = v;
        self
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Explicit id (needed when several inputs share a call site).
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// `Math.Round(value, decimals)` as float.
    fn round(&self, v: f32) -> f32 {
        lg_dotnet::round_double(v as f64, self.decimals as i32) as f32
    }
    /// `CurrentValue.ToString("0.000")`.
    fn fmt(&self, v: f32) -> String {
        let f = if self.decimals > 0 { format!("0.{}", "0".repeat(self.decimals)) } else { "0".to_owned() };
        lg_dotnet::fmt_float_custom(v, &f)
    }

    /// Draw with full event info.
    ///
    /// `ui`: parent. Returns [`NumericOutput`].
    pub fn show(self, ui: &mut Ui) -> NumericOutput<f32> {
        let id = self.id.unwrap_or_else(|| ui.next_auto_id());
        let th = Theme::current();
        let enabled = ui.is_enabled();
        let cur = self.round(*self.value);
        let in_error = self.validation && (cur > self.max || cur < self.min);
        let editing = ui.memory(|m| m.has_focus(id));
        let border = if editing { IN_EDIT_BORDER } else if in_error { named::RED } else { th.controls_border };
        let allow_neg = self.min < 0.0;
        let accept = move |c: char| c.is_ascii_digit() || c == '.' || c == ',' || (c == '-' && allow_neg);
        let (resp, began, ended, typing) = edit_box(ui, id, &self.fmt(cur), self.width, enabled, border, &accept);
        let mut out = NumericOutput { response: resp, committed: None, on_the_fly: None, began_edit: began, ended_edit: false, in_error };
        if let Some(t) = typing {
            out.on_the_fly = parse_f32(&t);
        }
        if let Some(t) = ended {
            out.ended_edit = true;
            // C# OnEndEdit: parse; same value or parse error -> just refresh
            if let Some(nv) = parse_f32(&t) {
                let nv = self.round(nv);
                if nv != cur && !(self.force_min_max && (nv < self.min || nv > self.max)) {
                    *self.value = nv;
                    out.committed = Some((cur, nv));
                    out.response.mark_changed();
                }
            }
        }
        out
    }
}

impl Widget for DecimalInputRanged<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

/// `float.Parse` with decimal comma tolerated.
fn parse_f32(s: &str) -> Option<f32> {
    let s = s.trim();
    let s = if s.contains(',') && !s.contains('.') { s.replace(',', ".") } else { s.to_owned() };
    lg_dotnet::parse_float(&s)
}

/// `IntegerInputRanged`: int with min/max validation and optional forced text.
pub struct IntegerInputRanged<'a> {
    value: &'a mut i32,
    min: i32,
    max: i32,
    validation: bool,
    force_min_max: bool,
    forced_text: Option<String>,
    width: f32,
    id: Option<Id>,
}

impl<'a> IntegerInputRanged<'a> {
    /// New input (C# defaults: range -100..100).
    ///
    /// `value`: bound value.
    pub fn new(value: &'a mut i32) -> Self {
        IntegerInputRanged { value, min: -100, max: 100, validation: true, force_min_max: false, forced_text: None, width: 78.0, id: None }
    }
    /// `MinValue`/`MaxValue`.
    pub fn range(mut self, min: i32, max: i32) -> Self {
        self.min = min;
        self.max = max;
        self
    }
    /// `ValidationEnabled`.
    pub fn validation(mut self, v: bool) -> Self {
        self.validation = v;
        self
    }
    /// `ForceMinMax`.
    pub fn force_min_max(mut self, v: bool) -> Self {
        self.force_min_max = v;
        self
    }
    /// `ForcedText` (shown instead of the number; no error border).
    pub fn forced_text(mut self, t: Option<String>) -> Self {
        self.forced_text = t;
        self
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Explicit id.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Draw with full event info.
    ///
    /// `ui`: parent. Returns [`NumericOutput`].
    pub fn show(self, ui: &mut Ui) -> NumericOutput<i32> {
        let id = self.id.unwrap_or_else(|| ui.next_auto_id());
        let th = Theme::current();
        let enabled = ui.is_enabled();
        let cur = *self.value;
        let in_error = self.validation && (cur > self.max || cur < self.min);
        let editing = ui.memory(|m| m.has_focus(id));
        let border = if editing {
            IN_EDIT_BORDER
        } else if in_error && self.forced_text.is_none() {
            named::RED
        } else {
            th.controls_border
        };
        let allow_neg = self.min < 0;
        let accept = move |c: char| c.is_ascii_digit() || (c == '-' && allow_neg);
        let display = self.forced_text.clone().unwrap_or_else(|| cur.to_string());
        let (resp, began, ended, typing) = edit_box(ui, id, &display, self.width, enabled, border, &accept);
        let mut out = NumericOutput { response: resp, committed: None, on_the_fly: None, began_edit: began, ended_edit: false, in_error };
        if let Some(t) = typing {
            out.on_the_fly = lg_dotnet::parse_int(t.trim());
        }
        if let Some(t) = ended {
            out.ended_edit = true;
            if let Some(nv) = lg_dotnet::parse_int(t.trim()) {
                if nv != cur && !(self.force_min_max && (nv < self.min || nv > self.max)) {
                    *self.value = nv;
                    out.committed = Some((cur, nv));
                    out.response.mark_changed();
                }
            }
        }
        out
    }
}

impl Widget for IntegerInputRanged<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

/// Painted spin button (egui fonts lack the triangle glyphs, so draw the arrow).
///
/// `ui`: parent. `enabled`: false greys it and ignores clicks. `up`: arrow direction.
/// Returns the click response.
fn spin_button(ui: &mut Ui, enabled: bool, up: bool) -> egui::Response {
    let sz = Vec2::new(16.0, 11.0);
    let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(sz, sense);
    let th = Theme::current();
    // flat button face, lighter on hover
    let fill = if resp.is_pointer_button_down_on() && enabled {
        ui.visuals().widgets.active.bg_fill
    } else if resp.hovered() && enabled {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    ui.painter().rect_filled(rect, 0.0, fill);
    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, th.controls_border), egui::StrokeKind::Inside);
    // triangle arrow
    let c = rect.center();
    let (hw, hh) = (3.5, 2.0);
    let pts = if up {
        vec![egui::pos2(c.x - hw, c.y + hh), egui::pos2(c.x + hw, c.y + hh), egui::pos2(c.x, c.y - hh)]
    } else {
        vec![egui::pos2(c.x - hw, c.y - hh), egui::pos2(c.x + hw, c.y - hh), egui::pos2(c.x, c.y + hh)]
    };
    let col = if enabled { th.form_fore_color } else { ui.visuals().widgets.noninteractive.fg_stroke.color.gamma_multiply(0.5) };
    ui.painter().add(egui::Shape::convex_polygon(pts, col, egui::Stroke::NONE));
    resp
}

/// `NumericUpDown` (WinForms spinner in a themed border).
pub struct NumericUpDown<'a> {
    value: &'a mut f64,
    min: f64,
    max: f64,
    increment: f64,
    decimals: usize,
    width: f32,
    id: Option<Id>,
}

impl<'a> NumericUpDown<'a> {
    /// New spinner (WinForms defaults: 0..100, increment 1, 0 decimals).
    ///
    /// `value`: bound value (clamped on show).
    pub fn new(value: &'a mut f64) -> Self {
        NumericUpDown { value, min: 0.0, max: 100.0, increment: 1.0, decimals: 0, width: 100.0, id: None }
    }
    /// `Minimum`/`Maximum`.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }
    /// `Increment`.
    pub fn increment(mut self, i: f64) -> Self {
        self.increment = i;
        self
    }
    /// `DecimalPlaces`.
    pub fn decimal_places(mut self, d: usize) -> Self {
        self.decimals = d;
        self
    }
    /// Width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Explicit id.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }
}

impl Widget for NumericUpDown<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let id = self.id.unwrap_or_else(|| ui.next_auto_id());
        let th = Theme::current();
        let enabled = ui.is_enabled();
        let mut changed = false;
        let clamp = |v: f64| lg_dotnet::round_double(v, self.decimals as i32).clamp(self.min, self.max);
        let mut cur = clamp(*self.value);
        let fmt = |v: f64| lg_dotnet::fmt_fixed(v, self.decimals);
        let editing = ui.memory(|m| m.has_focus(id));
        let allow_neg = self.min < 0.0;
        let dec = self.decimals > 0;
        let accept = move |c: char| c.is_ascii_digit() || (c == '.' && dec) || (c == ',' && dec) || (c == '-' && allow_neg);
        let border = if editing { IN_EDIT_BORDER } else { th.controls_border };
        let r = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let (resp, _b, ended, _t) = edit_box(ui, id, &fmt(cur), self.width - 16.0, enabled, border, &accept);
                if let Some(t) = ended {
                    if let Some(v) = lg_dotnet::parse_double(&t.replace(',', ".")) {
                        let nv = clamp(v);
                        if nv != cur {
                            cur = nv;
                            changed = true;
                        }
                    }
                }
                // spin buttons
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let up = spin_button(ui, enabled, true);
                    let dn = spin_button(ui, enabled, false);
                    if up.clicked() {
                        cur = clamp(cur + self.increment);
                        changed = true;
                    }
                    if dn.clicked() {
                        cur = clamp(cur - self.increment);
                        changed = true;
                    }
                });
                // keys & wheel when focused/hovered; WinForms wheel = 3 lines
                if enabled && ui.memory(|m| m.has_focus(id)) {
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowUp)) {
                        cur = clamp(cur + self.increment);
                        changed = true;
                    }
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowDown)) {
                        cur = clamp(cur - self.increment);
                        changed = true;
                    }
                }
                if enabled && resp.hovered() {
                    let n = super::graph::wheel_notches(ui);
                    if n != 0 {
                        cur = clamp(cur + n as f64 * self.increment * 3.0);
                        changed = true;
                    }
                }
                resp
            })
            .inner;
        if changed {
            *self.value = cur;
            let mut r = r;
            r.mark_changed();
            return r;
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One headless frame with two integer inputs.
    fn two_fields(ctx: &egui::Context, a: &mut i32, b: &mut i32, events: Vec<egui::Event>) {
        let ri = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300.0, 200.0))), events, ..Default::default() };
        let mut out = ctx.run_ui(ri, |ui| {
            IntegerInputRanged::new(a).range(0, 9999).id(Id::unique("num-a")).show(ui);
            IntegerInputRanged::new(b).range(0, 9999).id(Id::unique("num-b")).show(ui);
        });
        out.textures_delta.clear();
    }

    /// Mouse press + release at `p`.
    fn click(p: egui::Pos2) -> [Vec<egui::Event>; 2] {
        let btn = |pressed| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        [vec![egui::Event::PointerMoved(p), btn(true)], vec![btn(false)]]
    }

    #[test]
    fn edit_commits_when_focus_jumps_to_next_field() {
        let ctx = egui::Context::default();
        let (mut a, mut b) = (1000, 5);
        two_fields(&ctx, &mut a, &mut b, vec![]);
        let ra = ctx.read_response(Id::unique("num-a")).expect("a").rect;
        let rb = ctx.read_response(Id::unique("num-b")).expect("b").rect;
        for ev in click(ra.center()) {
            two_fields(&ctx, &mut a, &mut b, ev);
        }
        let all = egui::Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND };
        two_fields(&ctx, &mut a, &mut b, vec![all, egui::Event::Text("1500".into())]);
        // click straight into the second field: first one must keep its value
        for ev in click(rb.center()) {
            two_fields(&ctx, &mut a, &mut b, ev);
        }
        two_fields(&ctx, &mut a, &mut b, vec![]);
        assert_eq!((a, b), (1500, 5));
    }

    #[test]
    fn parse_comma() {
        assert_eq!(parse_f32("1,5"), Some(1.5));
        assert_eq!(parse_f32(" -2.25 "), Some(-2.25));
        assert_eq!(parse_f32(""), None);
    }
}
