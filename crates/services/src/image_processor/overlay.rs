//! Pixel drawing helpers of the previews (GDI+ `Graphics` calls reproduced by direct pixel writes).
//!
//! `PreviewLineByLine` draws 1 px white lines with alpha on an opaque grey bitmap. GDI+ draws
//! non-antialiased 1 px lines on the pixel grid; Bresenham gives the same pixels for horizontal,
//! vertical and 45 degree lines (the only ones used).

use super::{Direction, Tool};
use resvg::tiny_skia::Pixmap;
use std::sync::atomic::{AtomicBool, Ordering};

/// SourceOver of a white pixel with `alpha` on a straight-alpha RGBA pixel.
///
/// `px`: destination pixel (4 bytes). `alpha`: source alpha 0..=255.
fn blend_white(px: &mut [u8], alpha: u32) {
    let da = px[3] as u32;
    if da == 255 {
        for c in &mut px[..3] {
            *c = ((255 * alpha + *c as u32 * (255 - alpha) + 127) / 255) as u8;
        }
        return;
    }
    // general straight alpha: out_a = a + da*(1-a)
    let out_a = alpha * 255 + da * (255 - alpha);
    if out_a == 0 {
        return;
    }
    for c in &mut px[..3] {
        let num = 255 * alpha * 255 + *c as u32 * da * (255 - alpha);
        *c = ((num + out_a / 2) / out_a).min(255) as u8;
    }
    px[3] = ((out_a + 127) / 255).min(255) as u8;
}

/// Line drawer bound to an RGBA buffer.
struct Canvas<'a> {
    buf: &'a mut [u8],
    w: i32,
    h: i32,
    alpha: u32,
}

impl Canvas<'_> {
    fn plot(&mut self, x: i32, y: i32) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            let i = ((y * self.w + x) * 4) as usize;
            blend_white(&mut self.buf[i..i + 4], self.alpha);
        }
    }

    /// `DrawLine(pen, x0, y0, x1, y1)` for horizontal, vertical and 45 degree lines (inclusive ends).
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let (dx, dy) = ((x1 - x0).signum(), (y1 - y0).signum());
        let n = (x1 - x0).abs().max((y1 - y0).abs());
        for k in 0..=n {
            self.plot(x0 + dx * k, y0 + dy * k);
        }
    }
}

/// Direction that `PreviewLineByLine` draws for the current options.
///
/// `tool`: selected tool. `line_preview`: "preview" checkbox. `line_dir`/`fill_dir`: directions.
/// Returns the direction (None = nothing to draw). Mirrors the C# if chain, including its
/// dangling `else`.
pub fn preview_direction(tool: Tool, line_preview: bool, line_dir: Direction, fill_dir: Direction) -> Direction {
    let mut dir = Direction::None;
    if tool == Tool::Line2Line && line_preview {
        dir = line_dir;
    }
    if tool == Tool::Dithering && line_preview {
        dir = line_dir;
    } else if tool == Tool::Vectorize && fill_dir != Direction::None {
        dir = fill_dir;
    }
    if tool == Tool::NoProcessing {
        dir = Direction::Horizontal;
    }
    dir
}

/// `PreviewLineByLine(bmp)`: overlay the scan line pattern.
///
/// `buf`: RGBA bytes (straight alpha), `w` x `h` pixels. `tool`,`line_preview`,`line_dir`,
/// `fill_dir`: options (see [`preview_direction`]). `must_exit`: abort flag checked per line.
pub fn preview_line_by_line(buf: &mut [u8], w: i32, h: i32, tool: Tool, line_preview: bool, line_dir: Direction, fill_dir: Direction, must_exit: &AtomicBool) {
    use Direction::*;
    let dir = preview_direction(tool, line_preview, line_dir, fill_dir);
    if must_exit.load(Ordering::Relaxed) || dir == None {
        return;
    }
    let dithering = tool == Tool::Dithering;
    let exit = || must_exit.load(Ordering::Relaxed);
    if matches!(dir, Horizontal | NewHorizontal | NewGrid | NewCross) {
        let m = if dir == Horizontal { 2 } else { 3 };
        let mut c = Canvas { buf: &mut *buf, w, h, alpha: if dithering { 100 } else { 200 } };
        for y in 0..h {
            if exit() {
                break;
            }
            if y % m == 0 {
                c.line(0, y, w, y);
            }
        }
    }
    if matches!(dir, Vertical | NewVertical | NewGrid | NewCross) {
        let m = if dir == Vertical { 2 } else { 3 };
        let mut c = Canvas { buf: &mut *buf, w, h, alpha: if dithering { 100 } else { 200 } };
        for x in 0..w {
            if exit() {
                break;
            }
            if x % m == 0 {
                c.line(x, 0, x, h);
            }
        }
    }
    if matches!(dir, Diagonal | NewDiagonal | NewDiagonalGrid | NewDiagonalCross | NewSquares | NewZigZag) {
        let m = if dir == Diagonal { 3 } else { 5 };
        let mut c = Canvas { buf: &mut *buf, w, h, alpha: if dithering { 150 } else { 255 } };
        for i in 0..(w + h - 1) {
            if exit() {
                break;
            }
            if i % m == 0 {
                c.line(0, h - i, i, h);
            }
        }
    }
    if matches!(dir, NewReverseDiagonal | NewDiagonalGrid | NewDiagonalCross | NewSquares) {
        let mut c = Canvas { buf: &mut *buf, w, h, alpha: if dithering { 150 } else { 255 } };
        for i in 0..(w + h - 1) {
            if exit() {
                break;
            }
            if i % 5 == 0 {
                c.line(0, i, i, 0);
            }
        }
    }
}

/// `g.FillRectangle(new SolidBrush(Color.FromArgb(180, Color.White)), g.ClipBounds)`.
///
/// `buf`: RGBA bytes (straight alpha).
pub fn white_veil(buf: &mut [u8]) {
    for px in buf.chunks_exact_mut(4) {
        blend_white(px, 180);
    }
}

/// Straight-alpha RGBA image to a premultiplied pixmap.
///
/// `img`: source. Returns the pixmap or None for an empty image.
pub fn image_to_pixmap(img: &image::RgbaImage) -> Option<Pixmap> {
    let mut pm = Pixmap::new(img.width(), img.height())?;
    for (d, s) in pm.data_mut().chunks_exact_mut(4).zip(img.as_raw().chunks_exact(4)) {
        let a = s[3] as u32;
        d[0] = ((s[0] as u32 * a + 127) / 255) as u8;
        d[1] = ((s[1] as u32 * a + 127) / 255) as u8;
        d[2] = ((s[2] as u32 * a + 127) / 255) as u8;
        d[3] = s[3];
    }
    Some(pm)
}

/// Premultiplied pixmap back to a straight-alpha image.
///
/// `pm`: source. Returns the image.
pub fn pixmap_to_image(pm: &Pixmap) -> image::RgbaImage {
    let mut out = image::RgbaImage::new(pm.width(), pm.height());
    for (d, s) in out.as_mut().chunks_exact_mut(4).zip(pm.data().chunks_exact(4)) {
        let a = s[3] as u32;
        if a == 0 {
            d.copy_from_slice(&[0, 0, 0, 0]);
        } else {
            for c in 0..3 {
                d[c] = ((s[c] as u32 * 255 + a / 2) / a).min(255) as u8;
            }
            d[3] = s[3];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(w: u32, h: u32, v: u8) -> Vec<u8> {
        (0..w * h).flat_map(|_| [v, v, v, 255]).collect()
    }

    #[test]
    fn horizontal_every_second_row_alpha_200() {
        let (w, h) = (6, 6);
        let mut b = grey(w, h, 0);
        preview_line_by_line(&mut b, w as i32, h as i32, Tool::Line2Line, true, Direction::Horizontal, Direction::None, &AtomicBool::new(false));
        for y in 0..h {
            let px = &b[((y * w) * 4) as usize..][..4];
            // 0*55 + 255*200 = 51000/255 = 200
            assert_eq!(px, if y % 2 == 0 { [200, 200, 200, 255] } else { [0, 0, 0, 255] });
        }
    }

    #[test]
    fn vertical_mod3_dithering_alpha_100() {
        let (w, h) = (7, 2);
        let mut b = grey(w, h, 100);
        preview_line_by_line(&mut b, w as i32, h as i32, Tool::Dithering, true, Direction::NewVertical, Direction::None, &AtomicBool::new(false));
        let at = |x: u32, y: u32| b[((y * w + x) * 4) as usize];
        // (255*100 + 100*155 + 127) / 255 = 161
        assert_eq!((at(0, 0), at(1, 0), at(3, 1), at(6, 1)), (161, 100, 161, 161));
    }

    #[test]
    fn diagonal_pixels() {
        let (w, h) = (6, 6);
        let mut b = grey(w, h, 0);
        preview_line_by_line(&mut b, w as i32, h as i32, Tool::Line2Line, true, Direction::Diagonal, Direction::None, &AtomicBool::new(false));
        // I = 3: line (0,3)-(3,6): pixels (0,3) (1,4) (2,5); I = 0: (0,6) out of bounds
        assert_eq!(b[((3 * w) * 4) as usize], 255);
        assert_eq!(b[((4 * w + 1) * 4) as usize], 255);
        assert_eq!(b[((5 * w + 2) * 4) as usize], 255);
        assert_eq!(b[((3 * w + 1) * 4) as usize], 0);
    }

    #[test]
    fn none_and_abort_draw_nothing() {
        let mut b = grey(4, 4, 0);
        preview_line_by_line(&mut b, 4, 4, Tool::Line2Line, false, Direction::Horizontal, Direction::None, &AtomicBool::new(false));
        assert!(b.iter().step_by(4).all(|v| *v == 0));
        preview_line_by_line(&mut b, 4, 4, Tool::Line2Line, true, Direction::Horizontal, Direction::None, &AtomicBool::new(true));
        assert!(b.iter().step_by(4).all(|v| *v == 0));
    }

    #[test]
    fn direction_chain() {
        use Direction::*;
        assert_eq!(preview_direction(Tool::Vectorize, false, Horizontal, NewGrid), NewGrid);
        assert_eq!(preview_direction(Tool::Vectorize, true, Horizontal, None), None);
        assert_eq!(preview_direction(Tool::NoProcessing, false, None, None), Horizontal);
        assert_eq!(preview_direction(Tool::Centerline, true, Horizontal, NewGrid), None);
    }

    #[test]
    fn veil_and_pixmap_roundtrip() {
        let mut b = grey(1, 1, 0);
        white_veil(&mut b);
        // 255*180/255 = 180
        assert_eq!(b[0], 180);
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]));
        let pm = image_to_pixmap(&img).unwrap();
        assert_eq!(pixmap_to_image(&pm), img);
    }
}
