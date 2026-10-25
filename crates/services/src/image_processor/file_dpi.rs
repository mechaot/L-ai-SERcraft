// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! `Bitmap.HorizontalResolution` of the file (GDI+ reads it from the image header).
//!
//! The `image` crate does not expose DPI, so the headers are parsed minimally: PNG `pHYs`,
//! JPEG JFIF density, BMP `biXPelsPerMeter`. Everything else (and "unknown"/aspect-only
//! densities) gives 96 like GDI+.

/// GDI+ default resolution.
pub const DEFAULT_DPI: f32 = 96.0;

/// Pixels per metre to dots per inch (GDI+ float math).
///
/// `ppm`: pixels per metre. Returns dpi, or the default for 0.
fn from_ppm(ppm: u32) -> f32 {
    if ppm == 0 { DEFAULT_DPI } else { ppm as f32 * 0.0254 }
}

/// Horizontal resolution of an image file.
///
/// `bytes`: file content. Returns dpi as float (C# casts it to int, see [`file_dpi`]).
pub fn horizontal_resolution(bytes: &[u8]) -> f32 {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return png_dpi(bytes);
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        return jpeg_dpi(bytes);
    }
    if bytes.starts_with(b"BM") && bytes.len() >= 42 {
        let ppm = i32::from_le_bytes([bytes[38], bytes[39], bytes[40], bytes[41]]);
        return from_ppm(ppm.max(0) as u32);
    }
    DEFAULT_DPI
}

/// `(int)loadedBmp.HorizontalResolution`.
///
/// `bytes`: file content. Returns the truncated dpi.
pub fn file_dpi(bytes: &[u8]) -> i32 {
    horizontal_resolution(bytes) as i32
}

/// PNG: walk chunks until `pHYs` / `IDAT`.
fn png_dpi(b: &[u8]) -> f32 {
    let mut i = 8;
    while i + 8 <= b.len() {
        let len = u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize;
        let kind = &b[i + 4..i + 8];
        if kind == b"pHYs" && len >= 9 && i + 8 + 9 <= b.len() {
            let d = &b[i + 8..];
            let ppu = u32::from_be_bytes([d[0], d[1], d[2], d[3]]);
            // unit 1 = metre, 0 = unknown (aspect only)
            return if d[8] == 1 { from_ppm(ppu) } else { DEFAULT_DPI };
        }
        if kind == b"IDAT" || kind == b"IEND" {
            break;
        }
        i += 12 + len;
    }
    DEFAULT_DPI
}

/// JPEG: find the JFIF APP0 segment.
fn jpeg_dpi(b: &[u8]) -> f32 {
    let mut i = 2;
    while i + 4 <= b.len() {
        if b[i] != 0xFF {
            break;
        }
        let marker = b[i + 1];
        // stand-alone markers have no length
        if marker == 0xD8 || (0xD0..=0xD7).contains(&marker) || marker == 0x01 || marker == 0xFF {
            i += if marker == 0xFF { 1 } else { 2 };
            continue;
        }
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        if marker == 0xE0 && len >= 16 && i + 4 + 12 <= b.len() && &b[i + 4..i + 9] == b"JFIF\0" {
            let units = b[i + 11];
            let xd = u16::from_be_bytes([b[i + 12], b[i + 13]]) as f32;
            return match units {
                1 => xd,
                2 => xd * 2.54,
                _ => DEFAULT_DPI,
            };
        }
        // start of scan: header is over
        if marker == 0xDA {
            break;
        }
        i += 2 + len;
    }
    DEFAULT_DPI
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_with_phys(ppm: u32, unit: u8) -> Vec<u8> {
        let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
        // IHDR (dummy content, crc ignored)
        v.extend_from_slice(&13u32.to_be_bytes());
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&[0; 13]);
        v.extend_from_slice(&[0; 4]);
        v.extend_from_slice(&9u32.to_be_bytes());
        v.extend_from_slice(b"pHYs");
        v.extend_from_slice(&ppm.to_be_bytes());
        v.extend_from_slice(&ppm.to_be_bytes());
        v.push(unit);
        v.extend_from_slice(&[0; 4]);
        v
    }

    #[test]
    fn png_values() {
        assert_eq!(file_dpi(&png_with_phys(3780, 1)), 96);
        assert_eq!(file_dpi(&png_with_phys(2835, 1)), 72);
        // NOTE(upstream): 300 dpi PNGs (11811 ppm) truncate to 299 like the C# int cast
        assert_eq!(file_dpi(&png_with_phys(11811, 1)), 299);
        assert_eq!(file_dpi(&png_with_phys(5000, 0)), 96);
    }

    #[test]
    fn png_without_phys_and_unknown() {
        let img = image::RgbaImage::new(2, 2);
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        assert_eq!(file_dpi(buf.get_ref()), 96);
        assert_eq!(file_dpi(b"junk"), 96);
    }

    #[test]
    fn jpeg_jfif() {
        let mk = |units: u8, x: u16| {
            let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 16];
            v.extend_from_slice(b"JFIF\0");
            v.extend_from_slice(&[1, 1, units]);
            v.extend_from_slice(&x.to_be_bytes());
            v.extend_from_slice(&x.to_be_bytes());
            v.extend_from_slice(&[0, 0]);
            v
        };
        assert_eq!(file_dpi(&mk(1, 300)), 300);
        assert_eq!(file_dpi(&mk(2, 100)), 254);
        assert_eq!(file_dpi(&mk(0, 1)), 96);
    }

    #[test]
    fn bmp_header() {
        let mut v = vec![0u8; 54];
        v[0] = b'B';
        v[1] = b'M';
        v[38..42].copy_from_slice(&3780i32.to_le_bytes());
        assert_eq!(file_dpi(&v), 96);
        v[38..42].copy_from_slice(&0i32.to_le_bytes());
        assert_eq!(file_dpi(&v), 96);
        v[38..42].copy_from_slice(&11811i32.to_le_bytes());
        assert_eq!(file_dpi(&v), 299);
    }
}
