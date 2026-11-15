// Theme management: config values -> CSS custom properties on <html>.
// Colours arrive pre-validated (#rgb / #rrggbb) from the native side, and are
// re-checked here so nothing but a hex colour can ever reach CSS.

import type { Appearance, Backdrop } from "../lib/types";

export type SurfaceMode = "native" | "fallback" | "preview";

const HEX = /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i;

export function hexToRgb(hex: string, fallback = "11 13 19"): string {
  if (!HEX.test(hex)) return fallback;
  let h = hex.slice(1);
  if (h.length === 3) h = [...h].map((c) => c + c).join("");
  const n = Number.parseInt(h, 16);
  return `${(n >> 16) & 255} ${(n >> 8) & 255} ${n & 255}`;
}

const clamp01 = (v: number) => Math.min(1, Math.max(0, Number.isFinite(v) ? v : 0));

export function surfaceMode(backdrop: Backdrop, inTauri: boolean): SurfaceMode {
  if (!inTauri) return "preview";
  return backdrop === "none" ? "fallback" : "native";
}

export function applyTheme(a: Appearance, mode: SurfaceMode, root: HTMLElement = document.documentElement): void {
  const s = root.style;
  s.setProperty("--accent-rgb", hexToRgb(a.accentColor, "124 140 255"));
  s.setProperty("--tint-rgb", hexToRgb(a.glassTint));
  s.setProperty("--glass-opacity", String(clamp01(a.glassOpacity)));
  s.setProperty("--border-opacity", String(clamp01(a.borderOpacity)));
  s.setProperty("--radius", `${Math.max(0, a.cornerRadius)}px`);
  s.setProperty("--speed", String(Math.max(0.25, a.animationSpeed)));
  root.dataset.surface = mode;
  root.dataset.glow = a.glow ? "on" : "off";
}

/** Duration helper so every animation honours `animationSpeed`. */
export function dur(seconds: number, speed: number): number {
  return seconds / Math.max(0.25, speed);
}
