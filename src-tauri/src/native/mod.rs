//! Thin, audited wrappers over the Win32 APIs the dock needs. Everything
//! Windows-specific lives here so the rest of the app stays declarative.

pub mod app_windows;
pub mod appbar;
pub mod autostart;
pub mod backdrop;
pub mod com;
pub mod hotkey;
pub mod monitor;
pub mod shell_icon;
pub mod shell_item;
pub mod single_instance;
pub mod system_prefs;
pub mod window_style;

use windows::Win32::Foundation::HWND;

/// HWNDs are raw pointers and not `Send`; we store them as integers and only
/// turn them back into handles on the thread that uses them.
pub fn hwnd_to_isize(hwnd: HWND) -> isize {
    hwnd.0 as isize
}

pub fn hwnd_from_isize(raw: isize) -> HWND {
    HWND(raw as *mut core::ffi::c_void)
}
