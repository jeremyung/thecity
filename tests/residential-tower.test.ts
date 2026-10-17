import { expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import {
  buildResidentialTower,
  RESIDENTIAL_TOWERS,
} from '../web/src/render/architecture/residential-tower';
import { ApartmentResources } from '../web/src/render/architecture/resources';
import { disposeGroup } from '../web/src/render/kit';
import { BuildingAccessPaths } from '../web/src/render/building-access';
import { architectureCityStudy } from '../web/src/render/architecture/city-study';
import type { Building } from '../web/src/types';
const specimen = (width = 26, depth = 24, floors = 32): Building => ({
  id: 1,
  parcel_id: 1,
  x: 0,
  z: 0,
  rotation: 0,
  width,
  depth,
  floors,
  kind: 'apartment' as const,
  stage: 'open' as const,
  progress: 1,
  capacity: 100,
  occupancy: 70,
  built_day: 0,
  name: 'Tower prototype',
  appearance_seed: 32,
  anchor: false,
  history: [],
  vacant_days: 0,
  business_active: false,
});
const triangles = (g: THREE.Group) => {
  let n = 0;
  g.traverse((o) => {
    if (o instanceof THREE.Mesh)
      n += (o.geometry.index?.count ?? o.geometry.getAttribute('position').count) / 3;
  });
  return n;
};
it('builds all five tower options inside variable parcels without mutating authoritative inputs', () => {
  const error = vi.spyOn(console, 'error');
  const resources = new ApartmentResources();
  try {
    for (const recipe of Object.keys(RESIDENTIAL_TOWERS) as (keyof typeof RESIDENTIAL_TOWERS)[])
      for (const width of [14, 26, 40])
        for (const depth of [14, 24, 40])
          for (const floors of [12, 32, 60]) {
            const b = specimen(width, depth, floors),
              before = JSON.stringify(b),
              g = buildResidentialTower(b, resources.art, { recipe })!;
            const bounds = new THREE.Box3().setFromObject(g);
            expect(bounds.min.x).toBeGreaterThanOrEqual(-width / 2 - 0.001);
            expect(bounds.max.x).toBeLessThanOrEqual(width / 2 + 0.001);
            expect(bounds.min.z).toBeGreaterThanOrEqual(-depth / 2 - 0.001);
            expect(bounds.max.z).toBeLessThanOrEqual(depth / 2 + 0.001);
            expect(bounds.max.y).toBeGreaterThan(floors * 3.25);
            expect(g.userData.residentialTower.recipe).toBe(recipe);
            expect(JSON.stringify(b)).toBe(before);
            g.traverse((o) => {
              if (o instanceof THREE.Mesh) {
                expect(
                  Array.from(o.geometry.getAttribute('position').array).every(Number.isFinite),
                ).toBe(true);
                if (
                  (o.material as THREE.Material).userData.apartmentSurface &&
                  (o.material as THREE.Material).userData.apartmentRole !== 'glass'
                )
                  expect(o.geometry.getAttribute('architectureWear')).toBeDefined();
              }
            });
            disposeGroup(g);
          }
    expect(error).not.toHaveBeenCalled();
  } finally {
    resources.dispose();
    error.mockRestore();
  }
}, 20000);
it('distinguishes recipes, is deterministic, and reduces detail without losing its roof silhouette', () => {
  const resources = new ApartmentResources();
  try {
    const b = specimen(),
      counts: number[] = [];
    for (const recipe of Object.keys(RESIDENTIAL_TOWERS) as (keyof typeof RESIDENTIAL_TOWERS)[]) {
      const close = buildResidentialTower(b, resources.art, { recipe, detail: 'close' })!,
        repeat = buildResidentialTower(b, resources.art, { recipe, detail: 'close' })!,
        far = buildResidentialTower(b, resources.art, { recipe, detail: 'far' })!;
      counts.push(triangles(close));
      expect(triangles(close)).toBe(triangles(repeat));
      expect(triangles(far)).toBeLessThan(triangles(close));
      const bounds = new THREE.Box3().setFromObject(close),
        other = new THREE.Box3().setFromObject(far);
      expect(bounds.max.distanceTo(other.max)).toBeLessThan(0.001);
      for (let i = 0; i < close.children.length; i++)
        expect(
          Array.from((close.children[i] as THREE.Mesh).geometry.getAttribute('position').array),
        ).toEqual(
          Array.from((repeat.children[i] as THREE.Mesh).geometry.getAttribute('position').array),
        );
      disposeGroup(close);
      disposeGroup(repeat);
      disposeGroup(far);
    }
    expect(new Set(counts).size).toBe(5);
    expect(Object.keys(RESIDENTIAL_TOWERS)).toHaveLength(5);
  } finally {
    resources.dispose();
  }
});
it('supports foundation, frame and partial cladding with no completed crown, and rejects unsuitable inputs', () => {
  const resources = new ApartmentResources();
  try {
    for (const stage of ['foundation', 'structure', 'surface', 'finishing'] as const) {
      const b = { ...specimen(), stage, progress: 0.76 },
        g = buildResidentialTower(b, resources.art)!;
      expect(g.userData.residentialTower.roofInstalled).toBe(false);
      expect(g.userData.residentialTower.installedFloors).toBeLessThan(b.floors);
      expect(new THREE.Box3().setFromObject(g).max.y).toBeLessThanOrEqual(0.6 + b.floors * 3.25);
      disposeGroup(g);
    }
    for (const b of [
      { ...specimen(), floors: 11 },
      { ...specimen(), width: 8 },
      { ...specimen(), kind: 'office' as const },
      { ...specimen(), width: NaN },
    ])
      expect(buildResidentialTower(b, resources.art)).toBeNull();
  } finally {
    resources.dispose();
  }
});
it('uses seeded shaft proportions as well as unit-level glazing variation', () => {
  const resources = new ApartmentResources();
  try {
    const proportions = new Set<string>();
    for (let appearance_seed = 0; appearance_seed < 12; appearance_seed++) {
      const g = buildResidentialTower({ ...specimen(), appearance_seed }, resources.art, {
        recipe: 'ribbon',
      })!;
      const meta = g.userData.residentialTower;
      proportions.add(`${meta.shaftWidth}/${meta.shaftDepth}`);
      disposeGroup(g);
    }
    expect(proportions.size).toBe(3);
  } finally {
    resources.dispose();
  }
});

it('aligns actual entrance canopies and walking access for every tower recipe, seed residue, rotation and LOD', () => {
  const resources = new ApartmentResources(),
    paths = new BuildingAccessPaths(resources.art, (release) => release()),
    world = architectureCityStudy(32, true);
  try {
    for (const recipe of Object.keys(RESIDENTIAL_TOWERS) as (keyof typeof RESIDENTIAL_TOWERS)[])
      for (const appearance_seed of [0, 1, 2])
        for (const rotation of [0, Math.PI / 2, 0.73, -1.2])
          for (const detail of ['close', 'district', 'far'] as const) {
            const b = { ...specimen(), x: 73, z: -41, appearance_seed, rotation },
              before = JSON.stringify(b),
              group = buildResidentialTower(b, resources.art, { recipe, detail })!;
            // Locate the actual canopy's front top edge in merged geometry. This catches
            // a displaced entrance even if metadata accidentally reports a centred door.
            const front = (b.depth - 2) / 2,
              edgeX: number[] = [];
            group.traverse((o) => {
              if (o instanceof THREE.Mesh) {
                const p = o.geometry.getAttribute('position');
                for (let i = 0; i < p.count; i++)
                  if (
                    Math.abs(p.getY(i) - (0.58 + 3.05 + 0.045)) < 0.0001 &&
                    Math.abs(p.getZ(i) - (front + 0.35 + 0.95 / 2)) < 0.0001
                  )
                    edgeX.push(p.getX(i));
              }
            });
            expect(edgeX.length).toBeGreaterThanOrEqual(2);
            const entranceX = (Math.min(...edgeX) + Math.max(...edgeX)) / 2;
            expect(entranceX, `${recipe}/${appearance_seed}/${rotation}/${detail}`).toBeCloseTo(
              0,
              5,
            );
            group.position.set(b.x, 0, b.z);
            group.rotation.y = rotation;
            group.updateMatrixWorld(true);
            const visible = group.localToWorld(new THREE.Vector3(entranceX, 0, front));
            const outward = new THREE.Vector3(Math.sin(rotation), 0, Math.cos(rotation));
            // engine/src/lib.rs::building_access_segment owns this front-centre endpoint.
            const endpoint = new THREE.Vector3(b.x, 0, b.z).addScaledVector(
              outward,
              b.depth * 0.5 + 0.2,
            );
            const lateral = new THREE.Vector3(Math.cos(rotation), 0, -Math.sin(rotation));
            expect(visible.clone().sub(endpoint).dot(lateral)).toBeCloseTo(0, 5);
            const curb = endpoint.clone().addScaledVector(outward, 8);
            paths.setWorld({
              ...world,
              buildings: [b],
              building_access: [
                {
                  building_id: b.id,
                  road_id: 1,
                  points: [
                    { x: endpoint.x, z: endpoint.z },
                    { x: curb.x, z: curb.z },
                  ],
                },
              ],
            });
            expect(new THREE.Box3().setFromObject(paths.root).containsPoint(endpoint)).toBe(true);
            expect(JSON.stringify(b)).toBe(before);
            disposeGroup(group);
          }
  } finally {
    paths.reset();
    resources.dispose();
  }
}, 20000);
