# lg-dotnet

.NET Framework 4.x number, time and char semantics for the LaserGRBL Rust port. All output is
InvariantCulture. Golden data comes from Mono 6.12 (`tools/golden/dotnet/run.sh` regenerates
`tests/golden/dotnet/*.tsv` and `src/unicode_tables.rs`); `tests/golden.rs` compares byte for byte
(~190k rows). `tests/semantics.rs` holds hand-computed .NET Framework expectations.

## Semantics implemented

* `double.ToString()` = 15 significant digits ("G"), `E+15` / `E-05` style when exponent >= 15 or < -4
  (0.0001 stays fixed, 0.00001 is `1E-05`). `float` = 7 digits (digits come from the widened double).
  -0.0 prints `0`. NaN / `Infinity` / `-Infinity`.
* Custom formats (`0 # . , % ‰ 'lit' "lit" \x E+0 ;` sections) and standard formats (`F N P E G R`, `D X x`
  for integers). Digit string is built first (15 / 7 significant digits), then rounded half away from zero
  on the digit string: `fmt_double_custom(1.0005, "0.###") == "1.001"`.
* A value that rounds to zero loses its sign (`-0.0004` with `0.00` gives `0.00`), as Mono and the .NET Fx
  `RoundNumber` do.
* `decimal`: `fmt_decimal` keeps scale; `dec_add/sub/mul/div/rem` wrap `rust_decimal` and fix the scale rules
  that differ (zero results, division trailing zeros, remainder scale). `(decimal)double` is a port of oleaut
  `VarDecFromR8` (15 digits, FP multiply, ties-to-even), `(decimal)float` of `VarDecFromR4` (7 digits).
  `(double)decimal` = `(lo64 + hi32*2^64) / 10^scale` (not always correctly rounded, same as .NET).
* Parsing: `parse_double` (Float|AllowThousands), `parse_float` (parse as double, narrow, overflow = None),
  `parse_int` (Integer), `parse_decimal` (Number: incl. thousands and *trailing* sign, no exponent, 28-digit
  rounding like `NumberToDecimal`). `-0` parses to +0.0. Only `Infinity`, `-Infinity`, `NaN` (case sensitive).
  C# exceptions (format, overflow) are `None`.
* `round_double` / `round_double_away`: .NET Fx `Math.InternalRound` (multiply by 10^d in f64, round, divide);
  only for |v| < 1e16, digits 0..=15 (panics otherwise like the ArgumentOutOfRangeException).
* `f64_to_i32` etc.: unchecked x64 cast, NaN / out of range = `MIN`.
* `TimeSpan`: `from_*` round `value*scale` to whole milliseconds (half away from zero), `Total*` use the
  reciprocal multiply like Fx, `Display` is the "c" format.
* `is_digit/is_letter/is_number/is_white_space`: BMP tables generated from Mono (UTF-16 unit semantics:
  non-BMP chars are false).

## Known deviations / uncertainties

* `round_double(2.675, 2)` gives 2.68 here (x64 SSE2: `2.675*100 == 267.5` in f64). A 32-bit x87 JIT would
  give 2.67. Not reproduced.
* Sign dropping on round-to-zero and `-0` parse are verified against Mono only; the Fx source was recalled
  from memory.
* Unicode tables follow Mono 6.12's Unicode version, not necessarily Fx 4.x.
* `double.Parse` of an input a hair below half the smallest denormal (`2.4703282292062327e-324`): Mono rounds
  up, this crate rounds correctly (skipped in the golden test).
* Zero-product decimal scale when the other mantissa needs > 64 bits is empirical (matches Mono data).
* Unsupported standard letters (`C`, `D` on floats, ...) and out-of-range `TimeSpan`/`Math.Round` arguments
  panic where C# throws. `f64_to_decimal` saturates instead of throwing (`try_f64_to_decimal` returns None).
* Custom-format exponent for `E` uses only `E+0`, `E-0`, `E0` forms.
