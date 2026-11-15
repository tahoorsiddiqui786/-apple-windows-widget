//! Glass Desktop — native core.
//!
//! Module map:
//! * `config`   – schema, defaults, validation, persistence (config.json / state.json)
//! * `layout`   – dock geometry shared by native windows and the webview
//! * `dock`     – controller: windows, visibility, native events
//! * `launcher` – validated, shell-free app launching
//! * `icons`    – custom icon files and real Windows app icons as data URLs
//! * `native`   – Win32 wrappers (DWM blur backdrop, regions, hotkey, appbar…)
//! * `tray`     – system tray menu
//! * `commands` – the IPC surface granted to the dock webview

mod commands;
mod config;
mod dock;
mod icons;
mod launcher;
mod layout;
mod native;
mod tray;

pub fn run() {
    if !native::single_instance::acquire() {
        // Already running: ask that instance to show its dock, then exit.
        native::single_instance::signal_existing();
        return;
    }

    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_dock_state,
            commands::dock_ready,
            commands::launch_item,
            commands::reveal_item,
            commands::set_dock_interactive,
            commands::set_backdrop_extent,
            commands::hide_dock,
            commands::open_config,
        ])
        .setup(|app| {
            dock::init(app.handle())?;
            tray::init(app.handle())?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start Glass Desktop");

    app.run(|app, event| match event {
        // The dock window never closes, but keep the process alive regardless;
        // quitting is explicit (tray › Quit).
        tauri::RunEvent::ExitRequested { api, code: None, .. } => api.prevent_exit(),
        tauri::RunEvent::Exit => dock::shutdown(app),
        _ => {}
    });
}
