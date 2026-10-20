//! Port of `SvgConverter/gcodeRelated.cs` (static class `gcode`: G-code writer with modal compression).
//!
//! The C# class is fully static; here the same state lives in [`Gcode`], owned by the converter.
//! Settings the C# reads (`Settings.GetObject`, `GrblCore.Configuration.MaxPWM`) come in via [`SvgConvOptions`].

use crate::SvgError;
use crate::convert::LayerParams;
use crate::matrix::Point;
use lg_dotnet::{Decimal, decimal_to_f32, f64_to_i32, fmt_float, fmt_float_custom, fmt_i32};
use regex::Regex;

/// `LaserGRBL.Firmware` enum (only the value matters here).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Firmware {
    #[default]
    Grbl,
    Smoothie,
    Marlin,
    VigoWork,
}

/// Everything the C# SVG converter pulls from `Settings` / `GrblCore.Configuration`.
///
/// The caller fills this exactly where the C# would read the settings. Defaults = C# defaults.
#[derive(Clone, Debug)]
pub struct SvgConvOptions {
    /// Setting `"Support Hardware PWM"` (C# default `true`): S command vs M3/M5 switching.
    pub support_hardware_pwm: bool,
    /// Setting `"Disable G0 fast skip"` (default `false`): rapid moves use G1 when true.
    pub disable_g0_fast_skip: bool,
    /// Setting `"GrayScaleConversion.VectorizeOptions.BorderSpeed"` (int, default `1000`): G1 feed.
    pub border_speed: i32,
    /// Setting `"GrayScaleConversion.Gcode.LaserOptions.PowerMax"` (int, default `255`): S value when PWM supported.
    pub power_max: i32,
    /// Setting `"GrayScaleConversion.Gcode.LaserOptions.LaserOn"` (string, default `"M3"`).
    pub laser_on: String,
    /// Setting `"GrayScaleConversion.Gcode.LaserOptions.LaserOff"` (string, default `"M5"`).
    pub laser_off: String,
    /// `GrblCore.Configuration.MaxPWM` (C# `decimal`): S value when PWM is not supported.
    pub max_pwm: Decimal,
    /// Setting `"Firmware Type"` (default `Firmware.Grbl`); C# reads it once per process (static initialiser).
    pub firmware: Firmware,
}

impl Default for SvgConvOptions {
    /// C# defaults. `max_pwm` defaults to 1000 (typical GRBL `$30`).
    fn default() -> Self {
        SvgConvOptions {
            support_hardware_pwm: true,
            disable_g0_fast_skip: false,
            border_speed: 1000,
            power_max: 255,
            laser_on: "M3".into(),
            laser_off: "M5".into(),
            max_pwm: Decimal::from(1000),
            firmware: Firmware::Grbl,
        }
    }
}

/// State of the C# static class `gcode`.
#[derive(Debug)]
pub struct Gcode {
    format_code: &'static str,
    format_number: String,
    drag_compensation: bool,
    drag_radius: f32,
    drag_angle: f32,
    xy_feed: f32,
    spindle_speed: f32,
    spindle_cmd_on: String,
    spindle_cmd_off: String,
    active_laser_on: String,
    compress: bool,
    no_arcs: bool,
    angle_step: f32,
    decimal_places: usize,
    firmware: Firmware,
    rapidnum: i32,
    support_pwm: bool,
    apply_xy_feed_rate: bool,
    pub lastx: f32,
    pub lasty: f32,
    pub lastz: f32,
    pub lastg: f32,
    pub lastf: f32,
    pub lasts: f32,
    pub last_move_was_g0: bool,
    remaining_c: f32,
    seg_final_x: f32,
    seg_final_y: f32,
    seg_last_final_x: f32,
    seg_last_final_y: f32,
    drag_arc: bool,
    drag_1st_move: bool,
    orig_last_x: f32,
    orig_last_y: f32,
    orig_final_x: f32,
    orig_final_y: f32,
    drag_comp_i: f32,
    drag_comp_j: f32,
}

/// `fsqrt`.
fn fsqrt(x: f32) -> f32 {
    (x as f64).sqrt() as f32
}
/// `fvmag`.
fn fvmag(x: f32, y: f32) -> f32 {
    fsqrt(x * x + y * y)
}
/// `fdistance`.
fn fdistance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    fvmag(x2 - x1, y2 - y1)
}

impl Default for Gcode {
    /// Static field initialisers of the C# class (firmware Grbl until `setup`).
    fn default() -> Self {
        let mut g = Gcode {
            format_code: "0",
            format_number: "0.###".into(),
            drag_compensation: false,
            drag_radius: 5.0,
            drag_angle: 30.0,
            xy_feed: 1999.0,
            spindle_speed: 999.0,
            spindle_cmd_on: "M3".into(),
            spindle_cmd_off: "M4".into(),
            active_laser_on: "M3".into(),
            compress: true,
            no_arcs: false,
            angle_step: 1.0,
            decimal_places: 3,
            firmware: Firmware::Grbl,
            rapidnum: 0,
            support_pwm: true,
            apply_xy_feed_rate: true,
            lastx: 0.0,
            lasty: 0.0,
            lastz: 0.0,
            lastg: -1.0,
            lastf: 0.0,
            lasts: 0.0,
            last_move_was_g0: true,
            remaining_c: 10.0,
            seg_final_x: 0.0,
            seg_final_y: 0.0,
            seg_last_final_x: 0.0,
            seg_last_final_y: 0.0,
            drag_arc: false,
            drag_1st_move: true,
            orig_last_x: 0.0,
            orig_last_y: 0.0,
            orig_final_x: 0.0,
            orig_final_y: 0.0,
            drag_comp_i: 0.0,
            drag_comp_j: 0.0,
        };
        g.set_decimal_places(3);
        g
    }
}

impl Gcode {
    /// `gcode.setup(core)`: load settings snapshot, reset modal state.
    ///
    /// `o`: option snapshot (replaces `Settings.GetObject` + `GrblCore.Configuration.MaxPWM`).
    pub fn setup(&mut self, o: &SvgConvOptions) {
        self.firmware = o.firmware;
        self.support_pwm = o.support_hardware_pwm;
        self.set_decimal_places(self.decimal_places);
        self.derive_layer_values(o, o.border_speed, o.power_max, &o.laser_on);
        self.spindle_cmd_off = o.laser_off.clone();
        self.last_move_was_g0 = true;
        self.lastx = -1.0;
        self.lasty = -1.0;
        self.lastz = 0.0;
        self.lasts = -1.0;
        self.lastg = -1.0;
    }

    /// Derive feed, spindle speed and laser-on command from per-layer values.
    ///
    /// `o`: global options (PWM flag, max PWM, firmware). `border_speed`: G1 feed. `power_max`: S value
    /// (PWM mode). `laser_on`: laser-on command. No return value.
    fn derive_layer_values(&mut self, o: &SvgConvOptions, border_speed: i32, power_max: i32, laser_on: &str) {
        self.xy_feed = border_speed as f32;
        // PWM: S = power max, else S = controller max PWM
        if self.support_pwm {
            self.spindle_speed = power_max as f32;
        } else {
            self.spindle_speed = decimal_to_f32(o.max_pwm);
        }
        // Smoothieware wants 0.0..1.1
        if self.firmware == Firmware::Smoothie {
            self.spindle_speed /= 255.0f32;
        }
        self.spindle_cmd_on = laser_on.to_string();
        self.active_laser_on = laser_on.to_string();
    }

    /// Switch to the settings of one layer (no state reset, modal state kept).
    ///
    /// `o`: global options. `lp`: layer values. `out`: buffer; gets `"<mode> S0"` when PWM is on and
    /// the laser mode differs from the active one. No return value.
    pub fn set_layer(&mut self, o: &SvgConvOptions, lp: &LayerParams, out: &mut String) {
        let changed = self.active_laser_on != lp.laser_on;
        self.derive_layer_values(o, lp.border_speed, lp.power_max, &lp.laser_on);
        // PWM: laser mode only set once at start, so re-emit on change
        if self.support_pwm && changed {
            out.push_str(&format!("{} S0\r\n", lp.laser_on));
        }
    }

    /// `reduceGCode` getter. Returns whether modal compression is on.
    pub fn reduce_gcode(&self) -> bool {
        self.compress
    }

    /// `reduceGCode` setter. `v`: compression on/off.
    pub fn set_reduce_gcode(&mut self, v: bool) {
        self.compress = v;
        self.set_decimal_places(self.decimal_places);
    }

    /// `setRapidNum`. `num`: G number for rapid moves (0, or 1 when fast skip disabled).
    pub fn set_rapid_num(&mut self, num: i32) {
        self.rapidnum = num;
    }

    /// `setDecimalPlaces`. `num`: decimals; builds `0.###` (compress) or `0.000`.
    pub fn set_decimal_places(&mut self, num: usize) {
        let fill = if self.compress { '#' } else { '0' };
        let mut s = String::from("0.");
        while s.len() < num + 2 {
            s.push(fill);
        }
        self.format_number = s;
    }

    /// `getIntGCode`. `code`: letter. `tmp`: G-code line. Returns number after the code or -1.
    pub fn get_int_gcode(code: char, tmp: &str) -> Result<i32, SvgError> {
        let cmd = Self::get_str_gcode(code, tmp);
        if !cmd.is_empty() {
            return to_int16(&cmd[1..]);
        }
        Ok(-1)
    }

    /// `getStrGCode`. `code`: letter. `tmp`: line. Returns first `Gnn` match outside a comment or "".
    pub fn get_str_gcode(code: char, tmp: &str) -> String {
        let cmt = tmp.find('(');
        if cmt == Some(0) {
            return String::new();
        }
        let tmp = match cmt {
            Some(i) => &tmp[..i],
            None => tmp,
        };
        let re = Regex::new(&format!("{}\\d{{1,2}}", regex::escape(&code.to_string()))).expect("re");
        re.find(tmp).map(|m| m.as_str().to_string()).unwrap_or_default()
    }

    /// `getIntValue`. `code`: letter. `tmp`: line. Returns the int value or -1.
    pub fn get_int_value(code: char, tmp: &str) -> Result<i32, SvgError> {
        let cmd = Self::get_string_value(code, tmp);
        if !cmd.is_empty() {
            return to_int16(&cmd[1..]);
        }
        Ok(-1)
    }

    /// `getStringValue`. `code`: letter. `tmp`: line. Returns the last `X-1.5` style match or "".
    pub fn get_string_value(code: char, tmp: &str) -> String {
        // NOTE(upstream): '.' in the pattern is unescaped (any char), kept.
        let re = Regex::new(&format!("{}-?\\d+(.\\d+)?", regex::escape(&code.to_string()))).expect("re");
        re.find_iter(tmp).last().map(|m| m.as_str().to_string()).unwrap_or_default()
    }

    /// `frmtCode(int)`. `n`: G number. Returns text via format "0".
    pub fn frmt_code(&self, n: i32) -> String {
        fmt_i32(n, self.format_code)
    }

    /// `frmtNum(float)`. `n`: value. Returns text via the current number format.
    pub fn frmt_num(&self, n: f32) -> String {
        fmt_float_custom(n, &self.format_number)
    }

    /// `SpindleOn`. `out`: buffer. `cmt`: comment text.
    pub fn spindle_on(&self, out: &mut String, cmt: &str) {
        let cmt = if cmt.is_empty() { String::new() } else { format!(" ({cmt})") };
        if self.support_pwm {
            out.push_str(&format!("S{}{}\r\n", fmt_float(self.spindle_speed), cmt));
        } else {
            out.push_str(&format!("{}{}\r\n", self.spindle_cmd_on, cmt));
        }
    }

    /// `SpindleOff`. `out`: buffer. `cmt`: comment text.
    pub fn spindle_off(&self, out: &mut String, cmt: &str) {
        let cmt = if cmt.is_empty() { String::new() } else { format!(" ({cmt})") };
        if self.support_pwm {
            out.push_str(&format!("S0{cmt}\r\n"));
        } else {
            out.push_str(&format!("{}{}\r\n", self.spindle_cmd_off, cmt));
        }
    }

    /// `PutInitialCommand`. `out`: buffer.
    pub fn put_initial_command(&self, out: &mut String) {
        if self.support_pwm {
            out.push_str(&format!("{} S0\r\n", self.spindle_cmd_on));
        } else {
            out.push_str(&format!("{} S{}\r\n", self.spindle_cmd_off, fmt_float(self.spindle_speed)));
        }
    }

    /// `PutFinalCommand`. `out`: buffer.
    pub fn put_final_command(&self, out: &mut String) {
        out.push_str("M5 S0\r\n");
    }

    /// `PenDown`. `out`: buffer. `cmt`: comment.
    pub fn pen_down(&mut self, out: &mut String, cmt: &str) {
        self.drag_1st_move = true;
        self.orig_final_x = self.lastx;
        self.orig_final_y = self.lasty;
        self.apply_xy_feed_rate = true;
        self.spindle_on(out, cmt);
    }

    /// `PenUp`. `out`: buffer. `cmt`: comment.
    pub fn pen_up(&mut self, out: &mut String, cmt: &str) {
        self.drag_1st_move = true;
        self.orig_final_x = self.lastx;
        self.orig_final_y = self.lasty;
        self.spindle_off(out, cmt);
        self.drag_comp_i = 0.0;
        self.drag_comp_j = 0.0;
    }

    /// `MoveTo(Point)`: G1 to a point. `out`: buffer. `coord`: target. `cmt`: comment.
    pub fn move_to(&mut self, out: &mut String, coord: Point, cmt: &str) {
        let a = self.apply_xy_feed_rate;
        self.move_split(out, 1, coord.x as f32, coord.y as f32, None, a, cmt);
    }

    /// `MoveToRapid(Point)`: G0 (or G1) rapid. `out`: buffer. `coord`: target. `cmt`: comment.
    pub fn move_to_rapid(&mut self, out: &mut String, coord: Point, cmt: &str) {
        let g = self.rapidnum;
        self.mv(out, g, coord.x as f32, coord.y as f32, None, false, cmt);
        self.last_move_was_g0 = true;
    }

    /// `MoveSplit` (segmentation block is commented out upstream).
    ///
    /// Args: buffer, G number, target x/y, optional z, apply-feed flag (ignored like C#), comment.
    #[allow(clippy::too_many_arguments)]
    fn move_split(&mut self, out: &mut String, gnr: i32, mut finalx: f32, mut finaly: f32, z: Option<f32>, _apply_feed: bool, cmt: &str) {
        self.seg_last_final_x = self.seg_final_x;
        self.seg_last_final_y = self.seg_final_y;
        self.seg_final_x = finalx;
        self.seg_final_y = finaly;
        // drag knife compensation
        if self.drag_compensation {
            let tmp = self.drag_tool_compensation(out, finalx, finaly);
            finalx = tmp.x as f32;
            finaly = tmp.y as f32;
        }
        let a = self.apply_xy_feed_rate;
        self.mv(out, gnr, finalx, finaly, z, a, cmt);
        self.last_move_was_g0 = false;
    }

    /// `dragToolCompensation`. Args: buffer, target x/y. Returns the extended end point.
    fn drag_tool_compensation(&mut self, out: &mut String, finalx: f32, finaly: f32) -> Point {
        let dx = finalx - self.orig_final_x;
        let dy = finaly - self.orig_final_y;
        let move_length = (((dx * dx + dy * dy) as f64).sqrt()) as f32;
        if move_length == 0.0 {
            return Point::new(finalx as f64, finaly as f64);
        }
        // arc angle between last move and current move
        let rx = self.orig_final_x + self.drag_radius * dx / move_length;
        let ry = self.orig_final_y + self.drag_radius * dy / move_length;
        let mut angle = get_angle(self.lastx, self.lasty, rx, ry, self.drag_comp_i, self.drag_comp_j);
        if angle[2] > 180.0 {
            angle[2] -= 360.0;
        }
        if angle[2] < -180.0 {
            angle[2] += 360.0;
        }
        let mut gcnr = 2;
        if angle[2] > 0.0 {
            gcnr = 3;
        }
        self.drag_arc = angle[2].abs() > self.drag_angle;
        // arc before move
        if self.drag_arc {
            self.orig_last_x = self.lastx;
            self.orig_last_y = self.lasty;
            if angle[2] != 0.0 && !self.drag_1st_move {
                let a = self.apply_xy_feed_rate;
                let (i, j) = (self.drag_comp_i, self.drag_comp_j);
                self.move_arc(out, gcnr, rx, ry, i, j, a, "Drag t. comp.", false);
            }
            self.lastx = self.orig_last_x;
            self.lasty = self.orig_last_y;
        }
        // end pos with offset and new arc center
        self.orig_last_x = self.lastx;
        self.orig_last_y = self.lasty;
        self.orig_final_x = finalx;
        self.orig_final_y = finaly;
        let fx = finalx + self.drag_radius * dx / move_length;
        let fy = finaly + self.drag_radius * dy / move_length;
        self.drag_comp_i = self.orig_final_x - fx;
        self.drag_comp_j = self.orig_final_y - fy;
        self.drag_1st_move = false;
        Point::new(fx as f64, fy as f64)
    }

    /// `Move`: emit one G0/G1 line with modal compression.
    ///
    /// Args: buffer, G number, x, y, optional z, apply-feed, comment.
    #[allow(clippy::too_many_arguments)]
    fn mv(&mut self, out: &mut String, gnr: i32, x: f32, y: f32, z: Option<f32>, apply_feed: bool, cmt: &str) {
        if gnr == 0 {
            self.seg_last_final_x = self.seg_final_x;
            self.seg_last_final_y = self.seg_final_y;
            self.seg_final_x = x;
            self.seg_final_y = y;
        }
        let mut feed = String::new();
        let mut tmp = String::new();
        let mut isneeded = false;
        let mut tz = 0.0f32;
        if let Some(zv) = z {
            tz = zv;
        }
        // feed text only used by non-compressed output
        if apply_feed && gnr > 0 {
            feed = format!("F{}", fmt_float(self.xy_feed));
            self.apply_xy_feed_rate = false;
        }
        let cmt = if cmt.is_empty() { String::new() } else { format!("({cmt})") };
        if self.compress {
            if gnr > 0 || self.lastx != x || self.lasty != y || self.lastz != tz {
                if self.lastg != gnr as f32 {
                    tmp.push_str(&format!("G{}", self.frmt_code(gnr)));
                    isneeded = true;
                }
                if self.lastx != x {
                    tmp.push_str(&format!("X{}", self.frmt_num(x)));
                    isneeded = true;
                }
                if self.lasty != y {
                    tmp.push_str(&format!("Y{}", self.frmt_num(y)));
                    isneeded = true;
                }
                if let Some(zv) = z
                    && self.lastz != zv
                {
                    tmp.push_str(&format!("Z{}", self.frmt_num(zv)));
                    isneeded = true;
                }
                if (gnr == 1 && self.lastf != self.xy_feed) || apply_feed {
                    tmp.push_str(&format!("F{}", fmt_float(self.xy_feed)));
                    self.lastf = self.xy_feed;
                    isneeded = true;
                }
                // Smoothieware needs laser power on G1
                if gnr == 1 && self.lasts != self.spindle_speed && self.firmware == Firmware::Smoothie {
                    tmp.push_str(&format!("S{}", self.frmt_num(self.spindle_speed)));
                    self.lasts = self.spindle_speed;
                    isneeded = true;
                }
                tmp.push_str(&format!("{cmt}\r\n"));
                if isneeded {
                    out.push_str(&tmp);
                }
            }
        } else if let Some(zv) = z {
            out.push_str(&format!(
                "G{} X{} Y{} Z{} {} {}\r\n",
                self.frmt_code(gnr),
                self.frmt_num(x),
                self.frmt_num(y),
                self.frmt_num(zv),
                feed,
                cmt
            ));
        } else {
            out.push_str(&format!("G{} X{} Y{} {} {}\r\n", self.frmt_code(gnr), self.frmt_num(x), self.frmt_num(y), feed, cmt));
        }
        self.lastx = x;
        self.lasty = y;
        self.lastg = gnr as f32;
    }

    /// `splitLine`: break a line into steps. Args: buffer, G number, start/end points, max step, apply-feed, comment.
    #[allow(clippy::too_many_arguments)]
    pub fn split_line(&mut self, out: &mut String, gnr: i32, x1: f32, y1: f32, x2: f32, y2: f32, max_step: f32, mut apply_feed: bool, cmt: &str) {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let c = ((dx * dx + dy * dy) as f64).sqrt() as f32;
        let divid = f64_to_i32(((c / max_step) as f64).ceil());
        self.lastg = -1.0;
        let mut cmt = cmt.to_string();
        let mut i = 1;
        while i <= divid {
            let tx = x1 + i as f32 * dx / divid as f32;
            let ty = y1 + i as f32 * dy / divid as f32;
            if i > 1 {
                apply_feed = false;
                cmt.clear();
            }
            if gnr == 0 {
                self.mv(out, gnr, tx, ty, None, false, &cmt);
            } else {
                self.mv(out, gnr, tx, ty, None, apply_feed, &cmt);
            }
            i += 1;
        }
    }

    /// `Arc(Point...)`: G2/G3. Args: buffer, G number, end point, ij offsets, comment, avoidG23.
    pub fn arc(&mut self, out: &mut String, gnr: i32, xy: Point, ij: Point, cmt: &str, avoid_g23: bool) {
        let a = self.apply_xy_feed_rate;
        self.move_arc(out, gnr, xy.x as f32, xy.y as f32, ij.x as f32, ij.y as f32, a, cmt, avoid_g23);
    }

    /// `MoveArc`. Args: buffer, G number, end x/y, center offsets i/j, apply-feed, comment, avoidG23.
    #[allow(clippy::too_many_arguments)]
    fn move_arc(&mut self, out: &mut String, gnr: i32, x: f32, y: f32, i: f32, j: f32, apply_feed: bool, cmt: &str, avoid_g23: bool) {
        let mut feed = String::new();
        let mut spindle = String::new();
        if apply_feed {
            feed = format!("F{}", fmt_float(self.xy_feed));
            self.apply_xy_feed_rate = false;
        }
        let cmt = if cmt.is_empty() { String::new() } else { format!("({cmt})") };
        // Smoothieware needs laser power on arcs
        if self.firmware == Firmware::Smoothie {
            spindle = format!("S{}", self.frmt_num(self.spindle_speed));
        }
        if self.no_arcs || avoid_g23 {
            let (lx, ly) = (self.lastx, self.lasty);
            self.split_arc(out, gnr, lx, ly, x, y, i, j, apply_feed, &cmt);
        } else {
            out.push_str(&format!(
                "G{}X{}Y{}I{}J{}{}{}{}\r\n",
                self.frmt_code(gnr),
                self.frmt_num(x),
                self.frmt_num(y),
                self.frmt_num(i),
                self.frmt_num(j),
                feed,
                spindle,
                cmt
            ));
            self.lastg = gnr as f32;
        }
        self.lastx = x;
        self.lasty = y;
        self.lastf = self.xy_feed;
    }

    /// `splitArc`: arc to line segments. Args: buffer, G number, start x1/y1, end x2/y2, i/j, apply-feed, comment.
    #[allow(clippy::too_many_arguments)]
    pub fn split_arc(&mut self, out: &mut String, gnr: i32, x1: f32, y1: f32, x2: f32, y2: f32, i: f32, j: f32, _apply_feed: bool, cmt: &str) {
        let segment_length = 10.0f32;
        let equidistance = false;
        let mut cmt = cmt.to_string();
        let radius = ((i * i + j * j) as f64).sqrt() as f32;
        let cx = x1 + i;
        let cy = y1 + j;
        let ret = get_angle(x1, y1, x2, y2, i, j);
        let a1 = ret[0];
        let a2 = ret[1];
        let mut da;
        da = -(360.0 + a1 - a2);
        if gnr == 3 {
            da = (360.0 + a2 - a1).abs();
        }
        if da > 360.0 {
            da -= 360.0;
        }
        if da < -360.0 {
            da += 360.0;
        }
        if x1 == x2 && y1 == y2 {
            da = if gnr == 2 { -360.0 } else { 360.0 };
        }
        let step = ((self.angle_step as f64 / radius as f64).asin() * 180.0 / std::f64::consts::PI) as f32;
        self.apply_xy_feed_rate = true;
        let mut move_length = self.remaining_c;
        let mut count = 1;
        if da > 0.0 {
            // counter clock wise
            let mut angle = a1 + step;
            while angle < a1 + da {
                let x = cx + radius * (std::f64::consts::PI * angle as f64 / 180.0).cos() as f32;
                let y = cy + radius * (std::f64::consts::PI * angle as f64 / 180.0).sin() as f32;
                move_length += fdistance(x, y, self.lastx, self.lasty);
                let a = self.apply_xy_feed_rate;
                self.mv(out, 1, x, y, None, a, &cmt);
                if move_length >= count as f32 * segment_length {
                    self.apply_xy_feed_rate = true;
                    count += 1;
                }
                if cmt.encode_utf16().count() > 1 {
                    cmt.clear();
                }
                angle += step;
            }
        } else {
            // clock wise
            let mut angle = a1 - step;
            while angle > a1 + da {
                let x = cx + radius * (std::f64::consts::PI * angle as f64 / 180.0).cos() as f32;
                let y = cy + radius * (std::f64::consts::PI * angle as f64 / 180.0).sin() as f32;
                move_length += fdistance(x, y, self.lastx, self.lasty);
                let a = self.apply_xy_feed_rate;
                self.mv(out, 1, x, y, None, a, &cmt);
                if move_length >= count as f32 * segment_length {
                    self.apply_xy_feed_rate = true;
                    count += 1;
                }
                if cmt.encode_utf16().count() > 1 {
                    cmt.clear();
                }
                angle -= step;
            }
        }
        let a = self.apply_xy_feed_rate;
        self.mv(out, 1, x2, y2, None, a, "End Arc conversion");
        if move_length >= count as f32 * segment_length || equidistance {
            self.apply_xy_feed_rate = true;
        }
    }
}

/// `Convert.ToInt16(string)`: decimal parse with range check. `s`: text. Returns value or FormatException/Overflow.
fn to_int16(s: &str) -> Result<i32, SvgError> {
    match lg_dotnet::parse_int(s) {
        Some(v) if (i16::MIN as i32..=i16::MAX as i32).contains(&v) => Ok(v),
        Some(_) => Err(SvgError::new("System.OverflowException", "Value was either too large or too small for an Int16.")),
        None => Err(SvgError::new("System.FormatException", "Input string was not in a correct format.")),
    }
}

/// `getAngle`: start/end/delta angle in degrees. Args: start x1/y1, end x2/y2, center offsets i/j. Returns [a1, a2, da].
fn get_angle(x1: f32, y1: f32, x2: f32, y2: f32, i: f32, j: f32) -> [f32; 3] {
    let radius = ((i * i + j * j) as f64).sqrt() as f32;
    // start angle
    let mut cos1 = i / radius;
    cos1 = cos1.clamp(-1.0, 1.0); // NaN passes through like the C# compares
    let mut a1 = 180.0 - 180.0 * ((cos1 as f64).acos() / std::f64::consts::PI) as f32;
    if j > 0.0 {
        a1 = -a1;
    }
    // stop angle
    let mut cos2 = (x1 + i - x2) / radius;
    cos2 = cos2.clamp(-1.0, 1.0); // NaN passes through like the C# compares
    let mut a2 = 180.0 - 180.0 * ((cos2 as f64).acos() / std::f64::consts::PI) as f32;
    if (y1 + j - y2) > 0.0 {
        a2 = -a2;
    }
    let mut da = -(360.0 + a1 - a2);
    if da > 360.0 {
        da -= 360.0;
    }
    if da < -360.0 {
        da += 360.0;
    }
    [a1, a2, da]
}
