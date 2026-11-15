# Glass on Windows 11: investigation, implementation, fallback

## The problem

The dock window is a transparent Tauri (WebView2) window. CSS `backdrop-filter`
inside WebView2 only blurs **pixels the web page itself drew**. It never sees
the desktop or other apps behind a transparent native window. A transparent
webview on its own gives *tinted* glass, not *frosted* glass.

Blurring what is behind a window is only possible through the Desktop Window
Manager (DWM), and DWM applies it to a whole window.

## Options evaluated

| Approach | Blurs what's behind | Inactive windows | Verdict |
|---|---|---|---|
| CSS `backdrop-filter` in the webview | ✗ (page content only) | – | Browser preview only |
| `windowEffects: mica / tabbed` (`DWMWA_SYSTEMBACKDROP_TYPE`) | wallpaper tint, not a blur of windows | flat colour when inactive | Wrong look for a dock |
| `windowEffects: acrylic` on 22H2+ (`DWMSBT_TRANSIENTWINDOW`) | ✓ | flat colour when inactive | A dock is never active |
| `SetWindowCompositionAttribute` accent blur/acrylic (undocumented; used by `window-vibrancy`, TranslucentTB, most docks) | ✓ on most builds | ✓ on most builds | Implemented as **opt-in** |

## What we measured on the development PC

Windows 11 26H2, build 26300.9711, Intel Iris Xe (driver 32.0.101.7088),
transparency effects on, Energy Saver off, local session (not remote):

* **Every** DWM backdrop variant rendered as an **opaque tint** with no blur:
  the accent blur, the accent acrylic, and the documented
  `DWMSBT_TRANSIENTWINDOW` (grey fallback). This held for inactive windows and
  for an active foreground window.
* DWM drew those backdrops over the window's **full rectangle**, ignoring
  `SetWindowRgn`. The first build sized the backdrop window like the dock window
  and clipped it with a rounded region. On this PC that showed up as a 900×470
  dark box around the dock.
* The taskbar on the same PC also looked flat in captures. Windows itself
  doesn't appear to be rendering blur there at the moment.

## What the app does now

1. **Default `appearance.backdrop: "none"`.** The webview draws the glass:
   a near-opaque tint (≥ 86 %, so icons stay legible on any background),
   light gradient, inner top highlight, hairline edge, soft shadow and an
   accent underglow. It's anti-aliased with any corner radius and looks the
   same on every PC.
2. **Opt-in native blur, `"acrylic"` or `"blur"`.** A separate plain Win32
   window (no webview, no IPC) sits directly behind the bar and receives the
   accent blur. It is **sized to the bar** with `SetWindowPos` instead of
   clipped with a region, so it can never cover more than the bar, even on
   builds that ignore regions. During magnification it resizes with the bar,
   and a rounded region is still applied for builds that honour it.
   * On PCs where DWM blur works, you get real frosted glass.
   * On the development PC it renders as a solid tint with square corners,
     so it stays off there.

```
┌──────────── dock window (WebView2, transparent, owned by ▼) ────────────┐
│  headroom: magnified icons, labels, context menu                         │
│        ╭──────────────── glass bar (CSS) ────────────────╮               │
│        │ tint · highlights · hairline · icons            │               │
│        ╰─────────────────────────────────────────────────╯               │
└──────────────────────────────────────────────────────────────────────────┘
         ┌─────────── backdrop window (opt-in, Win32) ──────────┐
         │ DWM accent blur, exactly the bar's bounds             │  ← behind
         └───────────────────────────────────────────────────────┘
```

The dock window is **owned** by the backdrop window (`GWLP_HWNDPARENT`), so
Windows always keeps it above the backdrop. Both are
`WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST`.

## Click-through (verified)

A transparent WebView2 window still captures clicks on its transparent
pixels. At rest the dock window's region is shrunk to the bar plus a 10 px
halo. When the pointer enters, the UI calls `set_dock_interactive(true)` and
the region is removed so magnified icons and menus work. When the pointer
leaves and the animation settles, the region shrinks again. There's no
polling and no global mouse hook.

Measured with `WindowFromPoint` on the development PC:

| Point | At rest | While hovering | After leaving |
|---|---|---|---|
| Empty area above the bar | app behind the dock ✔ | dock | app behind the dock ✔ |
| On an icon | dock ✔ | dock ✔ | dock ✔ |

The foreground window was unchanged after hovering, and after clicking an
icon the launched app (not the dock) became the foreground window.

## Fallback rules

| Situation | Behaviour |
|---|---|
| `backdrop: "none"` (default) | CSS glass; backdrop window hidden |
| `"acrylic"` / `"blur"` but `SetWindowCompositionAttribute` unavailable | Same as `"none"` |
| Settings › Personalization › Colors › *Transparency effects* off | Same as `"none"`; re-checked on `WM_SETTINGCHANGE` |
| Browser preview (`npm run dev`) | CSS `backdrop-filter` over a sample background |

## Blur strength

DWM exposes no blur-radius control. "Blur intensity" is expressed as
`appearance.backdrop` (`"acrylic"`, the stronger one, or `"blur"`) plus
`appearance.glassOpacity`, the tint over the blur.

## Next ideas for real blur on PCs like the development one

* **Wallpaper-aware glass.** The app will manage the wallpaper (milestone 3),
  so the dock can show a pre-blurred crop of the wallpaper region under it.
  That's real frosted look over the desktop, at zero runtime cost. It falls
  back to the tint whenever a window covers the dock area.
* Re-test DWM blur after Windows or graphics-driver updates. The native path
  is one config value away.
