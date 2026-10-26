// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of the raster line-to-line part of `GrblFile.cs`: `L2LConf`, `ColorSegment` family,
//! `LoadImageL2L`, `ImageLine2Line`, `OptimizeLine2Line`, `GetSegments`, `ExtractSegment`, `GetColor`.

use crate::command::GrblCommand;
use crate::config::{configuration, Firmware};
use crate::grbl_file::GrblFile;
use crate::GResult;
use image::RgbaImage;
use lg_dotnet::{decimal_to_f64, f32_to_decimal, fmt_decimal, fmt_double, fmt_double_custom, round_double};
use lg_potrace::Direction;
use lg_raster::{rotate_flip, RotateFlipType};

/// Brightness to power mapping of the image dock (not in C#, intentional deviation).
///
/// `start` maps to S-MAX, `stop` to S-MIN, linear in between. `start > stop` is allowed
/// (inverted: brighter = more power). Brightness outside `[min(start,stop), max(start,stop)]`
/// gets power 0 (no burn).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerMap {
    pub start: u8,
    pub stop: u8,
}

impl PowerMap {
    /// Power for one brightness value.
    ///
    /// `brightness`: pixel brightness 0..=255 (0 black). `min`,`max`: S-MIN / S-MAX (any order).
    /// Returns 0 outside the range, else the linear value (rounded half away from zero,
    /// clamped to the min..max span). `start == stop` burns only that brightness, at `max`.
    pub fn power(&self, brightness: u8, min: i32, max: i32) -> i32 {
        let (lo, hi) = (self.start.min(self.stop), self.start.max(self.stop));
        // outside the range: no burn
        if brightness < lo || brightness > hi {
            return 0;
        }
        // degenerate range: single brightness at full power
        if self.start == self.stop {
            return max;
        }
        // linear part in f64 (multiply first, divide once), one rounding
        let num = (brightness as f64 - self.start as f64) * (min as f64 - max as f64);
        let v = (max as f64 + num / (self.stop as f64 - self.start as f64)).round() as i32;
        // clamp guard against min > max (f64/i32 clamp would panic)
        v.clamp(min.min(max), min.max(max))
    }

    /// Range test for the non-PWM (on/off) mode.
    ///
    /// `brightness`: pixel brightness 0..=255. Returns true when inside `[min(start,stop), max(start,stop)]`.
    pub fn in_range(&self, brightness: u8) -> bool {
        brightness >= self.start.min(self.stop) && brightness <= self.start.max(self.stop)
    }
}

/// Raster / vector conversion options (C# `GrblFile.L2LConf`).
#[derive(Debug, Clone)]
pub struct L2LConf {
    pub res: f64,
    pub o_x: f32,
    pub o_y: f32,
    pub mark_speed: i32,
    pub border_speed: i32,
    pub min_power: i32,
    pub max_power: i32,
    pub l_on: String,
    pub l_off: String,
    pub dir: Direction,
    pub pwm: bool,
    pub fres: f64,
    pub vectorfilling: bool,
    pub firmware_type: Firmware,
    /// Image dock only: brightness to power mapping. `None` = classic C# `GetColor`.
    pub power_map: Option<PowerMap>,
}

impl Default for L2LConf {
    /// C# field defaults (zero / null; strings empty).
    fn default() -> Self {
        Self {
            res: 0.0,
            o_x: 0.0,
            o_y: 0.0,
            mark_speed: 0,
            border_speed: 0,
            min_power: 0,
            max_power: 0,
            l_on: String::new(),
            l_off: String::new(),
            dir: Direction::None,
            pwm: false,
            fres: 0.0,
            vectorfilling: false,
            firmware_type: Firmware::Grbl,
            power_map: None,
        }
    }
}

/// `RasterFilling(dir)`: pixel based filling directions.
///
/// `dir`: direction. Returns true for Diagonal/Horizontal/Vertical.
pub fn raster_filling(dir: Direction) -> bool {
    matches!(dir, Direction::Diagonal | Direction::Horizontal | Direction::Vertical)
}

/// `VectorFilling(dir)`: clipper based filling directions.
///
/// `dir`: direction. Returns true for every `New*` direction.
pub fn vector_filling(dir: Direction) -> bool {
    matches!(
        dir,
        Direction::NewDiagonal
            | Direction::NewHorizontal
            | Direction::NewVertical
            | Direction::NewReverseDiagonal
            | Direction::NewGrid
            | Direction::NewDiagonalGrid
            | Direction::NewCross
            | Direction::NewDiagonalCross
            | Direction::NewSquares
            | Direction::NewZigZag
            | Direction::NewHilbert
            | Direction::NewInsetFilling
    )
}

/// `TimeConsumingFilling(dir)`.
///
/// `dir`: direction. Returns true for cross/diagonal-cross/squares.
pub fn time_consuming_filling(dir: Direction) -> bool {
    matches!(dir, Direction::NewCross | Direction::NewDiagonalCross | Direction::NewSquares)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegKind {
    X,
    Y,
    D,
    VSep,
    HSep,
}

/// One run of equal color along the scan line (C# `ColorSegment` subclasses).
#[derive(Debug, Clone, Copy)]
struct ColorSegment {
    kind: SegKind,
    color: i32,
    pix_len: i32,
}

impl ColorSegment {
    /// Segment ctor.
    ///
    /// `kind`: axis. `col`: color. `len`: pixels. `rev`: negative direction.
    fn new(kind: SegKind, col: i32, len: i32, rev: bool) -> Self {
        Self { kind, color: col, pix_len: if rev { -len } else { len } }
    }

    fn vsep() -> Self {
        Self::new(SegKind::VSep, 0, 1, false)
    }

    fn hsep() -> Self {
        Self::new(SegKind::HSep, 0, 1, false)
    }

    fn is_separator(&self) -> bool {
        matches!(self.kind, SegKind::VSep | SegKind::HSep)
    }

    /// Travel without burning.
    fn fast(&self, c: &L2LConf) -> bool {
        if c.pwm { self.color == 0 } else { self.color <= 125 }
    }

    /// Pixel position to mm text.
    ///
    /// `number`: pixel coordinate. `offset`: mm offset. `c`: conf.
    fn formatnumber(number: i32, offset: f32, c: &L2LConf) -> String {
        let div = if c.vectorfilling { c.fres } else { c.res };
        let dval = round_double(number as f64 / div + offset as f64, 3);
        fmt_double(dval)
    }

    /// `S` word for a color (Smoothie scales to 0..1).
    fn format_laser_power(color: i32, c: &L2LConf) -> String {
        if c.firmware_type == Firmware::Smoothie {
            format!("S{}", fmt_double_custom(color as f64 / 255.0, "0.00"))
        } else {
            format!("S{color}")
        }
    }

    /// Advance the cumulative position and render the move.
    ///
    /// `cum_x`,`cum_y`: cumulative pixel position. `c`: conf.
    fn to_gcode_number(self, cum_x: &mut i32, cum_y: &mut i32, c: &L2LConf) -> String {
        let tail = |s: Self| if c.pwm { Self::format_laser_power(s.color, c) } else if s.fast(c) { c.l_off.clone() } else { c.l_on.clone() };
        match self.kind {
            SegKind::X => {
                *cum_x += self.pix_len;
                format!("X{} {}", Self::formatnumber(*cum_x, c.o_x, c), tail(self))
            }
            SegKind::Y => {
                *cum_y += self.pix_len;
                format!("Y{} {}", Self::formatnumber(*cum_y, c.o_y, c), tail(self))
            }
            SegKind::D => {
                *cum_x += self.pix_len;
                *cum_y -= self.pix_len;
                format!("X{} Y{} {}", Self::formatnumber(*cum_x, c.o_x, c), Self::formatnumber(*cum_y, c.o_y, c), tail(self))
            }
            SegKind::VSep => {
                *cum_y += self.pix_len;
                format!("Y{}", Self::formatnumber(*cum_y, c.o_y, c))
            }
            SegKind::HSep => {
                *cum_x += self.pix_len;
                format!("X{}", Self::formatnumber(*cum_x, c.o_x, c))
            }
        }
    }
}

impl GrblFile {
    /// C# `LoadImageL2L`. Flips `bmp` vertically in place (like C# `RotateFlip`).
    ///
    /// `bmp`: source image (modified). `filename`: name for the events. `c`: conf. `append`: keep
    /// old commands.
    ///
    /// Returns Ok(false) when the file is busy (C# returns silently), Ok(true) when loaded.
    pub fn load_image_l2l(&mut self, bmp: &mut RgbaImage, filename: &str, c: &L2LConf, append: bool) -> GResult<bool> {
        if self.check_in_use() {
            return Ok(false);
        }
        self.load_image_l2l_body(bmp, filename, c, append)?;
        Ok(true)
    }

    /// `LoadImageL2L` body without the busy check, for use inside [`GrblFile::spawn_load`]
    /// (there the loading flag is already set, so the checked variant would skip silently).
    ///
    /// Arguments as [`GrblFile::load_image_l2l`]. Returns Ok(()) when loaded.
    pub fn load_image_l2l_body(&mut self, bmp: &mut RgbaImage, filename: &str, c: &L2LConf, append: bool) -> GResult<()> {
        self.skipcmd = if lg_base::settings::get("Disable G0 fast skip", false) { "G1".into() } else { "G0".into() };
        self.rise_on_file_loading(filename);
        *bmp = rotate_flip(bmp, RotateFlipType::RotateNoneFlipY);
        let start = lg_base::tools::hires_timer::total_milliseconds();
        if !append {
            self.clear_list();
        }
        self.range.reset_range();

        // fast move to offset and set mark speed
        self.list.push(GrblCommand::new(&format!(
            "{} X{} Y{} F{}",
            self.skipcmd,
            Self::formatnumber(c.o_x as f64),
            Self::formatnumber(c.o_y as f64),
            c.mark_speed
        )));
        if c.pwm {
            self.list.push(GrblCommand::new(&format!("{} S0", c.l_on)));
        } else {
            self.list.push(GrblCommand::new(&format!("{} S{}", c.l_off, fmt_decimal(configuration::configuration().max_pwm()))));
        }

        self.image_line2line(bmp, c);

        // laser off
        self.list.push(GrblCommand::new(&c.l_off));

        self.analyze()?;
        let elapsed = lg_base::tools::hires_timer::total_milliseconds() - start;
        self.rise_on_file_loaded(filename, elapsed);
        Ok(())
    }

    /// C# `ImageLine2Line`: scan the bitmap and append the commands.
    ///
    /// `bmp`: flipped bitmap. `c`: conf.
    pub(crate) fn image_line2line(&mut self, bmp: &RgbaImage, c: &L2LConf) {
        let mut fast = true;
        let segments = self.get_segments(bmp, c);
        let mut temp: Vec<GrblCommand> = Vec::new();
        let mut cum_x = 0;
        let mut cum_y = 0;

        for seg in &segments {
            let change_g_mode = fast != seg.fast(c);
            if seg.is_separator() && !fast {
                // previous segment was burning: switch laser off first
                if c.pwm {
                    temp.push(GrblCommand::new("S0"));
                } else {
                    temp.push(GrblCommand::new(&c.l_off));
                }
            }
            fast = seg.fast(c);
            let num = seg.to_gcode_number(&mut cum_x, &mut cum_y, c);
            if change_g_mode {
                temp.push(GrblCommand::new(&format!("{} {}", if fast { self.skipcmd.as_str() } else { "G1" }, num)));
            } else {
                temp.push(GrblCommand::new(&num));
            }
        }
        let temp = self.optimize_line2line(temp, c);
        self.list.extend(temp);
    }

    /// C# `OptimizeLine2Line`: drop moves made with the laser off, keep one jump.
    fn optimize_line2line(&self, temp: Vec<GrblCommand>, c: &L2LConf) -> Vec<GrblCommand> {
        let mut rv = Vec::new();
        let mut cur_x = f32_to_decimal(c.o_x);
        let mut cur_y = f32_to_decimal(c.o_y);
        let mut cumulate = false;

        for mut cmd in temp {
            cmd.build_helper();
            let oldcumulate = cumulate;
            if c.pwm {
                if let Some(s) = cmd.s() {
                    cumulate = s.number() == lg_dotnet::Decimal::ZERO;
                }
            } else if cmd.is_laser_off() {
                cumulate = true;
            } else if cmd.is_laser_on() {
                cumulate = false;
            }

            if oldcumulate && !cumulate {
                // cumulate down front: flush
                let (fx, fy) = (Self::formatnumber(decimal_to_f64(cur_x)), Self::formatnumber(decimal_to_f64(cur_y)));
                if c.pwm {
                    rv.push(GrblCommand::new(&format!("{} X{} Y{} S0", self.skipcmd, fx, fy)));
                } else {
                    rv.push(GrblCommand::new(&format!("{} X{} Y{} {}", self.skipcmd, fx, fy, c.l_off)));
                }
            }

            if cmd.is_movement() {
                if let Some(x) = cmd.x() {
                    cur_x = x.number();
                }
                if let Some(y) = cmd.y() {
                    cur_y = y.number();
                }
            }
            let keep = !cmd.is_movement() || !cumulate;
            cmd.delete_helper();
            if keep {
                rv.push(cmd);
            }
        }
        rv
    }

    /// C# `GetSegments`.
    fn get_segments(&self, bmp: &RgbaImage, c: &L2LConf) -> Vec<ColorSegment> {
        let uni: bool = lg_base::settings::get("Unidirectional Engraving", false);
        let (bw, bh) = (bmp.width() as i32, bmp.height() as i32);
        let mut rv = Vec::new();

        if c.dir == Direction::Horizontal || c.dir == Direction::Vertical {
            let h = c.dir == Direction::Horizontal;
            let outer = if h { bh } else { bw };
            for i in 0..outer {
                let d = uni || is_even(i);
                let mut prev_col = -1;
                let mut len = -1;
                let inner = if h { bw } else { bh };
                let mut j = if d { 0 } else { inner - 1 };
                while if d { j < inner } else { j >= 0 } {
                    if h {
                        extract_segment(bmp, j, i, !d, &mut len, &mut prev_col, &mut rv, c);
                    } else {
                        extract_segment(bmp, i, j, !d, &mut len, &mut prev_col, &mut rv, c);
                    }
                    j = if d { j + 1 } else { j - 1 };
                }
                // close last segment
                rv.push(ColorSegment::new(if h { SegKind::X } else { SegKind::Y }, prev_col, len + 1, !d));
                if uni {
                    // go back
                    if h {
                        rv.push(ColorSegment::new(SegKind::X, 0, bw, true));
                    } else {
                        rv.push(ColorSegment::new(SegKind::Y, 0, bh, true));
                    }
                }
                if i < (if h { bh - 1 } else { bw - 1 }) {
                    rv.push(if h { ColorSegment::vsep() } else { ColorSegment::hsep() });
                }
            }
        } else if c.dir == Direction::Diagonal {
            // diagonal strips; z1/z2 = missing elements lower-right / upper-left; direction flips each slice
            rv.push(ColorSegment::vsep());
            let (w, h) = (bw, bh);
            let mut slice = 0;
            while slice < w + h - 1 {
                let d = uni || is_even(slice);
                let mut prev_col = -1;
                let mut len = -1;
                let z1 = if slice < h { 0 } else { slice - h + 1 };
                let z2 = if slice < w { 0 } else { slice - w + 1 };
                let mut j = if d { z1 } else { slice - z2 };
                while if d { j <= slice - z2 } else { j >= z1 } {
                    extract_segment(bmp, j, slice - j, !d, &mut len, &mut prev_col, &mut rv, c);
                    j = if d { j + 1 } else { j - 1 };
                }
                rv.push(ColorSegment::new(SegKind::D, prev_col, len + 1, !d));
                if uni {
                    let slen = (slice - z1 - z2) + 1;
                    rv.push(ColorSegment::new(SegKind::D, 0, slen, true));
                }
                if slice < w.min(h) - 1 {
                    // first part of the image
                    rv.push(if d && !uni { ColorSegment::hsep() } else { ColorSegment::vsep() });
                } else if slice >= w.max(h) - 1 {
                    // third part
                    rv.push(if d && !uni { ColorSegment::vsep() } else { ColorSegment::hsep() });
                } else {
                    // central part
                    rv.push(if w > h { ColorSegment::hsep() } else { ColorSegment::vsep() });
                }
                slice += 1;
            }
        }
        rv
    }
}

/// C# `ExtractSegment`.
#[allow(clippy::too_many_arguments)]
fn extract_segment(image: &RgbaImage, x: i32, y: i32, reverse: bool, len: &mut i32, prev_col: &mut i32, rv: &mut Vec<ColorSegment>, c: &L2LConf) {
    *len += 1;
    let col = get_color(image, x, y, c);
    if *prev_col == -1 {
        *prev_col = col;
    }
    if *prev_col != col {
        match c.dir {
            Direction::Horizontal => rv.push(ColorSegment::new(SegKind::X, *prev_col, *len, reverse)),
            Direction::Vertical => rv.push(ColorSegment::new(SegKind::Y, *prev_col, *len, reverse)),
            Direction::Diagonal => rv.push(ColorSegment::new(SegKind::D, *prev_col, *len, reverse)),
            _ => {}
        }
        *len = 0;
    }
    *prev_col = col;
}

/// C# `GetColor`: darkness scaled by alpha, optionally mapped into the power range.
/// With `c.power_map` the brightness is mapped by [`PowerMap`] instead (image dock).
///
/// `i`: image. `x`,`y`: pixel. `c`: conf (power range, pwm, map).
fn get_color(i: &RgbaImage, x: i32, y: i32, c: &L2LConf) -> i32 {
    let (min, max, pwm) = (c.min_power, c.max_power, c.pwm);
    let p = i.get_pixel(x as u32, y as u32);
    let rv = (255 - p[0] as i32) * p[3] as i32 / 255;
    // image dock path: brightness -> power
    if let Some(m) = &c.power_map {
        let brightness = (255 - rv) as u8;
        return if pwm {
            m.power(brightness, min, max)
        } else if m.in_range(brightness) {
            255
        } else {
            0
        };
    }
    if rv == 0 {
        0
    } else if pwm {
        rv * (max - min) / 255 + min
    } else {
        rv
    }
}

/// C# `IsEven`.
fn is_even(v: i32) -> bool {
    v % 2 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn map(start: u8, stop: u8) -> PowerMap {
        PowerMap { start, stop }
    }

    #[test]
    fn normal_map_darker_is_more_power() {
        let m = map(16, 210);
        assert_eq!(m.power(16, 0, 1000), 1000);
        assert_eq!(m.power(210, 0, 1000), 0);
        // middle: 1000 + 97 * -1000 / 194 = 500
        assert_eq!(m.power(113, 0, 1000), 500);
        // monotonic non-increasing inside
        let mut prev = i32::MAX;
        for b in 16..=210u8 {
            let p = m.power(b, 100, 900);
            assert!(p <= prev && (100..=900).contains(&p));
            prev = p;
        }
    }

    #[test]
    fn inverted_map_brighter_is_more_power() {
        let m = map(210, 16);
        assert_eq!(m.power(210, 0, 1000), 1000);
        assert_eq!(m.power(16, 0, 1000), 0);
        assert_eq!(m.power(113, 0, 1000), 500);
        assert!(m.power(150, 0, 1000) > m.power(50, 0, 1000));
        // min > 0: stop end gives S-MIN, not 0
        assert_eq!(m.power(16, 100, 900), 100);
        assert_eq!(m.power(210, 100, 900), 900);
    }

    #[test]
    fn outside_range_is_zero_both_sides() {
        for m in [map(16, 210), map(210, 16)] {
            assert_eq!(m.power(0, 100, 900), 0);
            assert_eq!(m.power(15, 100, 900), 0);
            assert_eq!(m.power(211, 100, 900), 0);
            assert_eq!(m.power(255, 100, 900), 0);
            assert!(m.power(16, 100, 900) > 0 && m.power(210, 100, 900) > 0);
        }
    }

    #[test]
    fn equal_start_stop_burns_only_that_value() {
        let m = map(100, 100);
        assert_eq!(m.power(100, 0, 1000), 1000);
        assert_eq!(m.power(99, 0, 1000), 0);
        assert_eq!(m.power(101, 0, 1000), 0);
        assert!(m.in_range(100) && !m.in_range(99) && !m.in_range(101));
    }

    #[test]
    fn full_range_endpoints() {
        let m = map(0, 255);
        assert_eq!(m.power(0, 0, 255), 255);
        assert_eq!(m.power(255, 0, 255), 0);
        assert_eq!(m.power(255, 10, 255), 10);
        assert_eq!(m.power(0, 10, 255), 255);
    }

    #[test]
    fn rounding_half_away_from_zero() {
        // start 0, stop 2, min 0, max 5: b=1 -> 5 + 1*-5/2 = 2.5 -> 3
        assert_eq!(map(0, 2).power(1, 0, 5), 3);
        // b=1 of 0..4: 5 + -5/4 = 3.75 -> 4
        assert_eq!(map(0, 4).power(1, 0, 5), 4);
        // b=3 of 0..4: 5 - 15/4 = 1.25 -> 1
        assert_eq!(map(0, 4).power(3, 0, 5), 1);
    }

    #[test]
    fn min_greater_than_max_does_not_panic() {
        let m = map(0, 255);
        for b in [0u8, 1, 128, 254, 255] {
            let p = m.power(b, 800, 200);
            assert!((200..=800).contains(&p), "b={b} p={p}");
        }
        assert_eq!(m.power(0, 800, 200), 200);
        assert_eq!(m.power(255, 800, 200), 800);
    }

    #[test]
    fn get_color_without_map_is_classic() {
        let img = RgbaImage::from_pixel(1, 1, Rgba([100, 100, 100, 255]));
        let mut c = L2LConf { min_power: 10, max_power: 200, pwm: true, ..L2LConf::default() };
        // (255 - 100) * (200 - 10) / 255 + 10
        assert_eq!(get_color(&img, 0, 0, &c), 155 * 190 / 255 + 10);
        c.pwm = false;
        assert_eq!(get_color(&img, 0, 0, &c), 155);
        // white stays 0 in the classic path
        let white = RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255]));
        assert_eq!(get_color(&white, 0, 0, &c), 0);
    }

    #[test]
    fn get_color_with_map() {
        let px = |v: u8, a: u8| RgbaImage::from_pixel(1, 1, Rgba([v, v, v, a]));
        let mut c = L2LConf { min_power: 0, max_power: 1000, pwm: true, power_map: Some(map(0, 254)), ..L2LConf::default() };
        // black -> S-MAX, white (255, outside) -> 0
        assert_eq!(get_color(&px(0, 255), 0, 0, &c), 1000);
        assert_eq!(get_color(&px(255, 255), 0, 0, &c), 0);
        // transparent = white over white -> 0
        assert_eq!(get_color(&px(0, 0), 0, 0, &c), 0);
        // on/off mode: in range = 255, out = 0
        c.pwm = false;
        assert_eq!(get_color(&px(0, 255), 0, 0, &c), 255);
        assert_eq!(get_color(&px(255, 255), 0, 0, &c), 0);
        // inverted map: white burns, black does not
        c.power_map = Some(map(255, 100));
        assert_eq!(get_color(&px(255, 255), 0, 0, &c), 255);
        assert_eq!(get_color(&px(0, 255), 0, 0, &c), 0);
    }
}
