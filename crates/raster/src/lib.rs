//! `lg-raster`: port of `RasterConverter/ImageTransform.cs` and `RasterConverter/Dithering/*`.

pub mod color_matrix;
pub mod dithering;
pub mod image_transform;
pub mod resize;

pub use color_matrix::{ColorMatrix, ColorMatrixMode};
pub use image_transform::*;
