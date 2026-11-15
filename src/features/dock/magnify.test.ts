import { describe, expect, it } from "vitest";
import { computeFrame, falloff, indexAt, restCenter, type MagnifyParams } from "./magnify";

const p: MagnifyParams = { count: 6, iconSize: 52, spacing: 10, padding: 9, maxScale: 1.6, range: 2.6 };

describe("falloff", () => {
  it("peaks at 0 and vanishes at the edge", () => {
    expect(falloff(0)).toBe(1);
    expect(falloff(1)).toBe(0);
    expect(falloff(-2)).toBe(0);
    expect(falloff(0.5)).toBeCloseTo(0.5);
  });
});

describe("computeFrame", () => {
  it("is the identity when the envelope is zero", () => {
    const f = computeFrame(p, restCenter(p, 2), 0);
    expect(f.scales.every((s) => s === 1)).toBe(true);
    expect(f.shifts.every((s) => Math.abs(s) < 1e-9)).toBe(true);
    expect(f.expandLeft).toBeCloseTo(0);
    expect(f.expandRight).toBeCloseTo(0);
  });

  it("peaks under the pointer and decays with distance", () => {
    const f = computeFrame(p, restCenter(p, 2), 1);
    expect(f.scales[2]).toBeCloseTo(1.6);
    expect(f.scales[1]!).toBeLessThan(f.scales[2]!);
    expect(f.scales[0]!).toBeLessThan(f.scales[1]!);
  });

  it("keeps the point under the cursor stationary", () => {
    for (const x of [20, 100, restCenter(p, 3) + 7, 300]) {
      const f = computeFrame(p, x, 1);
      // Find the icon whose cell contains x; its new geometry must still contain x
      // at the same relative position.
      const i = indexAt(p, x);
      if (i < 0) continue;
      const pitch = p.iconSize + p.spacing;
      const cellLeftRest = restCenter(p, i) - pitch / 2;
      const frac = (x - cellLeftRest) / pitch;
      const e = p.iconSize * (f.scales[i]! - 1);
      const newCellLeft = restCenter(p, i) + f.shifts[i]! - (pitch + e) / 2;
      expect(newCellLeft + frac * (pitch + e)).toBeCloseTo(x, 6);
    }
  });

  it("icons never overlap while magnified", () => {
    for (let x = -20; x <= 400; x += 7) {
      const f = computeFrame(p, x, 1);
      for (let i = 1; i < p.count; i++) {
        const rightPrev = restCenter(p, i - 1) + f.shifts[i - 1]! + (p.iconSize * f.scales[i - 1]!) / 2;
        const leftCur = restCenter(p, i) + f.shifts[i]! - (p.iconSize * f.scales[i]!) / 2;
        expect(leftCur - rightPrev).toBeGreaterThanOrEqual(p.spacing - 1e-6);
      }
    }
  });

  it("bar growth equals the total extra width", () => {
    const f = computeFrame(p, 150, 1);
    const total = f.scales.reduce((acc, s) => acc + p.iconSize * (s - 1), 0);
    expect(f.expandLeft + f.expandRight).toBeCloseTo(total);
  });

  it("stays within the native window's expansion budget", () => {
    const budget = Math.ceil(p.iconSize * (p.maxScale - 1) * p.range * 1.1 + 8);
    for (let x = -100; x <= 500; x += 3) {
      const f = computeFrame(p, x, 1);
      expect(f.expandLeft).toBeLessThanOrEqual(budget);
      expect(f.expandRight).toBeLessThanOrEqual(budget);
    }
  });
});

describe("indexAt", () => {
  it("maps pointer positions to icon cells", () => {
    expect(indexAt(p, restCenter(p, 0))).toBe(0);
    expect(indexAt(p, restCenter(p, 5))).toBe(5);
    expect(indexAt(p, -50)).toBe(-1);
    expect(indexAt(p, 10_000)).toBe(-1);
  });
});
