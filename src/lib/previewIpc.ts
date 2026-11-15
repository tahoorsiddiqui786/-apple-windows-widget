// Browser-only stand-in for the native side, used by `npm run dev` outside
// Tauri. It never launches anything; it only logs what would happen.

import type { DockIpc } from "./ipc";
import type { DockState } from "./types";

const ICON = 52;
const GAP = 10;
const PAD = 9;
const MAG = 1.6;
const RANGE = 2.6;

function previewState(): DockState {
  const labels = ["File Explorer", "Microsoft Edge", "Terminal", "Notepad", "Calculator", "Settings"];
  const n = labels.length;
  const barWidth = Math.round(n * ICON + (n - 1) * GAP + 2 * PAD);
  const barHeight = ICON + 2 * PAD;
  const extra = ICON * (MAG - 1);
  const maxExpand = Math.ceil(extra * RANGE * 1.1 + 8);
  const headroom = Math.max(230, Math.ceil(extra + 72));
  const width = barWidth + 2 * (maxExpand + 12);
  const height = headroom + barHeight + 12;
  return {
    layout: {
      scale: window.devicePixelRatio || 1,
      width,
      height,
      barLeft: Math.round((width - barWidth) / 2),
      barTop: headroom,
      barWidth,
      barHeight,
      maxExpand,
      halo: 10,
      bottomPad: 12,
    },
    dock: {
      position: "bottom",
      iconSize: ICON,
      iconSpacing: GAP,
      padding: PAD,
      magnification: MAG,
      magnificationRange: RANGE,
      marginBottom: 12,
      showLabels: true,
    },
    appearance: {
      backdrop: "none",
      glassTint: "#0b0d13",
      glassOpacity: 0.42,
      borderOpacity: 0.16,
      cornerRadius: 22,
      accentColor: "#7c8cff",
      glow: true,
      animationSpeed: 1,
    },
    // In the browser there is no DWM; CSS backdrop-filter stands in for it.
    backdrop: "none",
    items: labels.map((label, i) => ({
      id: label.toLowerCase().replace(/\s+/g, "-"),
      label,
      badge: i === 1 ? "3" : null,
      icon: null,
      canReveal: i === 0,
    })),
    warnings: [],
    hotkey: "Ctrl+Alt+D",
  };
}

const log = (...args: unknown[]) => console.info("[preview ipc]", ...args);

export const previewIpc: DockIpc = {
  getDockState: async () => previewState(),
  dockReady: async () => log("dock_ready"),
  launchItem: async (id) => {
    log("launch_item", id);
    if (id === "settings") throw new Error("Preview: simulated launch failure");
  },
  revealItem: async (id) => log("reveal_item", id),
  setDockInteractive: async (expanded) => log("set_dock_interactive", expanded),
  setBackdropExtent: async () => {},
  hideDock: async () => log("hide_dock"),
  openConfig: async () => log("open_config"),
  onStateChanged: async () => () => {},
  onLayoutChanged: async () => () => {},
};
