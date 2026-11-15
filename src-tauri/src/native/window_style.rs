//! Window-level tweaks applied to the dock's Tauri webview window.

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{CreateRectRgn, CreateRoundRectRgn, SetWindowRgn, HRGN};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWLP_HWNDPARENT, GWL_EXSTYLE, HWND_TOPMOST,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOWNOACTIVATE,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

/// Makes a window "dock-like": it never takes keyboard focus or activation
/// when clicked (so the app you were typing in keeps focus), and it is hidden
/// from Alt+Tab and the taskbar.
pub fn make_non_activating_tool_window(hwnd: HWND) {
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let wanted = ex | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0) as isize;
        if wanted != ex {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, wanted);
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

/// Sets `owner` as the owner of `owned`. Owned windows always stay above their
/// owner, which keeps the dock content above its native blur backdrop.
pub fn set_owner(owned: HWND, owner: HWND) {
    unsafe {
        SetWindowLongPtrW(owned, GWLP_HWNDPARENT, owner.0 as isize);
    }
}

pub fn place_topmost(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, w, h, SWP_NOACTIVATE);
    }
}

/// Moves/resizes without touching z-order or activation.
pub fn move_resize(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        let _ = SetWindowPos(hwnd, None, x, y, w, h, SWP_NOACTIVATE | SWP_NOZORDER);
    }
}

pub fn show_no_activate(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

pub fn hide(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}

/// Restricts where a window is hit-testable and drawn. `None` removes the
/// restriction (whole window).
pub fn set_rect_region(hwnd: HWND, rect: Option<(i32, i32, i32, i32)>) {
    unsafe {
        let region: Option<HRGN> = rect.map(|(l, t, r, b)| CreateRectRgn(l, t, r, b));
        // On success the system owns the region; we must not delete it.
        SetWindowRgn(hwnd, region, true);
    }
}

pub fn set_round_rect_region(hwnd: HWND, l: i32, t: i32, r: i32, b: i32, radius: i32) {
    unsafe {
        let d = (radius * 2).max(0);
        // CreateRoundRectRgn excludes the right/bottom edge, hence +1.
        let region = CreateRoundRectRgn(l, t, r + 1, b + 1, d, d);
        SetWindowRgn(hwnd, Some(region), true);
    }
}
