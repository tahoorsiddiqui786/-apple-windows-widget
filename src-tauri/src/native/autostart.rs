//! Optional "launch at sign-in" using the per-user Run key
//! (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`). No admin rights,
//! no scheduled task, no service. Off by default; toggled from the tray.

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE_NAME: PCWSTR = w!("Glass Desktop");

fn open_run_key(write: bool) -> Option<HKEY> {
    let mut key = HKEY::default();
    let access = if write { KEY_SET_VALUE | KEY_QUERY_VALUE } else { KEY_QUERY_VALUE };
    let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, Some(0), access, &mut key) };
    (status == ERROR_SUCCESS).then_some(key)
}

pub fn is_enabled() -> bool {
    let Some(key) = open_run_key(false) else { return false };
    let status = unsafe { RegQueryValueExW(key, VALUE_NAME, None, None, None, None) };
    unsafe {
        let _ = RegCloseKey(key);
    }
    status == ERROR_SUCCESS
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let key = open_run_key(true).ok_or("Could not open the per-user Run key.")?;
    let status = if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let command = format!("\"{}\"", exe.display());
        let wide: Vec<u16> = command.encode_utf16().chain(Some(0)).collect();
        let bytes = unsafe { std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2) };
        unsafe { RegSetValueExW(key, VALUE_NAME, None, REG_SZ, Some(bytes)) }
    } else {
        unsafe { RegDeleteValueW(key, VALUE_NAME) }
    };
    unsafe {
        let _ = RegCloseKey(key);
    }
    if status == ERROR_SUCCESS || (!enabled && status.0 == 2 /* ERROR_FILE_NOT_FOUND */) {
        Ok(())
    } else {
        Err(format!("Registry update failed (error {}).", status.0))
    }
}
