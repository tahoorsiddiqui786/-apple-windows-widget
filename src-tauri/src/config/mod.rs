//! Local configuration: schema, defaults, validation and atomic persistence.
//!
//! The config lives in a single JSON file in the per-user app config folder
//! (`%APPDATA%\com.glassdesktop.shell\config.json`). Nothing is synced or
//! uploaded. Every value is clamped/validated on load so a hand-edited file can
//! never push the dock into an unusable or unsafe state.
//!
//! `config.json` is *yours*: the app writes it once on first run and then only
//! reads it. Runtime state the app changes by itself (e.g. "dock hidden") goes
//! to a separate `state.json`, so your edits are never rewritten.

mod defaults;
pub mod state;
mod validate;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use defaults::default_items;
pub use validate::is_valid_item_id;

pub const CONFIG_VERSION: u32 = 1;
pub const CONFIG_FILE_NAME: &str = "config.json";
pub const MAX_ITEMS: usize = 24;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    /// Schema version, for future migrations.
    pub version: u32,
    pub dock: DockConfig,
    pub appearance: Appearance,
    pub behavior: Behavior,
    pub paths: Paths,
    pub items: Vec<DockItem>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            dock: DockConfig::default(),
            appearance: Appearance::default(),
            behavior: Behavior::default(),
            paths: Paths::default(),
            items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum DockPosition {
    /// Bottom-centre of the primary monitor's work area (above the taskbar).
    #[default]
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DockConfig {
    pub position: DockPosition,
    /// Resting icon size in logical pixels.
    pub icon_size: f64,
    /// Gap between icons in logical pixels.
    pub icon_spacing: f64,
    /// Inner padding between the glass edge and the icons.
    pub padding: f64,
    /// Peak scale of the hovered icon (1.0 disables magnification).
    pub magnification: f64,
    /// How many neighbouring icons the magnification wave spreads over.
    pub magnification_range: f64,
    /// Gap between the dock and the bottom of the work area (taskbar edge).
    pub margin_bottom: f64,
    /// Show the app name above the hovered icon.
    pub show_labels: bool,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            position: DockPosition::Bottom,
            icon_size: 52.0,
            icon_spacing: 10.0,
            padding: 9.0,
            magnification: 1.6,
            magnification_range: 2.6,
            margin_bottom: 12.0,
            show_labels: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Backdrop {
    /// Native DWM acrylic (blur + luminosity + noise). Opt-in: how it renders
    /// depends on the Windows build and graphics driver (see docs/NATIVE_BLUR.md).
    Acrylic,
    /// Native DWM gaussian blur, lighter than acrylic. Opt-in, same caveat.
    Blur,
    /// Glass drawn by the dock itself (tint, highlights, hairline). Renders
    /// identically on every PC; the default.
    #[default]
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Appearance {
    pub backdrop: Backdrop,
    /// Base tint of the glass (hex `#rrggbb`).
    pub glass_tint: String,
    /// Tint opacity, 0 (clear) – 1 (solid).
    pub glass_opacity: f64,
    /// Opacity of the hairline edge highlight.
    pub border_opacity: f64,
    pub corner_radius: f64,
    /// Accent colour used for glows and indicators (hex).
    pub accent_color: String,
    /// Subtle accent glow under hovered icons.
    pub glow: bool,
    /// Multiplier for all animation speeds (1 = default, 2 = twice as fast).
    pub animation_speed: f64,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            backdrop: Backdrop::None,
            glass_tint: "#0b0d13".into(),
            glass_opacity: 0.42,
            border_opacity: 0.16,
            corner_radius: 22.0,
            accent_color: "#7c8cff".into(),
            glow: true,
            animation_speed: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Behavior {
    /// Temporarily hide the dock while a fullscreen app/game/presentation runs.
    pub hide_in_fullscreen: bool,
    /// Global show/hide shortcut, e.g. `Ctrl+Alt+D`. Empty string disables it.
    pub toggle_hotkey: String,
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            hide_in_fullscreen: true,
            toggle_hotkey: "Ctrl+Alt+D".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Paths {
    /// Folder holding `icons/` and `wallpapers/`. `null` = the default folder
    /// next to this config file.
    pub assets_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DockItem {
    /// Stable identifier, `[a-z0-9-]`, unique within the dock.
    pub id: String,
    pub label: String,
    pub target: LaunchTarget,
    /// Custom icon. A bare file name is looked up in `<assets>/icons/`; an
    /// absolute path is also accepted. PNG, SVG, ICO, JPG and WebP.
    /// When absent, the app's own Windows icon is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Optional short badge text (e.g. "3"), max 4 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,
}

/// What a dock item opens. There is deliberately no "command" or "script"
/// variant: the dock never runs a shell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LaunchTarget {
    /// An installed app from the Windows Apps folder (Start menu), identified
    /// by its AppUserModelID, e.g. `Microsoft.WindowsCalculator_8wekyb3d8bbwe!App`
    /// or `MSEdge`. Find IDs with PowerShell: `Get-StartApps`.
    App { aumid: String },
    /// A local `.exe` with optional arguments (passed verbatim, never through a shell).
    Exe {
        path: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
    },
    /// A Windows shortcut (`.lnk`).
    Shortcut { path: String },
    /// A local folder, opened in File Explorer.
    Folder { path: String },
}

/// Outcome of loading the config from disk. A file that fails to parse is
/// never overwritten: the app runs on defaults and reports a warning.
pub struct Loaded {
    pub config: AppConfig,
    pub warnings: Vec<String>,
}

pub fn config_path(config_dir: &Path) -> PathBuf {
    config_dir.join(CONFIG_FILE_NAME)
}

/// Loads the config, creating it with sensible defaults on first run.
pub fn load_or_create(config_dir: &Path) -> Loaded {
    let path = config_path(config_dir);
    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<AppConfig>(&text) {
            Ok(mut config) => {
                let warnings = validate::sanitize(&mut config);
                Loaded { config, warnings }
            }
            Err(err) => {
                let mut config = AppConfig::default();
                config.items = default_items();
                let warnings = validate::sanitize(&mut config);
                let mut all = vec![format!(
                    "config.json could not be parsed (line {}, column {}): {}. Running on defaults; the file was left untouched.",
                    err.line(),
                    err.column(),
                    err
                )];
                all.extend(warnings);
                Loaded { config, warnings: all }
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let mut config = AppConfig::default();
            config.items = default_items();
            let mut warnings = validate::sanitize(&mut config);
            if let Err(e) = save(config_dir, &config) {
                warnings.push(format!("Could not create config.json: {e}"));
            }
            Loaded { config, warnings }
        }
        Err(err) => {
            let mut config = AppConfig::default();
            config.items = default_items();
            let warnings = validate::sanitize(&mut config);
            let mut all = vec![format!("config.json could not be read: {err}. Running on defaults.")];
            all.extend(warnings);
            Loaded { config, warnings: all }
        }
    }
}

/// Writes the config atomically (temp file + rename) so a crash or power loss
/// mid-write can never leave a truncated config behind.
pub fn save(config_dir: &Path, config: &AppConfig) -> std::io::Result<()> {
    fs::create_dir_all(config_dir)?;
    let path = config_path(config_dir);
    let tmp = config_dir.join(format!("{CONFIG_FILE_NAME}.tmp"));
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(json.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
    }
    fs::rename(&tmp, &path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip() {
        let mut cfg = AppConfig::default();
        cfg.items = default_items();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn partial_file_fills_defaults() {
        let cfg: AppConfig = serde_json::from_str(r#"{ "dock": { "iconSize": 64 } }"#).unwrap();
        assert_eq!(cfg.dock.icon_size, 64.0);
        assert_eq!(cfg.dock.icon_spacing, DockConfig::default().icon_spacing);
        assert_eq!(cfg.appearance, Appearance::default());
    }

    #[test]
    fn target_kinds_parse() {
        let item: DockItem = serde_json::from_str(
            r#"{ "id": "code", "label": "Code", "target": { "kind": "exe", "path": "C:\\a\\b.exe", "args": ["--new-window"] } }"#,
        )
        .unwrap();
        assert!(matches!(item.target, LaunchTarget::Exe { ref args, .. } if args.len() == 1));
    }

    #[test]
    fn save_is_atomic_and_readable() {
        let dir = std::env::temp_dir().join(format!("glass-desktop-test-{}", std::process::id()));
        let mut cfg = AppConfig::default();
        cfg.items = default_items();
        save(&dir, &cfg).unwrap();
        let loaded = load_or_create(&dir);
        assert!(loaded.warnings.is_empty());
        assert_eq!(loaded.config.items.len(), cfg.items.len());
        assert!(!dir.join("config.json.tmp").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
