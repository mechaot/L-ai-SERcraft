//! Port of `CsPotrace/BezierToBiarc/*.cs` (bezier to biarc approximation, float precision like C#).

pub mod algorithm;
pub mod arc;
pub mod complex;
pub mod cubic_bezier;
pub mod line;
pub mod vector2;

pub use algorithm::approx_cubic_bezier;
pub use arc::{Arc, BiArc};
pub use complex::Complex;
pub use cubic_bezier::CubicBezier;
pub use line::Line;
pub use vector2::Vector2;
