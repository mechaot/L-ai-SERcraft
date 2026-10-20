//! Port of `Tools/MathHelper.cs`: `MathHelper` and `RulerStepCalculator`.

use lg_dotnet::{dec_add, dec_mul, dec_sub, decimal_to_f64, f64_to_decimal, Decimal};

/// Euclidean distance in decimal (C# `MathHelper.LinearDistance`).
///
/// `cur_x`,`cur_y`: start point. `new_x`,`new_y`: end point. Returns distance as decimal.
pub fn linear_distance(cur_x: Decimal, cur_y: Decimal, new_x: Decimal, new_y: Decimal) -> Decimal {
    // C# throws OverflowException on decimal overflow; we panic
    let ov = "decimal overflow";
    let dx = dec_sub(new_x, cur_x).expect(ov);
    let dy = dec_sub(new_y, cur_y).expect(ov);
    let sq = dec_add(dec_mul(dx, dx).expect(ov), dec_mul(dy, dy).expect(ov)).expect(ov);
    f64_to_decimal(decimal_to_f64(sq).sqrt())
}

/// Nice step sizes for ruler ticks.
#[derive(Debug, Clone, Copy)]
pub struct RulerStepCalculator {
    min: f64,
    max: f64,
    steps: i32,
    bigstep: f64,
    smallstep: f64,
    firstsmall: f64,
    firstbig: f64,
}

impl RulerStepCalculator {
    /// Compute steps.
    ///
    /// `min`,`max`: visible range. `steps`: wanted number of ticks.
    pub fn new(min: f64, max: f64, steps: i32) -> Self {
        const MTABLE: [f64; 4] = [1.0, 2.0, 5.0, 10.0];
        const STABLE: [f64; 4] = [2.0, 2.0, 5.0, 10.0];
        let mut logar = ((max - min) / steps as f64).log10();
        let multiplier = 10f64.powf(logar.floor());
        let mut i = 0usize;
        // IEEERemainder(logar, 1.0)
        logar -= logar.round_ties_even();
        if logar < 0.0 {
            logar += 1.0;
        }
        while logar >= MTABLE[i + 1].log10() {
            i += 1;
        }
        let bigstep = MTABLE[i] * multiplier;
        let smallstep = bigstep / STABLE[i];
        Self {
            min,
            max,
            steps,
            bigstep,
            smallstep,
            firstsmall: (min / smallstep).ceil() * smallstep,
            firstbig: (min / bigstep).ceil() * bigstep,
        }
    }

    /// Requested min.
    pub fn req_min(&self) -> f64 {
        self.min
    }
    /// Requested max.
    pub fn req_max(&self) -> f64 {
        self.max
    }
    /// Requested step count.
    pub fn req_steps(&self) -> f64 {
        self.steps as f64
    }
    /// Big tick step.
    pub fn big_step(&self) -> f64 {
        self.bigstep
    }
    /// Small tick step.
    pub fn small_step(&self) -> f64 {
        self.smallstep
    }
    /// First small tick >= min.
    pub fn first_small(&self) -> f64 {
        self.firstsmall
    }
    /// First big tick >= min.
    pub fn first_big(&self) -> f64 {
        self.firstbig
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruler_basic() {
        let r = RulerStepCalculator::new(0.0, 100.0, 10);
        assert_eq!(r.big_step(), 10.0);
        assert_eq!(r.small_step(), 5.0);
        let r = RulerStepCalculator::new(3.0, 53.0, 10);
        assert_eq!(r.big_step(), 5.0);
        assert_eq!(r.small_step(), 1.0);
        assert_eq!(r.first_big(), 5.0);
        assert_eq!(r.first_small(), 3.0);
        let r = RulerStepCalculator::new(0.0, 2.0, 10);
        assert_eq!(r.big_step(), 0.2);
    }

    #[test]
    fn distance() {
        let d = linear_distance(Decimal::from(0), Decimal::from(0), Decimal::from(3), Decimal::from(4));
        assert_eq!(lg_dotnet::fmt_decimal(d), "5");
    }
}
