# Assets

Project design assets: your wallpapers, icons and widget art. Nothing in
here is generated placeholder stock art.

```
assets/
  brand/        app identity (source of the app/tray icon)
  icons/        dock icons
  wallpapers/   desktop wallpapers
  widgets/      widget artwork (future)
  ui/           other UI pieces (future)
```

## Naming convention

All lowercase, words joined with `-`, no spaces.

| Folder | Pattern | Example |
|---|---|---|
| `icons/` | `app-<app-name>.<png\|svg\|ico>` | `app-spotify.png`, `app-vscode.svg` |
| `icons/` (variants) | `app-<app-name>-<variant>.<ext>` | `app-terminal-dark.png` |
| `wallpapers/` | `wallpaper-<theme>-<variant>-<width>x<height>.<jpg\|png\|webp>` | `wallpaper-nebula-dark-3840x2160.jpg` |
| `widgets/` | `widget-<widget>-<part>.<png\|svg>` | `widget-clock-face.svg` |
| `ui/` | `ui-<component>-<state>.<png\|svg>` | `ui-dock-shelf-glow.png` |
| `brand/` | `glass-desktop-<thing>.<svg\|png>` | `glass-desktop-logo.svg` |

### Recommended formats

* **Icons:** square, transparent background, **512×512 PNG** (or SVG). Keep
  ~6 % padding inside the canvas so the dock's soft shadow isn't clipped.
* **Wallpapers:** your screen's native resolution or larger (e.g.
  3840×2160), JPG quality 90 or PNG. Dark, low-contrast centres make the glass
  read best.

## Icons saved as JPEG (no transparency)

Images sent through WhatsApp or saved as JPEG lose transparency: the glass
tile sits on a white or black square. Keep those JPEGs in
`icons/originals/` and cut them into transparent PNGs:

```
powershell -ExecutionPolicy Bypass -File tools\icons\cut-all.ps1 -Icons assets\icons\originals -OutDir assets\icons
```

`icons/` then holds 512×512 PNGs with the same names. `icons/_duplicates/`
holds byte-identical copies found during import; they're safe to delete.

## Using your icons in the dock

The running app reads icons from its **assets folder** (tray › *Open Assets
Folder*), by default `%APPDATA%\com.glassdesktop.shell\assets\icons\`. Either:

1. copy files there and reference them by name: `"icon": "app-spotify.png"`, or
2. point the app at this folder in `config.json`:
   `"paths": { "assetsDir": "C:\\Users\\<you>\\Desktop\\projects\\apple-windows-widget\\assets" }`.

Wallpaper management arrives in a later milestone and will use
`<assetsDir>\wallpapers\` the same way.
