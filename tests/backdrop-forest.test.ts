import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { createBackdropGeometry } from '../web/src/render/backdrop';
import { backdropForestMatrices, backdropHeightAt } from '../web/src/render/backdrop-forest';

describe('backdrop forest', () => {
  const geometry = createBackdropGeometry(87231, [{ x: 480, z: -1700 }, { x: 480, z: 1700 }], 32);
  it('samples the heightfield exactly at grid vertices', () => {
    const p = geometry.getAttribute('position');
    for (const i of [0, 500, 4321, p.count - 1]) expect(backdropHeightAt(geometry, p.getX(i), p.getZ(i))).toBeCloseTo(p.getY(i), 4);
  });
  it('plants deterministic crowns only on hills outside the city basin', () => {
    const trees = backdropForestMatrices(geometry, 87231);
    expect(backdropForestMatrices(geometry, 87231).map((m) => m.elements)).toEqual(trees.map((m) => m.elements));
    expect(trees.length).toBeGreaterThan(1000);
    expect(trees.length).toBeLessThan(20000);
    const position = new THREE.Vector3();
    for (const matrix of trees) {
      position.setFromMatrixPosition(matrix);
      expect(Math.max(Math.abs(position.x), Math.abs(position.z))).toBeGreaterThan(470);
      expect(backdropHeightAt(geometry, position.x, position.z)).toBeGreaterThanOrEqual(30);
    }
  });
});
