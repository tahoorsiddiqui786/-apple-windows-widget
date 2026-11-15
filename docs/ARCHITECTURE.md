# Architecture

```
                 ┌───────────────────────── Rust (src-tauri) ─────────────────────────┐
 config.json ──► │ config/   schema · defaults · clamp/validate · atomic save          │
 state.json  ◄─► │ layout    dock geometry (physical px ⇄ CSS px, DPI-aware)            │
                 │ dock      controller: windows, visibility, regions, native events   │
                 │ launcher  validate target → ShellExecuteExW (no shell)              │
                 │ icons     custom icon files / real Windows icons → data: URLs       │
                 │ native/   Win32: DWM blur backdrop · window regions · hotkey ·      │
                 │           appbar fullscreen notice · autostart · single instance    │
                 │ tray      show/hide · edit/reload config · assets · sign-in · quit  │
                 │ commands  the 8 IPC commands granted to the dock webview            │
                 └──────────────────────────────▲──────────────────────────────────────┘
                                                │ IPC (capability-scoped)
                 ┌──────────────────────────────┴──── React (src) ─────────────────────┐
                 │ app/DockApp       load state, theme, tell native "ready"            │
                 │ features/dock/    Dock · DockItem · DockContextMenu                 │
                 │   magnify.ts      pure magnification math (unit-tested)             │
                 │   magnifyEngine   rAF loop, runs only while moving; DOM writes      │
                 │ theme/            config → CSS custom properties                    │
                 │ animation/        Framer Motion presets (menus, labels, badges)     │
                 │ lib/ipc           typed commands + browser preview mock             │
                 │ features/widgets  (future: clock, date, CPU/RAM, battery)           │
                 └─────────────────────────────────────────────────────────────────────┘
```

## Key decisions

* **Rust owns geometry and windows.** It computes the layout from the monitor
  work area and DPI, places both windows, and passes the numbers to the UI. CSS
  and native layers therefore always line up.
* **Glass that works everywhere, native blur where possible.** The webview
  draws the glass by default. For opt-in DWM blur, a plain Win32 backdrop
  window sized to the bar sits behind the webview. See
  [NATIVE_BLUR.md](NATIVE_BLUR.md).
* **Zero idle work.** No timers, polling or watchers. The animation loop runs
  only while something moves. Fullscreen, display and DPI changes, and the
  hotkey arrive as Windows messages.
* **No per-frame React renders.** The engine writes `transform`, `width` and
  `height` directly. React re-renders only when the hovered icon, the menu or
  the config changes.
* **The dock never takes focus.** Both windows are `WS_EX_NOACTIVATE`, and the
  app you were typing in keeps focus. Launching passes the foreground right on
  with `AllowSetForegroundWindow`.
* **Config is the user's.** Hand-editable JSON, validated on load and never
  rewritten. Runtime state is kept separately.

## Adding a widget later

Widgets will be separate Tauri windows (e.g. `widget-clock`) with their own
capability file and their own React entry under `src/features/widgets/<name>`.
They share `theme/` and `animation/`. System metrics (CPU, RAM, battery) will
come from a dedicated Rust module that samples only while a widget is visible.
