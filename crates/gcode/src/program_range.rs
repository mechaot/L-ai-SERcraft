//! Port of `ProgramRange` (bottom of `GrblFile.cs`).

use crate::command::Element;
use lg_dotnet::{dec_sub, decimal_to_f32, f64_to_decimal, Decimal};

/// `Math.Min(decimal, decimal)`.
fn dmin(a: Decimal, b: Decimal) -> Decimal {
    if b < a { b } else { a }
}

/// `Math.Max(decimal, decimal)`.
fn dmax(a: Decimal, b: Decimal) -> Decimal {
    if b > a { b } else { a }
}

/// Min/max pair of decimals, sentinels `Decimal::MAX` / `Decimal::MIN` when empty.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: Decimal,
    pub max: Decimal,
}

impl Default for Range {
    /// Reset state.
    fn default() -> Self {
        Self { min: Decimal::MAX, max: Decimal::MIN }
    }
}

impl Range {
    /// Widen range.
    ///
    /// `val`: new sample.
    pub fn update_range(&mut self, val: Decimal) {
        self.min = dmin(self.min, val);
        self.max = dmax(self.max, val);
    }

    /// Back to sentinels.
    pub fn reset_range(&mut self) {
        *self = Self::default();
    }

    /// Both ends set.
    pub fn valid_range(&self) -> bool {
        self.min != Decimal::MAX && self.max != Decimal::MIN
    }
}

/// X/Y extent (C# `XYRange`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct XYRange {
    pub x: Range,
    pub y: Range,
}

impl XYRange {
    /// Widen with optional words.
    ///
    /// `x`,`y`: X/Y words (None skipped).
    pub fn update_range(&mut self, x: Option<&Element>, y: Option<&Element>) {
        if let Some(x) = x {
            self.x.update_range(x.number());
        }
        if let Some(y) = y {
            self.y.update_range(y.number());
        }
    }

    /// Widen with a rectangle (bbox of arcs).
    ///
    /// `rect_x`,`rect_y`: origin. `rect_w`,`rect_h`: size.
    pub fn update_range_rect(&mut self, rect_x: f64, rect_y: f64, rect_w: f64, rect_h: f64) {
        self.x.update_range(f64_to_decimal(rect_x));
        self.x.update_range(f64_to_decimal(rect_x + rect_w));
        self.y.update_range(f64_to_decimal(rect_y));
        self.y.update_range(f64_to_decimal(rect_y + rect_h));
    }

    /// Reset both axes.
    pub fn reset_range(&mut self) {
        self.x.reset_range();
        self.y.reset_range();
    }

    /// Both axes valid.
    pub fn valid_range(&self) -> bool {
        self.x.valid_range() && self.y.valid_range()
    }

    /// `X.Max - X.Min`; None where C# throws OverflowException (invalid range).
    pub fn width(&self) -> Option<Decimal> {
        dec_sub(self.x.max, self.x.min)
    }

    /// `Y.Max - Y.Min`; None on overflow.
    pub fn height(&self) -> Option<Decimal> {
        dec_sub(self.y.max, self.y.min)
    }

    /// Center as f32 point `(x, y)`; (0,0) when invalid.
    pub fn center(&self) -> (f32, f32) {
        if !self.valid_range() {
            return (0.0, 0.0);
        }
        let (Some(w), Some(h)) = (self.width(), self.height()) else { return (0.0, 0.0) };
        (
            decimal_to_f32(self.x.min) + decimal_to_f32(w) / 2.0f32,
            decimal_to_f32(self.y.min) + decimal_to_f32(h) / 2.0f32,
        )
    }
}

/// Spindle (S word) extent (C# `SRange`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SRange {
    pub s: Range,
}

impl SRange {
    /// Widen.
    ///
    /// `s`: new S value.
    pub fn update_range(&mut self, s: Decimal) {
        self.s.update_range(s);
    }

    /// Reset.
    pub fn reset_range(&mut self) {
        self.s.reset_range();
    }

    /// C# `SRange.Range.ValidRange`: set, min != max and max > 0.
    pub fn valid_range(&self) -> bool {
        self.s.min != self.s.max && self.s.min != Decimal::MAX && self.s.max != Decimal::MIN && self.s.max > Decimal::ZERO
    }
}

/// Ranges of a whole program.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ProgramRange {
    pub drawing_range: XYRange,
    pub moving_range: XYRange,
    pub spindle_range: SRange,
}

impl ProgramRange {
    /// Update moving (always) and drawing (when `drawing`) range with words.
    ///
    /// `x`,`y`: words. `drawing`: laser burning.
    pub fn update_xy_range(&mut self, x: Option<&Element>, y: Option<&Element>, drawing: bool) {
        if drawing {
            self.drawing_range.update_range(x, y);
        }
        self.moving_range.update_range(x, y);
    }

    /// Same with a bbox rectangle.
    ///
    /// `rect_*`: bbox. `drawing`: laser burning.
    pub fn update_xy_range_rect(&mut self, rect_x: f64, rect_y: f64, rect_w: f64, rect_h: f64, drawing: bool) {
        if drawing {
            self.drawing_range.update_range_rect(rect_x, rect_y, rect_w, rect_h);
        }
        self.moving_range.update_range_rect(rect_x, rect_y, rect_w, rect_h);
    }

    /// Update spindle range.
    ///
    /// `s`: S word or None.
    pub fn update_s_range(&mut self, s: Option<&Element>) {
        if let Some(s) = s {
            self.spindle_range.update_range(s.number());
        }
    }

    /// Reset all ranges.
    pub fn reset_range(&mut self) {
        self.drawing_range.reset_range();
        self.moving_range.reset_range();
        self.spindle_range.reset_range();
    }
}

