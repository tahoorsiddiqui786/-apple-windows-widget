//! Clamping and structural validation of a freshly parsed config.
//! Filesystem checks (does the exe exist?) happen in `launcher`, at use time.

use std::collections::HashSet;

use super::{AppConfig, CONFIG_VERSION, MAX_ITEMS};

fn clamp(value: &mut f64, min: f64, max: f64, default: f64, name: &str, warnings: &mut Vec<String>) {
    if !value.is_finite() {
        warnings.push(format!("{name} is not a number; using {default}."));
        *value = default;
    } else if *value < min || *value > max {
        let clamped = value.clamp(min, max);
        warnings.push(format!("{name} {value} is out of range [{min}, {max}]; using {clamped}."));
        *value = clamped;
    }
}

/// `#rgb` or `#rrggbb` only. Anything else (CSS functions, names, `url()`)
/// is rejected so config values can never inject CSS.
pub fn is_hex_color(s: &str) -> bool {
    let Some(hex) = s.strip_prefix('#') else { return false };
    (hex.len() == 3 || hex.len() == 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn is_valid_item_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 48
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Drops control characters and bidi overrides (which can visually spoof a
/// label), then trims and caps the length.
fn clean_text(s: &str, max_chars: usize) -> String {
    let is_bidi = |c: char| {
        matches!(c, '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
    };
    s.chars()
        .filter(|&c| !c.is_control() && !is_bidi(c))
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn sanitize(cfg: &mut AppConfig) -> Vec<String> {
    let mut w = Vec::new();

    if cfg.version != CONFIG_VERSION {
        w.push(format!("Unknown config version {}; reading it as version {CONFIG_VERSION}.", cfg.version));
        cfg.version = CONFIG_VERSION;
    }

    let d = &mut cfg.dock;
    clamp(&mut d.icon_size, 24.0, 128.0, 52.0, "dock.iconSize", &mut w);
    clamp(&mut d.icon_spacing, 0.0, 48.0, 10.0, "dock.iconSpacing", &mut w);
    clamp(&mut d.padding, 2.0, 32.0, 9.0, "dock.padding", &mut w);
    clamp(&mut d.magnification, 1.0, 2.2, 1.6, "dock.magnification", &mut w);
    clamp(&mut d.magnification_range, 1.0, 6.0, 2.6, "dock.magnificationRange", &mut w);
    clamp(&mut d.margin_bottom, 0.0, 120.0, 12.0, "dock.marginBottom", &mut w);

    let a = &mut cfg.appearance;
    clamp(&mut a.glass_opacity, 0.0, 1.0, 0.42, "appearance.glassOpacity", &mut w);
    clamp(&mut a.border_opacity, 0.0, 1.0, 0.16, "appearance.borderOpacity", &mut w);
    clamp(&mut a.corner_radius, 0.0, 48.0, 22.0, "appearance.cornerRadius", &mut w);
    clamp(&mut a.animation_speed, 0.25, 4.0, 1.0, "appearance.animationSpeed", &mut w);
    if !is_hex_color(&a.glass_tint) {
        w.push(format!("appearance.glassTint {:?} is not a #rrggbb colour; using #0b0d13.", a.glass_tint));
        a.glass_tint = "#0b0d13".into();
    }
    if !is_hex_color(&a.accent_color) {
        w.push(format!("appearance.accentColor {:?} is not a #rrggbb colour; using #7c8cff.", a.accent_color));
        a.accent_color = "#7c8cff".into();
    }

    cfg.behavior.toggle_hotkey = clean_text(&cfg.behavior.toggle_hotkey, 40);

    if let Some(dir) = &cfg.paths.assets_dir {
        if dir.trim().is_empty() {
            cfg.paths.assets_dir = None;
        }
    }

    // Items: structural checks only.
    let mut seen = HashSet::new();
    cfg.items.retain_mut(|item| {
        item.id = item.id.trim().to_ascii_lowercase();
        if !is_valid_item_id(&item.id) {
            w.push(format!("Item id {:?} must be 1–48 characters of a–z, 0–9 or '-'; item skipped.", item.id));
            return false;
        }
        if !seen.insert(item.id.clone()) {
            w.push(format!("Duplicate item id {:?}; later copy skipped.", item.id));
            return false;
        }
        item.label = clean_text(&item.label, 64);
        if item.label.is_empty() {
            item.label = item.id.clone();
        }
        if let Some(badge) = &item.badge {
            let b = clean_text(badge, 4);
            item.badge = if b.is_empty() { None } else { Some(b) };
        }
        if let Some(icon) = &item.icon {
            if icon.trim().is_empty() {
                item.icon = None;
            }
        }
        true
    });
    if cfg.items.len() > MAX_ITEMS {
        w.push(format!("Only the first {MAX_ITEMS} of {} items are shown.", cfg.items.len()));
        cfg.items.truncate(MAX_ITEMS);
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{default_items, DockItem, LaunchTarget};

    #[test]
    fn clamps_out_of_range_values() {
        let mut cfg = AppConfig::default();
        cfg.dock.icon_size = 4000.0;
        cfg.appearance.glass_opacity = -1.0;
        cfg.dock.magnification = f64::NAN;
        let w = sanitize(&mut cfg);
        assert_eq!(cfg.dock.icon_size, 128.0);
        assert_eq!(cfg.appearance.glass_opacity, 0.0);
        assert_eq!(cfg.dock.magnification, 1.6);
        assert_eq!(w.len(), 3);
    }

    #[test]
    fn rejects_css_injection_in_colors() {
        let mut cfg = AppConfig::default();
        cfg.appearance.accent_color = "red; background: url(https://x)".into();
        sanitize(&mut cfg);
        assert_eq!(cfg.appearance.accent_color, "#7c8cff");
        assert!(is_hex_color("#abc"));
        assert!(is_hex_color("#A0B1C2"));
        assert!(!is_hex_color("#abcd"));
        assert!(!is_hex_color("abc"));
    }

    #[test]
    fn drops_bad_and_duplicate_ids() {
        let mut cfg = AppConfig::default();
        cfg.items = default_items();
        let mut dup = cfg.items[0].clone();
        dup.label = "dup".into();
        cfg.items.push(dup);
        cfg.items.push(DockItem {
            id: "../evil".into(),
            label: "x".into(),
            target: LaunchTarget::Folder { path: "C:\\".into() },
            icon: None,
            badge: None,
        });
        let w = sanitize(&mut cfg);
        assert_eq!(cfg.items.len(), default_items().len());
        assert_eq!(w.len(), 2);
    }

    #[test]
    fn strips_control_characters_from_labels() {
        let mut cfg = AppConfig::default();
        cfg.items = default_items();
        cfg.items[0].label = "Hi\u{0007}\u{202e}there".into();
        cfg.items[0].badge = Some("12345".into());
        sanitize(&mut cfg);
        assert_eq!(cfg.items[0].label, "Hithere");
        assert_eq!(cfg.items[0].badge.as_deref(), Some("1234"));
    }
}
