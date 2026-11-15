fn main() {
    // Every IPC command is declared here. Tauri then generates one permission
    // per command, and only the ones granted in `capabilities/dock.json` can be
    // called from the dock webview. Undeclared commands are unreachable.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_dock_state",
            "dock_ready",
            "launch_item",
            "reveal_item",
            "activate_item",
            "list_item_windows",
            "focus_item_window",
            "set_dock_interactive",
            "set_backdrop_extent",
            "hide_dock",
            "open_config",
        ]),
    ))
    .expect("failed to run tauri-build");
}
