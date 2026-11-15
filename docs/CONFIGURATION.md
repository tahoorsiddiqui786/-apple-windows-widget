# Configuration

Everything lives in one local file:

```
%APPDATA%\com.glassdesktop.shell\config.json
```

Open it from the tray (**Edit Configuration…**) or the dock's right-click menu
(**Edit Dock Settings…**). Save it, then choose **Reload Configuration** in
the tray. Out-of-range values are clamped. A file with a syntax error is left
untouched, and the dock runs on defaults until you fix it. Hover the tray icon
to see whether there are warnings.

The app writes `config.json` only on first run. Things it changes by itself,
such as "dock hidden", go to `state.json` next to it.

## Full example

```jsonc
{
  "version": 1,
  "dock": {
    "position": "bottom",        // bottom-centre of the primary monitor (more positions planned)
    "iconSize": 52,              // 24–128 px
    "iconSpacing": 10,           // 0–48 px
    "padding": 9,                // 2–32 px, glass edge to icons
    "magnification": 1.6,        // 1.0 (off) – 2.2, peak scale under the pointer
    "magnificationRange": 2.6,   // 1–6, how many neighbours the wave reaches
    "marginBottom": 12,          // 0–120 px above the taskbar
    "showLabels": true
  },
  "appearance": {
    "backdrop": "none",          // "none" (dock-drawn glass, default) | "acrylic" | "blur" (native DWM, opt-in, see NATIVE_BLUR.md)
    "glassTint": "#0b0d13",      // #rgb or #rrggbb
    "glassOpacity": 0.42,        // 0–1 tint over the blur
    "borderOpacity": 0.16,       // 0–1 hairline edge
    "cornerRadius": 22,          // 0–48 px
    "accentColor": "#7c8cff",    // glows and highlights
    "glow": true,
    "animationSpeed": 1.0        // 0.25–4, higher = faster
  },
  "behavior": {
    "hideInFullscreen": true,    // step aside for games, videos and slideshows
    "toggleHotkey": "Ctrl+Alt+D" // "" to disable. Modifiers: Ctrl Alt Shift Win
  },
  "paths": {
    "assetsDir": null            // null = %APPDATA%\com.glassdesktop.shell\assets
  },
  "items": [
    { "id": "explorer", "label": "File Explorer",
      "target": { "kind": "app", "aumid": "Microsoft.Windows.Explorer" } },
    { "id": "terminal", "label": "Terminal",
      "target": { "kind": "app", "aumid": "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App" },
      "icon": "app-terminal.png" },
    { "id": "code", "label": "VS Code",
      "target": { "kind": "exe", "path": "%LOCALAPPDATA%\\Programs\\Microsoft VS Code\\Code.exe",
                  "args": ["--new-window"] } },
    { "id": "spotify", "label": "Spotify",
      "target": { "kind": "shortcut", "path": "%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\Spotify.lnk" },
      "badge": "3" },
    { "id": "downloads", "label": "Downloads",
      "target": { "kind": "folder", "path": "%USERPROFILE%\\Downloads" } }
  ]
}
```

`config.json` is plain JSON, so leave out the `//` comments above.

## Dock items

| Field | Notes |
|---|---|
| `id` | Unique, 1–48 characters of `a-z 0-9 -`. |
| `label` | Shown above the icon on hover. |
| `target.kind = "app"` | `aumid`: the app's AppUserModelID. **Best for most apps**, since it launches exactly like the Start menu, including Store apps. List them with PowerShell: `Get-StartApps`. |
| `target.kind = "exe"` | `path` to a `.exe` and optional `args` (array, passed verbatim). |
| `target.kind = "shortcut"` | `path` to a `.lnk`. |
| `target.kind = "folder"` | `path` to a folder; opens in File Explorer. |
| `icon` | Optional. A file name in `<assetsDir>\icons\` (e.g. `app-terminal.png`) or an absolute path. PNG, SVG, ICO, JPG or WebP, ≤ 4 MB, ideally square and 256 px or larger. Without it, the app's own Windows icon is used. |
| `badge` | Optional short text (≤ 4 chars), shown as a red badge. |

Items whose target can't be found are hidden, not deleted, and listed as
warnings.

Allowed path variables: `%USERPROFILE%`, `%APPDATA%`, `%LOCALAPPDATA%`,
`%PROGRAMFILES%`, `%PROGRAMFILES(X86)%`, `%PROGRAMDATA%`, `%SYSTEMROOT%`,
`%WINDIR%`, `%PUBLIC%`. Network paths are not allowed.
