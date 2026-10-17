import { describe, expect, it } from 'vitest';
import { riverbankTreePoints } from '../web/src/render/riverbank-planting';
import { createFoliageGeometry, createTreeTrunkGeometry } from '../web/src/render/foliage';

describe('riverbank planting physical envelope', () => {
  it('keeps the rotated near/far crowns off water and the promenade railing', () => {
    const river = [
      { x: 0, z: -400 },
      { x: 14, z: -190 },
      { x: 0, z: 20 },
      { x: 18, z: 400 },
    ];
    const riverX = (z: number) => {
      const i = river.findIndex((p, i) => i > 0 && z <= p.z);
      const a = river[i - 1],
        b = river[i];
      return a.x + ((b.x - a.x) * (z - a.z)) / (b.z - a.z);
    };
    const points = riverbankTreePoints(river, 32);
    expect(points.length).toBeGreaterThan(0);
    for (const detail of ['near', 'far'] as const) {
      const geometry = createFoliageGeometry(detail);
      const positions = geometry.getAttribute('position');
      let radius = 0;
      for (let i = 0; i < positions.count; i++)
        radius = Math.max(radius, Math.hypot(positions.getX(i), positions.getZ(i)));
      for (const p of points) {
        const r = (radius * p.s) / 1.8;
        for (let sample = 0; sample < 64; sample++) {
          const angle = (sample * Math.PI) / 32,
            x = p.x + Math.cos(angle) * r,
            z = p.z + Math.sin(angle) * r;
          const offset = Math.abs(x - riverX(z)) - 32;
          expect(offset).toBeGreaterThan(0);
          expect(offset).toBeLessThan(5.4);
        }
        expect(2.7 - r).toBeGreaterThan(0);
        expect(2.7 + r).toBeLessThan(5.4);
        expect(Math.abs(p.x) + r).toBeLessThan(400);
        expect(Math.abs(p.z) + r).toBeLessThan(400);
      }
      geometry.dispose();
    }
  });
  it('keeps trunks outside the stone coping', () => {
    const geometry = createTreeTrunkGeometry(),
      p = geometry.getAttribute('position');
    let radius = 0;
    for (let i = 0; i < p.count; i++) radius = Math.max(radius, Math.hypot(p.getX(i), p.getZ(i)));
    expect(2.7 - (radius * 1.8) / 1.8).toBeGreaterThan(1.6);
    geometry.dispose();
  });
});
