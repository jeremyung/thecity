import * as THREE from 'three/webgpu';
import type { Kit } from './kit';

export interface CeramicRoofDetailOptions {
  width: number;
  halfDepth: number;
  rise: number;
  top: number;
  roof: THREE.Material;
  tile: THREE.Material;
  surfaceOffset?: number;
}

/** Adds bounded, low-poly ceramic rolls and course lips to a pitched gable. */
export function addCeramicRoofDetail(kit: Kit, { width, halfDepth, rise, top, roof, tile, surfaceOffset = 0.13 }: CeramicRoofDetailOptions): void {
  if (![width, halfDepth, rise, top, surfaceOffset].every(Number.isFinite) || width <= 0 || halfDepth <= 0 || rise < 0 || surfaceOffset < 0) return;

  const radius = 0.065;
  const angle = Math.atan2(rise, halfDepth);
  const slopeLength = Math.hypot(halfDepth, rise);
  const rolls = Math.min(32, Math.max(1, Math.floor(width / 0.5)));
  const courseCount = Math.min(12, Math.max(1, Math.floor(slopeLength / 0.6)));
  const longitudinalSegments = 1;
  const arcSegments = 4;
  const rollLength = Math.max(0, slopeLength - (surfaceOffset + radius) * Math.tan(angle));

  // Each indexed roll is an upper half-cylinder, sampled at five points around
  // its arc at each end. Its center follows the pitched plane.
  for (const side of [-1, 1] as const) {
    for (let roll = 0; roll < rolls; roll++) {
      const x = -width / 2 + width * (roll + 0.5) / rolls;
      const positions: number[] = [];
      const uvs: number[] = [];
      const indices: number[] = [];
      for (let along = 0; along <= longitudinalSegments; along++) {
        const distance = rollLength * along / longitudinalSegments;
        const planeY = top + rise * (1 - distance / slopeLength);
        for (let arc = 0; arc <= arcSegments; arc++) {
          const theta = Math.PI * arc / arcSegments;
          const across = radius * Math.cos(theta);
          const lift = radius * Math.sin(theta);
          positions.push(x + across, planeY + (surfaceOffset + lift) * Math.cos(angle), side * (halfDepth * distance / slopeLength + (surfaceOffset + lift) * Math.sin(angle)));
          uvs.push(along / longitudinalSegments, arc / arcSegments);
        }
      }
      const row = arcSegments + 1;
      for (let along = 0; along < longitudinalSegments; along++) {
        for (let arc = 0; arc < arcSegments; arc++) {
          const a = along * row + arc, b = a + row;
          if (side === 1) indices.push(a, a + 1, b, b, a + 1, b + 1);
          else indices.push(a, b, a + 1, b, b + 1, a + 1);
        }
      }
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
      geometry.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
      geometry.setIndex(indices);
      geometry.computeVertexNormals();
      kit.add(geometry, tile);
    }

    // Narrow raised lips read as overlapping tile courses without adding a
    // dense mesh or extending beyond the eave and ridge bounds.
    for (let course = 0; course < courseCount; course++) {
      const lipDepth = Math.min(0.055, slopeLength / (courseCount * 2));
      const endLimit = slopeLength - (surfaceOffset + lipDepth / 2) * Math.tan(angle);
      const distance = Math.min(endLimit, Math.max(0, slopeLength * (course + 0.5) / courseCount));
      const lipLift = surfaceOffset + 0.025;
      const y = top + rise * (1 - distance / slopeLength) + lipLift * Math.cos(angle);
      const z = side * (halfDepth * distance / slopeLength + lipLift * Math.sin(angle));
      const lip = new THREE.BoxGeometry(width, 0.035, lipDepth);
      lip.rotateX(side * angle);
      lip.translate(0, y, z);
      kit.add(lip, course % 3 === 0 ? roof : tile);
    }
  }
}
