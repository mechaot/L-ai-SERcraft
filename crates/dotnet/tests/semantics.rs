//! Hand-computed expectations from .NET Framework 4.x behaviour (not Mono-derived).

use lg_dotnet::*;

#[test]
fn double_to_string_rules() {
    assert_eq!(fmt_double(0.1 + 0.2), "0.3");
    assert_eq!(fmt_double(1.0), "1");
    assert_eq!(fmt_double(-0.0), "0");
    assert_eq!(fmt_double(1e15), "1E+15");
    assert_eq!(fmt_double(999999999999999.0), "999999999999999");
    assert_eq!(fmt_double(123456789012345680000.0), "1.23456789012346E+20");
    assert_eq!(fmt_double(0.0001), "0.0001");
    assert_eq!(fmt_double(0.00001), "1E-05");
    assert_eq!(fmt_double(1.5e-7), "1.5E-07");
    assert_eq!(fmt_double(f64::NAN), "NaN");
    assert_eq!(fmt_double(f64::INFINITY), "Infinity");
    assert_eq!(fmt_double(f64::NEG_INFINITY), "-Infinity");
    assert_eq!(fmt_double(100.0), "100");
    assert_eq!(fmt_double(1e100), "1E+100");
    assert_eq!(fmt_double(-1234.5678), "-1234.5678");
}

#[test]
fn float_to_string_rules() {
    assert_eq!(fmt_float(0.1), "0.1");
    assert_eq!(fmt_float(16777216.0), "1.677722E+07");
    assert_eq!(fmt_float(1234567.0), "1234567");
    assert_eq!(fmt_float(1e7), "1E+07");
    assert_eq!(fmt_float(1.0 / 3.0), "0.3333333");
    assert_eq!(fmt_float(-0.0), "0");
    assert_eq!(fmt_float(255.0 / 7.0), "36.42857");
}

#[test]
fn custom_format_midpoints() {
    // digit string (15 sig) is rounded half away from zero, not the binary value
    assert_eq!(fmt_double_custom(1.0005, "0.###"), "1.001");
    assert_eq!(fmt_double_custom(2.675, "0.00"), "2.68");
    assert_eq!(fmt_double_custom(-2.675, "0.00"), "-2.68");
    assert_eq!(fmt_double_custom(0.5, "0"), "1");
    assert_eq!(fmt_double_custom(1.5, "0"), "2");
    assert_eq!(fmt_double_custom(2.5, "0"), "3");
    assert_eq!(fmt_double_custom(-0.5, "0"), "-1");
    assert_eq!(fmt_double_custom(1234.5678, "#,##0.00"), "1,234.57");
    assert_eq!(fmt_double_custom(0.0, "0.###"), "0");
    assert_eq!(fmt_double_custom(5.0, "0.0#"), "5.0");
    assert_eq!(fmt_double_custom(5.25, "0.0#"), "5.25");
    assert_eq!(fmt_double_custom(0.5, "#0.00"), "0.50");
    assert_eq!(fmt_double_custom(7.0, "000"), "007");
    assert_eq!(fmt_double_custom(12.0, "+00.0;-00.0"), "+12.0");
    assert_eq!(fmt_double_custom(-12.0, "+00.0;-00.0"), "-12.0");
    assert_eq!(fmt_double_custom(1234567.0, "0,,"), "1");
    assert_eq!(fmt_double_custom(1234567.0, "0.0,,"), "1234567.0"); // commas after the point are ignored
    assert_eq!(fmt_double_custom(0.256, "0.0%"), "25.6%");
    assert_eq!(fmt_double_custom(12.5, "'x'0.0\"y\""), "x12.5y");
}

#[test]
fn negative_rounding_to_zero_has_no_sign() {
    // Mono and (per RoundNumber) .NET Fx drop the sign when the digits round to zero
    assert_eq!(fmt_double_custom(-0.0004, "0.###"), "0");
    assert_eq!(fmt_double_custom(-0.0004, "0.00"), "0.00");
    assert_eq!(fmt_double_custom(-0.0004, "F2"), "0.00");
    assert_eq!(fmt_double_custom(-0.0, "0.00"), "0.00");
    assert_eq!(fmt_double_custom(-0.0004, "0.0000"), "-0.0004");
}

#[test]
fn standard_formats() {
    assert_eq!(fmt_fixed(3.14999, 2), "3.15");
    assert_eq!(fmt_fixed(0.005, 2), "0.01");
    assert_eq!(fmt_fixed(-1234.5, 0), "-1235");
    assert_eq!(fmt_double_custom(1234567.891, "N"), "1,234,567.89");
    assert_eq!(fmt_double_custom(1234567.891, "N0"), "1,234,568");
    assert_eq!(fmt_double_custom(0.1234, "P1"), "12.3 %");
    assert_eq!(fmt_double_custom(0.5, "p0"), "50 %");
    assert_eq!(fmt_double_custom(12345.6789, "E3"), "1.235E+004");
    assert_eq!(fmt_double_custom(0.1, "G17"), "0.10000000000000001");
    assert_eq!(fmt_double_custom(0.1 + 0.2, "R"), "0.30000000000000004");
}

#[test]
fn integer_formats() {
    assert_eq!(fmt_i32(5, "D2"), "05");
    assert_eq!(fmt_i32(-5, "D3"), "-005");
    assert_eq!(fmt_i32(255, "X2"), "FF");
    assert_eq!(fmt_i32(10, "x8"), "0000000a");
    assert_eq!(fmt_i32(-1, "x"), "ffffffff");
    assert_eq!(fmt_i64(-1, "X"), "FFFFFFFFFFFFFFFF");
    assert_eq!(fmt_u32(7, "X2"), "07");
    assert_eq!(fmt_i32(42, "000000000"), "000000042");
    assert_eq!(fmt_i32(1234567, "N0"), "1,234,567");
    assert_eq!(fmt_i32(7, ""), "7");
}

#[test]
fn math_round_net_framework() {
    // x64 (SSE2) semantics: 2.675 * 100 = 267.5 exactly in f64 -> even -> 268 -> 2.68.
    // (32-bit x87 JIT keeps 80 bit and would give 2.67; not reproduced.)
    assert_eq!(round_double(2.675, 2), 2.68);
    assert_eq!(round_double(1.005, 2), 1.0);
    assert_eq!(round_double(0.285, 2), 0.28);
    assert_eq!(round_double(2.5, 0), 2.0);
    assert_eq!(round_double(3.5, 0), 4.0);
    assert_eq!(round_double(-2.5, 0), -2.0);
    assert_eq!(round_double(1.45, 1), 1.4); // 14.5 exact -> even
    assert_eq!(round_double(1.55, 1), 1.6); // 15.5 exact -> even 16
    assert_eq!(round_double(123.456789, 3), 123.457);
    assert_eq!(round_double(1e17 + 8.0, 2), 1e17 + 8.0); // >= 1e16 untouched
    assert!(round_double(f64::NAN, 2).is_nan());
    assert_eq!(round_half_even(0.5), 0.0);
    assert_eq!(round_half_even(1.5), 2.0);
    assert_eq!(round_half_even(-1.5), -2.0);
    assert_eq!(round_half_even(2.4999), 2.0);
}

#[test]
fn math_round_away_from_zero() {
    assert_eq!(round_double_away(2.5, 0), 3.0);
    assert_eq!(round_double_away(-2.5, 0), -3.0);
    assert_eq!(round_double_away(0.5, 0), 1.0);
    assert_eq!(round_double_away(-0.5, 0), -1.0);
    assert_eq!(round_double_away(1.45, 1), 1.5);
    assert_eq!(round_double_away(2.675, 2), 2.68); // 267.5 exactly after the f64 multiply
    assert_eq!(round_double_away(1.005, 2), 1.0); // 100.49999999999999
    assert_eq!(round_double_away(2.4999, 0), 2.0);
    assert_eq!(round_double_away(1.5, 1), 1.5);
}

#[test]
#[should_panic]
fn round_digits_out_of_range_panics() {
    let _ = round_double(1.0, 16);
}

#[test]
fn int_casts() {
    assert_eq!(f64_to_i32(1.9), 1);
    assert_eq!(f64_to_i32(-1.9), -1);
    assert_eq!(f64_to_i32(f64::NAN), i32::MIN);
    assert_eq!(f64_to_i32(f64::INFINITY), i32::MIN);
    assert_eq!(f64_to_i32(2147483648.0), i32::MIN);
    assert_eq!(f64_to_i32(2147483647.9), i32::MAX);
    assert_eq!(f64_to_i32(-2147483648.9), i32::MIN);
    assert_eq!(f64_to_i32(-2147483649.0), i32::MIN);
    assert_eq!(f64_to_i64(1e10), 10_000_000_000);
    assert_eq!(f64_to_i64(1e19), i64::MIN);
    assert_eq!(f64_to_i64(f64::NAN), i64::MIN);
    assert_eq!(f32_to_i32(3.99), 3);
}

#[test]
fn parsing() {
    assert_eq!(parse_double(" 1.5 "), Some(1.5));
    assert_eq!(parse_double("1,234.5"), Some(1234.5));
    assert_eq!(parse_double("1e3"), Some(1000.0));
    assert_eq!(parse_double("-.5"), Some(-0.5));
    assert_eq!(parse_double("1e309"), None);
    assert_eq!(parse_double(""), None);
    assert_eq!(parse_double("abc"), None);
    assert_eq!(parse_double("1.2.3"), None);
    assert!(parse_double("-0").unwrap().is_sign_positive()); // Fx quirk
    assert_eq!(parse_float("0.1"), Some(0.1f32));
    assert_eq!(parse_int("+12"), Some(12));
    assert_eq!(parse_int("12.0"), None);
    assert_eq!(parse_int("2147483648"), None);
    assert_eq!(parse_int("-2147483648"), Some(i32::MIN));
    assert_eq!(parse_int(" 7\t"), Some(7));
    assert_eq!(parse_decimal("1.50").map(fmt_decimal).as_deref(), Some("1.50"));
    assert_eq!(parse_decimal("1,000.5").map(fmt_decimal).as_deref(), Some("1000.5"));
    assert_eq!(parse_decimal("1e2"), None);
    assert_eq!(parse_decimal("1-").map(fmt_decimal).as_deref(), Some("-1"));
}

#[test]
fn decimal_basics() {
    let d = parse_decimal("1000.000").unwrap();
    assert_eq!(fmt_decimal(d), "1000.000");
    assert_eq!(fmt_decimal_custom(d, "0.##"), "1000");
    assert_eq!(fmt_decimal(f64_to_decimal(0.1 + 0.2)), "0.3");
    assert_eq!(fmt_decimal(f64_to_decimal(100.0)), "100");
    assert_eq!(fmt_decimal(f64_to_decimal(1.0 / 3.0)), "0.333333333333333");
    assert_eq!(fmt_decimal(f32_to_decimal(0.1)), "0.1");
    assert_eq!(decimal_to_f64(parse_decimal("0.1").unwrap()), 0.1);
    assert_eq!(fmt_decimal(round_decimal(parse_decimal("2.5").unwrap(), 0)), "2");
    assert_eq!(fmt_decimal(round_decimal_away(parse_decimal("2.5").unwrap(), 0)), "3");
    assert_eq!(fmt_decimal(dec_div(parse_decimal("1").unwrap(), parse_decimal("3").unwrap()).unwrap()), "0.3333333333333333333333333333");
    assert!(dec_div(Decimal::ONE, Decimal::ZERO).is_none());
}

#[test]
fn timespan_interval_rounds_to_ms() {
    assert_eq!(TimeSpan::from_minutes(1.5).ticks(), 900_000_000);
    assert_eq!(TimeSpan::from_seconds(0.0004).ticks(), 0);
    assert_eq!(TimeSpan::from_seconds(0.0005).ticks(), 10_000);
    assert_eq!(TimeSpan::from_seconds(-0.0005).ticks(), -10_000);
    assert_eq!(TimeSpan::from_seconds(1.0 / 3.0).ticks(), 3_330_000);
    assert_eq!(TimeSpan::from_milliseconds(0.5).ticks(), 10_000);
    assert_eq!(TimeSpan::from_hours(26.0).to_string(), "1.02:00:00");
    assert_eq!(TimeSpan::from_ticks(-1).to_string(), "-00:00:00.0000001");
    assert_eq!(TimeSpan::ZERO.to_string(), "00:00:00");
    let a = TimeSpan::from_seconds(90.0);
    let b = TimeSpan::from_seconds(30.0);
    assert_eq!((a + b).total_minutes(), 2.0);
    assert_eq!((a - b).total_seconds(), 60.0);
    assert!(a > b);
    assert_eq!(a.minutes(), 1);
    assert_eq!(a.seconds(), 30);
}

#[test]
fn chars_and_split() {
    assert!(is_digit('7') && is_digit('\u{0661}') && !is_digit('a'));
    assert!(is_letter('x') && is_letter('\u{00e9}') && !is_letter('1'));
    assert!(is_number('\u{00bd}') && !is_digit('\u{00bd}'));
    assert!(is_white_space('\u{00a0}') && is_white_space('\t') && !is_white_space('x'));
    assert_eq!(split_keep_empty("a,,b,", ','), vec!["a", "", "b", ""]);
}

/// Every format string used by the original sources must at least format without panic.
#[test]
fn all_original_formats_work() {
    let doubles = [0.0, 1.0, -1.5, 123.456789, 1e-7, 1e9, 0.5];
    for f in ["0.0", "0.00", "0.000", "0.0000", "0.###", "0.##", "F6", "P1", "00000", "000000000", "00000000",
              "000.##", "00000.##", "00.000", "+00.0;-00.0", "+00.00000;-00.00000", "G", "N", "p2"] {
        for v in doubles {
            let _ = fmt_double_custom(v, f);
            let _ = fmt_float_custom(v as f32, f);
            let _ = fmt_decimal_custom(f64_to_decimal(v), f);
        }
    }
    for f in ["X", "X2", "x", "x2", "x8", "D2", "00000"] {
        let _ = fmt_i32(171, f);
        let _ = fmt_u32(171, f);
    }
}

/// Cheap throughput smoke test (G-code writes millions of numbers).
#[test]
fn formatting_is_fast_enough() {
    let t = std::time::Instant::now();
    let mut n = 0usize;
    for i in 0..100_000 {
        let v = i as f64 * 0.0137 + 0.0005;
        n += fmt_double_custom(v, "0.###").len() + fmt_double(v).len();
    }
    assert!(n > 0);
    // debug build budget, only guards against pathological slowness (exact-expansion path)
    assert!(t.elapsed().as_secs() < 20, "too slow: {:?}", t.elapsed());
}
