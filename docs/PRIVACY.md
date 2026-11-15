# Privacy, security and permissions

Glass Desktop is **local-first**. It has no servers, accounts, analytics,
telemetry, crash reporting, ads or auto-updater.

## What the app does *not* do

* **The app's own code makes no network requests.** The Rust side links no
  HTTP client (check with `cargo tree -e normal | findstr /i "reqwest hyper ureq curl"`:
  no results). The webview's CSP sets `connect-src` to Tauri's local IPC
  only. See [the WebView2 runtime](#the-webview2-runtime-measured) for the one
  exception the app doesn't control.
* No remote scripts, fonts or images. Everything is bundled. The CSP allows
  only `'self'` scripts, plus `data:` images for icons.
* It does not read window titles, browsing history, running processes,
  other apps' windows, the clipboard or keystrokes.
* No background services, scheduled tasks, drivers or elevated helpers. It
  never asks for administrator rights.
* It never runs a shell (`cmd.exe`, PowerShell) and never builds command
  lines from UI input.

## What it does touch

| What | Where | Why |
|---|---|---|
| `config.json` | `%APPDATA%\com.glassdesktop.shell\` | Your dock settings. Written once on first run, then only read. |
| `state.json` | same folder | Whether you hid the dock. |
| `assets\icons`, `assets\wallpapers` | same folder (or your `paths.assetsDir`) | Your custom icons and wallpapers; created empty on first run. |
| `HKCU\…\CurrentVersion\Run` value `Glass Desktop` | your user registry | **Only** if you tick *Launch at Sign-in* in the tray. Removed when you untick it. |
| `HKCU\…\Themes\Personalize\EnableTransparency` | read-only | Opt-in native blur follows your *Transparency effects* setting. |
| Start-menu app list (Apps folder) | read-only | Resolves the app IDs **you configured** and their icons. It doesn't enumerate or store anything else. |

### Fullscreen detection

To stay out of the way of games, videos and presentations, the dock registers
an AppBar **without reserving screen space**, so Windows notifies it when a
fullscreen app opens (`ABN_FULLSCREENAPP`). It then asks Windows a yes/no
question (`SHQueryUserNotificationState`) before hiding. It never learns
*which* app is fullscreen. Set `behavior.hideInFullscreen: false` to switch
this off entirely.

### Global shortcut

`Ctrl+Alt+D` (configurable or disableable) is registered with
`RegisterHotKey`. Windows then delivers only that one key combination. It is
not a keyboard hook and sees no other keys.

## Security model

* **Least-privilege IPC.** The webview can call only eight app commands
  (declared in `src-tauri/build.rs`, granted in
  `src-tauri/capabilities/dock.json`) plus event listen/unlisten. No Tauri
  plugins are enabled: no `fs`, `shell`, `http`, `opener` or `process`. The
  asset protocol is disabled, and `withGlobalTauri` is off.
* **Launch by id, never by path.** The UI sends `launch_item("terminal")`.
  Rust looks the id up in the config it loaded from disk, re-validates the
  target, and only then opens it.
* **Validated targets.**
  * `app`: the AppUserModelID must resolve inside the Windows *Apps* folder,
    i.e. be an installed, registered app.
  * `exe`, `shortcut`, `folder`: absolute drive-letter path. UNC
    (`\\server\…`), device (`\\.\`) and verbatim (`\\?\`) paths are
    rejected, so a config can't trigger network access or NTLM leaks. No
    `..` segments. Extension allow-list: `.exe` for exe, `.lnk` for shortcut.
    The target must exist. `.bat`, `.cmd`, `.ps1`, `.vbs`, `.js` and other
    scripts can't be dock targets.
  * Only `%USERPROFILE%`, `%APPDATA%`, `%LOCALAPPDATA%`, `%PROGRAMFILES%`,
    `%PROGRAMFILES(X86)%`, `%PROGRAMDATA%`, `%SYSTEMROOT%`, `%WINDIR%` and
    `%PUBLIC%` expand in paths.
  * Arguments are quoted one by one with the MSVC `CommandLineToArgvW` rules
    and passed via `ShellExecuteExW` (no shell interpretation).
* **Launching** uses `ShellExecuteExW("open")`, as Explorer does. Windows'
  SmartScreen and Mark-of-the-Web checks stay on.
* **Config hardening.** Every number is clamped, colours must be `#rgb` or
  `#rrggbb` (no CSS injection), ids are `[a-z0-9-]`, and labels are stripped
  of control and bidi-override characters. A config that fails to parse is
  never overwritten. The app runs on defaults and tells you.
* **Icons.** Custom icon files must be PNG, SVG, ICO, JPG or WebP, at most
  4 MB. They are passed to the UI as `data:` URLs (SVG inside `<img>` can't
  run scripts).
* **Strict CSP**, `freezePrototype: true`, DevTools only in debug builds,
  browser shortcuts (reload, find, print, zoom) disabled.

## The WebView2 runtime (measured)

The UI is rendered by **WebView2**, Microsoft's Edge engine that ships with
Windows 11 and is updated by Windows, not by this app. WebView2 runs some
Microsoft background services of its own. The dock starts it with these
switched off (`src-tauri/src/dock.rs`, `WEBVIEW2_ARGS`):

`--disable-background-networking --disable-component-update
--disable-domain-reliability --disable-client-side-phishing-detection
--disable-sync --disable-breakpad --no-pings`, plus Tauri's defaults that
disable SmartScreen and Edge's out-of-process UI.

**Measured on the development PC (2026-11-16, release build):** even with
those switches, the WebView2 *main process* keeps one HTTPS connection to a
Microsoft address (seen: `40.104.56.130:443`, then `52.98.63.34:443`, both
in Microsoft's Office/Edge service range), opened at start-up. It comes from
Microsoft code inside the runtime, not from Chromium's network stack (the
switches above disable that) and not from the dock's code. It sends no dock
data: the page has no network access and the native side has no network code.
Its content is governed by Microsoft's settings for Edge/WebView2, chiefly
*Settings › Privacy & security › Diagnostics & feedback*.

Options, all yours to choose:

1. Set Windows diagnostic data to *required only*.
2. Accept it as part of the OS component.
3. Use a firewall rule on `msedgewebview2.exe`. Not recommended: that binary
   is shared by every WebView2 app (Widgets, the new Outlook, Teams…).

## Third-party components

* **WebView2**: see above. The installer is configured **not** to download
  WebView2 (`webviewInstallMode: skip`).
* **Development time only:** npm and crates.io supply libraries. `tauri build`
  downloads its NSIS installer tools from GitHub (`tauri-apps/binary-releases`)
  once, hash-checked, and caches them. None of this happens when the app runs.
