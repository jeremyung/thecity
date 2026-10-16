import * as THREE from 'three/webgpu';
import { Kit } from './kit';
import type { ArtMaterials } from './materials';

export type GroundSurface = { block: number; points: [number, number][] };

/** Independent from infrastructure: a new building site must not rebuild roads. */
export function buildGroundSurfaces(
  patches: readonly GroundSurface[],
  art: ArtMaterials,
): THREE.Group {
  const kit = new Kit();
  const tones = ['#a7a59a', '#9d9c92', '#aaa69a', '#a19f95'];
  for (const patch of patches) {
    if (patch.points.length < 3) continue;
    const shape = new THREE.Shape(patch.points.map(([x, z]) => new THREE.Vector2(x, -z)));
    const geometry = new THREE.ShapeGeometry(shape);
    geometry.rotateX(-Math.PI / 2);
    geometry.translate(0, 0.035, 0);
    kit.add(geometry, art.get(tones[patch.block % tones.length]));
  }
  return kit.finish();
}
