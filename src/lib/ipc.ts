// Typed wrappers around the dock's IPC commands. This file is the only place
// the UI talks to the native side. In a plain browser (`npm run dev` without
// Tauri) a local preview mock is used instead, so the UI can be developed and
// checked without the native shell.

import type { ActivateResult, DockState, Layout, WindowEntry } from "./types";
import { previewIpc } from "./previewIpc";

type Unlisten = () => void;

export interface DockIpc {
  getDockState(): Promise<DockState>;
  dockReady(): Promise<void>;
  /** Always starts a new instance/window. */
  launchItem(id: string): Promise<void>;
  /** Switches to the app if open, otherwise launches it. */
  activateItem(id: string): Promise<ActivateResult>;
  listItemWindows(id: string): Promise<WindowEntry[]>;
  focusItemWindow(id: string, handle: string): Promise<void>;
  revealItem(id: string): Promise<void>;
  setDockInteractive(expanded: boolean): Promise<void>;
  setBackdropExtent(left: number, right: number): Promise<void>;
  hideDock(): Promise<void>;
  openConfig(): Promise<void>;
  onStateChanged(cb: () => void): Promise<Unlisten>;
  onLayoutChanged(cb: (layout: Layout) => void): Promise<Unlisten>;
  onRunningChanged(cb: (running: Record<string, number>) => void): Promise<Unlisten>;
}

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function createTauriIpc(): Promise<DockIpc> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  return {
    getDockState: () => invoke<DockState>("get_dock_state"),
    dockReady: () => invoke("dock_ready"),
    launchItem: (id) => invoke("launch_item", { id }),
    activateItem: (id) => invoke<ActivateResult>("activate_item", { id }),
    listItemWindows: (id) => invoke<WindowEntry[]>("list_item_windows", { id }),
    focusItemWindow: (id, handle) => invoke("focus_item_window", { id, handle }),
    revealItem: (id) => invoke("reveal_item", { id }),
    setDockInteractive: (expanded) => invoke("set_dock_interactive", { expanded }),
    setBackdropExtent: (left, right) => invoke("set_backdrop_extent", { left, right }),
    hideDock: () => invoke("hide_dock"),
    openConfig: () => invoke("open_config"),
    onStateChanged: (cb) => listen("dock://state-changed", () => cb()),
    onLayoutChanged: (cb) => listen<Layout>("dock://layout-changed", (e) => cb(e.payload)),
    onRunningChanged: (cb) => listen<Record<string, number>>("dock://running-changed", (e) => cb(e.payload)),
  };
}

let ipcPromise: Promise<DockIpc> | null = null;

export function getIpc(): Promise<DockIpc> {
  ipcPromise ??= isTauri ? createTauriIpc() : Promise.resolve(previewIpc);
  return ipcPromise;
}
