use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};

/// Initialises a single-threaded COM apartment for the current thread and
/// balances it on drop. Shell APIs (ShellExecuteEx, IShellItem) require it.
pub struct ComGuard {
    initialized: bool,
}

impl ComGuard {
    pub fn new() -> Self {
        // S_OK and S_FALSE both need a matching CoUninitialize. RPC_E_CHANGED_MODE
        // (thread already MTA) does not, and the shell calls still work there.
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        Self { initialized: hr.is_ok() }
    }
}

impl Default for ComGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { CoUninitialize() };
        }
    }
}
