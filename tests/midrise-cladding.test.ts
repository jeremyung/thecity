import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingKit, disposeGroup } from '../web/src/render/kit';
import type { Building, Style } from '../web/src/types';

function midrise(kind: 'apartment' | 'office', stage: Building['stage'], progress: number, width: number, depth: number, floors = 8, seed = 9812): Building {
  return { id: 81, parcel_id: 28, x: 0, z: 0, rotation: 0, width, depth, floors, kind, stage, progress,
    capacity: 48, occupancy: 0, business_active: false, built_day: 4, name: `${kind} cladding`,
    appearance_seed: seed, history: [], anchor: false, vacant_days: 0 };
}

function boxesFor(root: THREE.Object3D, material: THREE.Material) {
  const boxes: string[] = [];
  root.traverse((object) => {
    if (!(object instanceof THREE.Mesh) || object.material !== material) return;
    const p = object.geometry.getAttribute('position');
    for (let start = 0; start + 23 < p.count; start += 24) {
      const bounds = [Infinity, -Infinity, Infinity, -Infinity, Infinity, -Infinity];
      for (let i = start; i < start + 24; i++) {
        const x = p.getX(i), y = p.getY(i), z = p.getZ(i);
        bounds[0] = Math.min(bounds[0], x); bounds[1] = Math.max(bounds[1], x);
        bounds[2] = Math.min(bounds[2], y); bounds[3] = Math.max(bounds[3], y);
        bounds[4] = Math.min(bounds[4], z); bounds[5] = Math.max(bounds[5], z);
      }
      boxes.push(bounds.map((value) => Math.round(value * 1000)).join(':'));
    }
  });
  return boxes.sort();
}

describe('midrise construction facade continuity', () => {
  it('installs authored glazing progressively and matches open glazing at full finish', () => {
    const art = new ArtMaterials();
    for (const style of ['anime', 'miniature'] as Style[]) {
      art.setStyle(style);
      const glass = art.get('#55757a', .25);
      const lightGlass = art.get('#94a7a0', .3);
      for (const [kind, width, depth, seed] of [
        ['apartment', 24, 19, 9812], // walk-up family
        ['office', 24, 19, 9812], // stepped office family
        ['office', 24, 19, 27], // slab office family
        ['apartment', 10, 10, 9812], ['office', 8, 12, 27], // narrow parcels
      ] as const) {
        const glow = art.get('#e9aa58', .42, true, kind === 'office' ? 'office' : 'home');
        const panes = (group: THREE.Object3D) => [glass, lightGlass, glow].flatMap(material => boxesFor(group, material)).sort();
        const partial = buildingKit(midrise(kind, 'structure', .77, width, depth, 8, seed), art);
        const finished = buildingKit(midrise(kind, 'surface', .92, width, depth, 8, seed), art);
        const openBuilding = midrise(kind, 'open', 1, width, depth, 8, seed);
        openBuilding.occupancy = 24;
        const open = buildingKit(openBuilding, art);
        const partialPanes = panes(partial);
        const finishedPanes = panes(finished);
        const openPanes = panes(open);
        expect(partialPanes.length, `${style} ${kind} partial authored panes`).toBeGreaterThan(0);
        expect(finishedPanes.length, `${style} ${kind} finished authored panes`).toBeGreaterThan(partialPanes.length);
        expect(finishedPanes, `${style} ${kind} finished/open glazing`).toEqual(openPanes);
        expect(boxesFor(partial, glow)).toHaveLength(0);
        expect(boxesFor(finished, glow)).toHaveLength(0);
        if (kind === 'apartment') expect(boxesFor(open, glow).length).toBeGreaterThan(0);
        for (const group of [partial, finished, open]) disposeGroup(group);
      }
    }
    art.dispose();
  });

  it('keeps rooftop details off the admitted facade until the full mass height is installed', () => {
    const art = new ArtMaterials();
    const early = buildingKit(midrise('office', 'structure', .77, 24, 19), art);
    const complete = buildingKit(midrise('office', 'surface', .92, 24, 19), art);
    const roofMaterials = ['#48575a', '#59676a', '#555d60', '#625d55'].map((color) => art.get(color));
    expect(roofMaterials.flatMap((material) => boxesFor(early, material))).toHaveLength(0);
    expect(roofMaterials.flatMap((material) => boxesFor(complete, material)).length).toBeGreaterThan(0);
    disposeGroup(early); disposeGroup(complete); art.dispose();
  });
});
