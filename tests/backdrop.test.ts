import { describe, expect, it } from 'vitest';
import { createBackdropGeometry } from '../web/src/render/backdrop';

function heights(seed: number) {
  const geometry = createBackdropGeometry(seed, [{ x: 480, z: -1700 }, { x: 480, z: 1700 }], 20);
  const positions = geometry.getAttribute('position');
  const values = new Float32Array(positions.count);
  for (let i = 0; i < positions.count; i += 1) values[i] = positions.getY(i);
  geometry.dispose();
  return values;
}

describe('seeded scenic backdrop', () => {
  it('uses a fixed finite 96 by 96-cell grid and is deterministic per seed', () => {
    const first = createBackdropGeometry(87231, [], 0);
    const second = createBackdropGeometry(87231, [], 0);
    const other = createBackdropGeometry(87232, [], 0);
    const a = first.getAttribute('position');
    const b = second.getAttribute('position');
    expect(a.count).toBe(97 * 97);
    expect(first.index?.count).toBe(96 * 96 * 6);
    expect(Array.from(a.array)).toEqual(Array.from(b.array));
    expect(Array.from(heights(87231))).not.toEqual(Array.from(heights(87232)));
    expect(first.boundingBox?.min.toArray()).toEqual([-1700, -3, -1700]);
    expect(first.boundingBox?.max.x).toBe(1700);
    expect(first.boundingBox?.max.z).toBe(1700);
    expect(first.boundingBox?.max.y).toBeLessThanOrEqual(137);
    expect(first.boundingBox?.max.y).toBeGreaterThan(60);
    for (let i = 0; i < a.count; i += 1) {
      expect(Number.isFinite(a.getX(i))).toBe(true);
      expect(Number.isFinite(a.getY(i))).toBe(true);
      expect(Number.isFinite(a.getZ(i))).toBe(true);
      expect(a.getY(i)).toBeGreaterThanOrEqual(-3);
    }
    first.dispose();
    second.dispose();
    other.dispose();
  });

  it('keeps the authoritative basin flat and the extended river valley below the hills', () => {
    const geometry = createBackdropGeometry(51, [{ x: 480, z: -1600 }, { x: 800, z: 1600 }], 24);
    const positions = geometry.getAttribute('position');
    let valleySamples = 0;
    let hillSamples = 0;
    let bankRise = 0;
    for (let i = 0; i < positions.count; i += 1) {
      const x = positions.getX(i);
      const z = positions.getZ(i);
      const y = positions.getY(i);
      if (Math.max(Math.abs(x), Math.abs(z)) <= 420) expect(y).toBe(-3);

      const riverX = 480 + (z + 1600) / 3200 * 320;
      const distance = Math.abs(x - riverX);
      if (Math.max(Math.abs(x), Math.abs(z)) > 1000 && distance <= 36) {
        expect(y).toBe(-3);
        valleySamples += 1;
      }
      if (Math.max(Math.abs(x), Math.abs(z)) > 1200 && distance > 270) {
        hillSamples += 1;
        if (y > -3) bankRise += 1;
      }
    }
    expect(valleySamples).toBeGreaterThan(10);
    expect(hillSamples).toBeGreaterThan(100);
    expect(bankRise).toBeGreaterThan(50);
    geometry.dispose();
  });

  it('produces upward-facing, finite normals and valid bounds', () => {
    const geometry = createBackdropGeometry(12, [{ x: 0, z: -1700 }, { x: 180, z: 1700 }], 30);
    const normals = geometry.getAttribute('normal');
    for (let i = 0; i < normals.count; i += 1) {
      expect(Number.isFinite(normals.getX(i))).toBe(true);
      expect(Number.isFinite(normals.getY(i))).toBe(true);
      expect(Number.isFinite(normals.getZ(i))).toBe(true);
      expect(normals.getY(i)).toBeGreaterThan(0);
    }
    expect(geometry.boundingSphere?.radius).toBeGreaterThan(0);
    geometry.dispose();
  });
});
