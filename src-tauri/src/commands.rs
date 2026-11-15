//! The complete IPC surface exposed to the dock webview. Each command is
//! declared in build.rs and granted in capabilities/dock.json; nothing else
//! (no fs, shell, http or window plugins) is reachable from JavaScript.
//!
//! Commands without `async` run on the main thread, which is where the
//! window-region updates must happen anyway.

use tauri::AppHandle;

use crate::dock::{self, DockState, WindowEntry};

#[tauri::command]
pub async fn get_dock_state(app: AppHandle) -> Result<DockState, String> {
    dock::get_state(app).await
}

#[tauri::command]
pub fn dock_ready(app: AppHandle) {
    dock::mark_ready(&app);
}

/// Launches a configured item by id. The webview cannot pass a path.
#[tauri::command]
pub async fn launch_item(app: AppHandle, id: String) -> Result<(), String> {
    dock::launch(app, id).await
}

#[tauri::command]
pub async fn reveal_item(app: AppHandle, id: String) -> Result<(), String> {
    dock::reveal(app, id).await
}

/// Click on an icon: switch to the app if it is open, otherwise launch it.
/// Returns "focused" or "launched".
#[tauri::command]
pub async fn activate_item(app: AppHandle, id: String) -> Result<&'static str, String> {
    dock::activate(app, id).await
}

/// Titles of an app's open windows, for its right-click menu. Read at call
/// time only; never stored.
#[tauri::command]
pub fn list_item_windows(app: AppHandle, id: String) -> Vec<WindowEntry> {
    dock::list_windows(&app, &id)
}

/// Brings one of an app's listed windows to the front.
#[tauri::command]
pub fn focus_item_window(app: AppHandle, id: String, handle: String) -> Result<(), String> {
    dock::focus_window(&app, &id, &handle)
}

/// `true` while the pointer is over the dock: the whole dock window becomes
/// hit-testable so magnified icons and menus can be used. `false` shrinks the
/// hit area back to the resting bar so clicks fall through to other apps.
#[tauri::command]
pub fn set_dock_interactive(app: AppHandle, expanded: bool) {
    dock::set_interactive(&app, expanded);
}

/// How far (logical px) the glass bar currently extends past its resting
/// edges while magnifying; the native blur follows it.
#[tauri::command]
pub fn set_backdrop_extent(app: AppHandle, left: f64, right: f64) {
    dock::set_extent(&app, left, right);
}

#[tauri::command]
pub fn hide_dock(app: AppHandle) {
    dock::set_hidden(&app, true);
}

#[tauri::command]
pub fn open_config(app: AppHandle) {
    dock::open_config(&app);
}
