import * as THREE from 'three/webgpu';
import type { Point } from '../types';

type RoadEnd = Point & { kind?: string };

/** A station access route turns at the road's terminal node. Continue the
 * carriageway through that turn by a pedestrian radius plus a small margin.
 * Ordinary ends and boundary gateways retain their existing footprint. */
export function groundRoadFootprint(a: RoadEnd, b: RoadEnd, width: number): Point[] {
  const dx = b.x - a.x,
    dz = b.z - a.z,
    length = Math.hypot(dx, dz);
  if (length < 1e-6) return [];
  const ux = dx / length,
    uz = dz / length;
  const nx = (-uz * width) / 2,
    nz = (ux * width) / 2;
  const start = a.kind === 'station' ? 0.4 : 0;
  const end = b.kind === 'station' ? 0.4 : 0;
  const ax = a.x - ux * start,
    az = a.z - uz * start;
  const bx = b.x + ux * end,
    bz = b.z + uz * end;
  return [
    { x: ax + nx, z: az + nz },
    { x: bx + nx, z: bz + nz },
    { x: bx - nx, z: bz - nz },
    { x: ax - nx, z: az - nz },
  ];
}

/** Shared mesh constructor keeps support checks on the production road datum. */
export function groundRoadSurfaceGeometry(points: readonly Point[]): THREE.BufferGeometry {
  const shape = new THREE.Shape(points.map((p) => new THREE.Vector2(p.x, -p.z)));
  const geometry = new THREE.ExtrudeGeometry(shape, { depth: 0.3, bevelEnabled: false, steps: 1 });
  geometry.setIndex(Array.from({ length: geometry.getAttribute('position').count }, (_, i) => i));
  geometry.rotateX(-Math.PI / 2);
  return geometry;
}
