//! `lg-svg2gcode`: port of LaserGRBL `SvgConverter/{GCodeFromSVG,gcodeRelated,BezierTools}.cs`
//! plus the WPF `Matrix`/`Point`/`Vector` and `ColorTranslator.FromHtml` pieces they rely on.

mod bezier;
mod color;
mod color_table;
mod convert;
mod gcode;
mod matrix;

pub use bezier::flatten_to;
pub use color::{ParsedColor, from_html as color_from_html};
pub use convert::{ColorFilter, GCodeFromSvg, LayerInfo, LayerParams, LayerRanges, list_layers_from_file, list_layers_from_text, PointF, bezier_approximation_old, convert_to_pixel, get_text_between, parse_transform_string};
pub use gcode::{Firmware, Gcode, SvgConvOptions};
pub use matrix::{Matrix, NotInvertible, Point, Vector};

/// Stand-in for a .NET exception thrown by the original converter.
///
/// `kind` is the exception's full type name (e.g. `System.IndexOutOfRangeException`,
/// `System.NullReferenceException`, `System.Xml.XmlException`, `System.ArgumentException`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvgError {
    /// Full .NET exception type name.
    pub kind: String,
    /// Human readable detail.
    pub message: String,
}

impl SvgError {
    /// Build an error. `kind`: .NET type name. `message`: detail text. Returns the error.
    pub fn new(kind: &str, message: impl Into<String>) -> Self {
        SvgError { kind: kind.to_string(), message: message.into() }
    }

    /// `IndexOutOfRangeException` shortcut. Returns the error.
    pub(crate) fn index() -> Self {
        Self::new("System.IndexOutOfRangeException", "Index was outside the bounds of the array.")
    }

    /// `NullReferenceException` shortcut. Returns the error.
    pub(crate) fn null_ref() -> Self {
        Self::new("System.NullReferenceException", "Object reference not set to an instance of an object.")
    }
}

impl std::fmt::Display for SvgError {
    /// Format as `kind: message`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for SvgError {}
