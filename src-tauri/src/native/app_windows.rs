//! Which of *your dock's* apps have windows open, and switching to them.
//!
//! Approved scope (see docs/PRIVACY.md):
//! * Enumerates top-level app windows the way the taskbar does, and identifies
//!   the app behind each one by its AppUserModelID or executable path, only
//!   to match it against the apps you put in the dock. Nothing else is kept.
//! * Window **titles are never read here** except by [`title`], which the dock
//!   calls only when you open an app's right-click menu. Titles are not stored,
//!   logged or sent anywhere.
//! * Updates are event-driven (shell hook, see backdrop.rs), never polled.

use core::ffi::c_void;

use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
use windows::Win32::Storage::Packaging::Appx::GetApplicationUserModelId;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, SetForegroundWindow, ShowWindow,
    GWL_EXSTYLE, GW_OWNER, SW_RESTORE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

use super::shell_item::store_string;

/// One app window, identified without reading its title.
#[derive(Debug, Clone)]
pub struct AppWindow {
    pub hwnd: isize,
    /// Lower-cased AppUserModelID, if the window/app has one.
    pub aumid: Option<String>,
    /// Lower-cased full path of the process executable.
    pub exe: Option<String>,
}

const SKIP_CLASSES: &[&str] = &["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

struct EnumCtx {
    own_pid: u32,
    out: Vec<AppWindow>,
}

/// App windows in z-order (most recently used first), like the taskbar shows
/// them: visible, not cloaked (other virtual desktops / suspended), unowned,
/// not tool windows. The dock's own windows are skipped.
pub fn enumerate() -> Vec<AppWindow> {
    let mut ctx = EnumCtx { own_pid: std::process::id(), out: Vec::new() };
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut ctx as *mut EnumCtx as isize));
    }
    ctx.out
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut EnumCtx);
    if let Some(w) = inspect(hwnd, ctx.own_pid) {
        ctx.out.push(w);
    }
    BOOL(1)
}

/// True when `hwnd` still exists and is visible (cheap re-check before focusing).
pub fn is_live(hwnd: HWND) -> bool {
    unsafe { IsWindow(Some(hwnd)).as_bool() && IsWindowVisible(hwnd).as_bool() }
}

unsafe fn inspect(hwnd: HWND, own_pid: u32) -> Option<AppWindow> {
    if !IsWindowVisible(hwnd).as_bool() {
        return None;
    }
    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    let app_window = ex & WS_EX_APPWINDOW.0 != 0;
    if ex & WS_EX_TOOLWINDOW.0 != 0 && !app_window {
        return None;
    }
    if !app_window && GetWindow(hwnd, GW_OWNER).is_ok_and(|o| !o.is_invalid()) {
        return None;
    }
    let mut cloaked: u32 = 0;
    let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut c_void, 4);
    if cloaked != 0 {
        return None;
    }
    // Length only: an untitled window is not a taskbar window. The text itself is not read.
    if GetWindowTextLengthW(hwnd) == 0 {
        return None;
    }
    let class = class_name(hwnd);
    if SKIP_CLASSES.contains(&class.as_str()) {
        return None;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == own_pid {
        return None;
    }
    let (exe, packaged_aumid) = process_identity(pid);
    let mut aumid = window_aumid(hwnd).or(packaged_aumid);
    // File Explorer windows carry no AppUserModelID; recognise them by class.
    if aumid.is_none() && class == "CabinetWClass" && exe.as_deref().is_some_and(|e| e.ends_with("\\explorer.exe")) {
        aumid = Some("microsoft.windows.explorer".into());
    }
    Some(AppWindow { hwnd: hwnd.0 as isize, aumid, exe })
}

unsafe fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 128];
    let n = GetClassNameW(hwnd, &mut buf);
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

/// Explicit AppUserModelID set on the window (Edge, Chrome, Store apps' frames…).
unsafe fn window_aumid(hwnd: HWND) -> Option<String> {
    let store: IPropertyStore = SHGetPropertyStoreForWindow(hwnd).ok()?;
    store_string(&store, &PKEY_AppUserModel_ID).map(|s| s.to_lowercase())
}

/// Executable path, plus the package's AppUserModelID for packaged apps
/// (Terminal, Notepad, WhatsApp…). Uses limited query rights only.
unsafe fn process_identity(pid: u32) -> (Option<String>, Option<String>) {
    let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
        return (None, None);
    };
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let exe = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len)
        .ok()
        .map(|_| String::from_utf16_lossy(&buf[..len as usize]).to_lowercase());
    let mut alen = 0u32;
    let mut aumid = None;
    // First call reports the size (ERROR_INSUFFICIENT_BUFFER); non-packaged processes fail.
    let _ = GetApplicationUserModelId(handle, &mut alen, None);
    if alen > 1 && alen < 1024 {
        let mut abuf = vec![0u16; alen as usize];
        if GetApplicationUserModelId(handle, &mut alen, Some(PWSTR(abuf.as_mut_ptr()))).is_ok() {
            let end = abuf.iter().position(|&c| c == 0).unwrap_or(abuf.len());
            aumid = Some(String::from_utf16_lossy(&abuf[..end]).to_lowercase());
        }
    }
    let _ = CloseHandle(handle);
    (exe, aumid)
}

/// Reads one window's title, cleaned for display. Called only on demand.
pub fn title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
    let raw = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
    let is_bidi = |c: char| matches!(c, '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}');
    raw.chars().filter(|&c| !c.is_control() && !is_bidi(c)).take(160).collect::<String>().trim().to_string()
}

pub fn is_minimized(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd).as_bool() }
}

pub fn foreground() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}

/// Restores (if minimized) and brings a window to the front. Allowed because
/// the dock process just received the user's click.
pub fn focus(hwnd: HWND) -> Result<(), String> {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        if SetForegroundWindow(hwnd).as_bool() {
            Ok(())
        } else {
            Err("Windows did not allow switching to that window.".into())
        }
    }
}
