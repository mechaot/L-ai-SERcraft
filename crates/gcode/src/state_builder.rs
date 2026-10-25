// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `StateBuilder.cs`: `StatePositionBuilder`, `StateBuilder`, `ModalElement`, `G2G3Helper`.
//!
//! C# `JogCommand` is the `is_jog` flag of [`GrblCommand`].

use crate::command::{Element, GrblCommand};
use crate::config::GrblConfST;
use crate::program_range::SRange;
use crate::{GResult, GcodeError};
use lg_dotnet::{
    dec_add, dec_div, dec_mul, dec_sub, decimal_to_f64, f64_to_decimal, round_double, Decimal, TimeSpan,
};
use rust_decimal::prelude::ToPrimitive;
use std::f64::consts::PI;
use std::ops::{Deref, DerefMut};

/// `Math.Min(double, double)` with .NET NaN propagation.
fn dn_min(a: f64, b: f64) -> f64 {
    if a < b || a.is_nan() { a } else { b }
}

/// `Math.Max(double, double)` with .NET NaN propagation.
fn dn_max(a: f64, b: f64) -> f64 {
    if a > b || a.is_nan() { a } else { b }
}

fn ov<T>(v: Option<T>) -> GResult<T> {
    v.ok_or(GcodeError::Overflow)
}

/// `System.Windows.Rect` (only the members used).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    /// `new Rect(x, y, w, h)`; C# throws on negative size, here Err.
    ///
    /// `x`,`y`: origin. `width`,`height`: size (>= 0).
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> GResult<Self> {
        if width < 0.0 || height < 0.0 {
            return Err(GcodeError::Argument("Width and Height must be non-negative".into()));
        }
        Ok(Self { x, y, width, height })
    }

    /// `new Rect(Point, Point)`: box spanning both points.
    ///
    /// `p1`,`p2`: corner points `(x, y)`.
    pub fn from_points(p1: (f64, f64), p2: (f64, f64)) -> Self {
        let x = dn_min(p1.0, p2.0);
        let y = dn_min(p1.1, p2.1);
        Self {
            x,
            y,
            width: dn_max(dn_max(p1.0, p2.0) - x, 0.0),
            height: dn_max(dn_max(p1.1, p2.1) - y, 0.0),
        }
    }

    /// `Rect.Offset(dx, dy)`.
    ///
    /// `dx`,`dy`: shift.
    pub fn offset(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
    }
}

/// Modal group value with allowed options (C# `StateBuilder.ModalElement`).
#[derive(Debug, Clone)]
pub struct ModalElement {
    elem: Element,
    options: Vec<Element>,
    default: Element,
    settled: bool,
}

impl ModalElement {
    /// Ctor.
    ///
    /// `defval`: default literal (e.g. "G0"). `options`: other allowed literals.
    pub fn new(defval: &str, options: &[&str]) -> Self {
        let d = Element::lit(defval);
        let mut opts = vec![d.clone()];
        opts.extend(options.iter().map(|o| Element::lit(o)));
        Self { elem: d.clone(), options: opts, default: d, settled: false }
    }

    /// Equals the default.
    pub fn is_default(&self) -> bool {
        self.elem == self.default
    }

    /// Was set at least once.
    pub fn is_settled(&self) -> bool {
        self.settled
    }

    /// Take `e` when it is one of the options.
    ///
    /// `e`: word of the command (None ignored).
    pub fn update(&mut self, e: Option<&Element>) {
        if let Some(e) = e
            && self.options.contains(e)
        {
            self.elem = e.clone();
            self.settled = true;
        }
    }
}

impl Deref for ModalElement {
    type Target = Element;
    /// Underlying element.
    fn deref(&self) -> &Element {
        &self.elem
    }
}

/// Position axis accumulating moves (C# `CumulativeElement`).
#[derive(Debug, Clone)]
pub struct CumulativeElement {
    elem: Element,
    default: Element,
    settled: bool,
    previous: Decimal,
}

impl CumulativeElement {
    /// Ctor.
    ///
    /// `defval`: default literal like "X0".
    pub fn new(defval: &str) -> Self {
        let d = Element::lit(defval);
        Self { elem: d.clone(), default: d, settled: false, previous: Decimal::ZERO }
    }

    /// Equals the default.
    pub fn is_default(&self) -> bool {
        self.elem == self.default
    }

    /// Was set at least once.
    pub fn is_settled(&self) -> bool {
        self.settled
    }

    /// Advance by a move.
    ///
    /// `e`: word (None keeps value). `absolute`: absolute mode. `offset`: WCO offset.
    pub fn update(&mut self, e: Option<&Element>, absolute: bool, offset: Decimal) -> GResult<()> {
        self.previous = self.elem.number;
        if let Some(e) = e {
            self.elem.number = if absolute { ov(dec_add(offset, e.number))? } else { ov(dec_add(self.elem.number, e.number))? };
            self.settled = true;
        }
        Ok(())
    }

    /// Value before the last update.
    pub fn previous(&self) -> Decimal {
        self.previous
    }
}

impl Deref for CumulativeElement {
    type Target = Element;
    /// Underlying element.
    fn deref(&self) -> &Element {
        &self.elem
    }
}

/// Last-value word (F, S) (C# `LastValueElement`).
#[derive(Debug, Clone)]
pub struct LastValueElement {
    elem: Element,
    default: Element,
    settled: bool,
}

impl LastValueElement {
    /// Ctor.
    ///
    /// `defval`: default literal like "F0".
    pub fn new(defval: &str) -> Self {
        let d = Element::lit(defval);
        Self { elem: d.clone(), default: d, settled: false }
    }

    /// Equals the default.
    pub fn is_default(&self) -> bool {
        self.elem == self.default
    }

    /// Was set at least once.
    pub fn is_settled(&self) -> bool {
        self.settled
    }

    /// Take the value.
    ///
    /// `e`: word (None ignored).
    pub fn update(&mut self, e: Option<&Element>) {
        if let Some(e) = e {
            self.elem.number = e.number;
            self.settled = true;
        }
    }
}

impl Deref for LastValueElement {
    type Target = Element;
    /// Underlying element.
    fn deref(&self) -> &Element {
        &self.elem
    }
}

/// Gcode parser state: modal groups (C# `StateBuilder`).
#[derive(Debug, Clone)]
pub struct StateBuilder {
    pub motion_mode: ModalElement,
    pub coordinate_select: ModalElement,
    pub plane_select: ModalElement,
    pub distance_mode: ModalElement,
    pub arc_distance_mode: ModalElement,
    pub feed_rate_mode: ModalElement,
    pub units_mode: ModalElement,
    pub cutter_radius_compensation: ModalElement,
    pub tool_length_offset: ModalElement,
    pub program_mode: ModalElement,
    pub coolant_state: ModalElement,
    pub spindle_state: ModalElement,
}

impl Default for StateBuilder {
    /// Fresh state with all modal defaults.
    fn default() -> Self {
        Self::new()
    }
}

impl StateBuilder {
    /// Fresh state.
    pub fn new() -> Self {
        Self {
            motion_mode: ModalElement::new("G0", &["G1", "G2", "G3", "G38.2", "G38.3", "G38.4", "G38.5", "G80"]),
            coordinate_select: ModalElement::new("G54", &["G55", "G56", "G57", "G58", "G59"]),
            plane_select: ModalElement::new("G17", &["G18", "G19"]),
            distance_mode: ModalElement::new("G90", &["G91"]),
            arc_distance_mode: ModalElement::new("G91.1", &[]),
            feed_rate_mode: ModalElement::new("G94", &["G93"]),
            units_mode: ModalElement::new("G21", &["G20"]),
            cutter_radius_compensation: ModalElement::new("G40", &[]),
            tool_length_offset: ModalElement::new("G49", &["G43.1"]),
            program_mode: ModalElement::new("M0", &["M1", "M2", "M30"]),
            coolant_state: ModalElement::new("M9", &["M7", "M8"]),
            spindle_state: ModalElement::new("M5", &["M3", "M4"]),
        }
    }

    /// Update modals from an already built command (jog commands ignored).
    ///
    /// `cmd`: built command.
    pub fn update_modals_nb(&mut self, cmd: &GrblCommand) {
        if cmd.is_jog() {
            return;
        }
        let g = cmd.g();
        self.motion_mode.update(g);
        self.coordinate_select.update(g);
        self.plane_select.update(g);
        self.distance_mode.update(g);
        self.arc_distance_mode.update(g);
        self.feed_rate_mode.update(g);
        self.cutter_radius_compensation.update(g);
        self.tool_length_offset.update(g);
        let m = cmd.m();
        self.program_mode.update(m);
        self.coolant_state.update(m);
        self.spindle_state.update(m);
    }

    /// Settled modals, fixed group order (motion and units are not listed, like C#).
    pub fn get_settled_modals(&self) -> Vec<Element> {
        [
            &self.coordinate_select,
            &self.plane_select,
            &self.distance_mode,
            &self.arc_distance_mode,
            &self.feed_rate_mode,
            &self.cutter_radius_compensation,
            &self.tool_length_offset,
            &self.program_mode,
            &self.coolant_state,
            &self.spindle_state,
        ]
        .into_iter()
        .filter(|m| m.is_settled())
        .map(|m| (**m).clone())
        .collect()
    }
}

/// Tracks position, feed, spindle and execution time (C# `StatePositionBuilder`).
#[derive(Debug, Clone)]
pub struct StatePositionBuilder {
    base: StateBuilder,
    support_pwm: bool,
    cur_x: CumulativeElement,
    cur_y: CumulativeElement,
    cur_z: CumulativeElement,
    wco_x: Decimal,
    wco_y: Decimal,
    wco_z: Decimal,
    cur_f: LastValueElement,
    cur_s: LastValueElement,
    /// Arc of the last time computation (None after a non-arc segment); can be stale on commands
    /// that do not compute a segment length.
    pub last_arc_helper_result: Option<G2G3Helper>,
    /// Counter bumped each time a new arc helper is created (detects "stale" results).
    pub arc_generation: u64,
}

impl Default for StatePositionBuilder {
    /// Same as `new()`.
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for StatePositionBuilder {
    type Target = StateBuilder;
    /// Base class view.
    fn deref(&self) -> &StateBuilder {
        &self.base
    }
}

impl DerefMut for StatePositionBuilder {
    /// Base class view (mutable).
    fn deref_mut(&mut self) -> &mut StateBuilder {
        &mut self.base
    }
}

impl StatePositionBuilder {
    /// Ctor; reads setting "Support Hardware PWM" now (C# field initialiser).
    pub fn new() -> Self {
        Self {
            base: StateBuilder::new(),
            support_pwm: lg_base::settings::get("Support Hardware PWM", true),
            cur_x: CumulativeElement::new("X0"),
            cur_y: CumulativeElement::new("Y0"),
            cur_z: CumulativeElement::new("Z0"),
            wco_x: Decimal::ZERO,
            wco_y: Decimal::ZERO,
            wco_z: Decimal::ZERO,
            cur_f: LastValueElement::new("F0"),
            cur_s: LastValueElement::new("S0"),
            last_arc_helper_result: None,
            arc_generation: 0,
        }
    }

    /// C# `AnalyzeCommand`: update state with `cmd`, optionally compute its duration.
    ///
    /// `cmd`: command (built here and un-built after when it was not built). `compute`: want
    /// the duration. `conf`: grbl config (needed for moves when `compute`). Returns the duration.
    pub fn analyze_command(&mut self, cmd: &mut GrblCommand, compute: bool, conf: Option<&GrblConfST>) -> GResult<TimeSpan> {
        let delete = !cmd.just_built();
        if !cmd.just_built() {
            cmd.build_helper();
        }
        let r = self.analyze_built(cmd, compute, conf);
        // C# would leave the helper on exception; keep same
        if r.is_ok() && delete {
            cmd.delete_helper();
        }
        r
    }

    /// Body of `AnalyzeCommand` after the build.
    fn analyze_built(&mut self, cmd: &GrblCommand, compute: bool, conf: Option<&GrblConfST>) -> GResult<TimeSpan> {
        self.base.update_modals_nb(cmd);
        self.update_wco(cmd)?;
        self.update_xyz(cmd)?;
        self.update_fs(cmd);
        if compute { self.compute_execution_time(cmd, conf) } else { Ok(TimeSpan::ZERO) }
    }

    fn update_fs(&mut self, cmd: &GrblCommand) {
        if cmd.is_jog() {
            return;
        }
        self.cur_f.update(cmd.f());
        self.cur_s.update(cmd.s());
    }

    fn update_xyz(&mut self, cmd: &GrblCommand) -> GResult<()> {
        if cmd.is_jog() {
            let abs = cmd.is_absolute_coord();
            self.cur_x.update(cmd.x(), abs, self.wco_x)?;
            self.cur_y.update(cmd.y(), abs, self.wco_y)?;
            self.cur_z.update(cmd.z(), abs, self.wco_z)?;
        } else if cmd.is_movement() {
            let abs = self.abs();
            self.cur_x.update(cmd.x(), abs, self.wco_x)?;
            self.cur_y.update(cmd.y(), abs, self.wco_y)?;
            self.cur_z.update(cmd.z(), abs, self.wco_z)?;
        }
        Ok(())
    }

    fn update_wco(&mut self, cmd: &GrblCommand) -> GResult<()> {
        if cmd.is_set_wco() {
            if let Some(x) = cmd.x() {
                self.wco_x = ov(dec_sub(self.cur_x.number, x.number))?;
            }
            if let Some(y) = cmd.y() {
                self.wco_y = ov(dec_sub(self.cur_y.number, y.number))?;
            }
            if let Some(z) = cmd.z() {
                self.wco_z = ov(dec_sub(self.cur_z.number, z.number))?;
            }
        }
        Ok(())
    }

    /// Any WCO offset set.
    pub fn has_wco(&self) -> bool {
        self.wco_x != Decimal::ZERO || self.wco_y != Decimal::ZERO || self.wco_z != Decimal::ZERO
    }
    /// WCO X.
    pub fn wco_x(&self) -> Decimal {
        self.wco_x
    }
    /// WCO Y.
    pub fn wco_y(&self) -> Decimal {
        self.wco_y
    }
    /// WCO Z.
    pub fn wco_z(&self) -> Decimal {
        self.wco_z
    }
    /// Current F.
    pub fn f(&self) -> &LastValueElement {
        &self.cur_f
    }
    /// Current S.
    pub fn s(&self) -> &LastValueElement {
        &self.cur_s
    }
    /// Current X.
    pub fn x(&self) -> &CumulativeElement {
        &self.cur_x
    }
    /// Current Y.
    pub fn y(&self) -> &CumulativeElement {
        &self.cur_y
    }
    /// Current Z.
    pub fn z(&self) -> &CumulativeElement {
        &self.cur_z
    }

    /// X or Y changed, or arc mode.
    pub fn true_movement(&self) -> bool {
        self.cur_x.number != self.cur_x.previous || self.cur_y.number != self.cur_y.previous || self.g2g3()
    }

    /// C# `ComputeExecutionTime`.
    fn compute_execution_time(&mut self, cmd: &GrblCommand, conf: Option<&GrblConfST>) -> GResult<TimeSpan> {
        let f = if cmd.is_jog() && cmd.f().is_some() { cmd.f().map(|e| e.number).unwrap_or_default() } else { self.cur_f.number };
        let null_conf = || GcodeError::Other("NullReferenceException: conf".into());

        if self.g0() && cmd.is_linear_movement() {
            let conf = conf.ok_or_else(null_conf)?;
            let len = decimal_to_f64(self.get_segment_lenght(cmd)?);
            // todo(upstream): better xy computation if x/y max speeds differ
            try_minutes(len / decimal_to_f64(conf.max_rate_x()))
        } else if self.g1g2g3() && cmd.is_movement() && f != Decimal::ZERO {
            let conf = conf.ok_or_else(null_conf)?;
            let len = decimal_to_f64(self.get_segment_lenght(cmd)?);
            let mr = conf.max_rate_x();
            let m = if f < mr { f } else { mr };
            try_minutes(len / decimal_to_f64(m))
        } else if cmd.is_pause() {
            if let Some(p) = cmd.p() {
                try_seconds(decimal_to_f64(p.number))
            } else if let Some(s) = cmd.s() {
                try_seconds(decimal_to_f64(s.number))
            } else {
                Ok(TimeSpan::ZERO)
            }
        } else {
            Ok(TimeSpan::ZERO)
        }
    }

    /// Length of the last segment (decimal), sets `last_arc_helper_result` for arcs.
    fn get_segment_lenght(&mut self, cmd: &GrblCommand) -> GResult<Decimal> {
        self.last_arc_helper_result = None;
        if cmd.is_linear_movement() {
            Ok(lg_base::tools::math_helper::linear_distance(self.cur_x.previous, self.cur_y.previous, self.cur_x.number, self.cur_y.number))
        } else if cmd.is_arc_movement() {
            let ah = self.get_arc_helper(cmd)?;
            Ok(f64_to_decimal(ah.abs_lenght()))
        } else {
            Ok(Decimal::ZERO)
        }
    }

    /// Alpha (0..255) of the preview line for the current S.
    ///
    /// `range`: spindle range of the program. Returns the alpha.
    pub fn get_current_alpha(&self, range: &SRange) -> GResult<i32> {
        if !self.laser_burning() {
            Ok(150) // supportPWM ? 150 : 50
        } else if self.support_pwm && range.valid_range() && self.cur_s.is_settled() {
            let num = ov(dec_mul(ov(dec_sub(self.cur_s.number, range.s.min))?, Decimal::from(255)))?;
            let den = ov(dec_sub(range.s.max, range.s.min))?;
            let q = ov(dec_div(num, den))?;
            q.trunc().to_i32().ok_or(GcodeError::Overflow)
        } else {
            Ok(255)
        }
    }

    /// G2.
    pub fn g2(&self) -> bool {
        self.base.motion_mode.number == Decimal::from(2)
    }
    /// G2 or G3.
    pub fn g2g3(&self) -> bool {
        let n = self.base.motion_mode.number;
        n == Decimal::from(2) || n == Decimal::from(3)
    }
    /// G0 or G1.
    pub fn g0g1(&self) -> bool {
        let n = self.base.motion_mode.number;
        n == Decimal::ZERO || n == Decimal::ONE
    }
    /// G0.
    pub fn g0(&self) -> bool {
        self.base.motion_mode.number == Decimal::ZERO
    }
    /// G1, G2 or G3.
    pub fn g1g2g3(&self) -> bool {
        let n = self.base.motion_mode.number;
        n == Decimal::ONE || n == Decimal::from(2) || n == Decimal::from(3)
    }
    /// G90 active.
    pub fn abs(&self) -> bool {
        self.base.distance_mode.number == Decimal::from(90)
    }
    /// M3 or M4 active.
    pub fn m3m4(&self) -> bool {
        let n = self.base.spindle_state.number;
        n == Decimal::from(3) || n == Decimal::from(4)
    }
    /// Laser is burning now.
    pub fn laser_burning(&self) -> bool {
        let sp = self.base.spindle_state.number;
        (!self.support_pwm || self.cur_s.number > Decimal::ZERO)
            && (sp == Decimal::from(3) || (sp == Decimal::from(4) && self.base.motion_mode.number != Decimal::ZERO))
    }

    /// Reset position to zero (homing).
    pub fn homing(&mut self) {
        self.cur_x = CumulativeElement::new("X0");
        self.cur_y = CumulativeElement::new("Y0");
        self.cur_z = CumulativeElement::new("Z0");
    }

    /// Build the arc geometry for `cmd` with the current state; also stored in the state.
    ///
    /// `cmd`: arc command. Returns the helper (a clone is kept in `last_arc_helper_result`).
    pub fn get_arc_helper(&mut self, cmd: &GrblCommand) -> GResult<G2G3Helper> {
        let h = G2G3Helper::new(self, cmd)?;
        self.arc_generation += 1;
        self.last_arc_helper_result = Some(h.clone());
        Ok(h)
    }
}

/// `TimeSpan.FromMinutes` that returns Err where .NET throws.
///
/// `v`: minutes.
fn try_minutes(v: f64) -> GResult<TimeSpan> {
    check_interval(v, 60_000.0)?;
    Ok(TimeSpan::from_minutes(v))
}

/// `TimeSpan.FromSeconds` that returns Err where .NET throws.
///
/// `v`: seconds.
fn try_seconds(v: f64) -> GResult<TimeSpan> {
    check_interval(v, 1_000.0)?;
    Ok(TimeSpan::from_seconds(v))
}

/// Same range test as `TimeSpan.Interval`.
///
/// `v`: amount. `scale`: ms per unit.
fn check_interval(v: f64, scale: f64) -> GResult<()> {
    if v.is_nan() {
        return Err(GcodeError::Argument("NaN".into()));
    }
    let ms = v * scale + if v >= 0.0 { 0.5 } else { -0.5 };
    if ms > (i64::MAX / 10_000) as f64 || ms < (i64::MIN / 10_000) as f64 {
        return Err(GcodeError::Overflow);
    }
    Ok(())
}

/// Geometry of a G2/G3 arc (C# `G2G3Helper`).
#[derive(Debug, Clone, PartialEq)]
pub struct G2G3Helper {
    pub center_x: f64,
    pub center_y: f64,
    pub cw: bool,
    pub ray: f64,
    pub rect_x: f64,
    pub rect_y: f64,
    pub rect_w: f64,
    pub rect_h: f64,
    pub b_box: Rect,
    pub start_angle: f64,
    pub end_angle: f64,
    pub angular_width: f64,
}

const A0: f64 = 0.0;
const A90: f64 = PI / 2.0;
const A180: f64 = PI;
const A270: f64 = PI * 3.0 / 2.0;

impl G2G3Helper {
    /// C# ctor.
    ///
    /// `spb`: state after the command was applied. `cmd`: arc command (built here if needed).
    pub fn new(spb: &StatePositionBuilder, cmd: &GrblCommand) -> GResult<Self> {
        // work on a built copy when needed (C# builds then deletes; rewrite of line is not observable here)
        let mut tmp;
        let cmd = if cmd.just_built() {
            cmd
        } else {
            tmp = cmd.clone();
            tmp.build_helper();
            &tmp
        };
        let cw = spb.g2();
        let a_x = decimal_to_f64(spb.cur_x.previous);
        let a_y = decimal_to_f64(spb.cur_y.previous);
        let b_x = decimal_to_f64(spb.cur_x.number);
        let b_y = decimal_to_f64(spb.cur_y.number);
        let (o_x, o_y);
        if cmd.r().is_some() {
            o_x = ((a_x + b_x) / 2.0) - a_x;
            o_y = ((a_y + b_y) / 2.0) - a_y;
        } else {
            o_x = cmd.i().map(|e| decimal_to_f64(e.number)).unwrap_or(0.0);
            o_y = cmd.j().map(|e| decimal_to_f64(e.number)).unwrap_or(0.0);
        }
        let center_x = a_x + o_x;
        let center_y = a_y + o_y;
        let ray = (o_x * o_x + o_y * o_y).sqrt();
        let rect_x = center_x - ray;
        let rect_y = center_y - ray;
        let rect_w = 2.0 * ray;
        let rect_h = 2.0 * ray;
        let start_angle = calculate_angle(center_x, center_y, a_x, a_y);
        let end_angle = calculate_angle(center_x, center_y, b_x, b_y);
        let angular_width = angular_distance(start_angle, end_angle, cw);

        let b_box = if a_x == b_x && a_y == b_y {
            Rect::new(rect_x, rect_y, rect_w, rect_h)?
        } else if cw {
            bb_box(end_angle, start_angle, ray, center_x, center_y)
        } else {
            bb_box(start_angle, end_angle, ray, center_x, center_y)
        };
        Ok(Self { center_x, center_y, cw, ray, rect_x, rect_y, rect_w, rect_h, b_box, start_angle, end_angle, angular_width })
    }

    /// Signed arc length.
    pub fn lenght(&self) -> f64 {
        self.angular_width * self.ray
    }

    /// Absolute arc length.
    pub fn abs_lenght(&self) -> f64 {
        self.lenght().abs()
    }

    /// Quadrant 1..4 of an angle.
    ///
    /// `angle`: radians.
    pub fn get_quadrant(angle: f64) -> i32 {
        let t = angle % (2.0 * PI);
        if (A0..A90).contains(&t) {
            1
        } else if (A90..A180).contains(&t) {
            2
        } else if (A180..A270).contains(&t) {
            3
        } else {
            4
        }
    }

    /// Arc bounding box (Oleg Petrochenko algorithm).
    ///
    /// `start_angle`,`end_angle`: radians (ccw from start to end). `r`: radius. `center_x`,`center_y`: center.
    pub fn bbbox(start_angle: f64, end_angle: f64, r: f64, center_x: f64, center_y: f64) -> Rect {
        bb_box(start_angle, end_angle, r, center_x, center_y)
    }
}

/// Angle of the line between two points and the X axis, by quadrant.
fn calculate_angle(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let mut angle;
    if x1 == x2 && y1 == y2 {
        return 0.0;
    } else if x1 == x2 {
        angle = PI / 2.0;
        if y1 > y2 {
            angle += PI;
        }
    } else if y1 == y2 {
        angle = 0.0;
        if x1 > x2 {
            angle += PI;
        }
    } else {
        angle = ((y2 - y1) / (x2 - x1)).abs().atan();
        if x1 > x2 && y1 < y2 {
            angle = PI - angle;
        } else if x1 > x2 && y1 > y2 {
            angle += PI;
        } else if x1 < x2 && y1 > y2 {
            angle = 2.0 * PI - angle;
        }
    }
    angle
}

/// Signed angular distance.
fn angular_distance(a_a: f64, b_a: f64, cw: bool) -> f64 {
    if cw {
        if b_a >= a_a { b_a - 2.0 * PI - a_a } else { b_a - a_a }
    } else {
        -(if a_a >= b_a { a_a - 2.0 * PI - b_a } else { a_a - b_a })
    }
}

/// Bounding box of the arc around (0,0), then offset to the center.
fn bb_box(start_angle: f64, end_angle: f64, r: f64, center_x: f64, center_y: f64) -> Rect {
    let start_quad = (G2G3Helper::get_quadrant(start_angle) - 1) as usize;
    let end_quad = (G2G3Helper::get_quadrant(end_angle) - 1) as usize;

    let st = (round_double(r * start_angle.cos(), 14), round_double(r * start_angle.sin(), 14));
    let en = (round_double(r * end_angle.cos(), 14), round_double(r * end_angle.sin(), 14));

    let mut min_x = st.0;
    let mut min_y = st.1;
    let mut max_x = st.0;
    let mut max_y = st.1;
    if max_x < en.0 {
        max_x = en.0;
    }
    if max_y < en.1 {
        max_y = en.1;
    }
    if min_x > en.0 {
        min_x = en.0;
    }
    if min_y > en.1 {
        min_y = en.1;
    }

    let x_max = [[max_x, r, r, r], [max_x, max_x, r, r], [max_x, max_x, max_x, r], [max_x, max_x, max_x, max_x]];
    let y_max = [[max_y, max_y, max_y, max_y], [r, max_y, r, r], [r, max_y, max_y, r], [r, max_y, max_y, max_y]];
    let x_min = [[min_x, -r, min_x, min_x], [min_x, min_x, min_x, min_x], [-r, -r, min_x, -r], [-r, -r, min_x, min_x]];
    let y_min = [[min_y, -r, -r, min_y], [min_y, min_y, -r, min_y], [min_y, min_y, min_y, min_y], [-r, -r, -r, min_y]];

    let start_pt = (x_min[end_quad][start_quad], y_min[end_quad][start_quad]);
    let end_pt = (x_max[end_quad][start_quad], y_max[end_quad][start_quad]);
    let mut rv = Rect::from_points(start_pt, end_pt);
    // maths done around origin; add the real center at the end
    rv.offset(center_x, center_y);
    rv
}
