import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { Kit, disposeGroup } from '../web/src/render/kit';
import { addExpresswayFurniture } from '../web/src/render/expressway-kit';
import {
  CURRENT_EXPRESSWAY_PROFILE,
  COMPACT_EXPRESSWAY_PROFILE,
} from '../web/src/render/expressway-profile';

describe('expressway deck furniture', () => {
  it('keeps paint and lamp supports on the deck across rotated and reversed segments', () => {
    const materials = {
      paint: new THREE.MeshBasicMaterial(),
      concrete: new THREE.MeshBasicMaterial(),
      metal: new THREE.MeshBasicMaterial(),
      lamp: new THREE.MeshBasicMaterial(),
    };
    try {
      for (const profile of [CURRENT_EXPRESSWAY_PROFILE, COMPACT_EXPRESSWAY_PROFILE])
        for (const deckHeight of [11, 20])
          for (const [a, b] of [
            [
              { x: 20, z: -30 },
              { x: 80, z: 50 },
            ],
            [
              { x: 80, z: 50 },
              { x: 20, z: -30 },
            ],
          ]) {
            const kit = new Kit();
            addExpresswayFurniture(kit, a, b, materials, deckHeight, undefined, profile);
            const group = kit.finish();
            const dx = b.x - a.x,
              dz = b.z - a.z,
              length = Math.hypot(dx, dz),
              surface = deckHeight + 0.15;
            let lamps = 0,
              paint = 0;
            group.traverse((object) => {
              if (!(object instanceof THREE.Mesh)) return;
              const positions = object.geometry.getAttribute('position');
              for (let i = 0; i < positions.count; i++) {
                const x = positions.getX(i) - a.x,
                  z = positions.getZ(i) - a.z,
                  y = positions.getY(i);
                expect(Math.abs((x * dz - z * dx) / length)).toBeLessThan(profile.width / 2);
                expect((x * dx + z * dz) / length).toBeGreaterThanOrEqual(-0.0001);
                expect((x * dx + z * dz) / length).toBeLessThanOrEqual(length + 0.0001);
                expect(y).toBeGreaterThanOrEqual(surface - 0.0001);
                if (object.material === materials.paint) {
                  expect(y).toBeLessThan(surface + 0.05);
                  paint++;
                }
                if (object.material === materials.lamp) {
                  expect(y).toBeGreaterThan(surface + 6);
                  lamps++;
                }
              }
            });
            expect(paint).toBeGreaterThan(0);
            expect(lamps).toBeGreaterThan(0);
            disposeGroup(group);
          }
    } finally {
      Object.values(materials).forEach((material) => material.dispose());
    }
  });
});
