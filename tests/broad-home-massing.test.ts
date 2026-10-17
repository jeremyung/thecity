import { describe, it, expect } from 'vitest';
import { broadHomeMassing } from '../web/src/render/broad-home-massing';
const home = { kind: 'home' as const, floors: 3, anchor: false };
describe('broad residence massing', () => {
  it('retains existing families outside the bounded residential scope', () => {
    for (const b of [
      { ...home, kind: 'shop' as const },
      { ...home, floors: 2 },
      { ...home, anchor: true },
    ])
      expect(broadHomeMassing(b, 14, 20)).toBeNull();
    for (const [w, d] of [
      [8, 20],
      [14, 12],
      [NaN, 20],
      [14, Infinity],
    ])
      expect(broadHomeMassing(home, w, d)).toBeNull();
  });
  it('supports the upper residence and keeps a centered ground entrance at the same front edge', () => {
    for (const [w, d] of [
      [14.156775, 20.232636],
      [10.208433, 20.232631],
    ]) {
      const layout = broadHomeMassing(home, w, d)!;
      const [lower, upper] = layout.masses;
      expect(lower.z + lower.depth / 2).toBe(d / 2);
      expect(lower.x).toBe(0);
      expect(lower.baseFloor).toBe(0);
      expect(lower.floors).toBe(upper.baseFloor);
      expect(upper.baseFloor + upper.floors).toBe(3);
      expect(upper.z - upper.depth / 2).toBeCloseTo(lower.z - lower.depth / 2);
      expect(lower.z + lower.depth / 2 - (upper.z + upper.depth / 2)).toBeCloseTo(2.4);
      for (const m of layout.masses) {
        expect(Math.abs(m.x) + m.width / 2).toBeLessThanOrEqual(w / 2);
        expect(Math.abs(m.z) + m.depth / 2).toBeLessThanOrEqual(d / 2 + 1e-8);
      }
    }
  });
});
