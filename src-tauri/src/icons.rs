//! Icon loading. Icons are handed to the webview as `data:` URLs, so the
//! webview needs no filesystem access and no asset protocol at all.

use std::fs;
use std::path::{Component, Path, PathBuf};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::launcher::{self, Validated};
use crate::native::{shell_icon, shell_item};

const MAX_ICON_BYTES: u64 = 4 * 1024 * 1024;
const SHELL_ICON_SIZE: i32 = 256;

/// Loads a user-supplied icon. A bare file name resolves inside
/// `<assets>/icons/`; an absolute local path is also accepted.
pub fn custom_icon_data_url(spec: &str, icons_dir: &Path) -> Result<String, String> {
    let path = resolve_icon_path(spec, icons_dir)?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => return Err(format!("Icon {} must be PNG, SVG, ICO, JPG or WebP.", path.display())),
    };
    let meta = fs::metadata(&path).map_err(|_| format!("Icon not found: {}", path.display()))?;
    if !meta.is_file() {
        return Err(format!("Icon is not a file: {}", path.display()));
    }
    if meta.len() > MAX_ICON_BYTES {
        return Err(format!("Icon is larger than 4 MB: {}", path.display()));
    }
    let bytes = fs::read(&path).map_err(|e| format!("Could not read icon {}: {e}", path.display()))?;
    Ok(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}

fn resolve_icon_path(spec: &str, icons_dir: &Path) -> Result<PathBuf, String> {
    let spec = spec.trim();
    let candidate = Path::new(spec);
    if candidate.is_absolute() || spec.starts_with('\\') || spec.starts_with('/') || spec.contains(':') {
        return launcher::validate_local_path(spec);
    }
    // Relative: plain names (optionally in sub-folders) inside the icons folder only.
    if candidate.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(format!("Icon \"{spec}\" must be a file name inside the icons folder."));
    }
    Ok(icons_dir.join(candidate))
}

/// The app's own Windows icon, as a PNG data URL. Needs COM on this thread.
pub fn shell_icon_data_url(target: &Validated) -> Result<String, String> {
    let item = match target {
        Validated::App { aumid } => shell_item::apps_folder_item(aumid),
        other => shell_item::path_item(other.reveal_path().expect("path targets have a path")),
    }
    .map_err(|e| e.message().to_string())?;
    let rgba = shell_icon::icon_rgba(&item, SHELL_ICON_SIZE)?;
    let png = encode_png(&rgba)?;
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

fn encode_png(img: &shell_icon::Rgba) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, img.width, img.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&img.pixels).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_icon_names_stay_inside_icons_dir() {
        let dir = Path::new(r"C:\icons");
        assert_eq!(resolve_icon_path("app-x.png", dir).unwrap(), PathBuf::from(r"C:\icons\app-x.png"));
        assert_eq!(resolve_icon_path(r"set\app-x.png", dir).unwrap(), PathBuf::from(r"C:\icons\set\app-x.png"));
        assert!(resolve_icon_path(r"..\secret.png", dir).is_err());
        assert!(resolve_icon_path(r"\\server\share\a.png", dir).is_err());
    }

    #[test]
    fn rejects_non_image_extensions() {
        let windir = std::env::var("WINDIR").unwrap();
        let err = custom_icon_data_url(&format!(r"{windir}\win.ini"), Path::new(r"C:\icons")).unwrap_err();
        assert!(err.contains("must be PNG"));
    }
}
