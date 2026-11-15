//! The dock's native glass backdrop.
//!
//! Why a separate window? CSS `backdrop-filter` inside WebView2 only blurs the
//! web page's own pixels, never the desktop behind a transparent window. Real
//! blur has to come from DWM, and DWM applies it to a *whole window*. The dock
//! window is taller than the glass bar (magnified icons, labels and menus need
//! headroom), so blurring it would blur that empty headroom too.
//!
//! Instead, this module creates a tiny, plain Win32 popup (no webview, no IPC)
//! that sits directly behind the dock, receives the DWM blur, and is clipped
//! with a rounded window region to the exact shape of the glass bar. The
//! webview draws the tint, hairline edge and icons on top.
//!
//! Blur source: `SetWindowCompositionAttribute(WCA_ACCENT_POLICY)`. It is
//! undocumented, but it is what Windows' own shell surfaces and most dock /
//! taskbar tools use, and unlike the documented Windows 11 system backdrops
//! (`DWMWA_SYSTEMBACKDROP_TYPE`) it keeps blurring while the window is
//! inactive. A dock never becomes the active window, so the documented
//! Mica/Acrylic backdrops would just show their flat fallback colour.
//! If the call is unavailable, the dock falls back to a tinted, unblurred
//! surface (see docs/NATIVE_BLUR.md).
//!
//! The window also serves as the app's hidden message sink for the global
//! hotkey, the fullscreen-app notification and display changes.

use core::ffi::c_void;
use std::sync::OnceLock;

use windows::core::{s, w, BOOL};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_BORDER_COLOR, DWMWA_NCRENDERING_POLICY,
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWINDOWATTRIBUTE,
};
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, FillRect, GetStockObject, BLACK_BRUSH, HBRUSH, PAINTSTRUCT};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, LoadCursorW, RegisterClassExW, RegisterWindowMessageW,
    CS_HREDRAW, CS_VREDRAW, HTTRANSPARENT, IDC_ARROW, MA_NOACTIVATE, WM_APP, WM_DISPLAYCHANGE, WM_DPICHANGED,
    WM_ERASEBKGND, WM_HOTKEY, WM_MOUSEACTIVATE, WM_NCHITTEST, WM_PAINT, WM_SETTINGCHANGE, WNDCLASSEXW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

pub const CLASS_NAME: windows::core::PCWSTR = w!("GlassDesktop.DockBackdrop");
/// Posted by a second app instance to ask this one to show the dock.
pub const WM_APP_SHOW_DOCK: u32 = WM_APP + 1;
/// AppBar callback (fullscreen notifications).
pub const WM_APP_APPBAR: u32 = WM_APP + 2;

/// Native events forwarded to the app (handled later on the event loop).
#[derive(Debug, Clone, Copy)]
pub enum NativeEvent {
    Hotkey(i32),
    ShowRequested,
    /// `true` when a fullscreen app opened on the monitor, `false` when it closed.
    Fullscreen(bool),
    DisplayChanged,
    ExplorerRestarted,
}

static HANDLER: OnceLock<Box<dyn Fn(NativeEvent) + Send + Sync>> = OnceLock::new();
static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();

/// Installs the callback for [`NativeEvent`]s. Call once before `create`.
pub fn set_event_handler(handler: impl Fn(NativeEvent) + Send + Sync + 'static) {
    let _ = HANDLER.set(Box::new(handler));
}

fn emit(ev: NativeEvent) {
    if let Some(h) = HANDLER.get() {
        h(ev);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlurKind {
    Acrylic,
    Blur,
}

/// Creates the (hidden) backdrop window on the calling thread, which must run
/// a message loop (Tauri's main thread does).
pub fn create() -> windows::core::Result<HWND> {
    unsafe {
        let _ = TASKBAR_CREATED.set(RegisterWindowMessageW(w!("TaskbarCreated")));
        let hinstance = GetModuleHandleW(None)?;
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        };
        // Returns 0 if the class already exists (e.g. after a config reload);
        // CreateWindowExW below reports any real failure.
        RegisterClassExW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            CLASS_NAME,
            w!("Glass Dock Backdrop"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(hinstance.into()),
            None,
        )?;
        // No DWM frame, shadow, border or automatic Windows 11 corner rounding:
        // the window region defines the exact glass shape.
        set_dwm_u32(hwnd, DWMWA_NCRENDERING_POLICY, DWMNCRP_DISABLED.0 as u32);
        set_dwm_u32(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND.0 as u32);
        set_dwm_u32(hwnd, DWMWA_BORDER_COLOR, 0xFFFF_FFFE); // DWMWA_COLOR_NONE
        Ok(hwnd)
    }
}

pub fn destroy(hwnd: HWND) {
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn set_dwm_u32(hwnd: HWND, attr: DWMWINDOWATTRIBUTE, value: u32) {
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, attr, &value as *const u32 as *const c_void, 4);
    }
}

#[repr(C)]
struct AccentPolicy {
    accent_state: u32,
    accent_flags: u32,
    gradient_color: u32,
    animation_id: u32,
}

#[repr(C)]
struct WindowCompositionAttribData {
    attrib: u32,
    pv_data: *mut c_void,
    cb_data: usize,
}

const WCA_ACCENT_POLICY: u32 = 19;
const ACCENT_DISABLED: u32 = 0;
const ACCENT_ENABLE_BLURBEHIND: u32 = 3;
const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;
/// Draw `gradient_color` as a tint over the blur.
const ACCENT_FLAG_GRADIENT: u32 = 2;

type SetWindowCompositionAttributeFn = unsafe extern "system" fn(HWND, *mut WindowCompositionAttribData) -> BOOL;

fn swca() -> Option<SetWindowCompositionAttributeFn> {
    unsafe {
        let user32 = GetModuleHandleW(w!("user32.dll")).ok()?;
        let proc = GetProcAddress(user32, s!("SetWindowCompositionAttribute"))?;
        Some(std::mem::transmute::<unsafe extern "system" fn() -> isize, SetWindowCompositionAttributeFn>(proc))
    }
}

/// Applies DWM blur with a tint. `tint` is RGB, `alpha` 0–255.
/// Returns false when the API is unavailable or refused the call.
pub fn apply_blur(hwnd: HWND, kind: BlurKind, tint: (u8, u8, u8), alpha: u8) -> bool {
    let Some(set_attr) = swca() else { return false };
    let state = match kind {
        BlurKind::Acrylic => ACCENT_ENABLE_ACRYLICBLURBEHIND,
        BlurKind::Blur => ACCENT_ENABLE_BLURBEHIND,
    };
    // Acrylic treats alpha 0 as fully opaque; keep it at least 1.
    let alpha = if kind == BlurKind::Acrylic { alpha.max(1) } else { alpha };
    let abgr = ((alpha as u32) << 24) | ((tint.2 as u32) << 16) | ((tint.1 as u32) << 8) | tint.0 as u32;
    let mut policy = AccentPolicy {
        accent_state: state,
        accent_flags: ACCENT_FLAG_GRADIENT,
        gradient_color: abgr,
        animation_id: 0,
    };
    let mut data = WindowCompositionAttribData {
        attrib: WCA_ACCENT_POLICY,
        pv_data: &mut policy as *mut _ as *mut c_void,
        cb_data: std::mem::size_of::<AccentPolicy>(),
    };
    unsafe { set_attr(hwnd, &mut data).as_bool() }
}

pub fn clear_blur(hwnd: HWND) {
    let Some(set_attr) = swca() else { return };
    let mut policy = AccentPolicy { accent_state: ACCENT_DISABLED, accent_flags: 0, gradient_color: 0, animation_id: 0 };
    let mut data = WindowCompositionAttribData {
        attrib: WCA_ACCENT_POLICY,
        pv_data: &mut policy as *mut _ as *mut c_void,
        cb_data: std::mem::size_of::<AccentPolicy>(),
    };
    unsafe {
        let _ = set_attr(hwnd, &mut data);
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        // Never take input or activation; the dock webview above handles input.
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            // Paint pure black: in a DWM-composed window GDI black has zero
            // alpha, so the accent blur shows through untouched.
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            FillRect(hdc, &ps.rcPaint, HBRUSH(GetStockObject(BLACK_BRUSH).0));
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_HOTKEY => {
            emit(NativeEvent::Hotkey(wparam.0 as i32));
            LRESULT(0)
        }
        WM_APP_SHOW_DOCK => {
            emit(NativeEvent::ShowRequested);
            LRESULT(0)
        }
        WM_APP_APPBAR => {
            const ABN_FULLSCREENAPP: usize = 2;
            if wparam.0 == ABN_FULLSCREENAPP {
                emit(NativeEvent::Fullscreen(lparam.0 != 0));
            }
            LRESULT(0)
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED => {
            emit(NativeEvent::DisplayChanged);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_SETTINGCHANGE => {
            // SPI_SETWORKAREA (taskbar moved/resized) or theme/transparency changes.
            emit(NativeEvent::DisplayChanged);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ if Some(&msg) == TASKBAR_CREATED.get() => {
            emit(NativeEvent::ExplorerRestarted);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
