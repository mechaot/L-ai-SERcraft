//! Ports `ColorScheme.cs` (7 schemes) and the intent of `UserControls/ThemeMgr.cs`
//! (apply dark/light look to the whole window; here: egui `Visuals`).

use eframe::egui::{self, Color32, Visuals};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

/// Settings key used by C# `MainForm` for the persisted scheme.
pub const SETTING_KEY: &str = "Color Schema";

/// Color scheme id. Variant names are persisted (same as C# enum names).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scheme {
    CADStyle,
    CADDark,
    BlueLaser,
    RedLaser,
    Dark,
    Hacker,
    Nighty,
}

impl Scheme {
    /// All schemes in C# enum order.
    pub const ALL: [Scheme; 7] = [
        Scheme::CADStyle,
        Scheme::CADDark,
        Scheme::BlueLaser,
        Scheme::RedLaser,
        Scheme::Dark,
        Scheme::Hacker,
        Scheme::Nighty,
    ];

    /// Enum index (storage in atomic).
    fn idx(self) -> u8 {
        Scheme::ALL.iter().position(|s| *s == self).unwrap_or(1) as u8
    }

    /// Variant name, same text as C# `ToString()`.
    pub fn name(self) -> &'static str {
        match self {
            Scheme::CADStyle => "CADStyle",
            Scheme::CADDark => "CADDark",
            Scheme::BlueLaser => "BlueLaser",
            Scheme::RedLaser => "RedLaser",
            Scheme::Dark => "Dark",
            Scheme::Hacker => "Hacker",
            Scheme::Nighty => "Nighty",
        }
    }
}

/// Log row color class. Core will provide a matching enum; map 1:1 to this.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LogRowColor {
    Command,
    Startup,
    Alarm,
    Config,
    Feedback,
    Position,
    Others,
    RightGood,
    RightBad,
    RightOthers,
}

/// `Color.FromArgb(r,g,b)`.
const fn c(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// .NET named colors used by the schemes and widgets.
pub mod named {
    use super::c;
    use eframe::egui::Color32;
    pub const BLACK: Color32 = c(0, 0, 0);
    pub const WHITE: Color32 = c(255, 255, 255);
    pub const RED: Color32 = c(255, 0, 0);
    pub const BLUE: Color32 = c(0, 0, 255);
    pub const LIME: Color32 = c(0, 255, 0);
    pub const YELLOW: Color32 = c(255, 255, 0);
    pub const AQUA: Color32 = c(0, 255, 255);
    pub const GRAY: Color32 = c(128, 128, 128);
    pub const PURPLE: Color32 = c(128, 0, 128);
    pub const DARK_GREEN: Color32 = c(0, 100, 0);
    pub const CRIMSON: Color32 = c(220, 20, 60);
    pub const DIM_GRAY: Color32 = c(105, 105, 105);
    pub const DODGER_BLUE: Color32 = c(30, 144, 255);
    pub const ORANGE_RED: Color32 = c(255, 69, 0);
    pub const DARK_BLUE: Color32 = c(0, 0, 139);
    pub const DARK_RED: Color32 = c(139, 0, 0);
    pub const LIGHT_PINK: Color32 = c(255, 182, 193);
    pub const PINK: Color32 = c(255, 192, 203);
    pub const LIGHT_GRAY: Color32 = c(211, 211, 211);
    pub const LIGHT_YELLOW: Color32 = c(255, 255, 224);
    pub const LIGHT_BLUE: Color32 = c(173, 216, 230);
    pub const DARK_GRAY: Color32 = c(169, 169, 169);
    pub const DARK_SLATE_GRAY: Color32 = c(47, 79, 79);
    pub const VIOLET: Color32 = c(238, 130, 238);
    pub const DARK_MAGENTA: Color32 = c(139, 0, 139);
    pub const LIME_GREEN: Color32 = c(50, 205, 50);
    pub const DARK_ORANGE: Color32 = c(255, 140, 0);
    pub const MEDIUM_VIOLET_RED: Color32 = c(199, 21, 133);
    pub const MEDIUM_PURPLE: Color32 = c(147, 112, 219);
    pub const GAINSBORO: Color32 = c(220, 220, 220);
    pub const SILVER: Color32 = c(192, 192, 192);
    pub const SKY_BLUE: Color32 = c(135, 206, 235);
    pub const DARK_SLATE_BLUE: Color32 = c(72, 61, 139);
    pub const CHARTREUSE: Color32 = c(127, 255, 0);
    pub const CORAL: Color32 = c(255, 127, 80);
    pub const LIGHT_CORAL: Color32 = c(240, 128, 128);
    pub const GREEN_YELLOW: Color32 = c(173, 255, 47);
    pub const GOLD: Color32 = c(255, 215, 0);
    pub const ORANGE: Color32 = c(255, 165, 0);
    pub const DARK_VIOLET: Color32 = c(148, 0, 211);
    /// `SystemColors.Control`.
    pub const CONTROL: Color32 = c(240, 240, 240);
    /// `SystemColors.ControlText`.
    pub const CONTROL_TEXT: Color32 = c(0, 0, 0);
}

use named::*;

/// Snapshot of every `ColorScheme` property of one scheme (all `IColorScheme` members).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub scheme: Scheme,
    pub is_dark: bool,
    pub form_back_color: Color32,
    pub form_fore_color: Color32,
    pub preview_back_color: Color32,
    pub preview_text: Color32,
    pub preview_ruler: Color32,
    pub preview_grid: Color32,
    pub preview_grid_minor: Color32,
    pub preview_job_range: Color32,
    pub preview_first_movement: Color32,
    pub preview_other_movement: Color32,
    pub preview_laser_power: Color32,
    pub preview_cross: Color32,
    pub preview_command_ok: Color32,
    pub preview_command_ko: Color32,
    pub preview_command_wait: Color32,
    pub preview_cross_cursor: Color32,
    pub log_back_color: Color32,
    pub log_left_command: Color32,
    pub log_left_startup: Color32,
    pub log_left_alarm: Color32,
    pub log_left_config: Color32,
    pub log_left_feedback: Color32,
    pub log_left_position: Color32,
    pub log_left_others: Color32,
    pub log_right_good: Color32,
    pub log_right_bad: Color32,
    pub log_right_others: Color32,
    pub text_box_color_override: Color32,
    pub link_color: Color32,
    pub visited_link_color: Color32,
    pub controls_border: Color32,
    pub controls_back_disabled: Color32,
    pub disabled_buttons: Color32,
    pub pressed_buttons: Color32,
}

static CURRENT: AtomicU8 = AtomicU8::new(1);

impl Theme {
    /// Colors of one scheme (C# `Scheme*` classes).
    ///
    /// `scheme`: scheme id. Returns its full color table.
    pub fn of(scheme: Scheme) -> Theme {
        match scheme {
            Scheme::CADStyle => Theme {
                scheme,
                is_dark: false,
                form_back_color: c(248, 248, 248),
                form_fore_color: CONTROL_TEXT,
                preview_back_color: c(33, 40, 48),
                preview_text: c(220, 220, 220),
                preview_ruler: c(69, 78, 101),
                preview_grid: c(49, 55, 70),
                preview_grid_minor: c(38, 45, 55),
                preview_job_range: c(200, 200, 140),
                preview_first_movement: BLUE,
                preview_other_movement: c(60, 82, 85),
                preview_laser_power: c(220, 220, 220),
                preview_cross: c(230, 230, 20),
                preview_command_ok: c(55, 199, 116),
                preview_command_ko: c(240, 50, 50),
                preview_command_wait: LIGHT_PINK,
                preview_cross_cursor: c(103, 107, 117),
                log_back_color: WHITE,
                log_left_command: BLACK,
                log_left_startup: DARK_GREEN,
                log_left_alarm: CRIMSON,
                log_left_config: DIM_GRAY,
                log_left_feedback: DODGER_BLUE,
                log_left_position: ORANGE_RED,
                log_left_others: PURPLE,
                log_right_good: DARK_BLUE,
                log_right_bad: RED,
                log_right_others: BLACK,
                text_box_color_override: BLACK,
                link_color: DODGER_BLUE,
                visited_link_color: PURPLE,
                controls_border: c(220, 220, 220),
                controls_back_disabled: c(180, 180, 180),
                disabled_buttons: c(140, 140, 140),
                pressed_buttons: c(222, 230, 7),
            },
            Scheme::CADDark => Theme {
                scheme,
                is_dark: true,
                form_back_color: c(35, 40, 51),
                form_fore_color: c(190, 190, 190),
                preview_back_color: c(23, 30, 38),
                preview_text: c(220, 220, 220),
                preview_ruler: c(69, 78, 101),
                preview_grid: c(39, 45, 60),
                preview_grid_minor: c(28, 35, 45),
                preview_job_range: c(200, 200, 140),
                preview_first_movement: BLUE,
                preview_other_movement: c(60, 82, 85),
                preview_laser_power: c(220, 220, 220),
                preview_cross: c(230, 230, 20),
                preview_command_ok: c(55, 199, 116),
                preview_command_ko: c(240, 50, 50),
                preview_command_wait: LIGHT_PINK,
                preview_cross_cursor: c(103, 107, 117),
                log_back_color: c(70, 79, 92),
                log_left_command: WHITE,
                log_left_startup: c(121, 214, 58),
                log_left_alarm: CRIMSON,
                log_left_config: LIGHT_GRAY,
                log_left_feedback: DODGER_BLUE,
                log_left_position: ORANGE_RED,
                log_left_others: PINK,
                log_right_good: DARK_BLUE,
                log_right_bad: RED,
                log_right_others: BLACK,
                text_box_color_override: BLACK,
                link_color: PINK,
                visited_link_color: PURPLE,
                controls_border: c(64, 67, 85),
                controls_back_disabled: c(180, 180, 180),
                disabled_buttons: c(85, 90, 105),
                pressed_buttons: c(84, 96, 122),
            },
            Scheme::BlueLaser => Theme {
                scheme,
                is_dark: false,
                form_back_color: CONTROL,
                form_fore_color: CONTROL_TEXT,
                preview_back_color: LIGHT_YELLOW,
                preview_text: BLACK,
                preview_ruler: DARK_GRAY,
                preview_grid: c(242, 242, 200),
                preview_grid_minor: c(248, 248, 220),
                preview_job_range: DARK_GRAY,
                preview_first_movement: BLUE,
                preview_other_movement: LIGHT_GRAY,
                preview_laser_power: RED,
                preview_cross: BLUE,
                preview_command_ok: DARK_GREEN,
                preview_command_ko: DARK_RED,
                preview_command_wait: LIGHT_PINK,
                preview_cross_cursor: c(184, 184, 184),
                log_back_color: WHITE,
                log_left_command: BLACK,
                log_left_startup: DARK_GREEN,
                log_left_alarm: CRIMSON,
                log_left_config: DIM_GRAY,
                log_left_feedback: DODGER_BLUE,
                log_left_position: ORANGE_RED,
                log_left_others: PURPLE,
                log_right_good: DARK_BLUE,
                log_right_bad: RED,
                log_right_others: BLACK,
                text_box_color_override: BLACK,
                link_color: DODGER_BLUE,
                visited_link_color: PURPLE,
                controls_border: c(220, 220, 220),
                controls_back_disabled: c(180, 180, 180),
                disabled_buttons: c(70, 70, 70),
                pressed_buttons: CRIMSON,
            },
            Scheme::RedLaser => Theme {
                scheme,
                preview_laser_power: DARK_BLUE,
                preview_cross: DARK_VIOLET,
                preview_command_wait: LIGHT_BLUE,
                log_right_good: DARK_GREEN,
                disabled_buttons: c(100, 100, 100),
                ..Theme::of(Scheme::BlueLaser)
            },
            Scheme::Dark => Theme {
                scheme,
                is_dark: true,
                form_back_color: c(29, 44, 75),
                form_fore_color: WHITE,
                preview_back_color: c(220, 220, 220),
                preview_text: BLACK,
                preview_ruler: DARK_GRAY,
                preview_grid: c(210, 210, 210),
                preview_grid_minor: c(220, 220, 220),
                preview_job_range: DIM_GRAY,
                preview_first_movement: BLUE,
                preview_other_movement: c(180, 118, 0),
                preview_laser_power: RED,
                preview_cross: DARK_MAGENTA,
                preview_command_ok: DARK_GREEN,
                preview_command_ko: DARK_RED,
                preview_command_wait: LIGHT_BLUE,
                preview_cross_cursor: GRAY,
                log_back_color: c(220, 220, 220),
                log_left_command: BLACK,
                log_left_startup: DARK_GREEN,
                log_left_alarm: DARK_RED,
                log_left_config: DARK_SLATE_GRAY,
                log_left_feedback: DARK_BLUE,
                log_left_position: ORANGE_RED,
                log_left_others: PURPLE,
                log_right_good: LIME,
                log_right_bad: ORANGE_RED,
                log_right_others: WHITE,
                text_box_color_override: WHITE,
                link_color: YELLOW,
                visited_link_color: VIOLET,
                controls_border: c(39, 54, 85),
                controls_back_disabled: c(180, 180, 180),
                disabled_buttons: c(100, 100, 100),
                pressed_buttons: CRIMSON,
            },
            Scheme::Hacker => Theme {
                scheme,
                form_back_color: c(0, 10, 35),
                form_fore_color: LIME_GREEN,
                preview_laser_power: DARK_GREEN,
                preview_command_ok: DARK_BLUE,
                preview_command_ko: ORANGE_RED,
                preview_command_wait: PINK,
                log_back_color: c(20, 20, 20),
                log_left_command: LIME_GREEN,
                log_left_startup: PINK,
                log_left_alarm: RED,
                log_left_config: LIGHT_GRAY,
                log_left_feedback: LIGHT_BLUE,
                log_left_others: PINK,
                log_right_good: LIGHT_BLUE,
                log_right_bad: RED,
                log_right_others: WHITE,
                controls_border: c(15, 25, 50),
                ..Theme::of(Scheme::Dark)
            },
            Scheme::Nighty => Theme {
                scheme,
                is_dark: true,
                form_back_color: c(25, 25, 25),
                form_fore_color: AQUA,
                preview_back_color: c(25, 25, 25),
                preview_text: AQUA,
                preview_ruler: AQUA,
                preview_grid: c(34, 34, 34),
                preview_grid_minor: c(28, 28, 28),
                preview_job_range: LIGHT_PINK,
                preview_first_movement: DARK_ORANGE,
                preview_other_movement: c(150, 0, 120),
                preview_laser_power: c(0, 125, 140),
                preview_cross: PINK,
                preview_command_ok: DARK_GREEN,
                preview_command_ko: DARK_RED,
                preview_command_wait: LIGHT_BLUE,
                preview_cross_cursor: GRAY,
                log_back_color: c(25, 25, 25),
                log_left_command: AQUA,
                log_left_startup: c(220, 30, 220),
                log_left_alarm: c(0, 127, 139),
                log_left_config: MEDIUM_VIOLET_RED,
                log_left_feedback: MEDIUM_VIOLET_RED,
                log_left_position: MEDIUM_PURPLE,
                log_left_others: PURPLE,
                log_right_good: DARK_GREEN,
                log_right_bad: RED,
                log_right_others: LIME_GREEN,
                text_box_color_override: LIME_GREEN,
                link_color: YELLOW,
                visited_link_color: VIOLET,
                controls_border: c(40, 40, 40),
                controls_back_disabled: c(180, 180, 180),
                disabled_buttons: c(100, 100, 100),
                pressed_buttons: CRIMSON,
            },
        }
    }

    /// Current scheme colors (C# static `ColorScheme.*`).
    pub fn current() -> Theme {
        Theme::of(Self::current_scheme())
    }

    /// Current scheme id (C# `ColorScheme.CurrentScheme`).
    pub fn current_scheme() -> Scheme {
        Scheme::ALL[(CURRENT.load(Ordering::Relaxed) as usize).min(6)]
    }

    /// Set current scheme in memory only (no persistence, no egui update).
    ///
    /// `scheme`: new scheme.
    pub fn set_current(scheme: Scheme) {
        CURRENT.store(scheme.idx(), Ordering::Relaxed);
    }

    /// Read persisted scheme (setting "Color Schema", default CADDark) and make it current.
    /// Returns the loaded scheme.
    pub fn load_from_settings() -> Scheme {
        let s: Scheme = lg_base::settings::get(SETTING_KEY, Scheme::CADDark);
        Theme::set_current(s);
        s
    }

    /// Switch scheme: persist, set current, restyle egui (C# `styleToolStripMenuItem_Click`).
    ///
    /// `ctx`: egui context to restyle. `scheme`: new scheme.
    pub fn switch(ctx: &egui::Context, scheme: Scheme) {
        lg_base::settings::set(SETTING_KEY, scheme);
        Theme::set_current(scheme);
        Theme::of(scheme).apply(ctx);
    }

    /// Log color for a row class (hook for core's log color enum).
    ///
    /// `row`: row class. Returns the color of the current scheme for it.
    pub fn log_color(&self, row: LogRowColor) -> Color32 {
        match row {
            LogRowColor::Command => self.log_left_command,
            LogRowColor::Startup => self.log_left_startup,
            LogRowColor::Alarm => self.log_left_alarm,
            LogRowColor::Config => self.log_left_config,
            LogRowColor::Feedback => self.log_left_feedback,
            LogRowColor::Position => self.log_left_position,
            LogRowColor::Others => self.log_left_others,
            LogRowColor::RightGood => self.log_right_good,
            LogRowColor::RightBad => self.log_right_bad,
            LogRowColor::RightOthers => self.log_right_others,
        }
    }

    /// `ColorScheme.FormButtonsColor`.
    pub fn form_buttons_color(&self) -> Color32 {
        let f = if self.is_dark { 0.1 } else { -0.1 };
        change_color_brightness(self.form_back_color, f)
    }

    /// `ColorScheme.MenuHighlightColor`.
    pub fn menu_highlight_color(&self) -> Color32 {
        let f = if self.is_dark { 0.2 } else { -0.1 };
        change_color_brightness(self.form_back_color, f)
    }

    /// `ColorScheme.MenuSeparatorColor`.
    pub fn menu_separator_color(&self) -> Color32 {
        let f = if self.is_dark { 0.15 } else { -0.1 };
        change_color_brightness(self.form_back_color, f)
    }

    /// Build egui `Visuals` matching this scheme.
    pub fn visuals(&self) -> Visuals {
        let mut v = if self.is_dark { Visuals::dark() } else { Visuals::light() };
        let stroke = |col: Color32| egui::Stroke::new(1.0, col);
        let buttons = self.form_buttons_color();
        let hl = self.menu_highlight_color();
        v.override_text_color = Some(self.form_fore_color);
        v.panel_fill = self.form_back_color;
        v.window_fill = self.form_back_color;
        v.window_stroke = stroke(self.controls_border);
        v.extreme_bg_color = self.log_back_color; // text edit / log bg
        v.faint_bg_color = buttons;
        v.code_bg_color = self.log_back_color;
        v.hyperlink_color = self.link_color;
        v.selection.bg_fill = if self.is_dark { c(51, 102, 170) } else { c(0, 120, 215) };
        v.selection.stroke = stroke(WHITE);
        v.widgets.noninteractive.bg_fill = self.form_back_color;
        v.widgets.noninteractive.weak_bg_fill = self.form_back_color;
        v.widgets.noninteractive.bg_stroke = stroke(self.menu_separator_color());
        v.widgets.noninteractive.fg_stroke = stroke(self.form_fore_color);
        v.widgets.inactive.bg_fill = buttons;
        v.widgets.inactive.weak_bg_fill = buttons;
        v.widgets.inactive.bg_stroke = stroke(self.controls_border);
        v.widgets.inactive.fg_stroke = stroke(self.form_fore_color);
        v.widgets.hovered.bg_fill = hl;
        v.widgets.hovered.weak_bg_fill = hl;
        v.widgets.hovered.bg_stroke = stroke(self.controls_border);
        v.widgets.hovered.fg_stroke = stroke(self.form_fore_color);
        let pressed = change_color_brightness(buttons, if self.is_dark { 0.25 } else { -0.2 });
        v.widgets.active.bg_fill = pressed;
        v.widgets.active.weak_bg_fill = pressed;
        v.widgets.active.bg_stroke = stroke(self.controls_border);
        v.widgets.active.fg_stroke = stroke(self.form_fore_color);
        v.widgets.open.bg_fill = hl;
        v.widgets.open.weak_bg_fill = hl;
        v.widgets.open.fg_stroke = stroke(self.form_fore_color);
        v
    }

    /// Apply to egui (ThemeMgr: dark vs light chrome + colors).
    ///
    /// `ctx`: context to restyle.
    pub fn apply(&self, ctx: &egui::Context) {
        let pref = if self.is_dark { egui::ThemePreference::Dark } else { egui::ThemePreference::Light };
        ctx.set_theme(pref);
        let v = self.visuals();
        ctx.set_visuals_of(if self.is_dark { egui::Theme::Dark } else { egui::Theme::Light }, v);
        ctx.request_repaint();
    }
}

/// `ColorScheme.ChangeColorBrightness`: factor < 0 darkens, > 0 lightens; alpha kept, ints truncated.
///
/// `color`: source. `factor`: -1..1. Returns adjusted color.
pub fn change_color_brightness(color: Color32, factor: f32) -> Color32 {
    let (mut r, mut g, mut b) = (color.r() as f32, color.g() as f32, color.b() as f32);
    if factor < 0.0 {
        let f = 1.0 + factor;
        r *= f;
        g *= f;
        b *= f;
    } else {
        r = (255.0 - r) * factor + r;
        g = (255.0 - g) * factor + g;
        b = (255.0 - b) * factor + b;
    }
    Color32::from_rgba_unmultiplied(r as u8, g as u8, b as u8, color.a())
}

/// WinForms `ControlPaint.Light(color)` approx (HLS luminosity boost).
///
/// `color`: source. Returns lighter color.
pub fn control_paint_light(color: Color32) -> Color32 {
    hls_scale(color, 0.0, true)
}

/// WinForms `ControlPaint.Dark(color)` approx (HLS luminosity cut).
///
/// `color`: source. Returns darker color.
pub fn control_paint_dark(color: Color32) -> Color32 {
    hls_scale(color, 0.5, false)
}

/// Scale HLS luminosity: lighten moves 50% toward 1.0 (0.5 luminosity bump), darken halves it.
fn hls_scale(color: Color32, k: f32, light: bool) -> Color32 {
    let (r, g, b) = (color.r() as f32 / 255.0, color.g() as f32 / 255.0, color.b() as f32 / 255.0);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let l = (mx + mn) / 2.0;
    let d = mx - mn;
    let s = if d == 0.0 { 0.0 } else { d / (1.0 - (2.0 * l - 1.0).abs()) };
    let h = if d == 0.0 {
        0.0
    } else if mx == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if mx == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    // .NET uses 0.5 luminosity scale for Dark, 0.5+ toward white for Light.
    let nl = if light { l + (1.0 - l) * 0.5 } else { l * k };
    let q = (1.0 - (2.0 * nl - 1.0).abs()) * s;
    let x = q * (1.0 - (h % 2.0 - 1.0).abs());
    let m = nl - q / 2.0;
    let (r1, g1, b1) = match h as u32 {
        0 => (q, x, 0.0),
        1 => (x, q, 0.0),
        2 => (0.0, q, x),
        3 => (0.0, x, q),
        4 => (x, 0.0, q),
        _ => (q, 0.0, x),
    };
    let f = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(f(r1), f(g1), f(b1), color.a())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brightness_matches_cs() {
        assert_eq!(change_color_brightness(c(100, 100, 100), -0.1), c(90, 90, 90));
        assert_eq!(change_color_brightness(c(0, 0, 0), 0.1), c(25, 25, 25));
    }

    #[test]
    fn red_laser_derives_from_blue() {
        let t = Theme::of(Scheme::RedLaser);
        assert_eq!(t.preview_laser_power, DARK_BLUE);
        assert_eq!(t.log_left_alarm, CRIMSON);
        assert!(!Theme::of(Scheme::Hacker).preview_back_color.eq(&BLACK));
        assert!(Theme::of(Scheme::Hacker).is_dark);
    }

    #[test]
    fn scheme_roundtrip() {
        for s in Scheme::ALL {
            Theme::set_current(s);
            assert_eq!(Theme::current_scheme(), s);
        }
    }
}
