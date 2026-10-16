import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingKit, disposeGroup } from '../web/src/render/kit';
import { WINDOW_ATTRIBUTE } from '../web/src/render/building-surfaces';
import type { Building, Style } from '../web/src/types';

function midrise(
  kind: 'apartment' | 'office',
  stage: Building['stage'],
  progress: number,
  width: number,
  depth: number,
  floors = 8,
  seed = 9812,
): Building {
  return {
    id: 81,
    parcel_id: 28,
    x: 0,
    z: 0,
    rotation: 0,
    width,
    depth,
    floors,
    kind,
    stage,
    progress,
    capacity: 48,
    occupancy: 0,
    business_active: false,
    built_day: 4,
    name: `${kind} cladding`,
    appearance_seed: seed,
    history: [],
    anchor: false,
    vacant_days: 0,
  };
}

function boxesFor(root: THREE.Object3D, material: THREE.Material) {
  const boxes: string[] = [];
  root.traverse((object) => {
    if (!(object instanceof THREE.Mesh) || object.material !== material) return;
    const p = object.geometry.getAttribute('position');
    for (let start = 0; start + 23 < p.count; start += 24) {
      const bounds = [Infinity, -Infinity, Infinity, -Infinity, Infinity, -Infinity];
      for (let i = start; i < start + 24; i++) {
        const x = p.getX(i),
          y = p.getY(i),
          z = p.getZ(i);
        bounds[0] = Math.min(bounds[0], x);
        bounds[1] = Math.max(bounds[1], x);
        bounds[2] = Math.min(bounds[2], y);
        bounds[3] = Math.max(bounds[3], y);
        bounds[4] = Math.min(bounds[4], z);
        bounds[5] = Math.max(bounds[5], z);
      }
      boxes.push(bounds.map((value) => Math.round(value * 1000)).join(':'));
    }
  });
  return boxes.sort();
}

describe('midrise construction facade continuity', () => {
  it('installs the terrace finish only with the completed upper tier', () => {
    const art = new ArtMaterials();
    try {
      const terraceY = 0.58 + 7 * 3.25;
      for (const [stage, progress, complete] of [
        ['structure', 0.77, false],
        ['surface', 0.92, true],
        ['open', 1, true],
      ] as const) {
        const group = buildingKit(midrise('apartment', stage, progress, 12, 14, 8, 32), art);
        group.updateMatrixWorld(true);
        const hit = new THREE.Raycaster(
          new THREE.Vector3(0, terraceY + 0.5, 4),
          new THREE.Vector3(0, -1, 0),
        ).intersectObject(group, true)[0];
        expect(hit).toBeDefined();
        if (complete) {
          expect(hit.point.y).toBeCloseTo(terraceY + 0.05, 5);
          expect((hit.object as THREE.Mesh).material).toBe(art.get('#aaa99e'));
        } else {
          expect(hit.point.y).toBeLessThan(terraceY);
        }
        disposeGroup(group);
      }
    } finally {
      art.dispose();
    }
  });

  it('installs authored glazing progressively and matches open glazing at full finish', () => {
    const art = new ArtMaterials();
    for (const style of ['anime', 'miniature'] as Style[]) {
      art.setStyle(style);
      const glass = art.get('#55757a', 0.25);
      const lightGlass = art.get('#94a7a0', 0.3);
      for (const [kind, width, depth, seed] of [
        ['apartment', 24, 19, 9812], // walk-up family
        ['apartment', 24, 19, 32],
        ['apartment', 8, 10, 32], // terrace family
        ['office', 24, 19, 9812], // stepped office family
        ['office', 24, 19, 27], // slab office family
        ['apartment', 10, 10, 9812],
        ['office', 8, 12, 27], // narrow parcels
      ] as const) {
        const glow = art.window(kind === 'office' ? 'office' : 'home');
        const panes = (group: THREE.Object3D) =>
          [glass, lightGlass, glow].flatMap((material) => boxesFor(group, material)).sort();
        const partial = buildingKit(midrise(kind, 'structure', 0.77, width, depth, 8, seed), art);
        const finished = buildingKit(midrise(kind, 'surface', 0.92, width, depth, 8, seed), art);
        const openBuilding = midrise(kind, 'open', 1, width, depth, 8, seed);
        openBuilding.occupancy = 24;
        const open = buildingKit(openBuilding, art);
        const partialPanes = panes(partial);
        const finishedPanes = panes(finished);
        const openPanes = panes(open);
        expect(partialPanes.length, `${style} ${kind} partial authored panes`).toBeGreaterThan(0);
        expect(finishedPanes.length, `${style} ${kind} finished authored panes`).toBeGreaterThan(
          partialPanes.length,
        );
        expect(finishedPanes, `${style} ${kind} finished/open glazing`).toEqual(openPanes);
        // Pane geometry is installed during construction, but occupancy stays
        // off until the building operates. Schedule and lived state are packed.
        for (const [group, lived] of [
          [partial, 0],
          [finished, 0],
          [open, kind === 'apartment' ? 1 : 0],
        ] as const) {
          const states: number[] = [];
          group.traverse((object) => {
            if (!(object instanceof THREE.Mesh) || object.material !== glow) return;
            const info = object.geometry.getAttribute(WINDOW_ATTRIBUTE);
            for (let i = 0; i < info.count; i++) states.push(info.getY(i));
          });
          expect(states.length).toBeGreaterThan(0);
          expect(new Set(states)).toEqual(new Set([lived + (kind === 'office' ? 2 : 0)]));
        }
        for (const group of [partial, finished, open]) disposeGroup(group);
      }
    }
    art.dispose();
  });

  it('installs the flat roof membrane only at full height and retains it after opening', () => {
    const art = new ArtMaterials();
    try {
      for (const style of ['anime', 'miniature'] as const) {
        art.setStyle(style);
        const early = buildingKit(midrise('office', 'structure', 0.77, 24, 19), art);
        const complete = buildingKit(midrise('office', 'surface', 0.92, 24, 19), art);
        const open = buildingKit(midrise('office', 'open', 1, 24, 19), art);
        const membranes = ['#8b908b'].map((color) => art.get(color, 0.92));
        const fields = (root: THREE.Object3D) =>
          membranes.flatMap((material) => boxesFor(root, material)).sort();
        expect(fields(early)).toHaveLength(0);
        const installed = fields(complete);
        expect(installed.length).toBeGreaterThan(0);
        expect(installed).toEqual(fields(open));
        // The membrane belongs above the eighth floor, never on a partial tier.
        for (const box of installed)
          expect(Number(box.split(':')[2]) / 1000).toBeGreaterThanOrEqual(0.58 + 8 * 3.25);
        disposeGroup(early);
        disposeGroup(complete);
        disposeGroup(open);
      }
    } finally {
      art.dispose();
    }
  });
});
