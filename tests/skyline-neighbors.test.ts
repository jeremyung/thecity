import { describe, expect, it } from 'vitest';
import type { Building } from '../web/src/types';
import { skylineNeighborhoods } from '../web/src/camera/skyline-neighbors';

function point(id: number, x: number, z: number): Building {
  return { id, x, z } as Building;
}

describe('skyline neighborhoods', () => {
  it('preserves original ordering and exact radius boundaries across negative cells', () => {
    const buildings = [point(7, 0, 0), point(3, -90, 0), point(9, 90, 0),
      point(2, 0, -90), point(5, 0, 90), point(8, 90.000001, 0),
      point(1, -180, 0), point(6, -90, -90), point(4, 0, 0)];
    const query = skylineNeighborhoods(buildings, 90);
    for (const seed of buildings) {
      expect(query(seed)).toEqual(buildings.filter(b => Math.hypot(b.x - seed.x, b.z - seed.z) <= 90));
    }
    expect(query(buildings[0]).map(b => b.id)).toEqual([7, 3, 9, 2, 5, 4]);
  });

  it('matches full scans for dispersed and dense deterministic layouts', () => {
    let rng = 87231;
    const random = () => ((rng = (Math.imul(rng, 1664525) + 1013904223) >>> 0) / 0x100000000);
    for (const span of [30, 180, 800, 2400]) {
      const buildings = Array.from({ length: 200 }, (_, id) => point(id, (random() - .5) * span, (random() - .5) * span));
      const query = skylineNeighborhoods(buildings, 90);
      for (const seed of buildings) expect(query(seed)).toEqual(buildings.filter(b => Math.hypot(b.x - seed.x, b.z - seed.z) <= 90));
    }
  });
});
