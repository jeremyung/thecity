import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingKit, disposeGroup } from '../web/src/render/kit';
import {
  BILLBOARDS,
  BLADE_LABELS,
  MAX_FACADE_FEATURES,
  billboardCell,
  bladeCell,
  hasRooftopBillboard,
  selectFacadeFeatures,
  signageContexts,
  stationProximity,
  streetSignSurface,
  videoScreenSurface,
} from '../web/src/render/signage';
import type { Building, World } from '../web/src/types';

// Canvas atlases are drawn through a no-op 2D context in Node.
beforeEach(() => {
  vi.stubGlobal('document', {
    createElement: () => {
      const context = new Proxy({} as Record<PropertyKey, unknown>, {
        get(target, key) {
          if (!(key in target)) target[key] = () => undefined;
          return target[key];
        },
        set(target, key, value) {
          target[key] = value;
          return true;
        },
      });
      return { width: 0, height: 0, getContext: () => context };
    },
  });
});
afterEach(() => vi.unstubAllGlobals());

function building(overrides: Partial<Building> = {}): Building {
  return {
    id: 1,
    parcel_id: 1,
    x: 0,
    z: 0,
    rotation: 0,
    width: 14,
    depth: 16,
    floors: 7,
    kind: 'office',
    stage: 'open',
    progress: 1,
    capacity: 35,
    occupancy: 0,
    built_day: 1,
    name: 'Station Office',
    appearance_seed: 0,
    history: [],
    anchor: false,
    vacant_days: 0,
    business_active: true,
    ...overrides,
  };
}
function meshesWith(root: THREE.Object3D, material: THREE.Material) {
  const found: THREE.Mesh[] = [];
  root.traverse((object) => {
    if (object instanceof THREE.Mesh && object.material === material) found.push(object);
  });
  return found;
}
function bounds(root: THREE.Object3D) {
  const box = new THREE.Box3();
  root.updateMatrixWorld(true);
  root.traverse((object) => {
    if (object instanceof THREE.Mesh) box.union(new THREE.Box3().setFromObject(object));
  });
  return box;
}
/** Seed whose blade and rooftop rolls both succeed for an office near the station. */
const SIGNED_SEED = (() => {
  for (let seed = 0; seed < 1 << 20; seed += 97) {
    if ((seed >>> 4) % 3 !== 0 && hasRooftopBillboard(building({ appearance_seed: seed }), 2))
      return seed;
  }
  throw new Error('no seed');
})();

describe('sign atlas', () => {
  it('keeps every blade and billboard cell inside the atlas without overlap', () => {
    const cells = [
      ...BLADE_LABELS.map((_, i) => bladeCell(i)),
      ...BILLBOARDS.map((_, i) => billboardCell(i)),
    ];
    for (const c of cells) {
      expect(c.u0).toBeGreaterThanOrEqual(0);
      expect(c.u1).toBeLessThanOrEqual(1);
      expect(c.v0).toBeGreaterThanOrEqual(0);
      expect(c.v1).toBeLessThanOrEqual(1);
      expect(c.u1).toBeGreaterThan(c.u0);
      expect(c.v1).toBeGreaterThan(c.v0);
    }
    for (let i = 0; i < cells.length; i++)
      for (let j = i + 1; j < cells.length; j++) {
        const a = cells[i],
          b = cells[j];
        expect(a.u1 <= b.u0 || b.u1 <= a.u0 || a.v1 <= b.v0 || b.v1 <= a.v0).toBe(true);
      }
    expect(bladeCell(BLADE_LABELS.length)).toEqual(bladeCell(0));
    expect(new Set(BLADE_LABELS).size).toBe(BLADE_LABELS.length);
  });
});

describe('commercial signage', () => {
  it('adds blades and a rooftop board to an open office near the station, inside its parcel', () => {
    const art = new ArtMaterials();
    try {
      const sign = streetSignSurface(art);
      const b = building({ appearance_seed: SIGNED_SEED });
      const group = buildingKit(b, art, { signage: { proximity: 2 } });
      const signs = meshesWith(group, sign);
      expect(signs.length).toBeGreaterThan(0);
      const box = bounds(group);
      expect(box.min.x).toBeGreaterThanOrEqual(-b.width / 2 - 1e-3);
      expect(box.max.x).toBeLessThanOrEqual(b.width / 2 + 1e-3);
      expect(box.min.z).toBeGreaterThanOrEqual(-b.depth / 2 - 1e-3);
      expect(box.max.z).toBeLessThanOrEqual(b.depth / 2 + 1e-3);
      // The rooftop board stands above the roof deck.
      const roofTop = 0.58 + b.floors * 3.25;
      const signTop = Math.max(...signs.map((mesh) => new THREE.Box3().setFromObject(mesh).max.y));
      expect(signTop).toBeGreaterThan(roofTop + 2.4);
      disposeGroup(group);

      // A closed business carries no signage; neither does an unfinished building.
      for (const quiet of [
        building({ appearance_seed: SIGNED_SEED, business_active: false }),
        building({ appearance_seed: SIGNED_SEED, stage: 'construction', progress: 0.5 }),
      ]) {
        const plain = buildingKit(quiet, art, { signage: { proximity: 2 } });
        expect(meshesWith(plain, sign)).toHaveLength(0);
        disposeGroup(plain);
      }
    } finally {
      art.dispose();
    }
  });

  it('puts a chosen video wall on a station tower podium within the parcel', () => {
    const art = new ArtMaterials();
    try {
      const tower = building({ floors: 20, width: 22, depth: 24, appearance_seed: 7 });
      const group = buildingKit(tower, art, { signage: { proximity: 2, feature: 'screen' } });
      const screens = meshesWith(group, videoScreenSurface(art));
      expect(screens).toHaveLength(1);
      const box = new THREE.Box3().setFromObject(screens[0]);
      expect(box.max.z).toBeLessThanOrEqual(tower.depth / 2);
      expect(box.min.y).toBeGreaterThan(0.58 + 3.25);
      expect(box.max.y).toBeLessThan(0.58 + 3 * 3.25);
      disposeGroup(group);
      const plain = buildingKit(tower, art, { signage: { proximity: 2 } });
      expect(meshesWith(plain, videoScreenSurface(art))).toHaveLength(0);
      disposeGroup(plain);
    } finally {
      art.dispose();
    }
  });

  it('makes rooftop boards likelier near the station and limits them to 4–11 floor commercial buildings', () => {
    const rate = (proximity: number) =>
      Array.from({ length: 400 }, (_, i) =>
        hasRooftopBillboard(building({ appearance_seed: i << 13 }), proximity),
      ).filter(Boolean).length;
    expect(rate(2)).toBeGreaterThan(rate(1));
    expect(rate(1)).toBeGreaterThan(rate(0));
    expect(rate(0)).toBeGreaterThan(0);
    for (const b of [
      building({ kind: 'apartment' }),
      building({ floors: 3 }),
      building({ floors: 12 }),
      building({ anchor: true }),
    ])
      expect(
        Array.from({ length: 50 }, (_, i) =>
          hasRooftopBillboard({ ...b, appearance_seed: i << 13 }, 2),
        ).some(Boolean),
      ).toBe(false);
    expect([stationProximity(50), stationProximity(200), stationProximity(400)]).toEqual([2, 1, 0]);
  });
});

describe('station-area displays', () => {
  const station = { id: 9, x: 0, z: 0, kind: 'station' };
  const world = (buildings: Building[]): Pick<World, 'nodes' | 'buildings'> => ({
    nodes: [station],
    buildings,
  });

  it('chooses at most six nearby hosts, spaced apart, alternating screens and ads', () => {
    const buildings = Array.from({ length: 30 }, (_, i) =>
      building({ id: i + 1, x: (i % 6) * 20, z: Math.floor(i / 6) * 20, floors: 14 }),
    );
    const chosen = selectFacadeFeatures(world(buildings));
    expect(chosen.size).toBe(MAX_FACADE_FEATURES);
    const hosts = [...chosen.keys()].map((id) => buildings.find((b) => b.id === id)!);
    for (let i = 0; i < hosts.length; i++)
      for (let j = i + 1; j < hosts.length; j++)
        expect(Math.hypot(hosts[i].x - hosts[j].x, hosts[i].z - hosts[j].z)).toBeGreaterThanOrEqual(
          30,
        );
    expect([...chosen.values()]).toEqual(['screen', 'ad', 'screen', 'ad', 'screen', 'ad']);
    expect(selectFacadeFeatures(world(buildings))).toEqual(chosen);
  });

  it('skips low, closed, distant and unfinished buildings and needs a station', () => {
    const buildings = [
      building({ id: 1, floors: 4 }),
      building({ id: 2, floors: 14, business_active: false }),
      building({ id: 3, floors: 14, x: 400 }),
      building({ id: 4, floors: 14, stage: 'construction' }),
      building({ id: 5, floors: 14, kind: 'home' }),
    ];
    expect(selectFacadeFeatures(world(buildings)).size).toBe(0);
    expect(selectFacadeFeatures({ nodes: [], buildings: [building({ floors: 14 })] }).size).toBe(0);
    const contexts = signageContexts(
      world([building({ id: 7, floors: 14 }), building({ id: 8, x: 200, floors: 5 })]),
    );
    expect(contexts.get(7)).toEqual({ proximity: 2, feature: 'screen' });
    expect(contexts.get(8)).toEqual({ proximity: 1 });
  });
});
