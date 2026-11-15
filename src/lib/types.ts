// Mirrors the Rust `DockState` returned by the `get_dock_state` command.

export type Backdrop = "acrylic" | "blur" | "none";

export interface Layout {
  scale: number;
  width: number;
  height: number;
  barLeft: number;
  barTop: number;
  barWidth: number;
  barHeight: number;
  maxExpand: number;
  halo: number;
  bottomPad: number;
}

export interface DockConfig {
  position: "bottom";
  iconSize: number;
  iconSpacing: number;
  padding: number;
  magnification: number;
  magnificationRange: number;
  marginBottom: number;
  showLabels: boolean;
}

export interface Appearance {
  backdrop: Backdrop;
  glassTint: string;
  glassOpacity: number;
  borderOpacity: number;
  cornerRadius: number;
  accentColor: string;
  glow: boolean;
  animationSpeed: number;
}

export interface DockItemView {
  id: string;
  label: string;
  badge: string | null;
  /** data: URL, or null when no icon could be loaded. */
  icon: string | null;
  canReveal: boolean;
}

export interface DockState {
  layout: Layout;
  dock: DockConfig;
  appearance: Appearance;
  /** Backdrop actually in effect after fallbacks (may differ from appearance.backdrop). */
  backdrop: Backdrop;
  items: DockItemView[];
  warnings: string[];
  hotkey: string;
  /** Open window count per item id; items with no open windows are absent. */
  running: Record<string, number>;
}

/** One open window of a dock app (right-click menu). */
export interface WindowEntry {
  /** Opaque token, only valid for the same item. */
  handle: string;
  title: string;
  minimized: boolean;
}

export type ActivateResult = "focused" | "launched";
