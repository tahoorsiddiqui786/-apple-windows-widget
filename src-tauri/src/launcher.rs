//! Safe application launching.
//!
//! Security model:
//! * The webview never sends a path or command. It sends a dock item *id*;
//!   the target is looked up in the config that Rust loaded from disk.
//! * Every target is validated before use: absolute local paths only (no
//!   UNC/network or device paths), an extension allow-list per kind, and the
//!   file/folder must exist. App IDs must resolve inside the Windows Apps folder.
//! * Launching uses `ShellExecuteExW` with the `open` verb, exactly what
//!   Explorer does on double-click. No `cmd.exe`, no PowerShell, no shell
//!   string parsing. `.bat`/`.cmd`/scripts are not launchable targets.

use std::path::{Component, Path, PathBuf, Prefix};

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::UI::Shell::{
    ILCreateFromPathW, ILFree, SHOpenFolderAndSelectItems, ShellExecuteExW, SEE_MASK_FLAG_NO_UI,
    SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY, SW_SHOWNORMAL};

use crate::config::LaunchTarget;
use crate::native::{com::ComGuard, shell_item};

const MAX_ARGS_LEN: usize = 4096;

/// A launch target that passed validation and is safe to hand to the shell.
#[derive(Debug, Clone, PartialEq)]
pub enum Validated {
    App { aumid: String },
    Exe { path: PathBuf, params: String },
    Shortcut { path: PathBuf },
    Folder { path: PathBuf },
}

impl Validated {
    /// Filesystem location for "Show in File Explorer" (apps have none).
    pub fn reveal_path(&self) -> Option<&Path> {
        match self {
            Validated::App { .. } => None,
            Validated::Exe { path, .. } | Validated::Shortcut { path } | Validated::Folder { path } => {
                Some(path)
            }
        }
    }
}

/// Structural + filesystem validation. Needs COM on the calling thread for
/// `App` targets (see [`ComGuard`]).
pub fn validate(target: &LaunchTarget) -> Result<Validated, String> {
    match target {
        LaunchTarget::App { aumid } => {
            let aumid = validate_aumid(aumid)?;
            shell_item::apps_folder_item(&aumid)
                .map_err(|_| format!("No installed app with ID \"{aumid}\" (see Get-StartApps)."))?;
            Ok(Validated::App { aumid })
        }
        LaunchTarget::Exe { path, args } => {
            let path = validate_local_path(path)?;
            require_extension(&path, &["exe"])?;
            require_file(&path)?;
            let params = build_params(args)?;
            Ok(Validated::Exe { path, params })
        }
        LaunchTarget::Shortcut { path } => {
            let path = validate_local_path(path)?;
            require_extension(&path, &["lnk"])?;
            require_file(&path)?;
            Ok(Validated::Shortcut { path })
        }
        LaunchTarget::Folder { path } => {
            let path = validate_local_path(path)?;
            if !path.is_dir() {
                return Err(format!("Folder not found: {}", path.display()));
            }
            Ok(Validated::Folder { path })
        }
    }
}

/// Opens a validated target. Blocking; call from a worker thread.
pub fn launch(target: &Validated) -> Result<(), String> {
    let _com = ComGuard::new();
    // The dock never takes focus, but it did receive the click, so it may pass
    // its foreground right on to the app it starts.
    unsafe {
        let _ = AllowSetForegroundWindow(ASFW_ANY);
    }
    match target {
        Validated::App { aumid } => shell_open(&format!("shell:AppsFolder\\{aumid}"), None, None),
        Validated::Exe { path, params } => {
            let dir = path.parent().map(|p| p.to_string_lossy().into_owned());
            let params = if params.is_empty() { None } else { Some(params.as_str()) };
            shell_open(&path.to_string_lossy(), params, dir.as_deref())
        }
        Validated::Shortcut { path } | Validated::Folder { path } => {
            shell_open(&path.to_string_lossy(), None, None)
        }
    }
}

/// Opens File Explorer with the target selected. Blocking.
pub fn reveal(path: &Path) -> Result<(), String> {
    let _com = ComGuard::new();
    unsafe {
        let _ = AllowSetForegroundWindow(ASFW_ANY);
        let pidl = ILCreateFromPathW(&HSTRING::from(path.as_os_str()));
        if pidl.is_null() {
            return Err(format!("Could not locate {}", path.display()));
        }
        let result = SHOpenFolderAndSelectItems(pidl, None, 0);
        ILFree(Some(pidl));
        result.map_err(|e| format!("Could not open File Explorer: {}", e.message()))
    }
}

/// Opens a file that belongs to this app (e.g. config.json) in the user's
/// default editor, falling back to Notepad when no editor is associated.
pub fn open_own_file(path: &Path) -> Result<(), String> {
    let _com = ComGuard::new();
    unsafe {
        let _ = AllowSetForegroundWindow(ASFW_ANY);
    }
    let file = path.to_string_lossy();
    shell_open(&file, None, None).or_else(|_| {
        let quoted = quote_arg(&file);
        shell_open("notepad.exe", Some(&quoted), None)
    })
}

fn shell_open(file: &str, params: Option<&str>, dir: Option<&str>) -> Result<(), String> {
    let verb = HSTRING::from("open");
    let file_w = HSTRING::from(file);
    let params_w = params.map(HSTRING::from);
    let dir_w = dir.map(HSTRING::from);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file_w.as_ptr()),
        lpParameters: params_w.as_ref().map_or(PCWSTR::null(), |p| PCWSTR(p.as_ptr())),
        lpDirectory: dir_w.as_ref().map_or(PCWSTR::null(), |d| PCWSTR(d.as_ptr())),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    unsafe { ShellExecuteExW(&mut info) }.map_err(|e| e.message().to_string())
}

fn validate_aumid(raw: &str) -> Result<String, String> {
    let aumid = raw.trim();
    let ok = !aumid.is_empty()
        && aumid.len() <= 512
        && !aumid.chars().any(|c| c.is_control() || matches!(c, '"' | ':' | '<' | '>' | '|' | '*' | '?'))
        && !aumid.starts_with('\\')
        && !aumid.split(['\\', '/']).any(|part| part == "..");
    if ok {
        Ok(aumid.to_string())
    } else {
        Err(format!("\"{aumid}\" is not a valid app ID."))
    }
}

/// Expands a small allow-list of environment variables, then requires an
/// absolute, drive-letter path. UNC (`\\server\share`), device (`\\.\`) and
/// verbatim (`\\?\`) paths are rejected so the dock never touches the network.
pub fn validate_local_path(raw: &str) -> Result<PathBuf, String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.contains('\0') {
        return Err("Path is empty or invalid.".into());
    }
    let expanded = expand_env(raw)?;
    if expanded.starts_with("\\\\") || expanded.starts_with("//") {
        return Err("Network and device paths are not allowed.".into());
    }
    let path = PathBuf::from(&expanded);
    let mut components = path.components();
    match components.next() {
        Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_)) => {}
        _ => return Err(format!("Path must be an absolute local path like C:\\...: {expanded}")),
    }
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(format!("Path must be absolute and must not contain '..': {expanded}"));
    }
    Ok(path)
}

fn expand_env(raw: &str) -> Result<String, String> {
    const ALLOWED: &[&str] = &[
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "PROGRAMFILES",
        "PROGRAMFILES(X86)",
        "PROGRAMDATA",
        "SYSTEMROOT",
        "WINDIR",
        "PUBLIC",
    ];
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            return Err("Unbalanced % in path.".into());
        };
        let name = &after[..end];
        let upper = name.to_ascii_uppercase();
        if !ALLOWED.contains(&upper.as_str()) {
            return Err(format!("%{name}% is not an allowed variable in paths."));
        }
        let value = std::env::var(&upper).map_err(|_| format!("%{name}% is not set."))?;
        out.push_str(&value);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

fn require_extension(path: &Path, allowed: &[&str]) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    if allowed.contains(&ext.as_str()) {
        Ok(())
    } else {
        Err(format!("Only {} files can be used here: {}", allowed.join("/"), path.display()))
    }
}

fn require_file(path: &Path) -> Result<(), String> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("File not found: {}", path.display()))
    }
}

fn build_params(args: &[String]) -> Result<String, String> {
    if args.iter().any(|a| a.contains('\0')) {
        return Err("Arguments must not contain NUL characters.".into());
    }
    let params = args.iter().map(|a| quote_arg(a)).collect::<Vec<_>>().join(" ");
    if params.len() > MAX_ARGS_LEN {
        return Err("Arguments are too long.".into());
    }
    Ok(params)
}

/// Quotes one argument following the MSVC `CommandLineToArgvW` rules, so the
/// target receives exactly the configured string as one argv entry.
pub fn quote_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '\n', '\u{0b}', '"']) {
        return arg.to_string();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut backslashes = 0usize;
    for c in arg.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.extend(std::iter::repeat_n('\\', backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_matches_msvc_rules() {
        assert_eq!(quote_arg("simple"), "simple");
        assert_eq!(quote_arg(""), "\"\"");
        assert_eq!(quote_arg("has space"), "\"has space\"");
        assert_eq!(quote_arg(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(quote_arg(r"C:\dir with space\"), r#""C:\dir with space\\""#);
        assert_eq!(quote_arg(r"C:\no\space"), r"C:\no\space");
    }

    #[test]
    fn rejects_network_relative_and_traversal_paths() {
        assert!(validate_local_path(r"\\server\share\app.exe").is_err());
        assert!(validate_local_path(r"\\?\C:\Windows\notepad.exe").is_err());
        assert!(validate_local_path(r"\\.\PhysicalDrive0").is_err());
        assert!(validate_local_path("//server/share").is_err());
        assert!(validate_local_path(r"relative\app.exe").is_err());
        assert!(validate_local_path(r"C:relative.exe").is_err());
        assert!(validate_local_path(r"C:\Windows\..\Windows\notepad.exe").is_err());
        assert!(validate_local_path(r"C:\Windows\notepad.exe").is_ok());
    }

    #[test]
    fn env_expansion_is_allow_listed() {
        assert!(validate_local_path(r"%WINDIR%\explorer.exe").is_ok());
        assert!(validate_local_path(r"%PATH%\x.exe").is_err());
        assert!(validate_local_path(r"%WINDIR\x.exe").is_err());
    }

    #[test]
    fn exe_targets_must_be_exe_files() {
        let windir = std::env::var("WINDIR").unwrap();
        let ok = validate(&LaunchTarget::Exe { path: format!("{windir}\\explorer.exe"), args: vec![] });
        assert!(ok.is_ok());
        let script = validate(&LaunchTarget::Exe { path: format!("{windir}\\win.ini"), args: vec![] });
        assert!(script.is_err());
        let missing = validate(&LaunchTarget::Exe { path: r"C:\nope\nope.exe".into(), args: vec![] });
        assert!(missing.is_err());
    }

    #[test]
    fn aumid_shape_checks() {
        assert!(validate_aumid("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App").is_ok());
        assert!(validate_aumid(r"{6D809377-6AF0-444B-8957-A3773F02200E}\Microsoft VS Code\Code.exe").is_ok());
        assert!(validate_aumid(r"..\..\cmd.exe").is_err());
        assert!(validate_aumid(r"C:\Windows\System32\cmd.exe").is_err());
        assert!(validate_aumid("x\" & calc").is_err());
        assert!(validate_aumid("").is_err());
    }

    #[test]
    fn installed_apps_resolve() {
        let _com = ComGuard::new();
        assert!(validate(&LaunchTarget::App { aumid: "Microsoft.Windows.Explorer".into() }).is_ok());
        assert!(validate(&LaunchTarget::App { aumid: "Definitely.Not.Installed_123!App".into() }).is_err());
    }
}
