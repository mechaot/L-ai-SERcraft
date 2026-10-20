//! Port of `RandomDithering.cs` plus .NET Framework `System.Random` (Knuth subtractive).

use super::argb_color::ArgbColor;
use super::error_diffusion::ErrorDiffusion;

const MBIG: i32 = i32::MAX;
const MSEED: i32 = 161_803_398;

/// .NET Framework `System.Random` seeded constructor, bit exact.
#[derive(Clone, Debug)]
pub struct DotNetRandom {
    inext: usize,
    inextp: usize,
    seed_array: [i32; 56],
}

impl DotNetRandom {
    /// C# `new Random(seed)`.
    /// Args: `seed` any int (negative uses abs, `i32::MIN` -> `i32::MAX`).
    /// Returns generator.
    pub fn new(seed: i32) -> Self {
        let mut sa = [0i32; 56];
        let subtraction = if seed == i32::MIN { i32::MAX } else { seed.abs() };
        let mut mj = MSEED - subtraction;
        sa[55] = mj;
        let mut mk = 1i32;
        for i in 1..55 {
            let ii = (21 * i) % 55;
            sa[ii] = mk;
            mk = mj - mk;
            if mk < 0 {
                mk += MBIG;
            }
            mj = sa[ii];
        }
        for _ in 1..5 {
            for i in 1..56 {
                sa[i] = sa[i].wrapping_sub(sa[1 + (i + 30) % 55]);
                if sa[i] < 0 {
                    sa[i] += MBIG;
                }
            }
        }
        Self { inext: 0, inextp: 21, seed_array: sa }
    }

    /// C# `InternalSample`. Returns int in `0..i32::MAX`.
    fn internal_sample(&mut self) -> i32 {
        let mut a = self.inext + 1;
        if a >= 56 {
            a = 1;
        }
        let mut b = self.inextp + 1;
        if b >= 56 {
            b = 1;
        }
        let mut ret = self.seed_array[a].wrapping_sub(self.seed_array[b]);
        if ret == MBIG {
            ret -= 1;
        }
        if ret < 0 {
            ret += MBIG;
        }
        self.seed_array[a] = ret;
        self.inext = a;
        self.inextp = b;
        ret
    }

    /// C# `Sample()`. Returns double in [0,1).
    fn sample(&mut self) -> f64 {
        self.internal_sample() as f64 * (1.0 / MBIG as f64)
    }

    /// C# `Next()`. Returns non-negative int.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> i32 {
        self.internal_sample()
    }

    /// C# `Next(min, max)`.
    /// Args: `min` inclusive lower; `max` exclusive upper (must be >= min).
    /// Returns int in `[min, max)`.
    pub fn next_range(&mut self, min: i32, max: i32) -> i32 {
        assert!(min <= max, "min > max");
        let range = max as i64 - min as i64;
        if range <= i32::MAX as i64 {
            (self.sample() * range as f64) as i32 + min
        } else {
            // large range path of .NET Fx GetSampleForLargeRange
            let mut result = self.internal_sample();
            if self.internal_sample() % 2 == 0 {
                result = -result;
            }
            let mut d = result as f64;
            d += (i32::MAX - 1) as f64;
            d /= 2.0 * (i32::MAX as f64) - 1.0;
            (d * range as f64) as i64 as i32 + min
        }
    }

    /// C# `NextDouble()`. Returns double in [0,1).
    pub fn next_double(&mut self) -> f64 {
        self.sample()
    }
}

/// C# `RandomDithering`.
#[derive(Clone, Debug)]
pub struct RandomDithering {
    random: DotNetRandom,
    white: ArgbColor,
    black: ArgbColor,
}

impl RandomDithering {
    /// C# `RandomDithering(int seed, ArgbColor white, ArgbColor black)`.
    /// Args: `seed` RNG seed; `white`/`black` output colors.
    /// Returns ditherer.
    pub fn with_colors(seed: i32, white: ArgbColor, black: ArgbColor) -> Self {
        Self { random: DotNetRandom::new(seed), white, black }
    }

    /// C# `RandomDithering(int seed)`: opaque white/black.
    /// Args: `seed` RNG seed. Returns ditherer.
    pub fn new(seed: i32) -> Self {
        Self::with_colors(seed, ArgbColor::new(255, 255, 255, 255), ArgbColor::new(255, 0, 0, 0))
    }

    /// C# `RandomDithering()`: seed from `Environment.TickCount` (ms clock, non reproducible).
    /// Returns ditherer.
    pub fn from_tick_count() -> Self {
        Self::new(tick_count())
    }
}

/// Emulates `Environment.TickCount` (ms, wraps as i32).
/// Returns ms since the unix epoch truncated to i32.
pub fn tick_count() -> i32 {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    ms as u64 as i32
}

impl ErrorDiffusion for RandomDithering {
    fn diffuse(
        &mut self,
        data: &mut [ArgbColor],
        original: ArgbColor,
        _transformed: ArgbColor,
        x: i32,
        y: i32,
        width: i32,
        _height: i32,
    ) {
        // double -> byte cast, truncates
        let gray = (0.299 * original.r as f64 + 0.587 * original.g as f64 + 0.114 * original.b as f64) as u8;
        data[(y * width + x) as usize] =
            if gray as i32 > self.random.next_range(0, 255) { self.white } else { self.black };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dotnet_framework_values() {
        // new Random(42).Next() == 1434747710 on .NET Framework
        assert_eq!(DotNetRandom::new(42).next(), 1_434_747_710);
    }

    #[test]
    fn next_range_in_bounds() {
        let mut r = DotNetRandom::new(1);
        for _ in 0..1000 {
            let v = r.next_range(0, 255);
            assert!((0..255).contains(&v));
        }
    }
}
