import { describe, expect, it } from 'vitest';
import {
  findSkylineCluster,
  skylineClusterKey,
  type SkylineCluster,
} from '../web/src/camera/director';
import { SkylineClusterCache } from '../web/src/camera/skyline-cluster';
import type { Building } from '../web/src/types';

function random(seed: number) {
  return () => {
    seed ^= seed << 13;
    seed ^= seed >>> 17;
    seed ^= seed << 5;
    return (seed >>> 0) / 4294967296;
  };
}

function building(
  id: number,
  x: number,
  z: number,
  floors: number,
  overrides: Partial<Building> = {},
): Building {
  return {
    id,
    parcel_id: id,
    x,
    z,
    rotation: 0,
    width: 12,
    depth: 10,
    floors,
    kind: 'office',
    stage: 'open',
    progress: 1,
    capacity: 8,
    occupancy: 4,
    built_day: 1,
    name: `b${id}`,
    appearance_seed: id,
    history: [],
    anchor: false,
    vacant_days: 0,
    ...overrides,
  };
}

/** The original O(n²) search the grid replaced. */
function reference(buildings: Building[]): SkylineCluster | null {
  let best: SkylineCluster | null = null;
  for (const seed of buildings) {
    const local = buildings.filter(
      (building) => Math.hypot(building.x - seed.x, building.z - seed.z) <= 90,
    );
    const floors = Math.max(...local.map((building) => building.floors));
    if (floors < 5) continue;
    const towers = local.filter((building) => building.floors >= Math.max(5, floors * 0.68));
    if (towers.length === 0) continue;
    const totalWeight = towers.reduce((sum, building) => sum + Math.max(1, building.floors), 0);
    const x =
      towers.reduce((sum, building) => sum + building.x * Math.max(1, building.floors), 0) /
      totalWeight;
    const z =
      towers.reduce((sum, building) => sum + building.z * Math.max(1, building.floors), 0) /
      totalWeight;
    const spread = Math.max(
      ...towers.map(
        (building) =>
          Math.hypot(building.x - x, building.z - z) +
          Math.hypot(building.width, building.depth) / 2,
      ),
    );
    const score = floors * 3 + towers.length * 1.2 + Math.min(local.length, 18) * 0.15;
    const frameBuildings = local.filter((building) => building.floors >= Math.max(5, floors * 0.4));
    if (!best || score > best.score) best = { x, z, floors, spread, score, towers, frameBuildings };
  }
  return best;
}

function expectSameCluster(actual: SkylineCluster | null, expected: SkylineCluster | null) {
  expect(actual).toEqual(expected);
  if (!actual || !expected) return;
  // Same building objects in the same world order, not just equal values.
  expect(actual.towers.length).toBe(expected.towers.length);
  actual.towers.forEach((b, i) => expect(b).toBe(expected.towers[i]));
  actual.frameBuildings.forEach((b, i) => expect(b).toBe(expected.frameBuildings[i]));
}

describe('skyline cluster search', () => {
  it('matches the full scan on random cities, including negative coordinates and cell edges', () => {
    for (const seed of [3, 11, 87231, 99991]) {
      const rand = random(seed);
      const buildings = Array.from({ length: 400 }, (_, id) => {
        // A third sit exactly on 90 m grid lines, where cell lookups are most fragile.
        const snap = id % 3 === 0;
        const x = snap ? Math.round((rand() - 0.5) * 12) * 90 : (rand() - 0.5) * 1100;
        const z = snap ? (rand() - 0.5) * 1100 : Math.round((rand() - 0.5) * 12) * 90;
        return building(id, x, z, 1 + Math.floor(rand() ** 3 * 30), {
          width: 6 + rand() * 30,
          depth: 6 + rand() * 30,
        });
      });
      const expected = reference(buildings);
      expectSameCluster(findSkylineCluster(buildings), expected);
      expectSameCluster(new SkylineClusterCache().get(buildings), expected);
    }
  });

  it('reuses the same cached result for framing-equivalent buildings and invalidates on every framing input', () => {
    const fields: Partial<Building>[] = [
      { id: 9 },
      { x: 2 },
      { z: 3 },
      { floors: 9 },
      { width: 20 },
      { depth: 17 },
      { rotation: 0.5 },
      { kind: 'apartment' },
      { anchor: true },
    ];
    for (const field of fields) {
      const cache = new SkylineClusterCache();
      const original = [building(1, 0, 0, 8), building(2, 20, 0, 8)];
      const first = cache.get(original);
      expect(
        cache.get(original.map((item) => ({ ...item, name: 'different', occupancy: 99 }))),
      ).toBe(first);
      const changed = original.map((item, index) => (index === 0 ? { ...item, ...field } : item));
      const next = cache.get(changed);
      expect(next).not.toBe(first);
      expect(next).toEqual(new SkylineClusterCache().get(changed));
    }
  });

  it('preserves seed ties in world order and clears results when eligibility changes', () => {
    const cache = new SkylineClusterCache();
    const west = building(1, -300, 0, 8),
      east = building(2, 300, 0, 8);
    expect(cache.get([west, east])?.towers[0].id).toBe(1);
    expect(cache.get([east, west])?.towers[0].id).toBe(2);
    expect(cache.get([])).toBeNull();
    expect(cache.get([west])?.towers).toEqual([west]);
    expect(cache.get([{ ...west, floors: 2 }])).toBeNull();
  });

  it('keeps the earliest seed on exact score ties and world order inside each cluster', () => {
    // Two identical clusters far apart; the second is listed first in world order for one of them.
    const make = (base: number, cx: number) => [
      building(base + 2, cx + 20, 0, 9),
      building(base + 1, cx, 0, 12),
      building(base + 3, cx - 30, 40, 8),
      building(base + 4, cx + 90, 0, 6),
    ];
    const west = make(10, -1000),
      east = make(20, 1000);
    const buildings = [...east.slice(0, 2), ...west, ...east.slice(2)];
    const actual = findSkylineCluster(buildings),
      expected = reference(buildings);
    expectSameCluster(actual, expected);
    expect(actual!.x).toBeGreaterThan(0); // The first seed reaching the tie score wins.
    // Exactly 90 m apart is inside the radius, as in the full scan.
    expect(actual!.frameBuildings.map((b) => b.id)).toEqual(
      expected!.frameBuildings.map((b) => b.id),
    );
    expect(actual!.frameBuildings.some((b) => b.id === 24)).toBe(true);
  });

  it('returns null when no neighborhood reaches five floors', () => {
    const buildings = [building(1, 0, 0, 4), building(2, 10, 0, 3)];
    expect(findSkylineCluster(buildings)).toBeNull();
    expect(findSkylineCluster([])).toBeNull();
  });

  it('changes the cache key for every field the cluster and its frame bounds read', () => {
    const base = [building(1, 0, 0, 12, { kind: 'apartment' }), building(2, 30, 10, 8)];
    const key = skylineClusterKey(base);
    expect(skylineClusterKey(base.map((b) => ({ ...b })))).toBe(key);
    const changes: Array<Partial<Building>> = [
      { id: 9 },
      { x: 0.5 },
      { z: -0.5 },
      { floors: 13 },
      { width: 12.5 },
      { depth: 10.5 },
      { rotation: 0.1 },
      { kind: 'office' },
      { anchor: true },
    ];
    for (const change of changes)
      expect(
        skylineClusterKey([{ ...base[0], ...change }, base[1]]),
        JSON.stringify(change),
      ).not.toBe(key);
    // Eligibility (stage, access) is applied before the search, so it shows up as membership.
    expect(skylineClusterKey([base[1]])).not.toBe(key);
    expect(skylineClusterKey([base[1], base[0]])).not.toBe(key);
  });
});
