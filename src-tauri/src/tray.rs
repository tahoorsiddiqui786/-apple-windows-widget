//! System tray: the dock has no window chrome, so this is where you show/hide
//! it, edit or reload the config, toggle launch-at-startup and quit.

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::dock;
use crate::native::autostart;

const TRAY_ID: &str = "glass-desktop";

struct TrayHandles {
    toggle: MenuItem<Wry>,
    autostart: CheckMenuItem<Wry>,
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let hidden = dock::is_hidden(app);
    let toggle = MenuItem::with_id(app, "toggle", toggle_label(hidden), true, None::<&str>)?;
    let edit = MenuItem::with_id(app, "edit-config", "Edit Configuration…", true, None::<&str>)?;
    let reload = MenuItem::with_id(app, "reload", "Reload Configuration", true, None::<&str>)?;
    let assets = MenuItem::with_id(app, "assets", "Open Assets Folder", true, None::<&str>)?;
    let startup = CheckMenuItem::with_id(app, "autostart", "Launch at Sign-in", true, autostart::is_enabled(), None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Glass Desktop", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &toggle,
            &PredefinedMenuItem::separator(app)?,
            &edit,
            &reload,
            &assets,
            &PredefinedMenuItem::separator(app)?,
            &startup,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Glass Desktop")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => dock::toggle(app),
            "edit-config" => dock::open_config(app),
            "reload" => dock::reload(app),
            "assets" => dock::open_assets_folder(app),
            "autostart" => toggle_autostart(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                dock::toggle(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    app.manage(TrayHandles { toggle, autostart: startup });
    Ok(())
}

fn toggle_label(hidden: bool) -> &'static str {
    if hidden {
        "Show Dock"
    } else {
        "Hide Dock"
    }
}

pub fn set_hidden(app: &AppHandle, hidden: bool) {
    if let Some(handles) = app.try_state::<TrayHandles>() {
        let _ = handles.toggle.set_text(toggle_label(hidden));
    }
}

/// Shows config problems in the tray tooltip (hover the tray icon).
pub fn set_status(app: &AppHandle, warnings: &[String]) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let text = match warnings.len() {
            0 => "Glass Desktop".to_string(),
            1 => "Glass Desktop — 1 configuration warning".to_string(),
            n => format!("Glass Desktop — {n} configuration warnings"),
        };
        let _ = tray.set_tooltip(Some(text));
    }
}

fn toggle_autostart(app: &AppHandle) {
    let enable = !autostart::is_enabled();
    if let Err(e) = autostart::set_enabled(enable) {
        eprintln!("[autostart] {e}");
    }
    if let Some(handles) = app.try_state::<TrayHandles>() {
        let _ = handles.autostart.set_checked(autostart::is_enabled());
    }
}
