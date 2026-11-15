//! Dock geometry. Rust owns window placement; the webview receives these
//! numbers and draws inside them, so native and CSS layers always line up.

use serde::Serialize;

use crate::config::DockConfig;
use crate::native::monitor::WorkArea;

/// Window rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// All values are logical (CSS) pixels relative to the dock window, except
/// `phys`, which is the window rect on screen in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub scale: f64,
    pub width: f64,
    pub height: f64,
    pub bar_left: f64,
    pub bar_top: f64,
    pub bar_width: f64,
    pub bar_height: f64,
    /// Upper bound of how far the bar may grow on each side while magnifying.
    pub max_expand: f64,
    /// Transparent margin kept hit-testable around the resting bar (for its shadow).
    pub halo: f64,
    pub bottom_pad: f64,
    #[serde(skip)]
    pub phys: PhysRect,
}

const SIDE_PAD: f64 = 12.0;
const HALO: f64 = 10.0;
/// Space above the bar for magnified icons, labels and the context menu.
const MIN_HEADROOM: f64 = 230.0;

pub fn compute(cfg: &DockConfig, item_count: usize, work: WorkArea) -> Layout {
    let n = item_count.max(1) as f64;
    let s = cfg.icon_size;
    let bar_width = (n * s + (n - 1.0) * cfg.icon_spacing + 2.0 * cfg.padding).round();
    let bar_height = (s + 2.0 * cfg.padding).round();
    let extra = s * (cfg.magnification - 1.0);
    let max_expand = (extra * cfg.magnification_range * 1.1 + 8.0).ceil();
    let headroom = (extra + 72.0).max(MIN_HEADROOM).ceil();
    let bottom_pad = cfg.margin_bottom.min(18.0).floor();

    let scale = if work.scale.is_finite() && work.scale > 0.0 { work.scale } else { 1.0 };
    let width = bar_width + 2.0 * (max_expand + SIDE_PAD);
    let height = headroom + bar_height + bottom_pad;
    let w_px = (width * scale).ceil() as i32;
    let h_px = (height * scale).ceil() as i32;
    let bar_bottom_px = work.bottom - (cfg.margin_bottom * scale).round() as i32;
    let y_px = bar_bottom_px - ((headroom + bar_height) * scale).round() as i32;
    let x_px = work.left + (work.width() - w_px) / 2;

    // What the webview sees as its viewport.
    let width_l = w_px as f64 / scale;
    let height_l = h_px as f64 / scale;
    Layout {
        scale,
        width: width_l,
        height: height_l,
        bar_left: ((width_l - bar_width) / 2.0).round(),
        bar_top: headroom,
        bar_width,
        bar_height,
        max_expand,
        halo: HALO,
        bottom_pad,
        phys: PhysRect { x: x_px, y: y_px, w: w_px, h: h_px },
    }
}

impl Layout {
    fn px(&self, v: f64) -> i32 {
        (v * self.scale).round() as i32
    }

    /// Screen rectangle (physical px) of the native blur backdrop: exactly the
    /// glass bar, grown by the current magnification extents, inset by one
    /// device pixel so its edge hides under the webview's hairline border.
    ///
    /// The backdrop window is *sized* to the bar rather than clipped with a
    /// region, because some Windows 11 builds draw DWM backdrops over the
    /// whole window rectangle and ignore window regions.
    pub fn backdrop_rect(&self, left_expand: f64, right_expand: f64) -> PhysRect {
        let l = left_expand.clamp(0.0, self.max_expand);
        let r = right_expand.clamp(0.0, self.max_expand);
        let inset = self.scale.floor().max(1.0) as i32;
        let x1 = self.phys.x + self.px(self.bar_left - l) + inset;
        let y1 = self.phys.y + self.px(self.bar_top) + inset;
        let x2 = self.phys.x + self.px(self.bar_left + self.bar_width + r) - inset;
        let y2 = self.phys.y + self.px(self.bar_top + self.bar_height) - inset;
        PhysRect { x: x1, y: y1, w: (x2 - x1).max(1), h: (y2 - y1).max(1) }
    }

    /// Corner radius of the backdrop in physical px.
    pub fn backdrop_radius(&self, radius: f64) -> i32 {
        let inset = self.scale.floor().max(1.0) as i32;
        (self.px(radius.min(self.bar_height / 2.0)) - inset).max(0)
    }

    /// Hit-test rectangle while the dock is at rest: the bar plus a thin halo.
    pub fn resting_hit_rect(&self) -> (i32, i32, i32, i32) {
        (
            self.px(self.bar_left - self.halo),
            self.px(self.bar_top - self.halo * 0.6),
            self.px(self.bar_left + self.bar_width + self.halo),
            self.px(self.bar_top + self.bar_height + self.bottom_pad),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work(scale: f64) -> WorkArea {
        WorkArea { left: 0, top: 0, right: 2560, bottom: 1550, scale }
    }

    #[test]
    fn centred_above_work_area_bottom() {
        let cfg = DockConfig::default();
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let l = compute(&cfg, 6, work(scale));
            let centre = l.phys.x + l.phys.w / 2;
            assert!((centre - 1280).abs() <= 1, "scale {scale}");
            let bar_bottom = l.phys.y as f64 + (l.bar_top + l.bar_height) * scale;
            let expected = 1550.0 - cfg.margin_bottom * scale;
            assert!((bar_bottom - expected).abs() <= 1.5, "scale {scale}: {bar_bottom} vs {expected}");
            assert!(l.phys.y + l.phys.h <= 1550, "window must stay inside the work area");
        }
    }

    #[test]
    fn bar_fits_window_with_room_to_magnify() {
        let cfg = DockConfig::default();
        let l = compute(&cfg, 6, work(1.25));
        assert!(l.bar_left >= l.max_expand);
        assert!(l.bar_left + l.bar_width + l.max_expand <= l.width + 0.5);
        assert!(l.bar_top >= cfg.icon_size * (cfg.magnification - 1.0) + 40.0);
    }

    #[test]
    fn backdrop_stays_inside_dock_window_and_covers_only_the_bar() {
        for scale in [1.0, 1.5] {
            let l = compute(&DockConfig::default(), 6, work(scale));
            let max = l.backdrop_rect(1e6, 1e6);
            assert!(max.x >= l.phys.x && max.x + max.w <= l.phys.x + l.phys.w);
            let rest = l.backdrop_rect(0.0, 0.0);
            let bar_h_px = (l.bar_height * scale).round() as i32;
            assert!(rest.h <= bar_h_px && rest.h >= bar_h_px - 3, "scale {scale}");
            assert!(rest.y > l.phys.y + (l.bar_top * scale) as i32 - 1, "backdrop must not cover the headroom");
        }
    }
}
