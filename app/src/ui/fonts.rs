//! Font setup: egui default fonts plus a system CJK fallback font when one exists.

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Name of the bold font family (`FontFamily::Name`), registered by `build_fonts` when a system bold font exists.
pub const BOLD_FAMILY: &str = "bold";

/// Set once a real bold font was installed.
static HAVE_BOLD: AtomicBool = AtomicBool::new(false);

/// Candidate bold sans fonts, most preferred first (matches the look of the egui default font best).
fn bold_candidates() -> Vec<PathBuf> {
    [
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-B.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        "/usr/share/fonts/liberation/LiberationSans-Bold.ttf",
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
        "C:\\Windows\\Fonts\\segoeuib.ttf",
        "C:\\Windows\\Fonts\\arialbd.ttf",
    ]
    .iter()
    .map(PathBuf::from)
    .collect()
}

/// Bold text helper (WinForms `FontStyle.Bold`).
///
/// `text`: caption. Returns a `RichText` in the bold family, or `.strong()` when no bold font was found.
pub fn bold(text: impl Into<String>) -> egui::RichText {
    let t = egui::RichText::new(text.into());
    if HAVE_BOLD.load(Ordering::Relaxed) {
        t.family(FontFamily::Name(BOLD_FAMILY.into()))
    } else {
        t.strong()
    }
}

/// Is a real bold font installed?
///
/// Returns true after `install` found one.
pub fn have_bold() -> bool {
    HAVE_BOLD.load(Ordering::Relaxed)
}

/// Candidate CJK font files, most preferred first.
fn candidates() -> Vec<PathBuf> {
    [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        "/usr/share/fonts/droid/DroidSansFallbackFull.ttf",
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyh.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
    ]
    .iter()
    .map(PathBuf::from)
    .collect()
}

/// Build font definitions: defaults + first readable CJK font appended as fallback.
///
/// Returns definitions and the path of the CJK font used (None if none found).
pub fn build_fonts() -> (FontDefinitions, Option<PathBuf>) {
    let mut fonts = FontDefinitions::default();
    let mut used = None;
    for p in candidates() {
        // first readable file wins
        if let Ok(bytes) = std::fs::read(&p) {
            fonts.font_data.insert("cjk-fallback".into(), Arc::new(FontData::from_owned(bytes)));
            for fam in [FontFamily::Proportional, FontFamily::Monospace] {
                fonts.families.entry(fam).or_default().push("cjk-fallback".into());
            }
            used = Some(p);
            break;
        }
    }
    // bold family: first readable system bold font, then the regular stack as glyph fallback
    let mut bold_stack: Vec<String> = Vec::new();
    for p in bold_candidates() {
        if let Ok(bytes) = std::fs::read(&p) {
            fonts.font_data.insert("bold-system".into(), Arc::new(FontData::from_owned(bytes)));
            bold_stack.push("bold-system".into());
            break;
        }
    }
    HAVE_BOLD.store(!bold_stack.is_empty(), Ordering::Relaxed);
    let fallback = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    bold_stack.extend(fallback);
    fonts.families.insert(FontFamily::Name(BOLD_FAMILY.into()), bold_stack);
    (fonts, used)
}

/// Install fonts into the context.
///
/// `ctx`: egui context. Returns path of the CJK fallback font if one was added.
pub fn install(ctx: &egui::Context) -> Option<PathBuf> {
    let (f, used) = build_fonts();
    ctx.set_fonts(f);
    used
}
