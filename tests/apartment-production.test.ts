import { expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { ApartmentResources } from '../web/src/render/architecture/resources';
import {
  buildApartmentBuilding,
  APARTMENT_RECIPE_IDS,
  buildResidentialTower,
  RESIDENTIAL_TOWERS,
} from '../web/src/render/architecture';
import { apartmentConstruction, apartmentPhase } from '../web/src/render/architecture/lifecycle';
import {
  selectApartmentArchitecture,
  resolveApartmentArchitectures,
  apartmentArchitectureKey,
} from '../web/src/render/architecture/era';
import { WINDOW_ATTRIBUTE } from '../web/src/render/building-surfaces';
import { disposeGroup } from '../web/src/render/kit';
import type { Building } from '../web/src/types';
const specimen = (floors = 7): Building => ({
  id: 1,
  parcel_id: 1,
  x: 0,
  z: 0,
  width: 26,
  depth: 24,
  rotation: 0,
  floors,
  appearance_seed: 32,
  kind: 'apartment' as const,
  stage: 'open' as const,
  progress: 1,
  capacity: 100,
  occupancy: 60,
  built_day: 0,
  name: 'Production apartment',
  history: [],
  anchor: false,
  vacant_days: 0,
  business_active: false,
});
it('keeps all fifteen recipes throughout demolition and removes roofs and upper floors monotonically', () => {
  const r = new ApartmentResources();
  try {
    for (const recipe of [...APARTMENT_RECIPE_IDS, ...Object.keys(RESIDENTIAL_TOWERS)]) {
      const tower = recipe in RESIDENTIAL_TOWERS,
        source = specimen(tower ? 32 : 7);
      let last = Infinity;
      for (const progress of [0, 0.2, 0.5, 0.8, 1]) {
        const b = { ...source, stage: 'demolition' as const, progress },
          state = apartmentConstruction(b),
          choice = selectApartmentArchitecture(b, 'contemporary');
        expect(choice.kind).not.toBe('legacy');
        const g = tower
          ? buildResidentialTower(b, r.art, { recipe: recipe as keyof typeof RESIDENTIAL_TOWERS })!
          : buildApartmentBuilding(b, r.art, {
              recipe: recipe as (typeof APARTMENT_RECIPE_IDS)[number],
            })!;
        const meta = g.userData.residentialTower ?? g.userData.apartmentPilot;
        expect(meta.installedFloors).toBe(state.installed);
        expect(meta.roofInstalled).toBe(false);
        const bounds = new THREE.Box3().setFromObject(g);
        expect(bounds.max.y).toBeLessThanOrEqual(last + 0.001);
        last = bounds.max.y;
        expect(bounds.min.x).toBeGreaterThanOrEqual(-b.width / 2 - 0.001);
        expect(bounds.max.x).toBeLessThanOrEqual(b.width / 2 + 0.001);
        expect(bounds.min.z).toBeGreaterThanOrEqual(-b.depth / 2 - 0.001);
        expect(bounds.max.z).toBeLessThanOrEqual(b.depth / 2 + 0.001);
        disposeGroup(g);
      }
    }
  } finally {
    r.dispose();
  }
}, 20000);
it('uses the production room shader and occupancy attributes, with unoccupied rooms in construction and demolition', () => {
  const r = new ApartmentResources();
  try {
    for (const floors of [7, 32])
      for (const stage of ['open', 'renovation', 'surface', 'demolition'] as const)
        for (const occupancy of [0, 60]) {
          const b = { ...specimen(floors), stage, occupancy, progress: 0.76 },
            g =
              floors < 12
                ? buildApartmentBuilding(b, r.art, { recipe: 'modern' })!
                : buildResidentialTower(b, r.art, { recipe: 'lantern' })!;
          let panes = 0;
          g.traverse((o) => {
            if (
              !(o instanceof THREE.Mesh) ||
              (o.material as THREE.Material).userData.windowPane !== true
            )
              return;
            panes++;
            const attribute = o.geometry.getAttribute(WINDOW_ATTRIBUTE);
            expect(attribute).toBeDefined();
            for (let i = 0; i < attribute.count; i++) {
              expect(attribute.getY(i)).toBe(stage === 'open' && occupancy > 0 ? 1 : 0);
              expect(attribute.getZ(i)).toBeGreaterThan(0);
              expect(attribute.getW(i)).toBeGreaterThan(0);
            }
          });
          expect(panes).toBeGreaterThan(0);
          disposeGroup(g);
        }
  } finally {
    r.dispose();
  }
});
it('preserves source ownership for dressing and ordinary batches across style switches', () => {
  const r = new ApartmentResources(),
    other = new ArtMaterials();
  try {
    const source = r.art.get('#d5c8b0');
    expect(r.owns(source)).toBe(true);
    expect(r.owns(other.get('#d5c8b0'))).toBe(false);
    const batched = r.batch(source).material;
    expect(r.owns(batched)).toBe(true);
    expect(r.variant(batched, 'anime')).not.toBe(r.variant(batched, 'miniature'));
  } finally {
    r.dispose();
    other.dispose();
  }
});
it('suppresses nearby identical facades deterministically without restyling older IDs on later infill', () => {
  const buildings = Array.from({ length: 5 }, (_, i) => ({
      ...specimen(),
      id: i + 1,
      parcel_id: i + 1,
      x: i * 14,
    })),
    before = JSON.stringify(buildings);
  const forward = resolveApartmentArchitectures(buildings, 'contemporary'),
    reverse = resolveApartmentArchitectures([...buildings].reverse(), 'contemporary');
  expect([...forward]).toEqual([...reverse]);
  for (let i = 1; i < buildings.length; i++)
    expect(apartmentArchitectureKey(forward.get(i)!)).not.toBe(
      apartmentArchitectureKey(forward.get(i + 1)!),
    );
  const added = resolveApartmentArchitectures(
    [...buildings, { ...specimen(), id: 100, x: 3 }],
    'contemporary',
  );
  for (const [id, choice] of forward) expect(added.get(id)).toEqual(choice);
  expect(JSON.stringify(buildings)).toBe(before);
  for (const b of buildings)
    expect(selectApartmentArchitecture(b, 'legacy')).toEqual({ kind: 'legacy' as const });
});
it('quantizes exact lifecycle counts for cache keys and clamps invalid progress', () => {
  for (const floors of [7, 32])
    for (const stage of ['structure', 'surface', 'demolition'] as const) {
      const a = { ...specimen(floors), stage, progress: 0.76 },
        b = { ...a, progress: 0.760001 };
      expect(apartmentPhase(a)).toBe(apartmentPhase(b));
      for (const progress of [-1, 0, 1, 3]) {
        const s = apartmentConstruction({ ...a, progress });
        expect(s.installed).toBeGreaterThanOrEqual(0);
        expect(s.installed).toBeLessThanOrEqual(floors);
      }
    }
});

it('defers the second geometry build when the first exhausts the per-frame budget', async () => {
  const { CityRenderer } = await import('../web/src/render/city');
  const renderer = Object.create(CityRenderer.prototype) as any;
  let clock = 0;
  const time = vi.spyOn(performance, 'now').mockImplementation(() => clock);
  const first = vi.fn(() => {
      clock = 8;
      return new THREE.Group();
    }),
    second = vi.fn(() => new THREE.Group());
  renderer.roadsLayer = { infrastructure: () => ({ canPresentBuilding: () => true }) };
  renderer.buildings = new Map();
  renderer.chunks = { set: vi.fn(), update: vi.fn() };
  renderer.objectQueue = new Map([
    [1, { building: specimen(), signature: 'a', source: { createGroup: first } }],
    [2, { building: { ...specimen(), id: 2 }, signature: 'b', source: { createGroup: second } }],
  ]);
  try {
    renderer.updateBuildingBatches();
    expect(first).toHaveBeenCalledOnce();
    expect(second).not.toHaveBeenCalled();
    expect(renderer.objectQueue.has(2)).toBe(true);
    renderer.updateBuildingBatches();
    expect(second).toHaveBeenCalledOnce();
  } finally {
    time.mockRestore();
  }
});
