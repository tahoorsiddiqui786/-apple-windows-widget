//! Small runtime state the app persists by itself (separate from config.json).

use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

const STATE_FILE_NAME: &str = "state.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RuntimeState {
    /// The user hid the dock (tray, hotkey or context menu).
    pub dock_hidden: bool,
}

pub fn load(config_dir: &Path) -> RuntimeState {
    fs::read_to_string(config_dir.join(STATE_FILE_NAME))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(config_dir: &Path, state: &RuntimeState) -> std::io::Result<()> {
    fs::create_dir_all(config_dir)?;
    let tmp = config_dir.join(format!("{STATE_FILE_NAME}.tmp"));
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, config_dir.join(STATE_FILE_NAME))
}
