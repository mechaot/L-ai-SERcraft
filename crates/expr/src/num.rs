//! .NET number glue for `Tools/ExpressionEvaluator.cs`: `double.Parse` / `double.ToString` wrappers.

use lg_dotnet::{fmt_double, parse_double};

/// `double.Parse(s, InvariantCulture)` incl. the "NaN" / "Infinity" symbols.
///
/// * `s` - text to parse.
///
/// Returns `None` where C# would throw (FormatException / OverflowException).
pub fn parse_net(s: &str) -> Option<f64> {
    if let Some(v) = parse_double(s) {
        return Some(v);
    }
    // symbols .NET (Fx / Mono) accepts, case-sensitive, "-Infinity" only (Mono rejects "+Infinity")
    match s.trim() {
        "NaN" => Some(f64::NAN),
        "Infinity" => Some(f64::INFINITY),
        "-Infinity" => Some(f64::NEG_INFINITY),
        _ => None,
    }
}

/// `double.ToString(InvariantCulture)`.
///
/// * `v` - value to format.
///
/// Returns the 15-significant-digit text used on the C# work stack.
pub fn to_net(v: f64) -> String {
    fmt_double(v)
}
