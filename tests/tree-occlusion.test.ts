import { describe, expect, it } from 'vitest';
import {
  buildingHidesTree,
  buildingTreeReach,
  treeCanopy,
  extensionHidesTree,
  roadHidesTree,
  treeOcclusion,
  type TreePoint,
} from '../web/src/render/tree-occlusion';
import type {
  Building,
  Point,
  RearExtensionView,
  Road,
  World,
  WorldNode,
  RoadKind,
} from '../web/src/types';

function random(seed: number) {
  return () => {
    seed ^= seed << 13;
    seed ^= seed >>> 17;
    seed ^= seed << 5;
    return (seed >>> 0) / 4294967296;
  };
}

function building(id: number, x: number, z: number, overrides: Partial<Building> = {}): Building {
  return {
    id,
    parcel_id: id,
    x,
    z,
    rotation: 0,
    width: 10,
    depth: 9,
    floors: 3,
    kind: 'shop' as const,
    stage: 'open',
    progress: 1,
    capacity: 8,
    occupancy: 4,
    business_active: false,
    built_day: 1,
    name: `b${id}`,
    appearance_seed: id,
    history: [],
    anchor: false,
    vacant_days: 0,
    ...overrides,
  };
}
function road(id: number, from: number, to: number, kind: RoadKind): Road {
  return { id, from, to, kind, stage: 'open', progress: 1, connected: true, length: 0, lanes: 2 };
}
function world(
  buildings: Building[],
  nodes: WorldNode[],
  roads: Road[],
  rear_extensions: RearExtensionView[] = [],
): World {
  return { buildings, nodes, roads, rear_extensions } as unknown as World;
}

/** The scan the grid replaced: every building, road and extension per tree. */
function bruteForce(w: World, p: TreePoint) {
  const nodes = new Map<number, Point>(w.nodes.map((n) => [n.id, n]));
  return (
    w.buildings.some((b) => buildingHidesTree(b, p)) ||
    w.roads.some((r) => roadHidesTree(r, nodes, p)) ||
    (w.rear_extensions ?? []).slice(0, 2).some((e) => extensionHidesTree(e, p))
  );
}

function randomWorld(seed: number) {
  const rand = random(seed);
  const span = 400;
  const at = () => (rand() - 0.5) * span;
  const buildings = Array.from({ length: 140 }, (_, id) =>
    building(id, at(), at(), {
      rotation: rand() * Math.PI * 2,
      width: 4 + rand() * 30,
      depth: 4 + rand() * 30,
      anchor: rand() < 0.15,
    }),
  );
  const nodes: WorldNode[] = Array.from({ length: 60 }, (_, id) => ({
    id,
    x: at(),
    z: at(),
    kind: 'junction',
  }));
  const kinds = [
    'lane',
    'local',
    'arterial',
    'bridge',
    'rail',
    'expressway',
    'collector',
  ] as RoadKind[];
  // Diagonal and long roads, plus a few with missing endpoints.
  const roads = Array.from({ length: 90 }, (_, id) =>
    road(id, Math.floor(rand() * 62), Math.floor(rand() * 60), kinds[id % kinds.length]),
  );
  const rear_extensions: RearExtensionView[] = [0, 1, 2].map((i) => ({
    project_id: i,
    building_id: i,
    x: at(),
    z: at(),
    width: 6 + rand() * 10,
    depth: 4 + rand() * 8,
    rotation: rand() * Math.PI,
    floors: 2,
    stage: i === 1 ? 'survey' : 'foundation',
    progress: 0.5,
  }));
  // Varied canopy sizes, many placed exactly on or just either side of 24 m cell lines.
  const points: TreePoint[] = Array.from({ length: 1500 }, (_, i) => {
    const s = 0.4 + rand() * 3.2;
    if (i % 3 === 0) {
      const line = Math.round(at() / 24) * 24,
        offset = [-1e-9, 0, 1e-9][((i % 9) / 3) | 0];
      return i % 2 ? { x: line + offset, z: at(), s } : { x: at(), z: line + offset, s };
    }
    return { x: at(), z: at(), s };
  });
  return { w: world(buildings, nodes, roads, rear_extensions), points };
}

describe('tree occlusion grid', () => {
  it('matches the full scan on random rotated buildings, diagonal roads and varied canopies', () => {
    for (const seed of [1, 7, 87231, 424242]) {
      const { w, points } = randomWorld(seed);
      const hides = treeOcclusion(w, points);
      const grid = points.map(hides),
        expected = points.map((p) => bruteForce(w, p));
      expect(grid).toEqual(expected);
      // Both visible and hidden trees are exercised.
      expect(expected.filter(Boolean).length).toBeGreaterThan(100);
      expect(expected.filter((hidden) => !hidden).length).toBeGreaterThan(100);
    }
  });

  it('registers each building for every tree position its test can hide', () => {
    const rand = random(5);
    for (let n = 0; n < 4000; n++) {
      const b = building(n, 0, 0, {
        rotation: rand() * Math.PI * 2,
        width: 2 + rand() * 40,
        depth: 2 + rand() * 40,
        anchor: rand() < 0.5,
      });
      const p = { x: 0, z: 0, s: 0.4 + rand() * 3.2 };
      // Sample right at the far corners of the local test box, rear reach included.
      const canopy = treeCanopy(p),
        c = Math.cos(b.rotation),
        s = Math.sin(b.rotation);
      const localX = (rand() < 0.5 ? -1 : 1) * (b.width / 2 + canopy) * (1 - 1e-9);
      const localZ =
        rand() < 0.5
          ? -(b.depth / 2 + canopy) * (1 - 1e-9)
          : (b.depth / 2 + canopy + (b.anchor ? 4 : 0)) * (1 - 1e-9);
      p.x = c * localX + s * localZ;
      p.z = -s * localX + c * localZ;
      expect(buildingHidesTree(b, p)).toBe(true);
      expect(Math.hypot(p.x, p.z)).toBeLessThanOrEqual(buildingTreeReach(b, canopy));
    }
  });

  it("reaches the rotated far corner of an anchor's rear frontage from another cell", () => {
    const rotation = 0.7,
      c = Math.cos(rotation),
      s = Math.sin(rotation);
    const anchor = building(1, 11, 11, { anchor: true, rotation, width: 14, depth: 12 });
    const big = { x: 0, z: 0, s: 3.6 },
      small = { x: 0, z: 0, s: 0.4 };
    const canopy = big.s * 1.7 + 2;
    // Just inside the far corner of the anchor's test box, including its 4 m rear reach.
    const localX = anchor.width / 2 + canopy - 0.01,
      localZ = anchor.depth / 2 + canopy + 4 - 0.01;
    // Inverse of buildingHidesTree's world→local rotation.
    const tree = {
      ...big,
      x: anchor.x + c * localX + s * localZ,
      z: anchor.z - s * localX + c * localZ,
    };
    expect(
      Math.floor(tree.x / 24) !== Math.floor(anchor.x / 24) ||
        Math.floor(tree.z / 24) !== Math.floor(anchor.z / 24),
    ).toBe(true);
    const w = world([anchor], [], []);
    // A small canopy elsewhere sets no smaller reach than the largest canopy present.
    const hides = treeOcclusion(w, [small, tree]);
    expect(bruteForce(w, tree)).toBe(true);
    expect(hides(tree)).toBe(true);
    const beyond = {
      ...big,
      x: anchor.x + c * localX + s * (localZ + 0.02),
      z: anchor.z - s * localX + c * (localZ + 0.02),
    };
    expect(hides(beyond)).toBe(false);
  });

  it('finds a diagonal road from a tree in a cell its endpoints never touch', () => {
    const nodes: WorldNode[] = [
      { id: 1, x: -100, z: -100, kind: 'junction' },
      { id: 2, x: 100, z: 100, kind: 'junction' },
    ];
    const w = world([], nodes, [road(1, 1, 2, 'arterial'), road(2, 2, 1, 'expressway')]);
    const onRoad = { x: 30, z: 30, s: 1 },
      off = { x: 30, z: 50, s: 1 };
    const hides = treeOcclusion(w, [onRoad, off]);
    expect(hides(onRoad)).toBe(true);
    expect(hides(off)).toBe(false);
  });
});
