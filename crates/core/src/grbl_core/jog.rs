// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Jog command construction of `GrblCore` (`EnqueueJogV09/V11`, `HandleContinuosJog`, `ContinuousJog`).
//!
//! All builders are pure and return the exact command strings the C# code enqueues.

use super::types::{JogDirection, PointF};
use lg_dotnet::{fmt_decimal_custom, fmt_float, fmt_float_custom, Decimal};
use std::sync::Arc;

/// Pending continuous jog request (C# nested class `ContinuousJog`).
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousJog {
    pub direction: JogDirection,
    pub speed: f32,
    pub target: PointF,
}

/// The C# static `mPrev` / `mCurr` pair. Requests are compared by identity like C# references.
#[derive(Default)]
pub struct JogTargets {
    prev: Option<Arc<ContinuousJog>>,
    curr: Option<Arc<ContinuousJog>>,
}

impl JogTargets {
    /// `SetJogTarget`.
    ///
    /// `j`: what to do now.
    fn set(&mut self, j: ContinuousJog) {
        self.curr = Some(Arc::new(j));
    }

    /// `Abort()`.
    pub fn abort(&mut self) {
        self.set(ContinuousJog { direction: JogDirection::Abort, speed: 0.0, target: PointF::default() });
    }

    /// `ToPosition(target, speed)`.
    ///
    /// `target`: machine target. `speed`: feed.
    pub fn to_position(&mut self, target: PointF, speed: f32) {
        self.set(ContinuousJog { direction: JogDirection::Position, speed, target });
    }

    /// `ToDirection(direction, speed)`.
    ///
    /// `direction`: not Abort/Position/Zup/Zdown (C# throws `ArgumentException`: Err here).
    pub fn to_direction(&mut self, direction: JogDirection, speed: f32) -> Result<(), String> {
        match direction {
            JogDirection::Abort | JogDirection::Position => Err("Invalid option".into()),
            JogDirection::Zup | JogDirection::Zdown => Err("Z Not supported in Continuous Jog".into()),
            _ => {
                self.set(ContinuousJog { direction, speed, target: PointF::default() });
                Ok(())
            }
        }
    }

    /// `GetAndClearTarget(out abortPrevCommand)`.
    ///
    /// Returns `(new request, abort previous)`; the request is None when nothing new is pending.
    pub fn get_and_clear(&mut self) -> (Option<Arc<ContinuousJog>>, bool) {
        if let Some(cur) = self.curr.clone() {
            let same = self.prev.as_ref().is_some_and(|p| Arc::ptr_eq(p, &cur));
            if !same {
                // need to abort the previous command unless it was an abort itself
                let abort_prev = self.prev.as_ref().is_some_and(|p| p.direction != JogDirection::Abort);
                self.prev = Some(cur.clone());
                return (Some(cur), abort_prev);
            }
        }
        (None, false)
    }
}

fn step_text(step: Decimal) -> String {
    fmt_decimal_custom(step, "0.0")
}

/// Axis words of a relative step (shared by V0.9 and V1.1).
fn axis_words(dir: JogDirection, step: Decimal) -> String {
    use JogDirection::*;
    let s = step_text(step);
    let mut cmd = String::new();
    if matches!(dir, NE | E | SE) {
        cmd += &format!("X{s}");
    }
    if matches!(dir, NW | W | SW) {
        cmd += &format!("X-{s}");
    }
    if matches!(dir, NW | N | NE) {
        cmd += &format!("Y{s}");
    }
    if matches!(dir, SW | S | SE) {
        cmd += &format!("Y-{s}");
    }
    if dir == Zdown {
        cmd += &format!("Z-{s}");
    }
    if dir == Zup {
        cmd += &format!("Z{s}");
    }
    cmd
}

/// `EnqueueJogV09(dir, step, speed)`: lines to enqueue in order (plain G-code).
///
/// `dir`: not Abort/Position. `step`: mm. `speed`: feed.
pub fn jog_v09_direction(dir: JogDirection, step: Decimal, speed: f32) -> Vec<String> {
    if dir == JogDirection::Home {
        return vec!["G90".to_string(), format!("G1X0Y0F{}", fmt_float(speed))];
    }
    let cmd = format!("G1{}F{}", axis_words(dir, step), fmt_float(speed));
    vec!["G91".to_string(), cmd, "G90".to_string()]
}

/// `EnqueueJogV09(target, speed)`.
///
/// `target`: absolute position. `speed`: feed.
pub fn jog_v09_position(target: PointF, speed: f32) -> Vec<String> {
    let cmd = format!("G1X{}Y{}F{}", fmt_float_custom(target.x, "0.00"), fmt_float_custom(target.y, "0.00"), fmt_float(speed));
    vec!["G90".to_string(), cmd]
}

/// `EnqueueJogV11(target, speed)`.
///
/// `target`: absolute position. `speed`: feed.
pub fn jog_v11_position(target: PointF, speed: f32) -> String {
    format!("$J=G90X{}Y{}F{}", fmt_float_custom(target.x, "0.00"), fmt_float_custom(target.y, "0.00"), fmt_float(speed))
}

/// `EnqueueJogV11(dir, step, speed)`.
///
/// `dir`: not Abort/Position. `step`: mm. `speed`: feed.
pub fn jog_v11_direction(dir: JogDirection, step: Decimal, speed: f32) -> String {
    if dir == JogDirection::Home {
        return format!("$J=G90X0Y0F{}", fmt_float(speed));
    }
    format!("$J=G91{}F{}", axis_words(dir, step), fmt_float(speed))
}

/// Command of `HandleContinuosJog` for a request.
///
/// `j`: request to send. `table_width`/`table_height`: `Configuration.TableWidth/Height`. Returns None for
/// Abort (nothing to send, the abort byte is sent by the caller).
pub fn continuous_jog_command(j: &ContinuousJog, table_width: Decimal, table_height: Decimal) -> Option<String> {
    use JogDirection::*;
    match j.direction {
        Abort => None,
        Position => Some(jog_v11_position(j.target, j.speed)),
        Home => Some(format!("$J=G90X0Y0F{}", fmt_float(j.speed))),
        dir => {
            let zero = lg_dotnet::fmt_i32(0, "0.0");
            let mut cmd = String::from("$J=G53");
            if matches!(dir, NE | E | SE) {
                cmd += &format!("X{}", fmt_decimal_custom(table_width, "0.0"));
            }
            if matches!(dir, NW | W | SW) {
                cmd += &format!("X{zero}");
            }
            if matches!(dir, NW | N | NE) {
                cmd += &format!("Y{}", fmt_decimal_custom(table_height, "0.0"));
            }
            if matches!(dir, SW | S | SE) {
                cmd += &format!("Y{zero}");
            }
            cmd += &format!("F{}", fmt_float(j.speed));
            Some(cmd)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use JogDirection::*;

    fn d(s: &str) -> Decimal {
        lg_dotnet::parse_decimal(s).unwrap()
    }

    #[test]
    fn v11_direction_strings() {
        assert_eq!(jog_v11_direction(NE, d("10"), 1000.0), "$J=G91X10.0Y10.0F1000");
        assert_eq!(jog_v11_direction(SW, d("0.5"), 3000.0), "$J=G91X-0.5Y-0.5F3000");
        assert_eq!(jog_v11_direction(W, d("1"), 100000.0), "$J=G91X-1.0F100000");
        assert_eq!(jog_v11_direction(N, d("1"), 1500.5), "$J=G91Y1.0F1500.5");
        assert_eq!(jog_v11_direction(S, d("100"), 1000.0), "$J=G91Y-100.0F1000");
        assert_eq!(jog_v11_direction(Zup, d("1"), 500.0), "$J=G91Z1.0F500");
        assert_eq!(jog_v11_direction(Zdown, d("0.1"), 500.0), "$J=G91Z-0.1F500");
        assert_eq!(jog_v11_direction(Home, d("1"), 1000.0), "$J=G90X0Y0F1000");
    }

    #[test]
    fn v09_strings() {
        assert_eq!(jog_v09_direction(E, d("10"), 1000.0), vec!["G91", "G1X10.0F1000", "G90"]);
        assert_eq!(jog_v09_direction(SE, d("2"), 1000.0), vec!["G91", "G1X2.0Y-2.0F1000", "G90"]);
        assert_eq!(jog_v09_direction(Home, d("2"), 1000.0), vec!["G90", "G1X0Y0F1000"]);
        assert_eq!(jog_v09_position(PointF::new(12.345, 0.0), 2000.0), vec!["G90", "G1X12.35Y0.00F2000"]);
    }

    #[test]
    fn position_strings() {
        assert_eq!(jog_v11_position(PointF::new(10.0, -5.256), 1000.0), "$J=G90X10.00Y-5.26F1000");
        assert_eq!(jog_v11_position(PointF::new(0.004, 1.0), 1.5), "$J=G90X0.00Y1.00F1.5");
    }

    #[test]
    fn continuous_commands() {
        let w = d("300");
        let h = d("200");
        let j = |dir| ContinuousJog { direction: dir, speed: 2000.0, target: PointF::default() };
        assert_eq!(continuous_jog_command(&j(NE), w, h).unwrap(), "$J=G53X300.0Y200.0F2000");
        assert_eq!(continuous_jog_command(&j(SW), w, h).unwrap(), "$J=G53X0.0Y0.0F2000");
        assert_eq!(continuous_jog_command(&j(E), w, h).unwrap(), "$J=G53X300.0F2000");
        assert_eq!(continuous_jog_command(&j(S), w, h).unwrap(), "$J=G53Y0.0F2000");
        assert_eq!(continuous_jog_command(&j(Home), w, h).unwrap(), "$J=G90X0Y0F2000");
        assert!(continuous_jog_command(&j(Abort), w, h).is_none());
        let p = ContinuousJog { direction: Position, speed: 100.0, target: PointF::new(1.0, 2.0) };
        assert_eq!(continuous_jog_command(&p, w, h).unwrap(), "$J=G90X1.00Y2.00F100");
    }

    #[test]
    fn target_lock_semantics() {
        let mut t = JogTargets::default();
        assert_eq!(t.get_and_clear().0, None);
        t.to_direction(E, 1000.0).unwrap();
        let (j, abort) = t.get_and_clear();
        assert_eq!(j.unwrap().direction, E);
        assert!(!abort);
        // nothing new
        assert!(t.get_and_clear().0.is_none());
        // new request while previous is a movement: abort it first
        t.to_direction(N, 1000.0).unwrap();
        let (j, abort) = t.get_and_clear();
        assert_eq!(j.unwrap().direction, N);
        assert!(abort);
        // abort request, then another: no abort of an abort
        t.abort();
        let (j, abort) = t.get_and_clear();
        assert_eq!(j.unwrap().direction, Abort);
        assert!(abort);
        t.to_position(PointF::new(1.0, 1.0), 10.0);
        let (_, abort) = t.get_and_clear();
        assert!(!abort);
        assert!(t.to_direction(Zup, 1.0).is_err());
        assert!(t.to_direction(Abort, 1.0).is_err());
        assert!(t.to_direction(Position, 1.0).is_err());
    }
}
