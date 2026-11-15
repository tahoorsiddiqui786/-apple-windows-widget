use std::path::Path;

use windows::core::{Result, GUID, HSTRING};
use windows::Win32::Storage::EnhancedStorage::{PKEY_AppUserModel_ID, PKEY_Link_TargetParsingPath};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreFromParsingName, GPS_DEFAULT};
use windows::Win32::UI::Shell::{
    FOLDERID_AppsFolder, IShellItem, SHCreateItemFromParsingName, SHCreateItemInKnownFolder, SHGetKnownFolderPath,
    KF_FLAG_DEFAULT,
};

/// Resolves an AppUserModelID inside the virtual Apps folder (what the Start
/// menu lists). Fails for anything that is not an installed, registered app.
pub fn apps_folder_item(aumid: &str) -> Result<IShellItem> {
    unsafe { SHCreateItemInKnownFolder(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, &HSTRING::from(aumid)) }
}

pub fn path_item(path: &Path) -> Result<IShellItem> {
    unsafe { SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None) }
}

/// Reads a string property; empty values count as missing.
pub fn store_string(store: &IPropertyStore, key: &PROPERTYKEY) -> Option<String> {
    unsafe {
        let mut value = store.GetValue(key).ok()?;
        let text = PropVariantToStringAlloc(&value).ok().map(|p| {
            let s = p.to_string().unwrap_or_default();
            CoTaskMemFree(Some(p.0 as *const core::ffi::c_void));
            s
        });
        let _ = PropVariantClear(&mut value);
        text.filter(|s| !s.is_empty())
    }
}

/// AppUserModelID and target executable stored in a `.lnk`. Needs COM.
pub fn shortcut_identity(lnk: &Path) -> (Option<String>, Option<String>) {
    let store: Option<IPropertyStore> =
        unsafe { SHGetPropertyStoreFromParsingName(&HSTRING::from(lnk.as_os_str()), None, GPS_DEFAULT).ok() };
    match store {
        Some(s) => (store_string(&s, &PKEY_AppUserModel_ID), store_string(&s, &PKEY_Link_TargetParsingPath)),
        None => (None, None),
    }
}

/// Expands Start-menu IDs of the form `{KNOWNFOLDERID}\relative\app.exe`
/// into a real path, e.g. `C:\Program Files\Microsoft VS Code\Code.exe`.
pub fn known_folder_app_path(aumid: &str) -> Option<String> {
    let rest = aumid.strip_prefix('{')?;
    let (guid, relative) = rest.split_once("}\\")?;
    let guid = GUID::try_from(guid).ok()?;
    unsafe {
        let base = SHGetKnownFolderPath(&guid, KF_FLAG_DEFAULT, None).ok()?;
        let path = base.to_string().ok();
        CoTaskMemFree(Some(base.0 as *const core::ffi::c_void));
        path.map(|p| format!("{p}\\{relative}"))
    }
}
