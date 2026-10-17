import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingKit, disposeGroup } from '../web/src/render/kit';
import {
  BILLBOARDS,
  BLADE_LABELS,
  screenPanels,
  addFacadeFeature,
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
import { Kit as GeometryKit } from '../web/src/render/architecture/geometry-kit';
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
    kind: 'office' as const,
    stage: 'open' as const,
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
        building({ appearance_seed: SIGNED_SEED, stage: 'foundation' as const, progress: 0.5 }),
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
      building({ kind: 'apartment' as const }),
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
  const station = { id: 9, x: 0, z: 0, kind: 'station' as const };
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
      building({ id: 4, floors: 14, stage: 'foundation' as const }),
      building({ id: 5, floors: 14, kind: 'home' as const }),
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

describe('media wall layouts', () => {
  it('shares a continuous campaign coordinate across synchronized tiles', () => {
    const panels = screenPanels(12, 6, 'triptych');
    expect(panels).toHaveLength(3);
    const ranges = panels.map((g) => {
      const uv = g.getAttribute('uv'),
        p = g.getAttribute('position');
      const values = Array.from({ length: uv.count }, (_, i) => uv.getX(i));
      for (let i = 0; i < p.count; i++) expect(uv.getX(i)).toBeCloseTo((p.getX(i) + 6) / 12, 6);
      return [Math.min(...values), Math.max(...values)];
    });
    expect(ranges[0][0]).toBeCloseTo(0, 6);
    expect(ranges[2][1]).toBeCloseTo(1, 6);
    expect(ranges[0][1]).toBeLessThan(ranges[1][0]);
    panels.forEach((g) => g.dispose());
  });
  it('curves within the reserved depth and retains outward unit normals', () => {
    const [g] = screenPanels(12, 6, 'curved'),
      p = g.getAttribute('position'),
      n = g.getAttribute('normal');
    let min = 0;
    for (let i = 0; i < p.count; i++) {
      min = Math.min(min, p.getZ(i));
      expect(p.getZ(i)).toBeLessThanOrEqual(0);
      expect(p.getZ(i)).toBeGreaterThanOrEqual(-0.281);
      expect(Math.hypot(n.getX(i), n.getY(i), n.getZ(i))).toBeCloseTo(1, 5);
      expect(n.getZ(i)).toBeGreaterThan(0.9);
    }
    expect(min).toBeLessThan(-0.2);
    g.dispose();
  });
  it('bounds the campaign cache to twelve shared style pairs', () => {
    const art = new ArtMaterials();
    try {
      const materials = Array.from({ length: 12 }, (_, i) => videoScreenSurface(art, i));
      expect(new Set(materials).size).toBe(12);
      expect(videoScreenSurface(art, 12)).toBe(materials[0]);
      expect(videoScreenSurface(art, -1)).toBe(materials[11]);
    } finally {
      art.dispose();
    }
  });
});

it('preserves screen dimensions when replacing animation with a static campaign', () => {
  const art = new ArtMaterials();
  try {
    for (const layout of ['curved', 'triptych'] as const) {
      const groups = ['screen', 'ad'].map((feature) => {
        const kit = new GeometryKit();
        addFacadeFeature(kit, art, art.get('#222222'), feature as 'screen' | 'ad', {
          frontZ: 0,
          width: 9,
          height: 3,
          bottom: 4,
          seed: 12,
          program: 7,
          layout,
          screenPoster: true,
        });
        return kit.finish();
      });
      const boxes = groups.map((g) => new THREE.Box3().setFromObject(g));
      expect(boxes[0].min.distanceTo(boxes[1].min)).toBeLessThan(0.00001);
      expect(boxes[0].max.distanceTo(boxes[1].max)).toBeLessThan(0.00001);
      groups.forEach(disposeGroup);
    }
  } finally {
    art.dispose();
  }
});
it('prioritizes landmark artwork without increasing host or animated-screen budgets', () => {
  const buildings = Array.from({ length: 8 }, (_, i) =>
    building({ id: i + 1, x: i * 31, floors: 14 }),
  );
  const world = { nodes: [{ id: 99, x: 0, z: 0, kind: 'station' as const }], buildings };
  const original = selectFacadeFeatures(world),
    preferred = new Set([2, 4, 6]),
    chosen = selectFacadeFeatures(world, undefined, preferred);
  expect([...chosen.keys()]).toEqual([...original.keys()]);
  expect([...chosen.values()].filter((v) => v === 'screen')).toHaveLength(3);
  for (const id of preferred) expect(chosen.get(id)).toBe('screen');
});
it('allocates landmark media at a major junction even without a station', () => {
  const nodes = [
    { id: 1, x: 0, z: 0, kind: 'junction' as const },
    { id: 2, x: -100, z: 0, kind: 'junction' as const },
    { id: 3, x: 100, z: 0, kind: 'junction' as const },
    { id: 4, x: 0, z: 100, kind: 'junction' as const },
  ];
  const world = {
    nodes,
    roads: [
      { from: 1, to: 2 },
      { from: 1, to: 3 },
      { from: 1, to: 4 },
    ] as World['roads'],
    buildings: [building({ floors: 10, kind: 'shop' as const, x: 12 })],
  };
  expect(signageContexts(world).get(1)?.feature).toBeUndefined();
  expect(signageContexts(world, undefined, new Set([1])).get(1)).toEqual({
    proximity: 2,
    feature: 'screen',
  });
});
