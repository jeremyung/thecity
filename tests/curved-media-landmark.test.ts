import { beforeEach, afterEach, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { mediaLandmarkStudy } from '../web/src/render/architecture/media-landmark-study';
import { resolveMediaLandmarks } from '../web/src/render/architecture/media-landmark-selection';
import { resolveBusinessRecipes } from '../web/src/render/architecture/business-recipes';
import { buildBusinessBuilding } from '../web/src/render/architecture/business';
import { curvedMediaRibbon } from '../web/src/render/architecture/curved-media-landmark';
import { ApartmentResources } from '../web/src/render/architecture/resources';
import { disposeGroup } from '../web/src/render/kit';
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
it('selects a fully curved building through production rules and bounds all lifecycle/LOD variants', () => {
  const world = mediaLandmarkStudy(32, true),
    b = world.buildings[0],
    recipe = resolveMediaLandmarks(
      world,
      resolveBusinessRecipes(world.buildings, 'mixed'),
      'shibuya',
    ).get(b.id)!;
  expect(recipe.mediaLandmark).toBe('curved');
  const owner = new ApartmentResources();
  try {
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
              recipe,
              detail,
              displays,
              signage: { proximity: 2, feature: 'screen' },
            })!,
            box = new THREE.Box3().setFromObject(g);
          expect(box.min.x).toBeGreaterThanOrEqual(-b.width / 2 - 0.001);
          expect(box.max.x).toBeLessThanOrEqual(b.width / 2 + 0.001);
          expect(box.min.z).toBeGreaterThanOrEqual(-b.depth / 2 - 0.001);
          expect(box.max.z).toBeLessThanOrEqual(b.depth / 2 + 0.001);
          expect(box.max.y).toBeLessThanOrEqual(0.58 + b.floors * 3.25 + 3);
          expect(g.userData.businessArchitecture.displayPanels).toBe(
            stage === 'open' && detail !== 'far' && displays !== 'off' ? 1 : 0,
          );
          g.traverse((o) => {
            if (o instanceof THREE.Mesh) {
              expect(owner.owns(o.material as THREE.Material)).toBe(true);
              expect(
                Array.from(o.geometry.getAttribute('position').array).every(Number.isFinite),
              ).toBe(true);
              if ((o.material as THREE.Material).userData.corporateGlass) {
                expect(o.geometry.hasAttribute('windowInfo')).toBe(true);
                const n = o.geometry.getAttribute('normal');
                for (let i = 0; i < n.count; i++)
                  expect(Math.hypot(n.getX(i), n.getY(i), n.getZ(i))).toBeCloseTo(1, 5);
              }
            }
          });
          disposeGroup(g);
        }
  } finally {
    owner.dispose();
  }
}, 30000);
it('carries one continuous panorama around both curved corners with outward analytic normals', () => {
  const g = curvedMediaRibbon(30, 24, 7, 18),
    p = g.getAttribute('position'),
    n = g.getAttribute('normal'),
    uv = g.getAttribute('uv');
  expect(n.getX(0)).toBeCloseTo(-1, 5);
  expect(n.getX(n.count - 1)).toBeCloseTo(1, 5);
  let last = -1;
  for (let i = 0; i < uv.count; i += 2) {
    expect(uv.getX(i)).toBeGreaterThanOrEqual(last);
    last = uv.getX(i);
    expect(Math.hypot(n.getX(i), n.getY(i), n.getZ(i))).toBeCloseTo(1, 5);
    expect(p.getX(i) * n.getX(i) + p.getZ(i) * n.getZ(i)).toBeGreaterThan(0);
  }
  expect(uv.getX(0)).toBe(0);
  expect(last).toBe(1);
  g.dispose();
});
