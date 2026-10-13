import { expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingKit, disposeGroup } from '../web/src/render/kit';
import { buildingRenderSignature } from '../web/src/render/city';
import type { Building } from '../web/src/types';

const building: Building = {
  id: 1, parcel_id: 1, x: 0, z: 0, rotation: 0, width: 22, depth: 20,
  floors: 10, kind: 'office', stage: 'demolition', progress: 0, capacity: 50,
  occupancy: 0, business_active: false, built_day: 0, name: 'Office',
  appearance_seed: 13, history: [], anchor: false, vacant_days: 0,
};

it('removes original upper floors in order, within the parcel, in both styles', () => {
  const art = new ArtMaterials();
  const signatures: number[][][] = [];
  for (const style of ['anime', 'miniature'] as const) {
    art.setStyle(style);
    const stages: number[][] = [];
    let lastHeight = Infinity;
    for (const progress of [0, .2, .5, .8, 1]) {
      const group = buildingKit({ ...building, progress }, art);
      const bounds = new THREE.Box3().setFromObject(group);
      expect(bounds.max.y).toBeLessThan(lastHeight);
      lastHeight = bounds.max.y;
      expect(bounds.min.x).toBeGreaterThanOrEqual(-11);
      expect(bounds.max.x).toBeLessThanOrEqual(11);
      expect(bounds.min.z).toBeGreaterThanOrEqual(-10);
      expect(bounds.max.z).toBeLessThanOrEqual(10);
      const vertices: number[] = [];
      group.traverse(object => {
        if (object instanceof THREE.Mesh) vertices.push(...object.geometry.getAttribute('position').array);
      });
      expect(vertices.every(Number.isFinite)).toBe(true);
      expect(vertices.length).toBeLessThan(40_000);
      stages.push(vertices);
      disposeGroup(group);
    }
    expect(lastHeight).toBe(2); // Only the site and hoarding remain.
    signatures.push(stages);
  }
  expect(signatures[0]).toEqual(signatures[1]);
  art.dispose();
});

it('invalidates each removed floor even within an ordinary construction bucket', () => {
  expect(buildingRenderSignature({ ...building, progress: .01 }))
    .not.toBe(buildingRenderSignature({ ...building, progress: .11 }));
  expect(buildingRenderSignature({ ...building, progress: .11 }))
    .toBe(buildingRenderSignature({ ...building, progress: .15 }));
});
