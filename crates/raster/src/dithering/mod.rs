//! Port of `RasterConverter/Dithering/*` (Cyotek dithering classes).

pub mod argb_color;
pub mod error_diffusion;
pub mod image_utilities;
pub mod random;

pub use argb_color::ArgbColor;
pub use error_diffusion::{ErrorDiffusion, ErrorDiffusionDithering};
pub use random::{DotNetRandom, RandomDithering};
