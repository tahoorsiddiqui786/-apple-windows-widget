//! Matching open app windows to dock items ("is it running?").

use std::collections::BTreeMap;

use crate::launcher::Validated;
use crate::native::app_windows::AppWindow;
use crate::native::shell_item;

/// How to recognise one dock item's windows. All strings lower-cased.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemMatcher {
    pub id: String,
    pub aumids: Vec<String>,
    pub exes: Vec<String>,
}

impl ItemMatcher {
    pub fn matches(&self, w: &AppWindow) -> bool {
        w.aumid.as_ref().is_some_and(|a| self.aumids.iter().any(|m| m == a))
            || w.exe.as_ref().is_some_and(|e| self.exes.iter().any(|m| m == e))
    }
}

/// Builds the matcher for a validated target. Needs COM (shortcut properties).
pub fn matcher_for(id: &str, target: &Validated) -> ItemMatcher {
    let mut m = ItemMatcher { id: id.to_string(), ..Default::default() };
    match target {
        Validated::App { aumid } => {
            m.aumids.push(aumid.to_lowercase());
            // Start-menu IDs of unpackaged apps can be known-folder paths,
            // e.g. "{6D809377-…}\Microsoft VS Code\Code.exe".
            if let Some(path) = shell_item::known_folder_app_path(aumid) {
                m.exes.push(path.to_lowercase());
            }
        }
        Validated::Exe { path, .. } => m.exes.push(path.to_string_lossy().to_lowercase()),
        Validated::Shortcut { path } => {
            let (aumid, target) = shell_item::shortcut_identity(path);
            m.aumids.extend(aumid.map(|a| a.to_lowercase()));
            m.exes.extend(target.map(|t| t.to_lowercase()));
        }
        // Folders are not apps; Explorer windows are not attributed to them.
        Validated::Folder { .. } => {}
    }
    m
}

/// Windows per item id, keeping z-order (most recently used first).
pub fn group(matchers: &[ItemMatcher], windows: &[AppWindow]) -> BTreeMap<String, Vec<isize>> {
    let mut out: BTreeMap<String, Vec<isize>> = BTreeMap::new();
    for m in matchers {
        let hwnds: Vec<isize> = windows.iter().filter(|w| m.matches(w)).map(|w| w.hwnd).collect();
        if !hwnds.is_empty() {
            out.insert(m.id.clone(), hwnds);
        }
    }
    out
}

pub fn counts(map: &BTreeMap<String, Vec<isize>>) -> BTreeMap<String, u32> {
    map.iter().map(|(k, v)| (k.clone(), v.len() as u32)).collect()
}

/// Which window a click on a running app should bring forward: the most
/// recent one, or, when the app is already in front, the next one (cycling).
pub fn window_to_activate(app_windows: &[isize], foreground: isize) -> Option<isize> {
    match app_windows.iter().position(|&h| h == foreground) {
        None => app_windows.first().copied(),
        Some(_) if app_windows.len() > 1 => app_windows.last().copied(),
        Some(_) => None, // already in front, single window: nothing to do
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(hwnd: isize, aumid: Option<&str>, exe: Option<&str>) -> AppWindow {
        AppWindow { hwnd, aumid: aumid.map(Into::into), exe: exe.map(Into::into) }
    }

    #[test]
    fn groups_by_aumid_and_exe_in_z_order() {
        let matchers = vec![
            ItemMatcher { id: "edge".into(), aumids: vec!["msedge".into()], exes: vec![] },
            ItemMatcher { id: "code".into(), aumids: vec![], exes: vec![r"c:\apps\code.exe".into()] },
            ItemMatcher { id: "downloads".into(), aumids: vec![], exes: vec![] },
        ];
        let windows = vec![
            win(1, Some("msedge"), Some(r"c:\edge\msedge.exe")),
            win(2, None, Some(r"c:\apps\code.exe")),
            win(3, Some("msedge"), Some(r"c:\edge\msedge.exe")),
            win(4, Some("chrome"), None),
        ];
        let g = group(&matchers, &windows);
        assert_eq!(g.get("edge"), Some(&vec![1, 3]));
        assert_eq!(g.get("code"), Some(&vec![2]));
        assert!(!g.contains_key("downloads"));
        assert_eq!(counts(&g).get("edge"), Some(&2));
    }

    #[test]
    fn edge_pwa_is_not_edge() {
        let edge = ItemMatcher { id: "edge".into(), aumids: vec!["msedge".into()], exes: vec![] };
        assert!(!edge.matches(&win(9, Some("msedge._crx_abcdef"), Some(r"c:\edge\msedge_proxy.exe"))));
    }

    #[test]
    fn click_activation_cycles_windows() {
        assert_eq!(window_to_activate(&[10, 20, 30], 99), Some(10)); // app in background: most recent
        assert_eq!(window_to_activate(&[10, 20, 30], 10), Some(30)); // app in front: cycle
        assert_eq!(window_to_activate(&[10], 10), None); // single window already in front
        assert_eq!(window_to_activate(&[], 10), None);
    }
}
