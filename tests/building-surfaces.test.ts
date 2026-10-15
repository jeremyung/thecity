import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { CLADDING_ATTRIBUTE, CLADDING_INDEX, claddingFor, paneSeed, tagWindowPane, WINDOW_ATTRIBUTE, windowActivity, windowLived, windowOccupancy, type Cladding } from '../web/src/render/building-surfaces';
import { ArtMaterials } from '../web/src/render/materials';
import { Kit } from '../web/src/render/kit';
import type { Building } from '../web/src/types';

const building = (kind: string, floors: number, seed: number, extra: Partial<Building> = {}): Building => ({
  id: seed, parcel_id: seed, x: 0, z: 0, rotation: 0, width: 10, depth: 10, floors, kind, stage: 'open', progress: 1,
  capacity: 10, occupancy: 5, built_day: 0, name: kind, appearance_seed: seed, history: [], anchor: false, vacant_days: 0, ...extra,
} as Building);

describe('cladding choice', () => {
  it('is stable per building and follows Tokyo finishes by use', () => {
    const tally = (kind: string, floors: number) => {
      const counts: Partial<Record<Cladding, number>> = {};
      for (let seed = 1; seed <= 2000; seed++) {
        const choice = claddingFor(building(kind, floors, seed * 7919));
        expect(claddingFor(building(kind, floors, seed * 7919))).toBe(choice);
        counts[choice] = (counts[choice] ?? 0) + 1;
      }
      return counts;
    };
    const homes = tally('home', 2);
    expect(Object.keys(homes).sort()).toEqual(['plaster', 'siding']);
    expect(homes.siding! / 2000).toBeGreaterThan(.45);
    const apartments = tally('apartment', 6);
    expect(apartments.tile! / 2000).toBeGreaterThan(.45);
    expect(apartments.siding).toBeUndefined();
    const offices = tally('office', 6);
    expect(new Set(Object.keys(offices))).toEqual(new Set(['tile', 'alc', 'concrete']));
    expect(claddingFor(building('station', 2, 5))).toBe('plaster');
  });
});

describe('window schedules', () => {
  it('fill homes after dusk, offices by day, and shops in trading hours', () => {
    const at = (activity: Parameters<typeof windowOccupancy>[0], hour: number) => windowOccupancy(activity, hour);
    expect(at('home', 20)).toBeGreaterThan(at('home', 12) * 3);
    expect(at('home', 3)).toBeLessThan(at('home', 20) / 2);
    expect(at('office', 12)).toBeGreaterThan(.8);
    expect(at('office', 2)).toBeLessThan(.1);
    expect(at('shop', 15)).toBeGreaterThan(.6);
    expect(at('shop', 4)).toBeLessThan(.1);
    // Continuous through the day, including across midnight, so lights switch on progressively.
    for (const activity of ['home', 'office', 'shop'] as const) {
      for (let hour = 0; hour < 24; hour += .25) {
        const value = at(activity, hour);
        expect(value).toBeGreaterThanOrEqual(0); expect(value).toBeLessThanOrEqual(1);
        expect(Math.abs(at(activity, hour + .25) - value)).toBeLessThan(.2);
      }
      expect(at(activity, 24)).toBeCloseTo(at(activity, 0), 9);
    }
    // Night emission stays restrained: at most about half the homes are lit at the evening peak.
    expect(Math.max(...Array.from({ length: 96 }, (_, i) => at('home', i / 4)))).toBeLessThanOrEqual(.55);
  });

  it('maps building uses to schedules and keeps vacant or unfinished buildings unlived', () => {
    expect(windowActivity('apartment')).toBe('home');
    expect(windowActivity('cafe')).toBe('shop');
    expect(windowActivity('station')).toBe('office');
    expect(windowLived(building('home', 2, 1))).toBe(1);
    expect(windowLived(building('home', 2, 1, { occupancy: 0 }))).toBe(0);
    expect(windowLived(building('shop', 2, 1, { business_active: false }))).toBe(0);
    expect(windowLived(building('office', 4, 1, { stage: 'construction' }))).toBe(0);
  });
});

describe('window panes', () => {
  it('tag each pane box with its seed, occupancy and size, deterministically', () => {
    const kit = new Kit(), art = new ArtMaterials();
    kit.windowContext = { seed: 1234, lived: 1 };
    const pane = art.window('home');
    kit.box(1, 2, 3, 1.8, 1.4, .04, pane);
    kit.box(-1, 2, 3, .04, 1.2, 1.1, pane);
    kit.box(0, 0, 0, 1, 1, 1, art.get('#aaa99e'));
    const group = kit.finish();
    const mesh = group.children.find(child => (child as THREE.Mesh).material === pane) as THREE.Mesh;
    const info = mesh.geometry.getAttribute(WINDOW_ATTRIBUTE);
    expect(info.count).toBe(48);
    expect([info.getX(0), info.getY(0), info.getZ(0), info.getW(0)]).toEqual([Math.fround(paneSeed(1234, 1, 2, 3)), 1, Math.fround(1.8), Math.fround(1.4)]);
    // The thin dimension is depth, whichever way the pane faces.
    expect(info.getZ(24)).toBeCloseTo(1.1, 6);
    expect(info.getW(24)).toBeCloseTo(1.2, 6);
    const other = group.children.find(child => (child as THREE.Mesh).material !== pane) as THREE.Mesh;
    expect(other.geometry.getAttribute(WINDOW_ATTRIBUTE)).toBeUndefined();
    expect(paneSeed(1234, 1, 2, 3)).not.toBe(paneSeed(1234, 1, 2, 3.4));
    expect(paneSeed(1234, 1, 2, 3)).toBeGreaterThanOrEqual(0); expect(paneSeed(1234, 1, 2, 3)).toBeLessThan(1);
    group.traverse(o => { if (o instanceof THREE.Mesh) o.geometry.dispose(); });
    art.dispose();
  });

  it('packs the schedule with occupancy and tags clad walls with their kind', () => {
    const kit = new Kit(), art = new ArtMaterials();
    kit.windowContext = { seed: 9, lived: 1 };
    kit.box(0, 0, 0, 2, 1.5, .04, art.window('office'));
    kit.box(3, 0, 0, 2, 1.5, .04, art.window('shop'));
    kit.box(0, 0, 3, 4, 3, 4, art.get('#c8b99f', .85, false, 'ambient', 'alc'));
    kit.add(new THREE.PlaneGeometry(1, 1), art.get('#c8b99f', .85, false, 'ambient', 'alc'));
    const group = kit.finish();
    const meshes = group.children as THREE.Mesh[];
    const office = meshes.find(mesh => mesh.material === art.window('office'))!.geometry.getAttribute(WINDOW_ATTRIBUTE);
    const shop = meshes.find(mesh => mesh.material === art.window('shop'))!.geometry.getAttribute(WINDOW_ATTRIBUTE);
    expect(office.getY(0)).toBe(1 + 2 * 1);
    expect(shop.getY(0)).toBe(1 + 2 * 2);
    const wall = meshes.find(mesh => mesh.geometry.getAttribute(CLADDING_ATTRIBUTE))!.geometry;
    expect(wall.getAttribute(CLADDING_ATTRIBUTE).count).toBe(wall.getAttribute('position').count);
    expect(new Set(Array.from(wall.getAttribute(CLADDING_ATTRIBUTE).array))).toEqual(new Set([CLADDING_INDEX.alc]));
    group.traverse(o => { if (o instanceof THREE.Mesh) o.geometry.dispose(); });
    art.dispose();
  });

  it('tags arbitrary geometry with one value per vertex', () => {
    const geometry = new THREE.BoxGeometry(1, 1, 1);
    tagWindowPane(geometry, .5, 0, 2, 1.5);
    const info = geometry.getAttribute(WINDOW_ATTRIBUTE);
    expect(info.itemSize).toBe(4);
    expect(info.count).toBe(geometry.getAttribute('position').count);
    expect(info.getY(7)).toBe(0);
    geometry.dispose();
  });
});

describe('surface materials', () => {
  it('give cladding its own batch class and share one window pair per schedule across styles', () => {
    const art = new ArtMaterials();
    const plaster = art.get('#c8b99f'), tile = art.get('#c8b99f', .85, false, 'ambient', 'tile');
    expect(tile).not.toBe(plaster);
    expect(art.get('#c8b99f', .85, false, 'ambient', 'plaster')).toBe(plaster);
    expect(tile.userData.artBuildingClass).toMatchObject({ painted: true, cladding: 'tile' });
    expect(tile.userData.artBuildingClass.key).not.toBe(plaster.userData.artBuildingClass.key);
    // All clad walls share one batch class; the kind travels per vertex.
    const siding = art.get('#9aa39a', .85, false, 'ambient', 'siding');
    expect(siding.userData.artBuildingClass.key).toBe(tile.userData.artBuildingClass.key);
    expect(art.buildingBatch(siding).material).toBe(art.buildingBatch(tile).material);
    // Every pane, whatever its schedule, shares one batch material.
    expect(art.buildingBatch(art.window('office')).material).toBe(art.buildingBatch(art.window('home')).material);
    // Glazing and emission never take cladding.
    expect(art.get('#55757a', .25, false, 'ambient', 'tile').userData.artBuildingClass.cladding).toBeUndefined();
    const home = art.window('home');
    expect(art.window('home')).toBe(home);
    expect(art.window('office')).not.toBe(home);
    expect(home.userData).toMatchObject({ windowPane: true, windowActivity: 'home' });
    art.setStyle('miniature');
    const miniature = art.window('home');
    expect(miniature).not.toBe(home);
    expect(art.variant(home)).toBe(miniature);
    art.dispose();
  });
});
