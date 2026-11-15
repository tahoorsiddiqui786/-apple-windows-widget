//! One dock per user session, via a session-local named mutex. A second launch
//! asks the running instance to show its dock and exits.

use std::sync::OnceLock;

use windows::core::w;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HANDLE, LPARAM, WPARAM};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW};

use super::backdrop::{CLASS_NAME, WM_APP_SHOW_DOCK};

struct Held(#[allow(dead_code)] HANDLE);
// The handle is only kept alive for the process lifetime, never used.
unsafe impl Send for Held {}
unsafe impl Sync for Held {}

static MUTEX: OnceLock<Held> = OnceLock::new();

/// Returns false if another instance already owns the dock.
pub fn acquire() -> bool {
    unsafe {
        match CreateMutexW(None, false, w!("Local\\GlassDesktop.Shell.SingleInstance")) {
            Ok(handle) => {
                let already = GetLastError() == ERROR_ALREADY_EXISTS;
                let _ = MUTEX.set(Held(handle));
                !already
            }
            // If the mutex cannot be created at all, run anyway rather than never start.
            Err(_) => true,
        }
    }
}

pub fn signal_existing() {
    unsafe {
        if let Ok(hwnd) = FindWindowW(CLASS_NAME, None) {
            let _ = PostMessageW(Some(hwnd), WM_APP_SHOW_DOCK, WPARAM(0), LPARAM(0));
        }
    }
}
