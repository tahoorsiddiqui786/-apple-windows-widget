// Typed wrappers around the dock's IPC commands. This file is the only place
// the UI talks to the native side. In a plain browser (`npm run dev` without
// Tauri) a local preview mock is used instead, so the UI can be developed and
// checked without the native shell.

import type { DockState, Layout } from "./types";
import { previewIpc } from "./previewIpc";

type Unlisten = () => void;

export interface DockIpc {
  getDockState(): Promise<DockState>;
  dockReady(): Promise<void>;
  launchItem(id: string): Promise<void>;
  revealItem(id: string): Promise<void>;
  setDockInteractive(expanded: boolean): Promise<void>;
  setBackdropExtent(left: number, right: number): Promise<void>;
  hideDock(): Promise<void>;
  openConfig(): Promise<void>;
  onStateChanged(cb: () => void): Promise<Unlisten>;
  onLayoutChanged(cb: (layout: Layout) => void): Promise<Unlisten>;
}

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function createTauriIpc(): Promise<DockIpc> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  return {
    getDockState: () => invoke<DockState>("get_dock_state"),
    dockReady: () => invoke("dock_ready"),
    launchItem: (id) => invoke("launch_item", { id }),
    revealItem: (id) => invoke("reveal_item", { id }),
    setDockInteractive: (expanded) => invoke("set_dock_interactive", { expanded }),
    setBackdropExtent: (left, right) => invoke("set_backdrop_extent", { left, right }),
    hideDock: () => invoke("hide_dock"),
    openConfig: () => invoke("open_config"),
    onStateChanged: (cb) => listen("dock://state-changed", () => cb()),
    onLayoutChanged: (cb) => listen<Layout>("dock://layout-changed", (e) => cb(e.payload)),
  };
}

let ipcPromise: Promise<DockIpc> | null = null;

export function getIpc(): Promise<DockIpc> {
  ipcPromise ??= isTauri ? createTauriIpc() : Promise.resolve(previewIpc);
  return ipcPromise;
}
