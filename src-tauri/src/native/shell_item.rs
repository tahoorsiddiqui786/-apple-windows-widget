use std::path::Path;

use windows::core::{Result, HSTRING};
use windows::Win32::UI::Shell::{
    FOLDERID_AppsFolder, IShellItem, SHCreateItemFromParsingName, SHCreateItemInKnownFolder, KF_FLAG_DEFAULT,
};

/// Resolves an AppUserModelID inside the virtual Apps folder (what the Start
/// menu lists). Fails for anything that is not an installed, registered app.
pub fn apps_folder_item(aumid: &str) -> Result<IShellItem> {
    unsafe { SHCreateItemInKnownFolder(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, &HSTRING::from(aumid)) }
}

pub fn path_item(path: &Path) -> Result<IShellItem> {
    unsafe { SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None) }
}
