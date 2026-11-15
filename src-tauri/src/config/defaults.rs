use super::{DockItem, LaunchTarget};

/// First-run dock contents: six apps that ship with Windows 11. They are
/// referenced by AppUserModelID so they launch exactly like the Start menu
/// does. Items whose app is missing are skipped at runtime (not deleted).
pub fn default_items() -> Vec<DockItem> {
    let app = |id: &str, label: &str, aumid: &str| DockItem {
        id: id.into(),
        label: label.into(),
        target: LaunchTarget::App { aumid: aumid.into() },
        icon: None,
        badge: None,
    };
    vec![
        app("explorer", "File Explorer", "Microsoft.Windows.Explorer"),
        app("edge", "Microsoft Edge", "MSEdge"),
        app("terminal", "Terminal", "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App"),
        app("notepad", "Notepad", "Microsoft.WindowsNotepad_8wekyb3d8bbwe!App"),
        app("calculator", "Calculator", "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
        app(
            "settings",
            "Settings",
            "windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel",
        ),
    ]
}
