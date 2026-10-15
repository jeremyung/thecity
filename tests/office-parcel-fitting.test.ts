import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingKit, disposeGroup, Kit, partyWallBodyDepth, partyWallBodyWidth } from '../web/src/render/kit';
import { midriseMassing } from '../web/src/render/building-massing';
import type { Building } from '../web/src/types';

function office(width: number, depth: number, floors: number, appearance_seed: number, stage: string, progress: number): Building {
  return {
    id: 4085, parcel_id: 318, x: 0, z: 0, rotation: 0, width, depth, floors,
    kind: 'office', stage, progress, capacity: 25, occupancy: 0,
    business_active: false, built_day: 4, name: 'Parcel fitting office',
    appearance_seed, history: [], anchor: false, vacant_days: 0,
  };
}

function expectInsideParcel(group: THREE.Object3D, b: Building, context: string) {
  const bounds = new THREE.Box3().setFromObject(group);
  expect([bounds.min.x, bounds.max.x, bounds.min.z, bounds.max.z].every(Number.isFinite), context).toBe(true);
  expect(bounds.min.x, `${context}: left edge`).toBeGreaterThanOrEqual(-b.width / 2 - 1e-6);
  expect(bounds.max.x, `${context}: right edge`).toBeLessThanOrEqual(b.width / 2 + 1e-6);
  expect(bounds.min.z, `${context}: rear edge`).toBeGreaterThanOrEqual(-b.depth / 2 - 1e-6);
  expect(bounds.max.z, `${context}: front edge`).toBeLessThanOrEqual(b.depth / 2 + 1e-6);
}

describe('office parcel fitting', () => {
  it('contains narrow and shallow generated office meshes through open, construction, and renovation stages', () => {
    const art = new ArtMaterials();
    // Dimensions and seeds are representative generated parcels: narrow offices
    // from walking-offset seeds 87231/0/5 and shallow offices from all three.
    const parcels = [
      { width: 5.046998, depth: 8.774051, floors: 5, seed: 1747112002, sample: '87231 office 6582' },
      { width: 5.0300255, depth: 7.9801025, floors: 2, seed: 4040583008, sample: '0 office 7195' },
      { width: 5.0227823, depth: 8.317944, floors: 6, seed: 282357413, sample: '5 office 4684' },
      { width: 7.789001, depth: 7.0423965, floors: 6, seed: 1374223127, sample: '87231 office 1366' },
      { width: 7.5018253, depth: 7.072311, floors: 3, seed: 3827544997, sample: '0 office 3715' },
      { width: 7.871857, depth: 7.008223, floors: 6, seed: 3168162433, sample: '5 office 1637' },
    ];
    const stages = [
      ['open', 1], ['construction', 0], ['construction', .8],
      ['construction', 1], ['renovation', .5],
    ] as const;

    try {
      for (const style of ['anime', 'miniature'] as const) {
        art.setStyle(style);
        for (const parcel of parcels) for (const [stage, progress] of stages) {
          const b = office(parcel.width, parcel.depth, parcel.floors, parcel.seed, stage, progress);
          const group = buildingKit(b, art);
          expectInsideParcel(group, b, `${style} ${parcel.sample} ${stage}/${progress}`);
          disposeGroup(group);
        }
      }
    } finally {
      art.dispose();
    }
  }, 30_000);

  it('preserves narrow-office setback massing from four through eleven floors', () => {
    const b = office(5.046998, 8.774051, 4, 1747112002, 'open', 1);
    for (const floors of [4, 5, 6, 7, 8, 9, 10, 11]) {
      const candidate = { ...b, floors };
      const massing = midriseMassing(candidate,
        partyWallBodyWidth(candidate.width, candidate.anchor, floors, candidate.kind),
        partyWallBodyDepth(candidate.depth));
      expect(massing, `${floors} floors`).not.toBeNull();
      expect(massing!.family, `${floors} floors`).toBe('setback-office');
      expect(massing!.masses.length, `${floors} floors has lower body and upper setback`).toBeGreaterThanOrEqual(2);
      const lower = massing!.masses[0], upper = massing!.masses.at(-1)!;
      expect(upper.baseFloor).toBeGreaterThan(0);
      expect(upper.width).toBeLessThan(lower.width);
      expect(upper.depth).toBeLessThan(lower.depth);
    }
  });

  it('keeps a centered entrance visible and a shallow-parcel AC assembly outside the rear wall', () => {
    const art = new ArtMaterials();
    try {
      const narrow = office(5.046998, 8.774051, 5, 1747112002, 'open', 1);
      for (const style of ['anime', 'miniature'] as const) {
        art.setStyle(style);
        const group = buildingKit(narrow, art);
        group.updateMatrixWorld(true);
        const hit = new THREE.Raycaster(new THREE.Vector3(0, 1.2, narrow.depth / 2 + 1),
          new THREE.Vector3(0, 0, -1)).intersectObject(group, true)[0];
        expect(hit, `${style} centered doorway ray`).toBeDefined();
        expect((hit!.object as THREE.Mesh).material, `${style} centered entrance glazing`)
          .toBe(art.get('#55757a', .25));
        disposeGroup(group);
      }

      // Seed-87231 office 7292 is a shallow two-floor office, so its normal
      // external AC assembly is emitted while still stressing rear clearance.
      const shallow = office(8.102772, 7.3573704, 2, 3524137459, 'open', 1);
      const parts: Array<{ material: THREE.Material; bounds: THREE.Box3 }> = [];
      const originalAdd = Kit.prototype.add;
      const addSpy = vi.spyOn(Kit.prototype, 'add').mockImplementation(function (this: Kit, geometry, material) {
        originalAdd.call(this, geometry, material);
        geometry.computeBoundingBox();
        if (geometry.boundingBox) parts.push({ material, bounds: geometry.boundingBox.clone() });
      });
      let shallowGroup: THREE.Group;
      try {
        shallowGroup = buildingKit(shallow, art);
      } finally {
        addSpy.mockRestore();
      }
      const d = partyWallBodyDepth(shallow.depth);
      const rearWall = -d / 2;
      const acCasing = parts.find(part => part.material === art.get('#c3b9a5')
        && Math.abs((part.bounds.max.x - part.bounds.min.x) - 1.05) < .02
        && Math.abs((part.bounds.max.y - part.bounds.min.y) - .72) < .02
        && part.bounds.max.z < rearWall && part.bounds.min.z > -shallow.depth / 2);
      expect(acCasing, 'AC casing occupies the rear setback').toBeDefined();
      expect(acCasing!.bounds.max.z - acCasing!.bounds.min.z).toBeGreaterThan(.05);

      const grille = parts.find(part => part.material === art.get('#344346')
        && Math.abs((part.bounds.max.x - part.bounds.min.x) - .68) < .02
        && Math.abs((part.bounds.max.y - part.bounds.min.y) - .44) < .02
        && part.bounds.max.z < rearWall && part.bounds.min.z > -shallow.depth / 2
        && part.bounds.min.y < acCasing!.bounds.max.y && part.bounds.max.y > acCasing!.bounds.min.y);
      expect(grille, 'grille remains outside the wall and aligned to the casing').toBeDefined();
      expect(grille!.bounds.max.z - grille!.bounds.min.z).toBeGreaterThan(0);

      const support = parts.find(part => part.material === art.get('#536164', .62)
        && Math.abs((part.bounds.max.x - part.bounds.min.x) - 1.24) < .02
        && Math.abs((part.bounds.max.y - part.bounds.min.y) - .08) < .02
        && part.bounds.max.z < rearWall && part.bounds.min.z > -shallow.depth / 2
        && part.bounds.max.y < acCasing!.bounds.min.y);
      expect(support, 'AC support stand remains below its casing in the setback').toBeDefined();
      expect(support!.bounds.max.z - support!.bounds.min.z).toBeGreaterThan(.05);
      expectInsideParcel(shallowGroup, shallow, 'shallow AC office');
      disposeGroup(shallowGroup);
    } finally {
      art.dispose();
    }
  });
});
