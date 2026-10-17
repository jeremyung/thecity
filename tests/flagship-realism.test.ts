import { addScrambleCrossing } from '../web/src/render/parks';
import { Kit, disposeGroup } from '../web/src/render/kit';
import { beforeEach, afterEach, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ApartmentResources } from '../web/src/render/architecture/resources';
import {
  campaignCell,
  flagshipCampaignAtlas,
  flagshipCampaignSurface,
  flagshipPosterSurface,
} from '../web/src/render/architecture/media-campaigns';
import {
  mediaIntersectionStudy,
  mediaIntersectionActors,
} from '../web/src/render/architecture/media-intersection-study';
import { MediaScreenLighting, MAX_MEDIA_SPILL_LIGHTS } from '../web/src/render/media-lighting';
import type { BusinessRecipe } from '../web/src/render/architecture/business-recipes';
beforeEach(() =>
  vi.stubGlobal('document', {
    createElement: () => ({
      width: 0,
      height: 0,
      getContext: () =>
        new Proxy({} as Record<PropertyKey, unknown>, {
          get: (t, k) =>
            k === 'createLinearGradient'
              ? () => ({ addColorStop() {} })
              : (t[k] ?? (() => undefined)),
          set: (t, k, v) => {
            t[k] = v;
            return true;
          },
        }),
    }),
  }),
);
afterEach(() => vi.unstubAllGlobals());
it('packs eight campaign scenes without overlap and owns one atlas for all campaign/style pairs', () => {
  const cells = Array.from({ length: 8 }, (_, i) => campaignCell(Math.floor(i / 2), i % 2));
  for (const c of cells) {
    expect(c.u0).toBeGreaterThan(0);
    expect(c.u1).toBeLessThan(1);
    expect(c.v0).toBeGreaterThan(0);
    expect(c.v1).toBeLessThan(1);
  }
  for (let i = 0; i < cells.length; i++)
    for (let j = i + 1; j < cells.length; j++) {
      const a = cells[i],
        b = cells[j];
      expect(a.u1 <= b.u0 || b.u1 <= a.u0 || a.v1 <= b.v0 || b.v1 <= a.v0).toBe(true);
    }
  const owner = new ApartmentResources(),
    atlas = flagshipCampaignAtlas(owner.art),
    dispose = vi.spyOn(atlas, 'dispose');
  expect(atlas.image.width).toBe(1024);
  expect(atlas.image.height).toBe(1024);
  expect(
    new Set(Array.from({ length: 12 }, (_, i) => flagshipCampaignSurface(owner.art, i))).size,
  ).toBe(4);
  expect(owner.owns(flagshipPosterSurface(owner.art))).toBe(true);
  expect(flagshipCampaignAtlas(owner.art)).toBe(atlas);
  owner.dispose();
  owner.dispose();
  expect(dispose).toHaveBeenCalledTimes(1);
});
it('limits screen spill to a fixed pool and respects day, off, far, activity and replacement', () => {
  const world = mediaIntersectionStudy(),
    hero = world.buildings[0],
    buildings = Array.from({ length: 4 }, (_, i) => ({ ...hero, id: i + 1, x: i * 10 }));
  const recipes = new Map(
      buildings.map((b) => [b.id, { mediaLandmark: 'curved' } as BusinessRecipe]),
    ),
    signage = new Map(
      buildings.map((b) => [b.id, { proximity: 2 as const, feature: 'screen' as const }]),
    ),
    ready = new Map(buildings.map((b) => [b.id, true])),
    details = new Map(buildings.map((b) => [b.id, 'close']));
  const pool = new MediaScreenLighting();
  try {
    expect(pool.lights).toHaveLength(MAX_MEDIA_SPILL_LIGHTS);
    expect(pool.root.children).toHaveLength(4);
    pool.setWorld({ buildings }, recipes, signage, 'animated');
    pool.update(new THREE.Vector3(), 3, 1, 'miniature', details, ready);
    expect(pool.lights.filter((l) => l.intensity > 0)).toHaveLength(2);
    expect(pool.lights.every((l) => !l.castShadow)).toBe(true);
    for (let i = 0; i < 2; i++) {
      const b = buildings[i],
        l = pool.lights[i];
      const outward =
        (l.position.x - b.x) * Math.sin(b.rotation) + (l.position.z - b.z) * Math.cos(b.rotation);
      expect(outward).toBeGreaterThan(b.depth / 2);
    }
    pool.update(new THREE.Vector3(), 4, 0, 'miniature', details, ready);
    expect(pool.lights.every((l) => l.intensity === 0)).toBe(true);
    for (const b of buildings) details.set(b.id, 'far');
    pool.update(new THREE.Vector3(), 4, 1, 'miniature', details, ready);
    expect(pool.lights.every((l) => l.intensity === 0)).toBe(true);
    pool.setWorld({ buildings }, recipes, signage, 'off');
    pool.update(new THREE.Vector3(), 4, 1, 'miniature', new Map(), ready);
    expect(pool.lights.every((l) => l.intensity === 0)).toBe(true);
    pool.setWorld(
      { buildings: buildings.map((b) => ({ ...b, business_active: false })) },
      recipes,
      signage,
      'animated',
    );
    pool.update(new THREE.Vector3(), 4, 1, 'miniature', new Map(), ready);
    expect(pool.lights.every((l) => l.intensity === 0)).toBe(true);
    pool.reset();
    expect(pool.lights.every((l) => l.intensity === 0)).toBe(true);
  } finally {
    pool.dispose();
  }
  expect(pool.root.children).toHaveLength(0);
});
it('places the flagship in a paved four-way district with non-overlapping lots and supported review actors', () => {
  const world = mediaIntersectionStudy();
  expect(world.roads).toHaveLength(4);
  expect(world.ground).toHaveLength(4);
  for (let i = 0; i < world.buildings.length; i++)
    for (let j = i + 1; j < world.buildings.length; j++) {
      const a = world.buildings[i],
        b = world.buildings[j];
      expect(
        Math.abs(a.x - b.x) >= (a.width + b.width) / 2 ||
          Math.abs(a.z - b.z) >= (a.depth + b.depth) / 2,
      ).toBe(true);
    }
  const actors = mediaIntersectionActors();
  expect(actors.movement.length / 6).toBe(actors.support.size);
  for (const data of actors.support.values()) {
    expect(data.resolved).toBe(true);
    expect(data.roads.every((id) => world.roads.some((r) => r.id === id))).toBe(true);
  }
  expect(world.buildings[0].width).toBe(38);
  const owner = new ApartmentResources(),
    kit = new Kit();
  try {
    expect(addScrambleCrossing(kit, world, owner.art)).toBe(1);
    disposeGroup(kit.finish());
  } finally {
    owner.dispose();
  }
});
