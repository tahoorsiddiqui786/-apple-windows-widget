//! Fullscreen detection without polling and without reading other windows.
//!
//! The backdrop window registers as an AppBar *without* reserving any screen
//! space (it never sends ABM_SETPOS), purely so the shell sends it
//! `ABN_FULLSCREENAPP` when a fullscreen app (game, video, slideshow) opens or
//! closes. Before hiding, `SHQueryUserNotificationState` confirms that Windows
//! itself considers the user busy in fullscreen. No window titles, process
//! names or rectangles of other apps are ever read.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    SHAppBarMessage, SHQueryUserNotificationState, ABM_NEW, ABM_REMOVE, APPBARDATA, QUNS_BUSY,
    QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
};

use super::backdrop::WM_APP_APPBAR;

pub fn register(hwnd: HWND) -> bool {
    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uCallbackMessage: WM_APP_APPBAR,
        ..Default::default()
    };
    unsafe { SHAppBarMessage(ABM_NEW, &mut data) != 0 }
}

pub fn unregister(hwnd: HWND) {
    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        ..Default::default()
    };
    unsafe {
        SHAppBarMessage(ABM_REMOVE, &mut data);
    }
}

/// True when Windows reports a fullscreen app, D3D game or presentation mode.
pub fn user_is_in_fullscreen() -> bool {
    match unsafe { SHQueryUserNotificationState() } {
        Ok(state) => state == QUNS_BUSY || state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE,
        Err(_) => false,
    }
}
