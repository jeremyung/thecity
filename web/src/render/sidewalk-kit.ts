import * as THREE from 'three/webgpu';
import type { Point, World } from '../types';
import type { ArtMaterials } from './materials';
import { Kit } from './kit';
import { arterialSidewalkPolygons } from './arterial-sidewalks';

/** Offline synchronous reference kit; CityRenderer uses CitySidewalks/SidewalkBuild.
 * Shared height with arterial slabs and doorway landings; only open roads emit. */
export function addArterialSidewalks(
  kit: Kit,
  world: Pick<World, 'nodes' | 'roads'>,
  art: ArtMaterials,
): void {
  const material = art.get('#b4b8a6');
  for (const polygon of arterialSidewalkPolygons(world)) {
    const geometry = sidewalkGeometry(polygon);
    kit.add(geometry, material);
  }
}

/** One convex fragment; exported for staged construction and stage-cost measurement. */
export function sidewalkGeometry(polygon: readonly Point[]): THREE.BufferGeometry {
  const shape = new THREE.Shape(polygon.map((p) => new THREE.Vector2(p.x, -p.z)));
  const geometry = new THREE.ExtrudeGeometry(shape, { depth: 0.35, bevelEnabled: false, steps: 1 });
  // Kit's other primitives are indexed; retain that shared batching contract.
  geometry.setIndex(Array.from({ length: geometry.getAttribute('position').count }, (_, i) => i));
  geometry.rotateX(-Math.PI / 2);
  geometry.translate(0, 0.125, 0);
  return geometry;
}
