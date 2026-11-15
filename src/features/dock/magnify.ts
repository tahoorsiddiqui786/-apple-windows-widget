// Pure magnification math (no DOM). macOS-style: icons near the pointer grow,
// neighbours make room, and the point under the cursor stays put so the dock
// never "slides away" from the mouse.

export interface MagnifyParams {
  count: number;
  iconSize: number;
  spacing: number;
  padding: number;
  /** Peak scale at the pointer (1 = off). */
  maxScale: number;
  /** Spread of the wave, in icon pitches on each side. */
  range: number;
}

export interface MagnifyFrame {
  /** Scale factor per icon. */
  scales: number[];
  /** Horizontal offset of each icon's centre from its resting centre (px). */
  shifts: number[];
  /** How far the bar grows past its resting left / right edges (px). */
  expandLeft: number;
  expandRight: number;
}

/** Resting centre of icon `i`, relative to the bar's left edge. */
export function restCenter(p: MagnifyParams, i: number): number {
  return p.padding + p.iconSize / 2 + i * (p.iconSize + p.spacing);
}

/** Smooth bump: 1 at distance 0, 0 at distance >= 1 (raised cosine). */
export function falloff(t: number): number {
  const a = Math.abs(t);
  return a >= 1 ? 0 : (1 + Math.cos(Math.PI * a)) / 2;
}

/**
 * @param pointerX pointer x relative to the bar's resting left edge.
 * @param amount   0..1 envelope (animated in/out on hover enter/leave).
 */
export function computeFrame(p: MagnifyParams, pointerX: number, amount: number): MagnifyFrame {
  const n = p.count;
  const pitch = p.iconSize + p.spacing;
  const reach = Math.max(pitch * p.range, 1);
  const scales = new Array<number>(n);
  const extras = new Array<number>(n);
  let total = 0;
  let leftOfPointer = 0;

  for (let i = 0; i < n; i++) {
    const c = restCenter(p, i);
    const s = 1 + (p.maxScale - 1) * falloff((pointerX - c) / reach) * amount;
    scales[i] = s;
    const e = p.iconSize * (s - 1);
    extras[i] = e;
    total += e;
    // Fraction of this icon's cell that lies left of the pointer.
    const cellLeft = c - pitch / 2;
    const frac = Math.min(1, Math.max(0, (pointerX - cellLeft) / pitch));
    leftOfPointer += e * frac;
  }

  const shifts = new Array<number>(n);
  let before = 0;
  for (let i = 0; i < n; i++) {
    const e = extras[i]!;
    shifts[i] = before + e / 2 - leftOfPointer;
    before += e;
  }

  return { scales, shifts, expandLeft: leftOfPointer, expandRight: total - leftOfPointer };
}

/** Index of the icon cell under `pointerX`, or -1 outside the icon row. */
export function indexAt(p: MagnifyParams, pointerX: number): number {
  const pitch = p.iconSize + p.spacing;
  const x = pointerX - p.padding + p.spacing / 2;
  const i = Math.floor(x / pitch);
  return i >= 0 && i < p.count ? i : -1;
}
