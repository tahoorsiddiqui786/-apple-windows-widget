// The per-frame half of the dock: a requestAnimationFrame loop that runs only
// while something is moving, and writes styles straight to DOM nodes. React
// never re-renders per frame, and when the pointer is away the loop is fully
// stopped (zero idle CPU).

import { computeFrame, indexAt, restCenter, type MagnifyParams } from "./magnify";

export interface EngineGeometry {
  barLeft: number;
  barTop: number;
  barWidth: number;
  barHeight: number;
}

export interface EngineCallbacks {
  /** Pointer entered (true) or the dock fully settled after leaving (false). */
  onInteractive(expanded: boolean): void;
  /** Bar growth past its resting edges, for the native blur backdrop.
   *  Calls are serialized: the next one waits for the previous promise. */
  onExtent(left: number, right: number): Promise<void>;
  /** Hovered icon index, or -1. Fires only when it changes. */
  onHoverIndex(index: number): void;
}

export interface EngineElements {
  bar: HTMLElement;
  items: (HTMLElement | null)[];
  /** Accent glow inside the bar (positioned relative to the bar). */
  glow: HTMLElement | null;
  /** Label anchor and the icon it should follow (-1 = none). */
  label: () => { el: HTMLElement | null; index: number };
}

const EXTENT_EPSILON = 0.5;

export class MagnifyEngine {
  private readonly params: MagnifyParams;
  private readonly geo: EngineGeometry;
  private readonly el: EngineElements;
  private readonly cb: EngineCallbacks;
  private readonly tauAmount: number;
  private readonly tauX: number;
  private hovering = false;
  private frozen = false;
  private holds = 0;
  private amount = 0;
  private amountTarget = 0;
  private x = 0;
  private xTarget = 0;
  private raf = 0;
  private lastTime = 0;
  private hoverIndex = -1;
  private interactive = false;
  private sentExtent: [number, number] = [0, 0];
  private extentBusy = false;
  private extentPending: [number, number] | null = null;
  private lastFrame: ReturnType<typeof computeFrame>;

  constructor(params: MagnifyParams, geo: EngineGeometry, el: EngineElements, cb: EngineCallbacks, speed: number) {
    this.params = params;
    this.geo = geo;
    this.el = el;
    this.cb = cb;
    const s = Math.max(0.25, speed);
    this.tauAmount = 0.085 / s;
    this.tauX = 0.03 / s;
    this.lastFrame = computeFrame(params, 0, 0);
    this.render(this.lastFrame);
  }

  /** Pointer position in window (CSS px) coordinates. */
  pointerMove(clientX: number, clientY: number): void {
    if (this.frozen) return;
    const px = clientX - this.geo.barLeft;
    if (!this.hovering) {
      if (!this.insideRestingBar(clientX, clientY)) return;
      this.hovering = true;
      this.x = px; // jump straight to the pointer on entry
      this.amountTarget = 1;
      this.setInteractive(true);
    } else if (!this.insideHoverZone(clientX, clientY)) {
      this.leave();
      return;
    }
    this.xTarget = px;
    this.kick();
  }

  leave(): void {
    if (this.frozen || !this.hovering) return;
    this.hovering = false;
    this.amountTarget = 0;
    this.kick();
  }

  /** Freezes the current magnification (e.g. while a context menu is open). */
  setFrozen(frozen: boolean, clientX?: number, clientY?: number): void {
    this.frozen = frozen;
    if (frozen) {
      this.setInteractive(true);
      return;
    }
    if (clientX !== undefined && clientY !== undefined && this.insideHoverZone(clientX, clientY)) {
      this.pointerMove(clientX, clientY);
    } else {
      this.hovering = true; // so leave() runs
      this.leave();
    }
  }

  /** Current on-screen box of an icon, in window coordinates. */
  iconBox(index: number): { cx: number; top: number; size: number } | null {
    if (index < 0 || index >= this.params.count) return null;
    const s = this.lastFrame.scales[index] ?? 1;
    const size = this.params.iconSize * s;
    const cx = this.geo.barLeft + restCenter(this.params, index) + (this.lastFrame.shifts[index] ?? 0);
    const top = this.geo.barTop + this.params.padding + this.params.iconSize - size;
    return { cx, top, size };
  }

  destroy(): void {
    cancelAnimationFrame(this.raf);
    this.raf = 0;
  }

  // ------------------------------------------------------------------ internals

  private insideRestingBar(x: number, y: number): boolean {
    const g = this.geo;
    return x >= g.barLeft && x <= g.barLeft + g.barWidth && y >= g.barTop && y <= g.barTop + g.barHeight;
  }

  /** Bar (as currently grown) plus the magnified icon rising above it. */
  private insideHoverZone(x: number, y: number): boolean {
    const g = this.geo;
    const f = this.lastFrame;
    const left = g.barLeft - f.expandLeft - 2;
    const right = g.barLeft + g.barWidth + f.expandRight + 2;
    if (x < left || x > right || y > g.barTop + g.barHeight + 2) return false;
    const i = indexAt(this.params, this.xTarget);
    const s = i >= 0 ? (f.scales[i] ?? 1) : 1;
    const rise = Math.max(0, this.params.iconSize * (s - 1) - this.params.padding);
    return y >= g.barTop - rise - 4;
  }

  private kick(): void {
    if (this.raf) return;
    this.lastTime = performance.now();
    this.raf = requestAnimationFrame(this.tick);
  }

  private readonly tick = (now: number): void => {
    this.raf = 0;
    const dt = Math.min(0.05, Math.max(0, (now - this.lastTime) / 1000));
    this.lastTime = now;

    this.amount += (this.amountTarget - this.amount) * (1 - Math.exp(-dt / this.tauAmount));
    this.x += (this.xTarget - this.x) * (1 - Math.exp(-dt / this.tauX));
    if (Math.abs(this.amount - this.amountTarget) < 0.002) this.amount = this.amountTarget;
    if (Math.abs(this.x - this.xTarget) < 0.05) this.x = this.xTarget;

    const frame = computeFrame(this.params, this.x, this.amount);
    this.lastFrame = frame;
    this.render(frame);
    this.reportExtent(frame.expandLeft, frame.expandRight);

    const idx = this.hovering ? indexAt(this.params, this.xTarget) : -1;
    if (idx !== this.hoverIndex) {
      this.hoverIndex = idx;
      this.cb.onHoverIndex(idx);
    }

    const settled = this.amount === this.amountTarget && this.x === this.xTarget;
    if (!settled) {
      this.raf = requestAnimationFrame(this.tick);
    } else if (!this.hovering && this.amount === 0 && !this.frozen) {
      this.setInteractive(false);
    }
  };

  private render(f: ReturnType<typeof computeFrame>): void {
    const { iconSize, padding } = this.params;
    const g = this.geo;
    const bar = this.el.bar.style;
    bar.transform = `translate3d(${g.barLeft - f.expandLeft}px, ${g.barTop}px, 0)`;
    bar.width = `${g.barWidth + f.expandLeft + f.expandRight}px`;

    const bottom = g.barTop + padding + iconSize;
    for (let i = 0; i < this.params.count; i++) {
      const node = this.el.items[i];
      if (!node) continue;
      const size = iconSize * (f.scales[i] ?? 1);
      const cx = g.barLeft + restCenter(this.params, i) + (f.shifts[i] ?? 0);
      const st = node.style;
      st.transform = `translate3d(${cx - size / 2}px, ${bottom - size}px, 0)`;
      st.width = `${size}px`;
      st.height = `${size}px`;
    }

    const glowIndex = this.hovering ? indexAt(this.params, this.xTarget) : this.hoverIndex;
    const glowBox = this.iconBox(glowIndex);
    if (this.el.glow) {
      const gs = this.el.glow.style;
      gs.opacity = glowBox ? String(this.amount) : "0";
      if (glowBox) {
        const localX = glowBox.cx - (g.barLeft - f.expandLeft);
        gs.transform = `translate3d(${localX}px, ${g.barHeight}px, 0)`;
      }
    }
    this.positionLabel();
  }

  /** Moves the label anchor above its icon. Also called by React after the label mounts. */
  positionLabel(): void {
    const { el, index } = this.el.label();
    const box = this.iconBox(index);
    if (el && box) el.style.transform = `translate3d(${box.cx}px, ${box.top - 10}px, 0)`;
  }

  /** Keeps the whole window hit-testable (menus, launch bounce) while held. */
  hold(): () => void {
    this.holds++;
    this.setInteractive(true);
    let released = false;
    return () => {
      if (released) return;
      released = true;
      this.holds--;
      if (this.holds === 0 && !this.hovering && !this.frozen && this.amount === 0 && !this.raf) {
        this.setInteractive(false);
      }
    };
  }

  /** Snap back to rest without animating (used when the dock is hidden). */
  reset(): void {
    cancelAnimationFrame(this.raf);
    this.raf = 0;
    this.hovering = false;
    this.frozen = false;
    this.amount = this.amountTarget = 0;
    this.interactive = false;
    this.sentExtent = [0, 0];
    this.extentPending = null;
    this.hoverIndex = -1;
    this.lastFrame = computeFrame(this.params, this.x, 0);
    this.render(this.lastFrame);
    this.cb.onHoverIndex(-1);
  }

  private setInteractive(expanded: boolean): void {
    if (!expanded && this.holds > 0) return;
    if (this.interactive === expanded) return;
    this.interactive = expanded;
    this.cb.onInteractive(expanded);
  }

  private reportExtent(left: number, right: number): void {
    const settledToZero = left === 0 && right === 0;
    const [sl, sr] = this.sentExtent;
    if (!settledToZero && Math.abs(left - sl) < EXTENT_EPSILON && Math.abs(right - sr) < EXTENT_EPSILON) return;
    if (settledToZero && sl === 0 && sr === 0) return;
    if (this.extentBusy) {
      this.extentPending = [left, right];
      return;
    }
    this.flushExtent(left, right);
  }

  private flushExtent(left: number, right: number): void {
    this.extentBusy = true;
    this.sentExtent = [left, right];
    this.cb
      .onExtent(left, right)
      .catch(() => {})
      .finally(() => {
        this.extentBusy = false;
        const next = this.extentPending;
        this.extentPending = null;
        if (next) this.flushExtent(next[0], next[1]);
      });
  }
}
