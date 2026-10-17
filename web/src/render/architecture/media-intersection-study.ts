import type { World, Building } from '../../types';
import type { MovementSupport } from '../../movement-support';
import { mediaLandmarkStudy } from './media-landmark-study';
/** Dense four-way review context; production renderer owns roads, crossings and sidewalks. */
export function mediaIntersectionStudy(seed = 32): World {
  const source = mediaLandmarkStudy(seed, true),
    hero = source.buildings[0];
  const lots = [
    [-45, 0, 25, 24, 8, 0],
    [-76, 0, 28, 25, 10, 0],
    [64, 0, 27, 25, 9, 0],
    [97, 0, 28, 25, 6, 0],
    [0, 72, 32, 24, 7, Math.PI],
    [65, 72, 27, 25, 11, Math.PI],
    [-43, 72, 28, 24, 5, Math.PI],
    [98, 72, 29, 25, 8, Math.PI],
  ];
  const buildings: Building[] = [
    hero,
    ...lots.map(([x, z, width, depth, floors, rotation], i) => ({
      ...hero,
      id: i + 2,
      parcel_id: i + 2,
      x,
      z,
      width,
      depth,
      floors,
      rotation,
      appearance_seed: (seed * 7919 + i * 104729) >>> 0,
      kind: i % 3 === 0 ? ('office' as const) : ('shop' as const),
      name: `Commercial corner tenant ${i + 1}`,
    })),
  ];
  const nodes = [
    { id: 1, x: 32, z: 32, kind: 'station' as const },
    { id: 2, x: -130, z: 32, kind: 'junction' as const },
    { id: 3, x: 155, z: 32, kind: 'junction' as const },
    { id: 4, x: 32, z: -110, kind: 'junction' as const },
    { id: 5, x: 32, z: 145, kind: 'junction' as const },
  ];
  const roads = nodes.slice(1).map((n, i) => ({
    id: i + 1,
    from: 1,
    to: n.id,
    kind: 'arterial' as const,
    stage: 'open' as const,
    progress: 1,
    connected: true,
    length: Math.hypot(n.x - 32, n.z - 32),
    lanes: 3,
  }));
  const blocks = [
    [-115, -65, 22, 22],
    [42, -65, 125, 22],
    [-115, 42, 22, 115],
    [42, 42, 125, 115],
  ];
  return {
    ...source,
    buildings,
    nodes,
    roads,
    bounds: { min_x: -145, max_x: 160, min_z: -115, max_z: 145 },
    ground: blocks.map(([x, z, xx, zz], block) => ({
      block,
      points: [
        [x, z],
        [xx, z],
        [xx, zz],
        [x, zz],
      ] as [number, number][],
    })),
    parcels: buildings.map((b) => ({
      id: b.id,
      x: b.x,
      z: b.z,
      width: b.width,
      depth: b.depth,
      rotation: b.rotation,
      access_node: 1,
      suitability: 1,
      protected: false,
      reservation: null,
    })),
    building_access: buildings.map((b) => ({
      building_id: b.id,
      road_id: b.x >= 42 ? 2 : 1,
      points: [
        { x: b.x, z: b.z + (b.rotation ? -b.depth / 2 : b.depth / 2) },
        { x: b.x, z: b.z < 32 ? 22 : 42 },
      ],
    })),
    stats: { ...source.stats, buildings: buildings.length, population: 120, active_travelers: 32 },
  };
}
/** Frozen review actors demonstrate scale; actual cities continue to use engine movement. */
export function mediaIntersectionActors(seed = 32) {
  const tuples: number[] = [],
    support = new Map<number, MovementSupport>();
  let id = 1;
  const add = (x: number, z: number, heading: number, kind: number, road: number, foot = 0.27) => {
    tuples.push(id, x, z, heading, kind, kind === 0 ? foot : -1);
    support.set(id, { resolved: true, roads: [road], platforms: [] });
    id++;
  };
  for (let i = 0; i < 24; i++) {
    const east = i >= 12,
      n = i % 12;
    add(
      east ? 45 + (n % 3) * 0.7 : -15 + n * 2.8,
      east ? -5 + Math.floor(n / 3) * 4 : 20 + (n % 3) * 0.45,
      east ? 0 : Math.PI / 2,
      0,
      east ? 3 : 1,
      east ? 0.27 : 0.04,
    );
  }
  for (let i = 0; i < 8; i++) add(14 + (i % 2) * 0.8, 26 + Math.floor(i / 2) * 3.5, 0, 0, 1, 0.08);
  add(-22, 29, Math.PI / 2, 5, 1);
  add(-52, 35, -Math.PI / 2, 6, 1);
  add(58, 29, Math.PI / 2, 1, 2);
  add(29, -17, 0, 4, 3);
  return { movement: new Float32Array(tuples), support };
}
