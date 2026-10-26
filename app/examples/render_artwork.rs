// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Andreas Pösch

//! Render branding SVGs (resources/branding/*.svg) to PNG. Text uses installed system fonts.
//!
//! Usage: `cargo run -p laisercraft --example render_artwork -- <in.svg> <out.png> <width_px>`

use std::path::Path;

/// Render one SVG to a PNG of the given width (height keeps the aspect ratio).
///
/// `input`: SVG file. `output`: PNG file. `width`: target width in px. Returns an error text on failure.
fn render(input: &Path, output: &Path, width: u32) -> Result<(), String> {
    let data = std::fs::read(input).map_err(|e| format!("read {}: {e}", input.display()))?;
    let mut opt = resvg::usvg::Options::default();
    // wordmark text needs real fonts
    opt.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_data(&data, &opt).map_err(|e| format!("parse: {e}"))?;
    let size = tree.size();
    let scale = width as f32 / size.width();
    let height = (size.height() * scale).round() as u32;
    let mut pm = resvg::tiny_skia::Pixmap::new(width, height).ok_or("bad size")?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pm.as_mut());
    // tiny-skia stores premultiplied RGBA; encode via image crate as straight RGBA
    let img = image::RgbaImage::from_fn(width, height, |x, y| {
        let c = pm.pixel(x, y).expect("pixel").demultiply();
        image::Rgba([c.red(), c.green(), c.blue(), c.alpha()])
    });
    img.save(output).map_err(|e| format!("write {}: {e}", output.display()))
}

/// Entry: parse args and render. Returns exit code 2 on bad args, 1 on render error.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (Some(i), Some(o), Some(w)) = (a.first(), a.get(1), a.get(2).and_then(|w| w.parse().ok())) else {
        eprintln!("usage: render_artwork <in.svg> <out.png> <width_px>");
        std::process::exit(2);
    };
    if let Err(e) = render(Path::new(i), Path::new(o), w) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
