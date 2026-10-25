// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `Icons/IconsMgr.cs`, `Icons/IIconsLoader.cs`, `Icons/LegacyIcons.cs`, `SvgIcons/SvgIcons.cs`.
//!
//! Two loaders (setting "LegacyIcons", bool, default false): SVG packs (rendered by resvg, tinted per
//! color tables) or legacy base64 PNGs (no tint). Textures are cached per (key, pixel size, tint).

use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::theme::Theme;

static MDI_DATA: &str = include_str!("../../../resources/svgicons/Mdi.data");
static MDI_HEADER: &str = include_str!("../../../resources/svgicons/Mdi.header");
static CUSTOM_DATA: &str = include_str!("../../../resources/svgicons/Custom.data");
static CUSTOM_HEADER: &str = include_str!("../../../resources/svgicons/Custom.header");
static FLAGS_DATA: &str = include_str!("../../../resources/svgicons/Flags.data");
static FLAGS_HEADER: &str = include_str!("../../../resources/svgicons/Flags.header");
static LEGACY_DATA: &str = include_str!("../../../resources/icons/LegacyIcons.data");
static LEGACY_HEADER: &str = include_str!("../../../resources/icons/LegacyIcons.header");

/// Fallback key used by C# `IconsMgr.LoadImage` when a key is unknown.
pub const FALLBACK_KEY: &str = "mdi-square-rounded";

/// Settings key selecting the legacy loader.
pub const SETTING_LEGACY: &str = "LegacyIcons";

/// Light-theme color table (`mLightIconColors`); empty key = default.
const LIGHT: &[(&str, (u8, u8, u8))] = &[
    ("", (70, 70, 70)),
    ("custom-reset", (242, 165, 1)),
    ("mdi-lightning-bolt", (242, 165, 1)),
    ("mdi-arrow-up-bold-outline", (0, 122, 217)),
    ("mdi-arrow-top-right-bold-outline", (0, 122, 217)),
    ("mdi-arrow-right-bold-outline", (0, 122, 217)),
    ("mdi-arrow-bottom-right-bold-outline", (0, 122, 217)),
    ("mdi-arrow-down-bold-outline", (0, 122, 217)),
    ("mdi-arrow-bottom-left-bold-outline", (0, 122, 217)),
    ("mdi-arrow-left-bold-outline", (0, 122, 217)),
    ("mdi-arrow-top-left-bold-outline", (0, 122, 217)),
    ("custom-zup01", (79, 188, 243)),
    ("custom-zup1", (79, 188, 243)),
    ("custom-zup10", (79, 188, 243)),
    ("custom-zdown01", (79, 188, 243)),
    ("custom-zdown1", (79, 188, 243)),
    ("custom-zdown10", (79, 188, 243)),
    ("custom-home", (79, 188, 243)),
    ("mdi-information-slab-box", (79, 188, 243)),
    ("mdi-safety-goggles", (79, 188, 243)),
    ("mdi-play", (17, 169, 38)),
    ("mdi-power-plug", (17, 169, 38)),
    ("mdi-checkbox-marked", (17, 169, 38)),
    ("mdi-play-circle", (17, 169, 38)),
    ("custom-resume", (17, 169, 38)),
    ("mdi-heart", (219, 20, 43)),
    ("mdi-stop", (219, 20, 43)),
    ("mdi-power-plug-off", (219, 20, 43)),
    ("mdi-close-box", (219, 20, 43)),
    ("mdi-stop-circle", (219, 20, 43)),
    ("custom-stop", (219, 20, 43)),
];

/// Dark-theme color table (`mDarkIconColors`).
const DARK: &[(&str, (u8, u8, u8))] = &[
    ("", (210, 210, 210)),
    ("custom-reset", (230, 230, 20)),
    ("mdi-lightning-bolt", (230, 230, 20)),
    ("mdi-arrow-up-bold-outline", (0, 122, 217)),
    ("mdi-arrow-top-right-bold-outline", (0, 122, 217)),
    ("mdi-arrow-right-bold-outline", (0, 122, 217)),
    ("mdi-arrow-bottom-right-bold-outline", (0, 122, 217)),
    ("mdi-arrow-down-bold-outline", (0, 122, 217)),
    ("mdi-arrow-bottom-left-bold-outline", (0, 122, 217)),
    ("mdi-arrow-left-bold-outline", (0, 122, 217)),
    ("mdi-arrow-top-left-bold-outline", (0, 122, 217)),
    ("custom-zup01", (79, 188, 243)),
    ("custom-zup1", (79, 188, 243)),
    ("custom-zup10", (79, 188, 243)),
    ("custom-zdown01", (79, 188, 243)),
    ("custom-zdown1", (79, 188, 243)),
    ("custom-zdown10", (79, 188, 243)),
    ("custom-home", (79, 188, 243)),
    ("mdi-information-slab-box", (79, 188, 243)),
    ("mdi-safety-goggles", (79, 188, 243)),
    ("mdi-play", (71, 200, 86)),
    ("mdi-power-plug", (71, 200, 86)),
    ("mdi-checkbox-marked", (71, 200, 86)),
    ("mdi-play-circle", (71, 200, 86)),
    ("custom-resume", (71, 200, 86)),
    ("mdi-heart", (248, 90, 90)),
    ("mdi-stop", (248, 90, 90)),
    ("mdi-power-plug-off", (248, 90, 90)),
    ("mdi-close-box", (248, 90, 90)),
    ("mdi-stop-circle", (248, 90, 90)),
    ("custom-stop", (248, 90, 90)),
];

/// How to tint a loaded icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tint {
    /// C# `colorize = true`: color from the light/dark table of the current scheme (SVG loader only).
    Theme,
    /// C# `colorize = false`: keep original colors (flags!).
    None,
    /// Force one color (e.g. disabled look). SVG loader only.
    Color(Color32),
}

/// Cache key: icon key, pixel size, resolved tint color.
type CacheKey = (String, u32, Option<[u8; 4]>);

/// Icon manager. Get the process-wide one with [`icons`].
pub struct Icons {
    legacy: bool,
    svg: HashMap<String, &'static str>,
    legacy_b64: HashMap<String, &'static str>,
    cache: Mutex<HashMap<CacheKey, TextureHandle>>,
}

static INSTANCE: OnceLock<Icons> = OnceLock::new();

/// Process-wide icon manager (C# `IconsMgr.Initialize` happens on first use; legacy flag read once).
pub fn icons() -> &'static Icons {
    INSTANCE.get_or_init(|| Icons::new(lg_base::settings::get(SETTING_LEGACY, false)))
}

/// Shortcut: themed-tint icon image at `size` points.
///
/// `ctx`: egui context. `key`: e.g. "mdi-power-plug". `size`: edge in points. Returns an `Image`.
pub fn icon(ctx: &egui::Context, key: &str, size: f32) -> egui::Image<'static> {
    icons().image(ctx, key, size)
}

/// Read paired header/data lines into `out` with key prefix.
fn load_pack(out: &mut HashMap<String, &'static str>, prefix: &str, header: &'static str, data: &'static str) {
    // BOM may prefix first header line
    let header = header.trim_start_matches('\u{feff}');
    for (h, d) in header.lines().zip(data.lines()) {
        out.insert(format!("{prefix}{}", h.trim()), d);
    }
}

impl Icons {
    /// Build loaders.
    ///
    /// `legacy`: true = LegacyIcons PNG loader, false = SVG loader.
    pub fn new(legacy: bool) -> Icons {
        let mut svg = HashMap::new();
        let mut leg = HashMap::new();
        if legacy {
            load_pack(&mut leg, "", LEGACY_HEADER, LEGACY_DATA);
        } else {
            load_pack(&mut svg, "mdi-", MDI_HEADER, MDI_DATA);
            load_pack(&mut svg, "custom-", CUSTOM_HEADER, CUSTOM_DATA);
            load_pack(&mut svg, "flags-", FLAGS_HEADER, FLAGS_DATA);
        }
        Icons { legacy, svg, legacy_b64: leg, cache: Mutex::new(HashMap::new()) }
    }

    /// True if legacy loader is active (`IconsMgr.LegacyIcons`).
    pub fn is_legacy(&self) -> bool {
        self.legacy
    }

    /// `IIconsLoader.Contains`.
    ///
    /// `key`: icon key. Returns whether the active loader knows it.
    pub fn contains(&self, key: &str) -> bool {
        if self.legacy { self.legacy_b64.contains_key(key) } else { self.svg.contains_key(key) }
    }

    /// All keys of the active loader starting with `prefix`, sorted.
    ///
    /// `prefix`: e.g. "custom-". Returns key list.
    pub fn keys(&self, prefix: &str) -> Vec<&str> {
        let it: Box<dyn Iterator<Item = &String>> =
            if self.legacy { Box::new(self.legacy_b64.keys()) } else { Box::new(self.svg.keys()) };
        let mut v: Vec<&str> = it.filter(|k| k.starts_with(prefix)).map(|k| k.as_str()).collect();
        v.sort_unstable();
        v
    }

    /// Table color for a key (`mIconColors[key]` else default entry).
    ///
    /// `key`: icon key. `dark`: use dark table. Returns tint color.
    pub fn color_for(key: &str, dark: bool) -> Color32 {
        let table = if dark { DARK } else { LIGHT };
        let find = |k: &str| table.iter().find(|(n, _)| *n == k).map(|(_, c)| *c);
        let (r, g, b) = find(key).or_else(|| find("")).unwrap_or((70, 70, 70));
        Color32::from_rgb(r, g, b)
    }

    /// Resolve tint to concrete color (None = keep).
    fn resolve(&self, key: &str, tint: Tint) -> Option<Color32> {
        if self.legacy {
            return None; // C#: colorize only for the SVG loader
        }
        match tint {
            Tint::None => None,
            Tint::Color(c) => Some(c),
            Tint::Theme => Some(Self::color_for(key, Theme::current().is_dark)),
        }
    }

    /// Get (or render + cache) a square texture.
    ///
    /// `ctx`: egui context. `key`: icon key (unknown -> fallback). `px`: edge in pixels. `tint`: see [`Tint`].
    /// Returns the texture handle.
    pub fn texture(&self, ctx: &egui::Context, key: &str, px: u32, tint: Tint) -> TextureHandle {
        let px = px.clamp(1, 2048);
        let color = self.resolve(key, tint);
        let ck: CacheKey = (key.to_owned(), px, color.map(|c| c.to_array()));
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(h) = cache.get(&ck) {
            return h.clone();
        }
        let img = self.render(key, px, color);
        let h = ctx.load_texture(format!("icon:{key}:{px}"), img, TextureOptions::LINEAR);
        cache.insert(ck, h.clone());
        h
    }

    /// Render one icon to pixels.
    fn render(&self, key: &str, px: u32, color: Option<Color32>) -> ColorImage {
        let empty = || ColorImage::filled([px as usize, px as usize], Color32::TRANSPARENT);
        if self.legacy {
            let Some(b64) = self.legacy_b64.get(key) else { return ColorImage::filled([1, 1], Color32::TRANSPARENT) };
            let Some(bytes) = base64_decode(b64) else { return empty() };
            let Ok(img) = image::load_from_memory(&bytes) else { return empty() };
            // resize to requested size like C# DrawImage
            let img = img.resize_exact(px, px, image::imageops::FilterType::CatmullRom).to_rgba8();
            return ColorImage::from_rgba_unmultiplied([px as usize, px as usize], img.as_raw());
        }
        let svg = self.svg.get(key).or_else(|| self.svg.get(FALLBACK_KEY));
        let Some(svg) = svg else { return empty() };
        render_svg(svg, px, color).unwrap_or_else(empty)
    }

    /// Themed-tint image (C# `colorize=true`).
    ///
    /// `ctx`: egui context. `key`: icon key. `size`: edge in points. Returns `egui::Image`.
    pub fn image(&self, ctx: &egui::Context, key: &str, size: f32) -> egui::Image<'static> {
        self.image_tinted(ctx, key, size, Tint::Theme)
    }

    /// Image with original colors (C# `colorize=false`).
    ///
    /// `ctx`: egui context. `key`: icon key. `size`: edge in points. Returns `egui::Image`.
    pub fn image_plain(&self, ctx: &egui::Context, key: &str, size: f32) -> egui::Image<'static> {
        self.image_tinted(ctx, key, size, Tint::None)
    }

    /// Image with explicit tint mode.
    ///
    /// `ctx`: egui context. `key`: icon key. `size`: edge in points. `tint`: tint mode. Returns `egui::Image`.
    pub fn image_tinted(&self, ctx: &egui::Context, key: &str, size: f32, tint: Tint) -> egui::Image<'static> {
        let px = (size * ctx.pixels_per_point()).round().max(1.0) as u32;
        let tex = self.texture(ctx, key, px, tint);
        let sized = egui::load::SizedTexture::new(tex.id(), egui::vec2(size, size));
        egui::Image::from_texture(sized).fit_to_exact_size(egui::vec2(size, size))
    }
}

/// Render SVG text into a square, aspect-fit, optionally flat-tinted image.
///
/// `svg`: SVG source. `px`: edge. `color`: if set, every pixel gets this RGB (alpha kept = C# MakeWhite + SetColor).
/// Returns image or None on parse error.
fn render_svg(svg: &str, px: u32, color: Option<Color32>) -> Option<ColorImage> {
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg, &opt).ok()?;
    let mut pm = resvg::tiny_skia::Pixmap::new(px, px)?;
    let s = tree.size();
    let scale = (px as f32 / s.width()).min(px as f32 / s.height());
    let tx = (px as f32 - s.width() * scale) / 2.0;
    let ty = (px as f32 - s.height() * scale) / 2.0;
    let ts = resvg::tiny_skia::Transform::from_scale(scale, scale).post_translate(tx, ty);
    resvg::render(&tree, ts, &mut pm.as_mut());
    let mut data = pm.take(); // premultiplied RGBA
    if let Some(c) = color {
        // premultiplied tint: rgb = color * a
        for p in data.chunks_exact_mut(4) {
            let a = p[3] as u32;
            p[0] = (c.r() as u32 * a / 255) as u8;
            p[1] = (c.g() as u32 * a / 255) as u8;
            p[2] = (c.b() as u32 * a / 255) as u8;
        }
    }
    Some(ColorImage::from_rgba_premultiplied([px as usize, px as usize], &data))
}

/// Minimal standard base64 decoder (`Convert.FromBase64String`), ignores whitespace and BOM.
///
/// `s`: base64 text. Returns bytes or None on invalid input.
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for ch in s.trim_start_matches('\u{feff}').bytes() {
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b' ' | b'\r' | b'\n' | b'\t' => continue,
            _ => return None,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_pack_loads() {
        let i = Icons::new(false);
        assert!(i.contains("mdi-power-plug"));
        assert!(i.contains("custom-zup10"));
        assert!(i.contains("flags-it"));
        assert!(i.contains(FALLBACK_KEY));
    }

    #[test]
    fn legacy_pack_loads_and_decodes() {
        let i = Icons::new(true);
        let key = i.keys("").first().map(|s| s.to_string()).unwrap();
        let b = base64_decode(i.legacy_b64.get(key.as_str()).unwrap()).unwrap();
        assert_eq!(&b[1..4], b"PNG");
    }

    #[test]
    fn render_tints_and_keeps_alpha() {
        let i = Icons::new(false);
        let img = i.render("mdi-power-plug", 24, Some(Color32::from_rgb(255, 0, 0)));
        assert!(img.pixels.iter().any(|p| p.a() == 255 && p.r() == 255 && p.g() == 0));
        assert!(img.pixels.iter().any(|p| p.a() == 0));
    }

    #[test]
    fn color_tables() {
        assert_eq!(Icons::color_for("mdi-play", false), Color32::from_rgb(17, 169, 38));
        assert_eq!(Icons::color_for("mdi-unknown", true), Color32::from_rgb(210, 210, 210));
    }
}
