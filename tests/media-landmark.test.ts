import { beforeEach, afterEach, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { mediaLandmarkStudy } from '../web/src/render/architecture/media-landmark-study';
import { resolveMediaLandmarks } from '../web/src/render/architecture/media-landmark-selection';
import {
  resolveBusinessRecipes,
  businessCanHostAdvertising,
} from '../web/src/render/architecture/business-recipes';
import { buildBusinessBuilding } from '../web/src/render/architecture/business';
import { ApartmentResources } from '../web/src/render/architecture/resources';
import { signageContexts, mediaArtworkSurface } from '../web/src/render/signage';
import { landmarkScreen } from '../web/src/render/architecture/media-landmark';
import { disposeGroup } from '../web/src/render/kit';
beforeEach(() =>
  vi.stubGlobal('document', {
    createElement: () => ({
      width: 0,
      height: 0,
      getContext: () =>
        new Proxy({} as Record<PropertyKey, unknown>, {
          get: (t, k) => t[k] ?? (() => undefined),
          set: (t, k, v) => {
            t[k] = v;
            return true;
          },
        }),
    }),
  }),
);
afterEach(() => vi.unstubAllGlobals());
const fixture = () => {
  const world = mediaLandmarkStudy(),
    recipes = resolveMediaLandmarks(
      world,
      resolveBusinessRecipes(world.buildings, 'mixed'),
      'shibuya',
    );
  return { world, recipes };
};
it('selects all three forms through production rules and excludes inappropriate hosts', () => {
  const { world, recipes } = fixture();
  expect([...recipes.values()].map((r) => r.mediaLandmark)).toEqual(['wrap', 'stack', 'frame']);
  expect(
    Array.from(
      resolveMediaLandmarks(
        world,
        resolveBusinessRecipes(world.buildings, 'older'),
        'shibuya',
      ).values(),
    ).every((r) => !r.mediaLandmark),
  ).toBe(true);
  expect(
    Array.from(
      resolveMediaLandmarks(
        { ...world, nodes: [], roads: [] },
        resolveBusinessRecipes(world.buildings, 'mixed'),
        'shibuya',
      ).values(),
    ).every((r) => !r.mediaLandmark),
  ).toBe(true);
  expect(
    Array.from(
      resolveMediaLandmarks(
        world,
        resolveBusinessRecipes(world.buildings, 'mixed'),
        'ameyoko',
      ).values(),
    ).every((r) => !r.mediaLandmark),
  ).toBe(true);
  const dense = {
    ...world,
    buildings: Array.from({ length: 10 }, (_, i) => ({
      ...world.buildings[0],
      id: i + 1,
      x: i * 8,
    })),
  };
  const choices = resolveMediaLandmarks(
    dense,
    resolveBusinessRecipes(dense.buildings, 'contemporary'),
    'shibuya',
  );
  const picked = dense.buildings.filter((b) => choices.get(b.id)?.mediaLandmark);
  expect(picked.length).toBeLessThanOrEqual(3);
  for (let i = 0; i < picked.length; i++)
    for (let j = i + 1; j < picked.length; j++)
      expect(
        Math.hypot(picked[i].x - picked[j].x, picked[i].z - picked[j].z),
      ).toBeGreaterThanOrEqual(100);
});
it('keeps all landmark forms in parcel bounds across lifecycle, LOD and display modes', () => {
  const { world, recipes } = fixture(),
    owner = new ApartmentResources();
  const contexts = signageContexts(
    world,
    new Set(
      world.buildings
        .filter((b) => businessCanHostAdvertising(recipes.get(b.id)!))
        .map((b) => b.id),
    ),
  );
  try {
    for (const b of world.buildings)
      for (const detail of ['close', 'district', 'far'] as const)
        for (const stage of [
          'open',
          'foundation',
          'structure',
          'surface',
          'renovation',
          'demolition',
        ] as const)
          for (const displays of ['animated', 'static', 'off'] as const) {
            const g = buildBusinessBuilding({ ...b, stage, progress: 0.76 }, owner.art, {
              recipe: recipes.get(b.id)!,
              detail,
              displays,
              signage: contexts.get(b.id),
            })!;
            const box = new THREE.Box3().setFromObject(g);
            expect(box.min.x).toBeGreaterThanOrEqual(-b.width / 2 - 0.001);
            expect(box.max.x).toBeLessThanOrEqual(b.width / 2 + 0.001);
            expect(box.min.z).toBeGreaterThanOrEqual(-b.depth / 2 - 0.001);
            expect(box.max.z).toBeLessThanOrEqual(b.depth / 2 + 0.001);
            expect(box.max.y).toBeLessThanOrEqual(0.58 + b.floors * 3.25 + 3);
            const expected = stage === 'open' && detail !== 'far' && displays !== 'off';
            expect(g.userData.businessArchitecture.displayPanels > 0).toBe(expected);
            g.traverse((o) => {
              if (o instanceof THREE.Mesh) {
                expect(owner.owns(o.material as THREE.Material)).toBe(true);
                expect(
                  Array.from(o.geometry.getAttribute('position').array).every(Number.isFinite),
                ).toBe(true);
              }
            });
            disposeGroup(g);
          }
  } finally {
    owner.dispose();
  }
}, 30000);
it('wraps the artwork continuously with outward normals and shares four campaign pairs', () => {
  const g = landmarkScreen(28, 24, 1.3),
    p = g.getAttribute('position'),
    uv = g.getAttribute('uv'),
    normal = g.getAttribute('normal');
  for (let i = 0; i < p.count; i++) {
    expect(p.getZ(i)).toBeGreaterThanOrEqual(-0.001);
    expect(p.getZ(i)).toBeLessThanOrEqual(1.301);
    expect(Math.hypot(normal.getX(i), normal.getY(i), normal.getZ(i))).toBeCloseTo(1, 5);
    expect(normal.getZ(i)).toBeGreaterThan(0);
    expect(uv.getX(i)).toBeGreaterThanOrEqual(0);
    expect(uv.getX(i)).toBeLessThanOrEqual(1);
  }
  g.dispose();
  const owner = new ApartmentResources();
  try {
    expect(
      new Set(Array.from({ length: 12 }, (_, i) => mediaArtworkSurface(owner.art, i))).size,
    ).toBe(4);
  } finally {
    owner.dispose();
  }
});
it('scales independent width, depth and height inputs without escaping the lot', () => {
  const { world, recipes } = fixture(),
    owner = new ApartmentResources();
  try {
    for (const b of world.buildings)
      for (const [width, depth, floors] of [
        [18, 14, 6],
        [45, 38, 16],
        [90, 80, 20],
      ]) {
        const g = buildBusinessBuilding({ ...b, width, depth, floors }, owner.art, {
          recipe: recipes.get(b.id)!,
          detail: 'close',
          signage: { proximity: 2, feature: 'screen' },
        })!;
        const box = new THREE.Box3().setFromObject(g);
        expect(Math.max(Math.abs(box.min.x), Math.abs(box.max.x))).toBeLessThanOrEqual(
          width / 2 + 0.001,
        );
        expect(Math.max(Math.abs(box.min.z), Math.abs(box.max.z))).toBeLessThanOrEqual(
          depth / 2 + 0.001,
        );
        expect(box.max.y).toBeLessThanOrEqual(0.58 + floors * 3.25 + 3);
        disposeGroup(g);
      }
  } finally {
    owner.dispose();
  }
});
