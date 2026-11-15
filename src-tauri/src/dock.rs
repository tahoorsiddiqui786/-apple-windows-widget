//! Dock controller: owns the config, the two dock windows (native backdrop +
//! webview), their geometry, visibility and the bridge to native events.
//!
//! Threading: window operations run on the main (UI) thread. Commands that do
//! blocking shell work (icon extraction, launching) run on worker threads.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::config::{self, state::RuntimeState, AppConfig, Appearance, Backdrop, DockConfig, DockItem};
use crate::layout::{self, Layout};
use crate::native::backdrop::{self, BlurKind, NativeEvent};
use crate::native::{appbar, com::ComGuard, hotkey, hwnd_from_isize, hwnd_to_isize, monitor, system_prefs, window_style};
use crate::{icons, launcher, tray};

pub const DOCK_LABEL: &str = "dock";

/// WebView2 (the Edge engine that renders the UI) runs Microsoft background
/// services by default: feature-flag/config fetches, component updates,
/// reliability pings, crash upload, SmartScreen. The dock only shows bundled,
/// local content, so all of that is switched off. The first group repeats
/// Tauri's own defaults, which this setting replaces.
const WEBVIEW2_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
--disable-background-networking \
--disable-component-update \
--disable-domain-reliability \
--disable-client-side-phishing-detection \
--disable-sync \
--disable-breakpad \
--no-pings \
--no-first-run";
pub const EVT_STATE_CHANGED: &str = "dock://state-changed";
pub const EVT_LAYOUT_CHANGED: &str = "dock://layout-changed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemView {
    pub id: String,
    pub label: String,
    pub badge: Option<String>,
    /// `data:` URL, or none (the UI then draws a monogram tile).
    pub icon: Option<String>,
    pub can_reveal: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockState {
    pub layout: Layout,
    pub dock: DockConfig,
    pub appearance: Appearance,
    /// The backdrop actually in use after fallbacks.
    pub backdrop: Backdrop,
    pub items: Vec<ItemView>,
    pub warnings: Vec<String>,
    pub hotkey: String,
}

pub struct Dock {
    inner: Mutex<Inner>,
}

struct Inner {
    config_dir: PathBuf,
    config: AppConfig,
    runtime: RuntimeState,
    config_warnings: Vec<String>,
    item_warnings: Vec<String>,
    items: Option<Vec<ItemView>>,
    generation: u64,
    layout: Layout,
    backdrop_hwnd: isize,
    dock_hwnd: isize,
    effective_backdrop: Backdrop,
    fullscreen_hidden: bool,
    ready: bool,
    extent: (f64, f64),
    interactive: bool,
    hotkey_registered: bool,
    hotkey_error: Option<String>,
    appbar_registered: bool,
}

impl Dock {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        // A panic while holding the lock must not brick the dock.
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Inner {
    fn assets_dir(&self) -> PathBuf {
        match &self.config.paths.assets_dir {
            Some(dir) => PathBuf::from(dir),
            None => self.config_dir.join("assets"),
        }
    }

    fn icons_dir(&self) -> PathBuf {
        self.assets_dir().join("icons")
    }

    fn warnings(&self) -> Vec<String> {
        let mut all = self.config_warnings.clone();
        all.extend(self.item_warnings.iter().cloned());
        if let Some(e) = &self.hotkey_error {
            all.push(e.clone());
        }
        all
    }

    fn visible(&self) -> bool {
        self.ready && !self.runtime.dock_hidden && !self.fullscreen_hidden
    }

    fn item_count(&self) -> usize {
        self.items.as_ref().map_or(self.config.items.len(), Vec::len)
    }

    fn state(&self, items: Vec<ItemView>) -> DockState {
        DockState {
            layout: self.layout,
            dock: self.config.dock.clone(),
            appearance: self.config.appearance.clone(),
            backdrop: self.effective_backdrop,
            items,
            warnings: self.warnings(),
            hotkey: if self.hotkey_registered { self.config.behavior.toggle_hotkey.clone() } else { String::new() },
        }
    }
}

// ---------------------------------------------------------------- startup

pub fn init(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = app.path().app_config_dir()?;
    let loaded = config::load_or_create(&config_dir);
    for w in &loaded.warnings {
        eprintln!("[config] {w}");
    }
    let runtime = config::state::load(&config_dir);
    if loaded.config.paths.assets_dir.is_none() {
        // Our own folder: safe to create. A user-chosen assets folder is left alone.
        let assets = config_dir.join("assets");
        let _ = std::fs::create_dir_all(assets.join("icons"));
        let _ = std::fs::create_dir_all(assets.join("wallpapers"));
    }

    let handle = app.clone();
    backdrop::set_event_handler(move |ev| {
        // Defer out of the window procedure; never re-enter while Win32 is mid-call.
        let h = handle.clone();
        let _ = handle.run_on_main_thread(move || on_native_event(&h, ev));
    });
    let backdrop_hwnd = backdrop::create()?;

    let layout = layout::compute(&loaded.config.dock, loaded.config.items.len(), monitor::primary_work_area());
    let window = WebviewWindowBuilder::new(app, DOCK_LABEL, WebviewUrl::App("index.html".into()))
        .title("Glass Dock")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .closable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .visible(false)
        .inner_size(layout.width, layout.height)
        .additional_browser_args(WEBVIEW2_ARGS)
        .build()?;
    window.on_window_event(|event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close(); // quit only from the tray
        }
    });
    let dock_hwnd = window.hwnd()?;
    window_style::make_non_activating_tool_window(dock_hwnd);
    window_style::set_owner(dock_hwnd, backdrop_hwnd);

    app.manage(Dock {
        inner: Mutex::new(Inner {
            config_dir,
            config: loaded.config,
            runtime,
            config_warnings: loaded.warnings,
            item_warnings: Vec::new(),
            items: None,
            generation: 0,
            layout,
            backdrop_hwnd: hwnd_to_isize(backdrop_hwnd),
            dock_hwnd: hwnd_to_isize(dock_hwnd),
            effective_backdrop: Backdrop::None,
            fullscreen_hidden: false,
            ready: false,
            extent: (0.0, 0.0),
            interactive: false,
            hotkey_registered: false,
            hotkey_error: None,
            appbar_registered: false,
        }),
    });

    let dock = app.state::<Dock>();
    let mut inner = dock.lock();
    apply_backdrop_effect(&mut inner);
    apply_geometry(&inner);
    register_hotkey(&mut inner);
    sync_appbar(&mut inner);
    Ok(())
}

pub fn shutdown(app: &AppHandle) {
    let Some(dock) = app.try_state::<Dock>() else { return };
    let inner = dock.lock();
    let a = hwnd_from_isize(inner.backdrop_hwnd);
    if inner.hotkey_registered {
        hotkey::unregister(a, hotkey::TOGGLE_DOCK_ID);
    }
    if inner.appbar_registered {
        appbar::unregister(a);
    }
    backdrop::destroy(a);
}

// ---------------------------------------------------------------- state for the UI

/// Resolves items (validation + icons) once per config generation and
/// returns everything the dock UI needs to render.
pub async fn get_state(app: AppHandle) -> Result<DockState, String> {
    let (items_cfg, icons_dir, generation) = {
        let dock = app.state::<Dock>();
        let inner = dock.lock();
        if let Some(items) = &inner.items {
            return Ok(inner.state(items.clone()));
        }
        (inner.config.items.clone(), inner.icons_dir(), inner.generation)
    };

    let (items, warnings) = tauri::async_runtime::spawn_blocking(move || resolve_items(&items_cfg, &icons_dir))
        .await
        .map_err(|e| e.to_string())?;

    let state = {
        let dock = app.state::<Dock>();
        let mut inner = dock.lock();
        if inner.generation != generation {
            return Err("Configuration changed while loading; retrying.".into());
        }
        for w in &warnings {
            eprintln!("[items] {w}");
        }
        inner.item_warnings = warnings;
        inner.items = Some(items.clone());
        inner.layout = layout::compute(&inner.config.dock, items.len(), monitor::primary_work_area());
        inner.state(items)
    };
    tray::set_status(&app, &state.warnings);
    let h = app.clone();
    app.run_on_main_thread(move || {
        let dock = h.state::<Dock>();
        let inner = dock.lock();
        apply_geometry(&inner);
    })
    .map_err(|e| e.to_string())?;
    Ok(state)
}

fn resolve_items(items: &[DockItem], icons_dir: &Path) -> (Vec<ItemView>, Vec<String>) {
    let _com = ComGuard::new();
    let mut out = Vec::with_capacity(items.len());
    let mut warnings = Vec::new();
    for item in items {
        let validated = match launcher::validate(&item.target) {
            Ok(v) => v,
            Err(e) => {
                warnings.push(format!("\"{}\" is not shown: {e}", item.label));
                continue;
            }
        };
        let custom = item.icon.as_deref().and_then(|spec| {
            icons::custom_icon_data_url(spec, icons_dir)
                .map_err(|e| warnings.push(format!("\"{}\": {e} Using the app icon.", item.label)))
                .ok()
        });
        let icon = custom.or_else(|| {
            icons::shell_icon_data_url(&validated)
                .map_err(|e| warnings.push(format!("\"{}\": no Windows icon ({e}).", item.label)))
                .ok()
        });
        out.push(ItemView {
            id: item.id.clone(),
            label: item.label.clone(),
            badge: item.badge.clone(),
            icon,
            can_reveal: validated.reveal_path().is_some(),
        });
    }
    (out, warnings)
}

// ---------------------------------------------------------------- actions

fn find_target(app: &AppHandle, id: &str) -> Result<config::LaunchTarget, String> {
    if !config::is_valid_item_id(id) {
        return Err("Invalid item id.".into());
    }
    let dock = app.state::<Dock>();
    let inner = dock.lock();
    inner
        .config
        .items
        .iter()
        .find(|i| i.id == id)
        .map(|i| i.target.clone())
        .ok_or_else(|| "That item is no longer in the dock.".into())
}

pub async fn launch(app: AppHandle, id: String) -> Result<(), String> {
    let target = find_target(&app, &id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _com = ComGuard::new();
        // Re-validate at click time: the file may have moved since startup.
        let validated = launcher::validate(&target)?;
        launcher::launch(&validated)
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn reveal(app: AppHandle, id: String) -> Result<(), String> {
    let target = find_target(&app, &id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _com = ComGuard::new();
        let validated = launcher::validate(&target)?;
        let path = validated.reveal_path().ok_or("Apps have no file location.")?;
        launcher::reveal(path)
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn open_config(app: &AppHandle) {
    let path = {
        let dock = app.state::<Dock>();
        let inner = dock.lock();
        config::config_path(&inner.config_dir)
    };
    std::thread::spawn(move || {
        if let Err(e) = launcher::open_own_file(&path) {
            eprintln!("[config] could not open {}: {e}", path.display());
        }
    });
}

pub fn open_assets_folder(app: &AppHandle) {
    let dir = {
        let dock = app.state::<Dock>();
        let inner = dock.lock();
        inner.assets_dir()
    };
    std::thread::spawn(move || {
        let _ = std::fs::create_dir_all(&dir);
        if let Err(e) = launcher::open_own_file(&dir) {
            eprintln!("[assets] could not open {}: {e}", dir.display());
        }
    });
}

/// Called by the UI after its first render, so the dock never flashes empty.
pub fn mark_ready(app: &AppHandle) {
    let dock = app.state::<Dock>();
    let mut inner = dock.lock();
    inner.ready = true;
    apply_visibility(&inner);
}

pub fn set_interactive(app: &AppHandle, expanded: bool) {
    let dock = app.state::<Dock>();
    let mut inner = dock.lock();
    if inner.interactive != expanded {
        inner.interactive = expanded;
        apply_hit_region(&inner);
    }
}

pub fn set_extent(app: &AppHandle, left: f64, right: f64) {
    if !left.is_finite() || !right.is_finite() {
        return;
    }
    let dock = app.state::<Dock>();
    let mut inner = dock.lock();
    let max = inner.layout.max_expand;
    let next = (left.clamp(0.0, max), right.clamp(0.0, max));
    if next != inner.extent {
        inner.extent = next;
        apply_backdrop_region(&inner);
    }
}

pub fn is_hidden(app: &AppHandle) -> bool {
    app.try_state::<Dock>().is_some_and(|d| d.lock().runtime.dock_hidden)
}

pub fn set_hidden(app: &AppHandle, hidden: bool) {
    {
        let dock = app.state::<Dock>();
        let mut inner = dock.lock();
        if inner.runtime.dock_hidden != hidden {
            inner.runtime.dock_hidden = hidden;
            if let Err(e) = config::state::save(&inner.config_dir, &inner.runtime) {
                eprintln!("[state] could not save: {e}");
            }
        }
        if hidden {
            inner.interactive = false;
            inner.extent = (0.0, 0.0);
            apply_hit_region(&inner);
            apply_backdrop_region(&inner);
        }
        apply_visibility(&inner);
    }
    tray::set_hidden(app, hidden);
}

pub fn toggle(app: &AppHandle) {
    set_hidden(app, !is_hidden(app));
}

/// Re-reads config.json and redraws the dock.
pub fn reload(app: &AppHandle) {
    {
        let dock = app.state::<Dock>();
        let mut inner = dock.lock();
        let loaded = config::load_or_create(&inner.config_dir);
        for w in &loaded.warnings {
            eprintln!("[config] {w}");
        }
        inner.config = loaded.config;
        inner.config_warnings = loaded.warnings;
        inner.items = None;
        inner.item_warnings.clear();
        inner.generation += 1;
        inner.extent = (0.0, 0.0);
        inner.interactive = false;
        inner.layout = layout::compute(&inner.config.dock, inner.config.items.len(), monitor::primary_work_area());
        register_hotkey(&mut inner);
        sync_appbar(&mut inner);
        apply_backdrop_effect(&mut inner);
        apply_geometry(&inner);
        apply_visibility(&inner);
    }
    let _ = app.emit_to(DOCK_LABEL, EVT_STATE_CHANGED, ());
}

// ---------------------------------------------------------------- native events

fn on_native_event(app: &AppHandle, ev: NativeEvent) {
    match ev {
        NativeEvent::Hotkey(id) if id == hotkey::TOGGLE_DOCK_ID => toggle(app),
        NativeEvent::Hotkey(_) => {}
        NativeEvent::ShowRequested => set_hidden(app, false),
        NativeEvent::Fullscreen(opened) => {
            let dock = app.state::<Dock>();
            let mut inner = dock.lock();
            if !inner.config.behavior.hide_in_fullscreen {
                return;
            }
            let hide = opened && appbar::user_is_in_fullscreen();
            if hide != inner.fullscreen_hidden {
                inner.fullscreen_hidden = hide;
                apply_visibility(&inner);
            }
        }
        NativeEvent::DisplayChanged => {
            let (layout_changed, backdrop_changed, layout) = {
                let dock = app.state::<Dock>();
                let mut inner = dock.lock();
                let before_backdrop = inner.effective_backdrop;
                apply_backdrop_effect(&mut inner);
                let next = layout::compute(&inner.config.dock, inner.item_count(), monitor::primary_work_area());
                let changed = next != inner.layout;
                if changed {
                    inner.layout = next;
                    apply_geometry(&inner);
                }
                apply_visibility(&inner);
                (changed, before_backdrop != inner.effective_backdrop, inner.layout)
            };
            if backdrop_changed {
                let _ = app.emit_to(DOCK_LABEL, EVT_STATE_CHANGED, ());
            } else if layout_changed {
                let _ = app.emit_to(DOCK_LABEL, EVT_LAYOUT_CHANGED, layout);
            }
        }
        NativeEvent::ExplorerRestarted => {
            let dock = app.state::<Dock>();
            let mut inner = dock.lock();
            inner.appbar_registered = false;
            sync_appbar(&mut inner);
        }
    }
}

// ---------------------------------------------------------------- window plumbing (main thread)

fn parse_hex(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    let full: String = if h.len() == 3 { h.chars().flat_map(|c| [c, c]).collect() } else { h.to_string() };
    let byte = |i: usize| u8::from_str_radix(full.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    (byte(0), byte(2), byte(4))
}

fn apply_backdrop_effect(inner: &mut Inner) {
    let a = hwnd_from_isize(inner.backdrop_hwnd);
    let wanted = if system_prefs::transparency_enabled() { inner.config.appearance.backdrop } else { Backdrop::None };
    inner.effective_backdrop = match wanted {
        Backdrop::None => {
            backdrop::clear_blur(a);
            Backdrop::None
        }
        kind => {
            let blur = if kind == Backdrop::Acrylic { BlurKind::Acrylic } else { BlurKind::Blur };
            let tint = parse_hex(&inner.config.appearance.glass_tint);
            let alpha = (inner.config.appearance.glass_opacity * 255.0).round().clamp(0.0, 255.0) as u8;
            if backdrop::apply_blur(a, blur, tint, alpha) {
                kind
            } else {
                eprintln!("[backdrop] DWM blur unavailable; using tinted fallback.");
                Backdrop::None
            }
        }
    };
}

fn apply_geometry(inner: &Inner) {
    let r = inner.layout.phys;
    let b = inner.layout.backdrop_rect(inner.extent.0, inner.extent.1);
    window_style::place_topmost(hwnd_from_isize(inner.backdrop_hwnd), b.x, b.y, b.w, b.h);
    window_style::place_topmost(hwnd_from_isize(inner.dock_hwnd), r.x, r.y, r.w, r.h);
    apply_backdrop_region(inner);
    apply_hit_region(inner);
}

/// Sizes the backdrop window to the (possibly magnified) glass bar and rounds
/// its corners. Called per animation frame while the bar grows or shrinks.
fn apply_backdrop_region(inner: &Inner) {
    let a = hwnd_from_isize(inner.backdrop_hwnd);
    let b = inner.layout.backdrop_rect(inner.extent.0, inner.extent.1);
    window_style::move_resize(a, b.x, b.y, b.w, b.h);
    let radius = inner.layout.backdrop_radius(inner.config.appearance.corner_radius);
    window_style::set_round_rect_region(a, 0, 0, b.w - 1, b.h - 1, radius);
}

fn apply_hit_region(inner: &Inner) {
    let rect = if inner.interactive { None } else { Some(inner.layout.resting_hit_rect()) };
    window_style::set_rect_region(hwnd_from_isize(inner.dock_hwnd), rect);
}

fn apply_visibility(inner: &Inner) {
    let a = hwnd_from_isize(inner.backdrop_hwnd);
    let b = hwnd_from_isize(inner.dock_hwnd);
    if inner.visible() {
        if inner.effective_backdrop == Backdrop::None {
            window_style::hide(a);
        } else {
            window_style::show_no_activate(a);
        }
        window_style::show_no_activate(b);
    } else {
        window_style::hide(b);
        window_style::hide(a);
    }
}

fn register_hotkey(inner: &mut Inner) {
    let a = hwnd_from_isize(inner.backdrop_hwnd);
    if inner.hotkey_registered {
        hotkey::unregister(a, hotkey::TOGGLE_DOCK_ID);
        inner.hotkey_registered = false;
    }
    inner.hotkey_error = match hotkey::parse(&inner.config.behavior.toggle_hotkey) {
        Ok(None) => None,
        Ok(Some(hk)) => match hotkey::register(a, hotkey::TOGGLE_DOCK_ID, hk) {
            Ok(()) => {
                inner.hotkey_registered = true;
                None
            }
            Err(e) => Some(format!("Hotkey {}: {e}", inner.config.behavior.toggle_hotkey)),
        },
        Err(e) => Some(e),
    };
}

fn sync_appbar(inner: &mut Inner) {
    let a = hwnd_from_isize(inner.backdrop_hwnd);
    let wanted = inner.config.behavior.hide_in_fullscreen;
    if wanted && !inner.appbar_registered {
        inner.appbar_registered = appbar::register(a);
    } else if !wanted && inner.appbar_registered {
        appbar::unregister(a);
        inner.appbar_registered = false;
        inner.fullscreen_hidden = false;
    }
}

#[cfg(test)]
mod tests {
    use super::parse_hex;

    #[test]
    fn hex_parsing() {
        assert_eq!(parse_hex("#0b0d13"), (0x0b, 0x0d, 0x13));
        assert_eq!(parse_hex("#fff"), (255, 255, 255));
    }
}
