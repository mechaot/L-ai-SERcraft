// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Orthographic camera + ruler math of `GrblPanel3D.cs` (SetWorldPosition, resize, AutoSizeDrawing,
//! CenterZoom, mouse wheel, `GetRulerStep`, `HumanReadableLength`). Pure math, unit tested.

use lg_dotnet::round_half_even;

/// Max half viewport (C# `Grid3D.ViewportSize`).
pub const VIEWPORT_SIZE: f64 = 1_000_000.0;

/// Viewport padding (C# `mPadding` = 70,0,0,30). Rulers live there.
#[derive(Debug, Clone, Copy)]
pub struct Padding {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// The one padding of the panel.
pub const PADDING: Padding = Padding { left: 70.0, top: 0.0, right: 0.0, bottom: 30.0 };

/// Bounds used by auto size: `(x_min, x_max, y_min, y_max)` in mm.
pub type Bounds = (f64, f64, f64, f64);

/// World window shown by the control (C# `OrthographicCamera` left/right/bottom/top).
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub left: f64,
    pub right: f64,
    pub bottom: f64,
    pub top: f64,
    /// Control size at the last resize (C# `mLastControlSize`).
    pub last_size: (f64, f64),
}

impl Default for Camera {
    /// Window -100..100 like the empty-file auto size.
    fn default() -> Self {
        Self { left: -100.0, right: 100.0, bottom: -100.0, top: 100.0, last_size: (0.0, 0.0) }
    }
}

impl Camera {
    /// World width.
    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    /// World height.
    pub fn height(&self) -> f64 {
        self.top - self.bottom
    }

    /// C# `SetWorldPosition`: clamp to the max viewport and store.
    ///
    /// `left`,`right`,`bottom`,`top`: wanted window. Returns true when applied.
    pub fn set_world_position(&mut self, mut left: f64, mut right: f64, mut bottom: f64, mut top: f64) -> bool {
        let max_viewport = VIEWPORT_SIZE * 2.0;
        if right - left > max_viewport {
            return false;
        }
        // C# compares bottom - top (always <= 0), so this guard never fires there; kept for parity
        if bottom - top > max_viewport {
            return false;
        }
        if left < -max_viewport / 2.0 {
            right += -max_viewport / 2.0 - left;
            left = -max_viewport / 2.0;
        }
        if right > max_viewport / 2.0 {
            left += max_viewport / 2.0 - right;
            right = max_viewport / 2.0;
        }
        if bottom < -max_viewport / 2.0 {
            top += -max_viewport / 2.0 - bottom;
            bottom = -max_viewport / 2.0;
        }
        if top > max_viewport / 2.0 {
            bottom += max_viewport / 2.0 - top;
            top = max_viewport / 2.0;
        }
        self.left = left;
        self.right = right;
        self.bottom = bottom;
        self.top = top;
        true
    }

    /// C# `GrblPanel3D_Resize`: keep center, scale window with the control size.
    ///
    /// `w`,`h`: new control size in points. Returns nothing; no-op on first call or zero size.
    pub fn on_resize(&mut self, w: f64, h: f64) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        if self.last_size.0 <= 0.0 || self.last_size.1 <= 0.0 {
            self.last_size = (w, h);
            return;
        }
        let w_ratio = w / self.last_size.0;
        let h_ratio = h / self.last_size.1;
        let w_inc = self.width() - self.width() * w_ratio;
        let h_inc = self.height() - self.height() * h_ratio;
        let mut new_left = self.left + w_inc / 2.0;
        let mut new_right = self.right - w_inc / 2.0;
        let mut new_top = self.top - h_inc / 2.0;
        let mut new_bottom = self.bottom + h_inc / 2.0;
        let (mut rw, mut rh) = (1.0, 1.0);
        let max_viewport = VIEWPORT_SIZE * 2.0;
        if new_right - new_left > max_viewport {
            rw = max_viewport / (new_right - new_left);
        }
        if new_bottom - new_top > max_viewport {
            rh = max_viewport / (new_bottom - new_top);
        }
        let r = rw.min(rh);
        new_left *= r;
        new_right *= r;
        new_top *= r;
        new_bottom *= r;
        self.set_world_position(new_left, new_right, new_bottom, new_top);
        self.last_size = (w, h);
    }

    /// C# `AutoSizeDrawing`.
    ///
    /// `w`,`h`: control size. `bounds`: drawing/moving range (None or degenerate = +-100 default).
    pub fn auto_size(&mut self, w: f64, h: f64, bounds: Option<Bounds>) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let pad = PADDING;
        let ratio = w / h;
        let avail_w = w - pad.left as f64 - pad.right as f64;
        let avail_h = h - pad.bottom as f64 - pad.top as f64;
        const GROW: f64 = 1.1;
        const DEFAULT_VIEWPORT: f64 = 100.0;
        let (xmin, xmax, ymin, ymax) = match bounds {
            Some(b) if (b.1 - b.0) > 0.0 || (b.3 - b.2) > 0.0 => b,
            _ => (-DEFAULT_VIEWPORT, DEFAULT_VIEWPORT, -DEFAULT_VIEWPORT, DEFAULT_VIEWPORT),
        };
        let (cx, cy) = ((xmin + xmax) / 2.0, (ymin + ymax) / 2.0);
        let drawing_w = (xmax - xmin) * GROW;
        let drawing_h = (ymax - ymin) * GROW;
        let width_ratio = drawing_w / avail_w;
        let pad_left_world = pad.left as f64 * width_ratio;
        let pad_right_world = pad.right as f64 * width_ratio;
        let height_ratio = drawing_h / avail_h;
        let pad_bottom_world = pad.bottom as f64 * height_ratio;
        let pad_top_world = pad.top as f64 * height_ratio;
        let (mut left, mut right, mut bottom, mut top);
        if drawing_w / drawing_h > avail_w / avail_h {
            // width limited
            left = cx - drawing_w / 2.0 - pad_left_world;
            right = cx + drawing_w / 2.0 + pad_right_world;
            let resized_h = (right - left) / ratio;
            bottom = cy - resized_h / 2.0;
            top = cy + resized_h / 2.0;
            let bottom_shift = (top - bottom) / h * pad.bottom as f64 / 2.0;
            bottom -= bottom_shift;
            top -= bottom_shift;
        } else {
            // height limited
            bottom = cy - drawing_h / 2.0 - pad_bottom_world;
            top = cy + drawing_h / 2.0 + pad_top_world;
            let resized_w = (top - bottom) * ratio;
            left = cx - resized_w / 2.0;
            right = cx + resized_w / 2.0;
            let left_shift = (right - left) / w * pad.left as f64 / 2.0;
            left -= left_shift;
            right -= left_shift;
        }
        self.set_world_position(left, right, bottom, top);
        self.last_size = (w, h);
    }

    /// C# `CenterZoom`: scale the window around its center.
    ///
    /// `k`: factor on half sizes (1.1 = zoom out, 1/1.1 = zoom in).
    pub fn center_zoom(&mut self, k: f64) {
        let cx = (self.left + self.right) / 2.0;
        let cy = (self.bottom + self.top) / 2.0;
        let hw = self.width() / 2.0 * k;
        let hh = self.height() / 2.0 * k;
        self.set_world_position(cx - hw, cx + hw, cy - hh, cy + hh);
    }

    /// C# `MouseWheel`: zoom towards the mouse world position.
    ///
    /// `mx`,`my`: mouse world position. `k`: `Delta / 1000` (+0.12 per notch up = zoom in).
    pub fn wheel_zoom(&mut self, mx: f64, my: f64, k: f64) {
        self.set_world_position(
            self.left + (mx - self.left) * k,
            self.right - (self.right - mx) * k,
            self.bottom + (my - self.bottom) * k,
            self.top - (self.top - my) * k,
        );
    }

    /// C# drag: shift window by a pixel delta.
    ///
    /// `dx`,`dy`: mouse delta in points (screen y down). `w`: control width.
    pub fn pan(&mut self, dx: f64, dy: f64, w: f64) {
        let k = self.width() / w;
        self.set_world_position(self.left - dx * k, self.right - dx * k, self.bottom + dy * k, self.top + dy * k);
    }

    /// Control point (relative to widget top-left, points) to world.
    ///
    /// `x`,`y`: point. `w`,`h`: control size. Returns `(wx, wy)`.
    pub fn screen_to_world(&self, x: f64, y: f64, w: f64, h: f64) -> (f64, f64) {
        (x / w * self.width() + self.left, (h - y) / h * self.height() + self.bottom)
    }

    /// World to control point (relative to widget top-left, points).
    ///
    /// `wx`,`wy`: world point. `w`,`h`: control size. Returns `(x, y)`.
    pub fn world_to_screen(&self, wx: f64, wy: f64, w: f64, h: f64) -> (f64, f64) {
        ((wx - self.left) / self.width() * w, h - (wy - self.bottom) / self.height() * h)
    }

    /// C# `Grid3D.GridSize`: world units per 10 px.
    ///
    /// `w`: control width.
    pub fn grid_size(&self, w: f64) -> f64 {
        if w > 0.0 { self.width() * 10.0 / w } else { 0.0 }
    }
}

/// C# `GetRulerStep`.
///
/// `n`: world width. Returns the label step in mm.
pub fn ruler_step(n: f64) -> f64 {
    let n = n.abs();
    let digit_count = ((n.log10() + 1.0).floor()).max(3.0) as i32;
    let power = 10f64.powi(digit_count - 1);
    let step = n / power;
    let result = if step < 1.0 {
        5.0
    } else if step < 2.0 {
        10.0
    } else if step < 6.0 {
        30.0
    } else {
        50.0
    };
    result * 10f64.powi(digit_count - 3)
}

/// C# `HumanReadableLength`.
///
/// `mm`: value in mm. `world_width`: window width. Returns `(text, unit)`.
pub fn human_readable_length(mm: i64, world_width: f64) -> (String, &'static str) {
    if world_width > 10000.0 {
        (format!("{}", round_half_even(mm as f64 / 1000.0) as i64), "m")
    } else if world_width > 1000.0 {
        (format!("{}", round_half_even(mm as f64 / 10.0) as i64), "cm")
    } else {
        (format!("{mm}"), "mm")
    }
}

/// Ruler label positions between `start` and `end` (inclusive, mm), C# `DrawRulers` loops.
///
/// `start`,`end`: integer range. `step`: from [`ruler_step`]. `bbox`: drawing range (min, max) when the
/// bounding box is shown. Returns `(value, is_bbox_edge)` sorted ascending.
pub fn ruler_ticks(start: i64, end: i64, step: f64, bbox: Option<(i64, i64)>) -> Vec<(i64, bool)> {
    let mut out = Vec::new();
    if end < start || step <= 0.0 {
        return out;
    }
    let half = (step / 2.0) as i64;
    let mut cand = Vec::new();
    // multiples of step inside the range
    let first = (start as f64 / step).ceil() as i64;
    let last = (end as f64 / step).floor() as i64;
    for m in first..=last {
        cand.push((m as f64 * step) as i64);
    }
    if let Some((mn, mx)) = bbox {
        cand.push(mn);
        cand.push(mx);
    }
    cand.sort_unstable();
    cand.dedup();
    for i in cand {
        if i < start || i > end {
            continue;
        }
        let mut can = (i as f64 % step) == 0.0;
        let mut edge = false;
        if let Some((mn, mx)) = bbox {
            can &= i < mn - half || (i > mn + half && i < mx - half) || i > mx + half;
            edge = i == mn || i == mx;
            can |= edge;
        }
        if can {
            out.push((i, edge));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruler_step_table() {
        assert_eq!(ruler_step(50.0), 5.0); // step 0.5
        assert_eq!(ruler_step(150.0), 10.0);
        assert_eq!(ruler_step(300.0), 30.0);
        assert_eq!(ruler_step(900.0), 50.0);
        assert_eq!(ruler_step(2000.0), 300.0); // 4 digits, step 2 -> 30 * 10
    }

    #[test]
    fn human_units() {
        assert_eq!(human_readable_length(1500, 20000.0), ("2".into(), "m"));
        assert_eq!(human_readable_length(1500, 2000.0), ("150".into(), "cm"));
        assert_eq!(human_readable_length(15, 200.0), ("15".into(), "mm"));
    }

    #[test]
    fn world_clamped() {
        let mut c = Camera::default();
        c.set_world_position(-5_000_000.0, -3_000_000.0, 0.0, 10.0);
        assert!(c.left >= -VIEWPORT_SIZE - 1e-6);
        let mut c = Camera::default();
        assert!(!c.set_world_position(0.0, 3_000_000.0, 0.0, 1.0));
    }

    #[test]
    fn auto_size_contains_range() {
        let mut c = Camera::default();
        c.auto_size(800.0, 600.0, Some((0.0, 100.0, 0.0, 50.0)));
        assert!(c.left < 0.0 && c.right > 100.0 && c.bottom < 0.0 && c.top > 50.0);
        // aspect equals control aspect
        assert!((c.width() / c.height() - 800.0 / 600.0).abs() < 1e-9);
    }

    #[test]
    fn wheel_keeps_mouse_point() {
        let mut c = Camera::default();
        let (w, h) = (500.0, 400.0);
        let (mx, my) = c.screen_to_world(100.0, 100.0, w, h);
        c.wheel_zoom(mx, my, 0.12);
        let (mx2, my2) = c.screen_to_world(100.0, 100.0, w, h);
        assert!((mx - mx2).abs() < 1e-9 && (my - my2).abs() < 1e-9);
        assert!(c.width() < 200.0);
    }

    #[test]
    fn ticks_skip_near_bbox() {
        let t = ruler_ticks(0, 200, 10.0, Some((3, 100)));
        let vals: Vec<i64> = t.iter().map(|x| x.0).collect();
        assert!(vals.contains(&3) && vals.contains(&100));
        assert!(!vals.contains(&0) && vals.contains(&10)); // 0 is within half step of the min edge
        assert!(vals.contains(&150));
        let plain = ruler_ticks(-20, 20, 10.0, None);
        assert_eq!(plain.iter().map(|x| x.0).collect::<Vec<_>>(), vec![-20, -10, 0, 10, 20]);
    }
}
