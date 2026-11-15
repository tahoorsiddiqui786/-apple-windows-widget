# Glass Desktop

A local-first, macOS-inspired **dark frosted-glass** layer for Windows 11,
built with Tauri 2, React, TypeScript and Rust. It sits alongside Windows: the
taskbar, Explorer and your workflow stay exactly as they are.

**Milestone 1: the dock.** A floating dark-glass dock at the bottom centre of
your screen, with macOS-style magnification, real app icons or your own,
click-to-launch, a right-click menu, a tray menu and a local JSON config.

> **Status (2026-11-16): running and verified on the development PC.**
> 8 UI tests and 23 Rust tests pass. Measured live on Windows 11 26H2:
> * magnification, labels and the launch bounce work;
> * click-to-launch works, and the launched app takes focus while the dock
>   never does;
> * empty space around the dock passes clicks through to the app behind it;
> * idle CPU is about 0.01 % of the whole CPU.
>
> On that PC Windows doesn't render DWM blur, so the dock draws its own glass
> by default. See [docs/NATIVE_BLUR.md](docs/NATIVE_BLUR.md).

## Features

* Dark glass drawn by the dock itself (tint, highlights, hairline edge, accent
  glow), crisp at any corner radius. Native DWM acrylic or blur is available as
  an opt-in where Windows renders it. See [docs/NATIVE_BLUR.md](docs/NATIVE_BLUR.md).
* Hover magnification that keeps the icon under your cursor in place. The
  animation runs only while moving: zero CPU at rest.
* Click to launch, with a bounce. A launch error appears above the icon.
* Right-click menu: Open, Show in File Explorer, Hide Dock, Edit Dock Settings.
* Never steals focus; clicks outside the glass go straight to the apps below.
* Steps aside for fullscreen games, videos and presentations.
* Tray menu: show/hide (or `Ctrl+Alt+D`), edit/reload config, open assets
  folder, launch at sign-in (off by default), quit.
* Custom PNG/SVG/ICO/JPG/WebP icons, or each app's real Windows icon.
* Notification badges (set per item in config for now).
* Fully configurable: size, spacing, magnification, opacity, tint, radius,
  accent, animation speed. See [docs/CONFIGURATION.md](docs/CONFIGURATION.md).

## Privacy

No telemetry, no analytics, no accounts, no admin rights, no shell, and no
network code in the app. The webview can call only 8 narrowly scoped
commands. One caveat, measured and documented: Microsoft's WebView2 runtime,
which renders the UI, keeps one connection to Microsoft at start-up, even with
its background services switched off. Details:
[docs/PRIVACY.md](docs/PRIVACY.md).

## Requirements

| Tool | Version used | Install |
|---|---|---|
| Windows 11 | 26H2 (build 26300) | – |
| WebView2 Runtime | 157 | ships with Windows 11 |
| Node.js | 26 LTS | `winget install OpenJS.NodeJS.LTS` |
| Rust (MSVC toolchain) | 1.100 stable | `winget install Rustlang.Rustup` |
| VS Build Tools: C++ workload + Windows SDK | 2026 / SDK 10.0.26100 | `winget install Microsoft.VisualStudio.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"` |

## Development

```bash
npm install
```

```bash
npm run app:dev
```

`app:dev` starts Vite on `127.0.0.1:1420` (localhost only) and the native dock
with hot reload.

To design the UI without the native shell, open the browser preview. It uses
a mock backend and CSS blur:

```bash
npm run dev
```

Checks:

```bash
npm run typecheck
```

```bash
npm test
```

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Production build

```bash
npm run app:build
```

This produces a per-user NSIS installer (no admin rights) in
`src-tauri/target/release/bundle/nsis/`, plus the portable
`src-tauri/target/release/glass-desktop.exe`.

The build is **unsigned**. With Smart App Control on, Windows will block it.
To run it with Smart App Control on, sign it with a code-signing certificate
from a trusted CA (for example Azure Trusted Signing) and set
`bundle.windows.signCommand` in `src-tauri/tauri.conf.json`.

## Using the dock

* **Hover** to magnify, **click** to open, **right-click** for the menu.
* **Tray icon:** left-click toggles the dock; right-click shows the menu.
* **Your icons:** tray › *Open Assets Folder* › `icons\`, then set
  `"icon": "app-name.png"` on an item and choose *Reload Configuration*.
  Naming conventions: [assets/README.md](assets/README.md).

## Project layout

```
src/                     React UI (dock, theme, animation, typed IPC)
src-tauri/src/           Rust core (config, layout, dock, launcher, icons, native/, tray)
src-tauri/capabilities/  the dock window's permission set
assets/                  your wallpapers, icons and brand art
docs/                    architecture, native blur, configuration, privacy
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Troubleshooting

**`cargo` fails with "An Application Control policy has blocked this file (os
error 4551)".** Windows Smart App Control blocks newly compiled, unsigned
programs, which includes every Rust build script. Check its state:

```bash
powershell -NoProfile -Command "(Get-MpComputerStatus).SmartAppControlState"
```

To develop locally, turn it off yourself under *Windows Security › App &
browser control › Smart App Control*. On current Windows 11 builds it can be
switched back on later from the same page. Microsoft Defender antivirus keeps
running either way.

**Native blur (`"acrylic"`/`"blur"`) shows a solid bar with square corners.**
Your Windows build or graphics driver isn't rendering DWM blur. Switch back to
`"backdrop": "none"`. Native blur also turns off automatically when
*Transparency effects* is off (Settings › Personalization › Colors).

**An app doesn't appear.** Hover the tray icon. Warnings list items whose
target couldn't be found. For Store apps, use `Get-StartApps` to find the ID.

## Roadmap

1. ✅ Milestone 1: glass dock
2. Settings window (live preview, drag-to-reorder, add apps from Start menu)
3. Wallpaper manager (`assets/wallpapers`)
4. Widgets: clock, date, CPU/RAM, battery ([src/features/widgets](src/features/widgets/README.md))
5. Dock positions (left/right) and multi-monitor placement
