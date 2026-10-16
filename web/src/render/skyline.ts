import * as THREE from 'three/webgpu';

export interface SkylineRiverPoint {
  x: number;
  z: number;
}

const INNER_RADIUS = 1800;
const OUTER_RADIUS = 2350;
const CLUSTERS = 6;
const TOWERS_PER_CLUSTER = 48;
const RIVER_CLEARANCE = 260;

function mulberry(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 0x1_0000_0000;
  };
}

function riverXAt(river: ReadonlyArray<SkylineRiverPoint>, z: number): number | null {
  if (river.length === 0) return null;
  const sorted = [...river].sort((a, b) => a.z - b.z);
  if (z <= sorted[0].z) return sorted[0].x;
  for (let i = 0; i < sorted.length - 1; i += 1) {
    const a = sorted[i],
      b = sorted[i + 1];
    if (z <= b.z) return b.z === a.z ? a.x : a.x + ((b.x - a.x) * (z - a.z)) / (b.z - a.z);
  }
  return sorted[sorted.length - 1].x;
}

/**
 * Visual-only distant metropolis beyond the scenic hills (spec 02: inexpensive
 * silhouettes outside the domain). Seeded tower clusters give the horizon the
 * skyline the concepts show; nothing here has parcels, routing or growth.
 * Positions are world coordinates so the material can derive window cells.
 */
export function createSkylineGeometry(
  seed: number,
  river: ReadonlyArray<SkylineRiverPoint>,
): THREE.BufferGeometry {
  const random = mulberry(seed ^ 0x5a17c3e1);
  const positions: number[] = [];
  const indices: number[] = [];
  const box = (x: number, z: number, width: number, depth: number, height: number, yaw: number) => {
    const base = positions.length / 3;
    const c = Math.cos(yaw),
      s = Math.sin(yaw);
    for (const y of [-8, height]) {
      for (const [dx, dz] of [
        [-1, -1],
        [1, -1],
        [1, 1],
        [-1, 1],
      ]) {
        const lx = (dx * width) / 2,
          lz = (dz * depth) / 2;
        positions.push(x + lx * c - lz * s, y, z + lx * s + lz * c);
      }
    }
    // Four walls and a roof; the base is never visible.
    for (let i = 0; i < 4; i += 1) {
      const j = (i + 1) % 4;
      indices.push(base + i, base + j, base + 4 + j, base + i, base + 4 + j, base + 4 + i);
    }
    indices.push(base + 4, base + 6, base + 5, base + 4, base + 7, base + 6);
  };

  // At 2 km the depth buffer resolves only ~0.5 m, so overlapping towers
  // z-fight into stripes. Keep footprints apart (bounding circles).
  const placed: Array<{ x: number; z: number; r: number }> = [];
  const start = random() * Math.PI * 2;
  for (let cluster = 0; cluster < CLUSTERS; cluster += 1) {
    const centre = start + (cluster * Math.PI * 2) / CLUSTERS + (random() - 0.5) * 0.7;
    const spread = 0.2 + random() * 0.16;
    // Peaks must clear the 100-140 m hill crest at 1.7 km from street height.
    const peak = 190 + random() * 120;
    for (let i = 0; i < TOWERS_PER_CLUSTER; i += 1) {
      const offset = (random() + random() + random() - 1.5) / 1.5;
      const angle = centre + offset * spread;
      const radius = INNER_RADIUS + random() * (OUTER_RADIUS - INNER_RADIUS);
      let x = Math.cos(angle) * radius;
      const z = Math.sin(angle) * radius;
      // Keep the river's vista open by pushing towers to its banks, so a
      // cluster straddling the valley still frames it rather than vanishing.
      const riverX = riverXAt(river, z);
      if (riverX !== null && Math.abs(x - riverX) < RIVER_CLEARANCE) {
        // Spread displaced towers over a wide band (and drop half of them):
        // stacking them on one line made coplanar walls z-fight as a stripe.
        if (random() < 0.5) continue;
        x = riverX + Math.sign(x - riverX || 1) * (RIVER_CLEARANCE + random() * 520);
      }
      // Taller towers gather at each cluster's centre, as around a terminal.
      const core = 1 - Math.abs(offset);
      const height = 30 + peak * core * core * (0.45 + random() * 0.55) + random() * 22;
      const width = 14 + random() * 22,
        depth = width * (0.6 + random() * 0.6);
      const r = Math.hypot(width, depth) / 2 + 1;
      if (placed.some((p) => Math.hypot(p.x - x, p.z - z) < p.r + r)) continue;
      placed.push({ x, z, r });
      box(x, z, width, depth, height, angle + (random() - 0.5) * 0.5);
    }
  }

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geometry.setIndex(indices);
  geometry.computeVertexNormals();
  geometry.computeBoundingBox();
  geometry.computeBoundingSphere();
  return geometry;
}
