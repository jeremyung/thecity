import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { createFoliageGeometry, createTreeTrunkGeometry } from '../web/src/render/foliage';

function triangles(geometry: THREE.BufferGeometry): number {
  return geometry.index ? geometry.index.count / 3 : geometry.getAttribute('position').count / 3;
}

function expectFinite(geometry: THREE.BufferGeometry): void {
  const positions = geometry.getAttribute('position');
  for (let i = 0; i < positions.count; i++) {
    expect(Number.isFinite(positions.getX(i))).toBe(true);
    expect(Number.isFinite(positions.getY(i))).toBe(true);
    expect(Number.isFinite(positions.getZ(i))).toBe(true);
  }
}

describe('authored tree geometry', () => {
  it('keeps cross-card crowns broad, bounded, and within the foliage triangle budget', () => {
    const near = createFoliageGeometry('near');
    const far = createFoliageGeometry('far');
    near.computeBoundingBox();
    far.computeBoundingBox();
    expect(triangles(near)).toBeGreaterThan(0);
    expect(triangles(near)).toBeLessThan(1_000);
    expect(triangles(far)).toBeLessThan(triangles(near));
    expect(near.boundingBox!.min.y).toBeGreaterThan(1.9);
    expect(near.boundingBox!.max.y).toBeLessThan(5.5);
    const crownWidth = near.boundingBox!.max.x - near.boundingBox!.min.x;
    const crownDepth = near.boundingBox!.max.z - near.boundingBox!.min.z;
    const crownHeight = near.boundingBox!.max.y - near.boundingBox!.min.y;
    expect(crownWidth).toBeGreaterThan(3.2);
    expect(crownDepth).toBeGreaterThan(3.0);
    expect(Math.max(crownWidth, crownDepth)).toBeGreaterThan(crownHeight);
    expect(near.getAttribute('uv').count).toBe(near.getAttribute('position').count);
    for (const detail of [near, far]) {
      const uv = detail.getAttribute('uv');
      for (let i = 0; i < uv.count; i++) {
        expect(uv.getX(i)).toBeGreaterThanOrEqual(0);
        expect(uv.getX(i)).toBeLessThanOrEqual(1);
        expect(uv.getY(i)).toBeGreaterThanOrEqual(0);
        expect(uv.getY(i)).toBeLessThanOrEqual(1);
      }
    }
    expectFinite(near);
    expectFinite(far);
    near.dispose();
    far.dispose();
  });

  it('uses deterministic neutral vertex colors across UV-mapped cards', () => {
    const first = createFoliageGeometry('near', 9281);
    const second = createFoliageGeometry('near', 9281);
    const other = createFoliageGeometry('near', 9282);
    const firstColors = first.getAttribute('color');
    const secondColors = second.getAttribute('color');
    const otherColors = other.getAttribute('color');
    expect(firstColors.count).toBe(first.getAttribute('position').count);
    expect(Array.from(firstColors.array)).toEqual(Array.from(secondColors.array));
    expect(Array.from(firstColors.array)).not.toEqual(Array.from(otherColors.array));

    for (let i = 0; i < firstColors.count; i++) {
      for (const component of [firstColors.getX(i), firstColors.getY(i), firstColors.getZ(i)])
        expect(Number.isFinite(component)).toBe(true);
    }
    const uv = first.getAttribute('uv');
    expect(uv.count).toBe(firstColors.count);
    const firstRgb = [firstColors.getX(0), firstColors.getY(0), firstColors.getZ(0)];
    expect(Math.min(...firstRgb)).toBeGreaterThan(0.1);
    expect(Math.max(...firstRgb) - Math.min(...firstRgb)).toBeLessThan(0.2);
    first.dispose();
    second.dispose();
    other.dispose();
  });

  it('builds a tapered, branched trunk in the authored 0–4.6m local range', () => {
    const trunk = createTreeTrunkGeometry();
    trunk.computeBoundingBox();
    expect(triangles(trunk)).toBeLessThan(250);
    expect(trunk.boundingBox!.min.y).toBeCloseTo(0, 4);
    expect(trunk.boundingBox!.max.y).toBeCloseTo(4.6, 2);
    expect(trunk.boundingBox!.max.x - trunk.boundingBox!.min.x).toBeGreaterThan(1.5);
    expect(trunk.boundingBox!.max.z - trunk.boundingBox!.min.z).toBeGreaterThan(1.3);
    expectFinite(trunk);
    trunk.dispose();
  });
});
