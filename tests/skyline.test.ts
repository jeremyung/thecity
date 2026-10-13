import { describe, expect, it } from 'vitest';
import { createSkylineGeometry } from '../web/src/render/skyline';

const river = [{ x: 480, z: -1400 }, { x: 520, z: 1400 }];

function towers(seed: number) {
  const geometry = createSkylineGeometry(seed, river);
  const positions = geometry.getAttribute('position');
  const result: Array<{ x: number; z: number; height: number }> = [];
  for (let i = 0; i < positions.count; i += 8) {
    let x = 0, z = 0;
    for (let k = 0; k < 4; k += 1) { x += positions.getX(i + k) / 4; z += positions.getZ(i + k) / 4; }
    result.push({ x, z, height: positions.getY(i + 4) });
  }
  geometry.dispose();
  return result;
}

describe('distant skyline backdrop', () => {
  it('is deterministic per seed and differs between seeds', () => {
    expect(towers(87231)).toEqual(towers(87231));
    expect(towers(87231)).not.toEqual(towers(87232));
  });

  it('stays outside the scenic hills and inside the camera far plane', () => {
    for (const tower of towers(51)) {
      const radius = Math.hypot(tower.x, tower.z);
      expect(radius).toBeGreaterThan(1700);
      expect(radius).toBeLessThan(2600);
    }
  });

  it('keeps the river vista open and raises peaks above the hill crest', () => {
    const all = towers(87231);
    for (const tower of all) {
      const riverX = 480 + (Math.max(-1400, Math.min(1400, tower.z)) + 1400) / 2800 * 40;
      // Pushed centres clear the corridor; footprints are at most ~20 m half-width.
      expect(Math.abs(tower.x - riverX)).toBeGreaterThan(200);
    }
    expect(Math.max(...all.map((tower) => tower.height))).toBeGreaterThan(160);
    expect(all.length).toBeGreaterThan(6 * 48 * 0.35);
    expect(all.length).toBeLessThanOrEqual(6 * 48);
  });
});

describe('distant skyline spacing', () => {
  it('never overlaps tower footprints, which would z-fight at 2 km', () => {
    const all = towers(87231);
    for (let i = 0; i < all.length; i += 1) for (let j = i + 1; j < all.length; j += 1) {
      expect(Math.hypot(all[i].x - all[j].x, all[i].z - all[j].z)).toBeGreaterThan(10);
    }
  });
});
