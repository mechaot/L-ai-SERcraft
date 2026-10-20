//! LaserGRBL egui application entry (ports `Program.cs` startup + `MainForm.RefreshFormTitle`).

mod app_state;
mod forms;
mod main_form;
mod ui;

use eframe::egui;

/// `Program.CurrentVersion.ToString(3)`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Window title as in `RefreshFormTitle`: "LaserGRBL v7.14.1" (+ " (for X)" for non-GRBL firmware).
///
/// `firmware_suffix`: Some("Marlin") for non-Grbl firmware. Returns title text.
pub fn form_title(firmware_suffix: Option<&str>) -> String {
    match firmware_suffix {
        Some(f) => format!("LaserGRBL v{VERSION} (for {f})"),
        None => format!("LaserGRBL v{VERSION}"),
    }
}

/// Decode `LaserGrbl.ico` (largest entry; PNG or BMP-DIB) into an egui icon.
fn load_icon() -> Option<egui::IconData> {
    let ico: &[u8] = include_bytes!("../../resources/grafica/LaserGrbl.ico");
    let count = u16::from_le_bytes([*ico.get(4)?, *ico.get(5)?]) as usize;
    let mut best: Option<(u32, usize, usize)> = None; // (w, size, offset)
    for i in 0..count {
        let e = ico.get(6 + i * 16..6 + i * 16 + 16)?;
        let w = if e[0] == 0 { 256 } else { e[0] as u32 };
        let size = u32::from_le_bytes([e[8], e[9], e[10], e[11]]) as usize;
        let off = u32::from_le_bytes([e[12], e[13], e[14], e[15]]) as usize;
        if best.is_none_or(|b| w > b.0) {
            best = Some((w, size, off));
        }
    }
    let (_, size, off) = best?;
    let data = ico.get(off..off + size)?;
    if data.starts_with(b"\x89PNG") {
        let img = image::load_from_memory(data).ok()?.to_rgba8();
        return Some(egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() });
    }
    // 32-bit BMP DIB: 40 byte header, BGRA rows bottom-up (AND mask ign    // 32-bit BMP DIB: 40 byte header, BGRA rows bottom-up (AND mask ignored)
    let le = |o: usize| u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
    if data.len() < 40 || le(0) != 40 || u16::from_le_bytes([data[14], data[15]]) != 32 {
        return None;
    }
    let (w, h) = (le(4) as usize, le(8) as usize / 2);
    let px = data.get(40..40 + w * h * 4)?;
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        let src = &px[(h - 1 - y) * w * 4..(h - y) * w * 4];
        for x in 0..w {
            let (b, g, r, a) = (src[x * 4], src[x * 4 + 1], src[x * 4 + 2], src[x * 4 + 3]);
            rgba[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&[r, g, b, a]);
        }
    }
    Some(egui::IconData { width: w as u32, height: h as u32, rgba })
}

/// Top-level eframe app.
struct App {
    main_form: Option<main_form::MainForm>,
    gallery: ui::gallery::Gallery,
    gallery_open: bool,
    gallery_only: bool,
}

impl App {
    /// Build app: apply fonts and persisted theme, create the main window state.
    ///
    /// `cc`: eframe creation context. `gallery`: show only the gallery (`--gallery`). `args`: file arguments.
    fn new(cc: &eframe::CreationContext<'_>, gallery: bool, args: Vec<String>) -> Self {
        ui::fonts::install(&cc.egui_ctx);
        let scheme = ui::theme::Theme::load_from_settings();
        ui::theme::Theme::of(scheme).apply(&cc.egui_ctx);
        let main_form = if gallery { None } else { Some(main_form::MainForm::new(args)) };
        App { main_form, gallery: Default::default(), gallery_open: false, gallery_only: gallery }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.gallery_only {
            egui::CentralPanel::default().show(ui, |ui| self.gallery.ui(ui));
            self.gallery.dialogs(&ctx);
        } else if let Some(mf) = self.main_form.as_mut() {
            // main-thread panic: the hook queued a report, the exception dialog offers "Continue"
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| mf.ui(ui)));
            if r.is_err() {
                ctx.request_repaint();
            }
            if self.gallery_open {
                let mut open = true;
                self.gallery.window(&ctx, &mut open);
                self.gallery_open = open;
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(mf) = self.main_form.as_mut() {
            mf.on_exit();
        }
    }
}

/// Saved window geometry (`Mainform Size and Position`): `[[w,h],[x,y],state]`, state 0 = normal.
///
/// Returns (size, position, maximized); defaults to maximized like C# without a saved value.
fn saved_geometry() -> (Option<[f32; 2]>, Option<[f32; 2]>, bool) {
    let v: serde_json::Value = lg_base::settings::get("Mainform Size and Position", serde_json::Value::Null);
    let pair = |x: &serde_json::Value| Some([x.get(0)?.as_f64()? as f32, x.get(1)?.as_f64()? as f32]);
    match (pair(&v[0]), pair(&v[1]), v[2].as_i64()) {
        (Some(s), Some(p), Some(0)) => (Some(s), Some(p), false),
        _ => (None, None, true),
    }
}

/// `Program.Main` argument handling: `nogl` / `swgl` force a graphic mode; the rest are files.
///
/// `args`: command line without the program name. Returns (file args, gallery flag).
fn parse_args(args: &[String]) -> (Vec<String>, bool) {
    use lg_base::settings::{self, GraphicMode};
    let mut files = Vec::new();
    let mut gallery = std::env::var_os("LASERGRBL_GALLERY").is_some();
    for a in args {
        let l = a.to_lowercase();
        if l == "--gallery" {
            gallery = true;
        } else if l == "nogl" && settings::forced_graphic_mode() == GraphicMode::Auto {
            settings::set_forced_graphic_mode(GraphicMode::Gdi);
        } else if l == "swgl" && settings::forced_graphic_mode() == GraphicMode::Auto {
            settings::set_forced_graphic_mode(GraphicMode::Dib);
        } else {
            files.push(a.clone());
        }
    }
    (files, gallery)
}

fn main() -> eframe::Result {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    forms::main::exception::register_handler();
    lg_base::logger::start();
    lasergrbl_core::usage_stats::load_file();
    let (files, gallery) = parse_args(&argv);
    let (size, pos, maximized) = saved_geometry();
    let mut vp = egui::ViewportBuilder::default().with_title(form_title(None)).with_inner_size(size.unwrap_or([1024.0, 768.0])).with_maximized(maximized);
    if let Some(p) = pos {
        vp = vp.with_position(p);
    }
    if let Some(icon) = load_icon() {
        vp = vp.with_icon(icon);
    }
    let opts = eframe::NativeOptions { viewport: vp, ..Default::default() };
    eframe::run_native("LaserGRBL", opts, Box::new(move |cc| Ok(Box::new(App::new(cc, gallery, files)))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title() {
        assert_eq!(form_title(None), "LaserGRBL v7.14.1");
        assert_eq!(form_title(Some("Marlin")), "LaserGRBL v7.14.1 (for Marlin)");
    }

    #[test]
    fn icon_decodes() {
        let i = load_icon().expect("ico");
        assert!(i.width >= 16 && i.rgba.len() == (i.width * i.height * 4) as usize);
    }
}
