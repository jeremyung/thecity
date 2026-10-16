import { describe, it, expect } from 'vitest';
import * as THREE from 'three/webgpu';
import {
  groundRoadFootprint,
  groundRoadSurfaceGeometry,
} from '../web/src/render/ground-road-footprint';

describe('station road end landing', () => {
  it('supports the complete turning body at either station end and both street sides', () => {
    for (const [ux, uz] of [
      [1, 0],
      [0, 1],
      [0.6, 0.8],
    ])
      for (const reverse of [false, true]) {
        const station = { x: 12, z: -8, kind: 'station' },
          street = { x: 12 + ux * 11, z: -8 + uz * 11, kind: 'local' };
        const points = groundRoadFootprint(
          reverse ? street : station,
          reverse ? station : street,
          6.5,
        );
        const geometry = groundRoadSurfaceGeometry(points);
        const material = new THREE.MeshBasicMaterial({ side: THREE.DoubleSide });
        const mesh = new THREE.Mesh(geometry, material);
        mesh.updateMatrixWorld(true);
        for (const side of [-1, 1])
          for (let angle = 0; angle < Math.PI * 2; angle += Math.PI / 16) {
            const x = station.x - uz * 2.53 * side + 0.33 * Math.cos(angle),
              z = station.z + ux * 2.53 * side + 0.33 * Math.sin(angle);
            const hit = new THREE.Raycaster(
              new THREE.Vector3(x, 1, z),
              new THREE.Vector3(0, -1, 0),
            ).intersectObject(mesh)[0];
            expect(hit?.point.y).toBeCloseTo(0.3, 5);
          }
        geometry.dispose();
        material.dispose();
      }
  });
  it('preserves ordinary and boundary gateway endpoints', () => {
    for (const kind of ['local', 'gateway', undefined]) {
      const points = groundRoadFootprint({ x: 0, z: 0, kind }, { x: 0, z: 10, kind }, 4);
      expect(Math.min(...points.map((p) => p.z))).toBe(0);
      expect(Math.max(...points.map((p) => p.z))).toBe(10);
    }
  });
});
