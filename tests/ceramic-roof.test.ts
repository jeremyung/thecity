import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { addCeramicRoofDetail } from '../web/src/render/ceramic-roof';
import { Kit } from '../web/src/render/kit';

function geometries(kit: Kit) {
  return (kit as unknown as { parts: Map<THREE.Material, THREE.BufferGeometry[]> }).parts;
}

describe('ceramic gable roof detail', () => {
  it('puts sampled indexed rolls on both slopes with the ridge above both eaves', () => {
    const kit = new Kit();
    const roof = new THREE.MeshStandardMaterial();
    const tile = new THREE.MeshStandardMaterial();
    addCeramicRoofDetail(kit, { width: 12, halfDepth: 6, rise: 1.5, top: 8, roof, tile });

    const rolls = geometries(kit).get(tile)!.filter(geometry => geometry.getAttribute('position').count === 10);
    expect(rolls.length).toBeGreaterThanOrEqual(40);
    const sides = new Set<number>();
    for (const geometry of rolls) {
      expect(geometry.index).not.toBeNull();
      expect(geometry.getAttribute('normal').count).toBe(10);
      expect(geometry.getAttribute('uv').count).toBe(10);
      const position = geometry.getAttribute('position');
      const centerAtRidgeY = position.getY(2);
      const centerAtEaveY = position.getY(7);
      const centerAtRidgeZ = position.getZ(2);
      const centerAtEaveZ = position.getZ(7);
      expect(centerAtRidgeY).toBeGreaterThan(centerAtEaveY);
      expect(Math.abs(centerAtEaveZ)).toBeGreaterThan(Math.abs(centerAtRidgeZ));
      sides.add(Math.sign(centerAtEaveZ));
      for (let vertex = 0; vertex < position.count; vertex++) {
        expect(position.getX(vertex)).toBeGreaterThanOrEqual(-6.001);
        expect(position.getX(vertex)).toBeLessThanOrEqual(6.001);
        expect(Math.abs(position.getZ(vertex))).toBeLessThanOrEqual(6.001);
        expect(position.getY(vertex)).toBeLessThanOrEqual(9.72);
        for (const value of [position.getX(vertex), position.getY(vertex), position.getZ(vertex), geometry.getAttribute('normal').getX(vertex), geometry.getAttribute('normal').getY(vertex), geometry.getAttribute('normal').getZ(vertex)]) {
          expect(Number.isFinite(value)).toBe(true);
        }
        expect(geometry.getAttribute('normal').getY(vertex)).toBeGreaterThan(0);
      }
      const slope = Math.atan2(1.5, 6);
      for (const along of [0, 1]) {
        const center = along * 5 + 2;
        const fraction = along;
        const baseY = 8 + 1.5 * (1 - fraction * (Math.hypot(6, 1.5) - (0.13 + 0.065) * Math.tan(slope)) / Math.hypot(6, 1.5));
        const baseZ = Math.sign(position.getZ(center)) * 6 * fraction * (Math.hypot(6, 1.5) - (0.13 + 0.065) * Math.tan(slope)) / Math.hypot(6, 1.5);
        const normalDistance = (position.getY(center) - baseY) * Math.cos(slope) + (position.getZ(center) - baseZ) * Math.sign(position.getZ(center)) * Math.sin(slope);
        expect(normalDistance).toBeGreaterThanOrEqual(0.129);
      }
    }
    expect(sides).toEqual(new Set([-1, 1]));
  });

  it('keeps the large roof under the geometry budget and skips invalid dimensions', () => {
    const kit = new Kit();
    const roof = new THREE.MeshStandardMaterial();
    const tile = new THREE.MeshStandardMaterial();
    addCeramicRoofDetail(kit, { width: 20, halfDepth: 6, rise: 1.8, top: 10, roof, tile });
    let triangles = 0;
    for (const parts of geometries(kit).values()) {
      for (const geometry of parts) triangles += (geometry.index?.count ?? geometry.getAttribute('position').count) / 3;
    }
    expect(triangles).toBeLessThanOrEqual(850);

    const invalid = new Kit();
    addCeramicRoofDetail(invalid, { width: 0, halfDepth: 6, rise: 1.8, top: 10, roof, tile });
    expect(geometries(invalid).size).toBe(0);
  });
});
