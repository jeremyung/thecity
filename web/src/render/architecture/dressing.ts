import * as THREE from 'three/webgpu';
import type { Kit } from './geometry-kit';
import { architecturalChoice } from './variation';

/** A restrained household profile; empty balconies are deliberately common. */
export function householdDressing(seed: number, floor: number, bay: number) {
  const value = architecturalChoice(seed, 61, floor * 131 + bay);
  return {
    laundry: floor % 3 === architecturalChoice(seed, 63, bay) % 3 && value % 2 === 0,
    items: 1 + ((value >>> 8) % 4),
    offset: (((value >>> 13) % 7) - 3) * 0.07,
    storage: (value >>> 18) % 9 === 0,
    plants: (value >>> 23) % 5 === 0,
    variant: value >>> 3,
  };
}
export function hangingLaundry(
  kit: Kit,
  x: number,
  y: number,
  z: number,
  profile: ReturnType<typeof householdDressing>,
  fabric: THREE.Material[],
  metal: THREE.Material,
) {
  const width = 1.12;
  kit.box(x, y, z, width, 0.025, 0.025, metal);
  for (const dx of [-width / 2, width / 2])
    kit.box(x + dx, y + 0.09, z - 0.15, 0.025, 0.2, 0.32, metal);
  for (let j = 0; j < profile.items; j++) {
    const v = architecturalChoice(profile.variant, 62, j),
      cx = x - width * 0.4 + (j * width * 0.8) / Math.max(1, profile.items - 1),
      cloth = fabric[v % fabric.length];
    const kind = (v >>> 4) % 3,
      length = 0.32 + ((v >>> 9) % 6) * 0.065;
    // Towels, shirts with sleeves, and split trouser legs have different silhouettes.
    if (kind === 0)
      kit.box(cx, y - length / 2, z, 0.22 + ((v >>> 16) % 3) * 0.055, length, 0.025, cloth);
    if (kind === 1) {
      kit.box(cx, y - 0.19, z, 0.27, 0.38, 0.035, cloth);
      kit.box(cx - 0.18, y - 0.075, z, 0.13, 0.15, 0.035, cloth);
      kit.box(cx + 0.18, y - 0.075, z, 0.13, 0.15, 0.035, cloth);
    }
    if (kind === 2)
      for (const dx of [-0.072, 0.072])
        kit.box(cx + dx, y - length / 2, z, 0.11, length, 0.03, cloth);
    for (const dx of [-0.07, 0.07]) kit.box(cx + dx, y + 0.015, z, 0.015, 0.045, 0.04, metal);
  }
}
/** Ground-level cycle parking, with a narrow depth footprint and shared materials. */
export function parkedBicycle(
  kit: Kit,
  x: number,
  y: number,
  z: number,
  metal: THREE.Material,
  rubber: THREE.Material,
  frame: THREE.Material,
) {
  const bar = (a: number[], b: number[], r: number, mat: THREE.Material) => {
    const start = new THREE.Vector3(...a),
      end = new THREE.Vector3(...b),
      delta = end.clone().sub(start);
    const geo = new THREE.CylinderGeometry(r, r, delta.length(), 5);
    geo.applyQuaternion(
      new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0, 1, 0), delta.normalize()),
    );
    geo.translate(...start.add(end).multiplyScalar(0.5).toArray());
    kit.add(geo, mat);
  };
  kit.withOffset(x, y, z, () => {
    for (const wx of [-0.53, 0.53]) {
      const wheel = new THREE.TorusGeometry(0.31, 0.025, 5, 16);
      wheel.translate(wx, 0.33, 0);
      kit.add(wheel, rubber);
      for (let k = 0; k < 6; k++) {
        const a = (k * Math.PI) / 3;
        bar([wx, 0.33, 0], [wx + Math.cos(a) * 0.29, 0.33 + Math.sin(a) * 0.29, 0], 0.006, metal);
      }
    }
    const a = [-0.53, 0.33, 0],
      b = [-0.12, 0.35, 0],
      c = [-0.24, 0.85, 0],
      d = [0.32, 0.85, 0],
      e = [0.53, 0.33, 0];
    for (const [p, q] of [
      [a, b],
      [a, c],
      [b, c],
      [b, d],
      [c, d],
      [d, e],
    ])
      bar(p, q, 0.023, frame);
    bar(c, [-0.27, 0.97, 0], 0.022, metal);
    kit.box(-0.27, 0.98, 0, 0.29, 0.055, 0.15, rubber);
    bar(d, [0.29, 1.04, 0], 0.02, metal);
    bar([0.29, 1.04, -0.2], [0.29, 1.04, 0.2], 0.018, metal);
    kit.box(0.48, 0.87, 0, 0.31, 0.24, 0.28, metal);
    kit.box(0.48, 0.91, 0, 0.27, 0.17, 0.24, rubber);
    bar([-0.12, 0.35, 0], [-0.12, 0.35, 0.15], 0.025, metal);
    kit.box(-0.12, 0.35, 0.17, 0.14, 0.04, 0.08, rubber);
  });
}
