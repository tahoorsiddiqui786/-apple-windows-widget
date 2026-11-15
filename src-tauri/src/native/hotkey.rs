//! Global show/hide shortcut via `RegisterHotKey`. This only receives the one
//! registered key combination; it is not a keyboard hook and sees no other keys.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
};

pub const TOGGLE_DOCK_ID: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotkey {
    pub modifiers: u32,
    pub vk: u32,
}

/// Parses strings like `Ctrl+Alt+D`, `Win+Shift+F2`, `Ctrl+Alt+Space`.
/// At least one modifier is required so a bare key can never be captured.
pub fn parse(spec: &str) -> Result<Option<Hotkey>, String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Ok(None);
    }
    let mut modifiers = 0u32;
    let mut vk = None;
    for part in spec.split('+').map(str::trim) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= MOD_CONTROL.0,
            "alt" => modifiers |= MOD_ALT.0,
            "shift" => modifiers |= MOD_SHIFT.0,
            "win" | "super" | "meta" => modifiers |= MOD_WIN.0,
            key => {
                if vk.is_some() {
                    return Err(format!("Hotkey \"{spec}\" has more than one key."));
                }
                vk = Some(parse_key(key).ok_or_else(|| format!("Unknown key \"{part}\" in hotkey \"{spec}\"."))?);
            }
        }
    }
    let vk = vk.ok_or_else(|| format!("Hotkey \"{spec}\" has no key."))?;
    if modifiers == 0 {
        return Err(format!("Hotkey \"{spec}\" needs at least one of Ctrl, Alt, Shift or Win."));
    }
    Ok(Some(Hotkey { modifiers, vk }))
}

fn parse_key(key: &str) -> Option<u32> {
    let upper = key.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    if bytes.len() == 1 && (bytes[0].is_ascii_uppercase() || bytes[0].is_ascii_digit()) {
        return Some(bytes[0] as u32); // VK codes for A–Z / 0–9 match ASCII
    }
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
        if (1..=24).contains(&n) {
            return Some(0x70 + n - 1); // VK_F1..VK_F24
        }
    }
    match upper.as_str() {
        "SPACE" => Some(0x20),
        "TAB" => Some(0x09),
        "HOME" => Some(0x24),
        "END" => Some(0x23),
        "INSERT" => Some(0x2D),
        "PAGEUP" => Some(0x21),
        "PAGEDOWN" => Some(0x22),
        _ => None,
    }
}

pub fn register(hwnd: HWND, id: i32, hotkey: Hotkey) -> Result<(), String> {
    unsafe {
        RegisterHotKey(Some(hwnd), id, HOT_KEY_MODIFIERS(hotkey.modifiers | MOD_NOREPEAT.0), hotkey.vk)
            .map_err(|_| "That shortcut is already used by another app.".to_string())
    }
}

pub fn unregister(hwnd: HWND, id: i32) {
    unsafe {
        let _ = UnregisterHotKey(Some(hwnd), id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_specs() {
        let hk = parse("Ctrl+Alt+D").unwrap().unwrap();
        assert_eq!(hk.vk, 'D' as u32);
        assert_eq!(hk.modifiers, MOD_CONTROL.0 | MOD_ALT.0);
        assert_eq!(parse("win + shift + f2").unwrap().unwrap().vk, 0x71);
        assert_eq!(parse("").unwrap(), None);
    }

    #[test]
    fn rejects_bare_keys_and_garbage() {
        assert!(parse("D").is_err());
        assert!(parse("Ctrl+Alt").is_err());
        assert!(parse("Ctrl+D+E").is_err());
        assert!(parse("Ctrl+Banana").is_err());
    }
}
