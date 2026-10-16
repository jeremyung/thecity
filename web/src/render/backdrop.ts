import * as THREE from 'three/webgpu';

export interface BackdropRiverPoint {
  x: number;
  z: number;
}

const CELLS = 96;
const EXTENT = 1700;
const BASIN_EDGE = 420;
const HILLS_BEGIN = 450;
const HILLS_FULL = 1000;
const MAX_RELIEF = 140;

function smoothstep(edge0: number, edge1: number, value: number): number {
  const t = Math.max(0, Math.min(1, (value - edge0) / (edge1 - edge0)));
  return t * t * (3 - 2 * t);
}

function seedPhase(seed: number, salt: number): number {
  let value = (seed | 0) ^ salt;
  value = Math.imul(value ^ (value >>> 16), 0x7feb352d);
  value = Math.imul(value ^ (value >>> 15), 0x846ca68b);
  value = (value ^ (value >>> 16)) >>> 0;
  return (value / 0x1_0000_0000) * Math.PI * 2;
}

/**
 * Creates a finite, visual-only ring of distant hills around the simulation's
 * flat basin. It is intentionally not connected to parcels or world state.
 */
export function createBackdropGeometry(
  seed: number,
  river: ReadonlyArray<BackdropRiverPoint>,
  halfWidth: number,
): THREE.BufferGeometry {
  const phases = [
    seedPhase(seed, 0x13579bdf),
    seedPhase(seed, 0x2468ace0),
    seedPhase(seed, 0x51ed270b),
    seedPhase(seed, 0x6c8e9cf5),
  ];
  const usableRiver = river.filter((point) => Number.isFinite(point.x) && Number.isFinite(point.z));
  const riverWidth = Number.isFinite(halfWidth) ? Math.max(0, halfWidth) + 12 : 12;

  const riverXAt = (z: number): number | null => {
    if (usableRiver.length === 0) return null;
    if (usableRiver.length === 1) return usableRiver[0].x;
    const first = usableRiver[0];
    const last = usableRiver[usableRiver.length - 1];
    const increasing = last.z >= first.z;
    if ((increasing && z <= first.z) || (!increasing && z >= first.z)) return first.x;
    if ((increasing && z >= last.z) || (!increasing && z <= last.z)) return last.x;

    for (let i = 0; i < usableRiver.length - 1; i += 1) {
      const a = usableRiver[i];
      const b = usableRiver[i + 1];
      const lo = Math.min(a.z, b.z);
      const hi = Math.max(a.z, b.z);
      if (z < lo || z > hi) continue;
      const dz = b.z - a.z;
      if (Math.abs(dz) < 1e-8) return (a.x + b.x) * 0.5;
      const t = (z - a.z) / dz;
      return a.x + (b.x - a.x) * t;
    }

    // For a non-monotone route with a gap in its z projection, use the closest
    // endpoint rather than inventing a distant, discontinuous valley.
    let closest = usableRiver[0];
    for (const point of usableRiver) {
      if (Math.abs(point.z - z) < Math.abs(closest.z - z)) closest = point;
    }
    return closest.x;
  };

  const hillHeightAt = (x: number, z: number): number => {
    const broad = Math.sin(x * 0.0032 + phases[0]) * Math.cos(z * 0.0025 + phases[1]);
    const crossRidge = Math.sin((x + z * 0.31) * 0.0045 + phases[2]);
    const fine = Math.sin(x * 0.009 + z * 0.006 + phases[3]);
    const height = 73 + broad * 32 + crossRidge * 19 + fine * 8;
    return Math.max(0, Math.min(MAX_RELIEF, height));
  };

  const vertexCount = (CELLS + 1) * (CELLS + 1);
  const positions = new Float32Array(vertexCount * 3);
  const indices = new Uint16Array(CELLS * CELLS * 6);
  const spacing = (EXTENT * 2) / CELLS;

  for (let row = 0; row <= CELLS; row += 1) {
    const z = -EXTENT + row * spacing;
    for (let column = 0; column <= CELLS; column += 1) {
      const x = -EXTENT + column * spacing;
      const vertex = row * (CELLS + 1) + column;
      const offset = vertex * 3;
      const edgeDistance = Math.max(Math.abs(x), Math.abs(z));
      let elevation = -3;

      // The city domain remains level. Beyond it, terrain rises progressively
      // into broad seeded hills with a continuous bank around the river.
      if (edgeDistance > BASIN_EDGE) {
        const basinBlend = smoothstep(HILLS_BEGIN, HILLS_FULL, edgeDistance);
        const nearestRiverX = riverXAt(z);
        const bankBlend =
          nearestRiverX === null
            ? 1
            : smoothstep(riverWidth, riverWidth + 220, Math.abs(x - nearestRiverX));
        elevation += hillHeightAt(x, z) * basinBlend * bankBlend;
      }

      positions[offset] = x;
      positions[offset + 1] = elevation;
      positions[offset + 2] = z;
    }
  }

  let index = 0;
  for (let row = 0; row < CELLS; row += 1) {
    for (let column = 0; column < CELLS; column += 1) {
      const a = row * (CELLS + 1) + column;
      const b = a + 1;
      const c = a + CELLS + 1;
      const d = c + 1;
      // Counter-clockwise from above: positive Y normals.
      indices[index++] = a;
      indices[index++] = c;
      indices[index++] = b;
      indices[index++] = b;
      indices[index++] = c;
      indices[index++] = d;
    }
  }

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  geometry.setIndex(new THREE.BufferAttribute(indices, 1));
  geometry.computeVertexNormals();
  geometry.computeBoundingBox();
  geometry.computeBoundingSphere();
  return geometry;
}
