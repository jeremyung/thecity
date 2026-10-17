import { expect, it, beforeAll, afterAll, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { disposeGroup } from '../web/src/render/kit';
import {
  buildApartmentBuilding,
  APARTMENT_RECIPE_IDS,
  apartmentRecipe,
} from '../web/src/render/architecture';
import { blockStudyBuildings, buildBlockStudy } from '../web/src/render/architecture/block-study';
import { ApartmentResources } from '../web/src/render/architecture/resources';
import type { Building } from '../web/src/types';
const canvasContext = {
  fillRect() {},
  fillText() {},
  strokeRect() {},
  beginPath() {},
  moveTo() {},
  quadraticCurveTo() {},
  stroke() {},
  ellipse() {},
  fill() {},
};
beforeAll(() =>
  vi.stubGlobal('document', {
    createElement: () => ({ width: 0, height: 0, getContext: () => canvasContext }),
  }),
);
afterAll(() => vi.unstubAllGlobals());
const specimen = (width = 12, depth = 14, floors = 7): Building => ({
  id: 1,
  parcel_id: 1,
  x: 0,
  z: 0,
  rotation: 0,
  width,
  depth,
  floors,
  appearance_seed: 32,
  kind: 'apartment' as const,
  stage: 'open' as const,
  progress: 1,
  capacity: 40,
  occupancy: 20,
  built_day: 0,
  name: 'pipeline test',
  history: [],
  anchor: false,
  vacant_days: 0,
  business_active: false,
});
const triangles = (group: THREE.Object3D) => {
  let total = 0;
  group.traverse((o) => {
    if (o instanceof THREE.Mesh)
      total += (o.geometry.index?.count ?? o.geometry.getAttribute('position').count) / 3;
  });
  return total;
};
it('all ten recipes fit variable parcels and keep independent depth, width and floor parameters', () => {
  const art = new ArtMaterials();
  try {
    for (const recipe of APARTMENT_RECIPE_IDS)
      for (const width of [5, 8.5, 12, 24])
        for (const depth of [8, 24])
          for (const floors of [4, 11]) {
            const b = specimen(width, depth, floors),
              original = JSON.stringify(b),
              g = buildApartmentBuilding(b, art, { recipe })!;
            const bounds = new THREE.Box3().setFromObject(g),
              epsilon = 1e-5;
            expect(bounds.min.x, `${recipe}/${width}/${depth}`).toBeGreaterThanOrEqual(
              -width / 2 - epsilon,
            );
            expect(bounds.max.x).toBeLessThanOrEqual(width / 2 + epsilon);
            expect(bounds.min.z).toBeGreaterThanOrEqual(-depth / 2 - epsilon);
            expect(bounds.max.z).toBeLessThanOrEqual(depth / 2 + epsilon);
            expect(g.userData.apartmentPilot.recipe).toBe(recipe);
            expect(JSON.stringify(b)).toBe(original);
            disposeGroup(g);
          }
  } finally {
    art.dispose();
  }
}, 20000);
it('district detail reduces geometry while preserving silhouette and entrances', () => {
  const art = new ArtMaterials();
  try {
    for (const recipe of APARTMENT_RECIPE_IDS) {
      const b = specimen(),
        close = buildApartmentBuilding(b, art, { recipe, detail: 'close' })!,
        district = buildApartmentBuilding(b, art, { recipe, detail: 'district' })!;
      expect(triangles(district)).toBeLessThan(triangles(close));
      const a = new THREE.Box3().setFromObject(close),
        c = new THREE.Box3().setFromObject(district);
      expect(a.min.distanceTo(c.min)).toBeLessThan(0.001);
      expect(a.max.distanceTo(c.max)).toBeLessThan(0.001);
      const ray = new THREE.Raycaster(
        new THREE.Vector3(0, 1.8, b.depth / 2 + 2),
        new THREE.Vector3(0, 0, -1),
      );
      expect(ray.intersectObject(district, true)[0]).toBeDefined();
      disposeGroup(close);
      disposeGroup(district);
    }
  } finally {
    art.dispose();
  }
});
it('selects recipes deterministically and rejects unsupported integration inputs', () => {
  const art = new ArtMaterials();
  try {
    expect(
      new Set(
        Array.from(
          { length: 128 },
          (_, appearance_seed) => apartmentRecipe({ appearance_seed }).id,
        ),
      ).size,
    ).toBe(10);
    expect(apartmentRecipe({ appearance_seed: 32 }).id).toBe(
      apartmentRecipe({ appearance_seed: 32 }).id,
    );
    for (const b of [
      { ...specimen(), width: NaN },
      { ...specimen(), depth: Infinity },
      { ...specimen(), floors: 2 },
      { ...specimen(), kind: 'office' as const },
    ])
      expect(buildApartmentBuilding(b, art)).toBeNull();
  } finally {
    art.dispose();
  }
});
it('builds a mixed block without overlapping parcels or sharing mutable geometry', () => {
  const art = new ArtMaterials();
  try {
    const specs = blockStudyBuildings(),
      block = buildBlockStudy(art);
    expect(specs).toHaveLength(24);
    expect(block.userData.buildings).toBe(24);
    for (let i = 0; i < specs.length; i++)
      for (let j = i + 1; j < specs.length; j++) {
        const a = specs[i],
          b = specs[j];
        expect(
          Math.abs(a.x - b.x) >= (a.width + b.width) / 2 ||
            Math.abs(a.z - b.z) >= (a.depth + b.depth) / 2,
        ).toBe(true);
      }
    const one = buildApartmentBuilding(specimen(), art, { recipe: 'gallery' })!,
      two = buildApartmentBuilding(specimen(), art, { recipe: 'gallery' })!;
    const g1 = (one.children[0] as THREE.Mesh).geometry,
      g2 = (two.children[0] as THREE.Mesh).geometry;
    expect(g1).not.toBe(g2);
    expect(g1.getAttribute('position').array).not.toBe(g2.getAttribute('position').array);
    disposeGroup(one);
    expect(triangles(two)).toBeGreaterThan(0);
    disposeGroup(two);
    disposeGroup(block);
  } finally {
    art.dispose();
  }
});
it('owns surface resources per session and allows idempotent teardown', async () => {
  const resources = new ApartmentResources();
  resources.dispose();
  resources.dispose();
  expect(await resources.loadTextures()).toBe(false);
});
it('adjacent seeds change architectural geometry within one fixed recipe and remain repeatable', () => {
  const art = new ArtMaterials();
  const signature = (g: THREE.Group) => {
    let hash = 2166136261;
    g.traverse((o) => {
      if (o instanceof THREE.Mesh)
        for (const v of o.geometry.getAttribute('position').array) {
          hash = Math.imul(hash ^ Math.round(v * 10000), 16777619);
        }
    });
    return hash >>> 0;
  };
  try {
    const signatures: number[] = [],
      features: Record<string, unknown>[] = [];
    for (let seed = 0; seed < 12; seed++) {
      const b = { ...specimen(8.5, 8, 4), appearance_seed: seed };
      const a = buildApartmentBuilding(b, art, { recipe: 'gallery', detail: 'district' })!,
        repeat = buildApartmentBuilding(b, art, { recipe: 'gallery', detail: 'district' })!;
      expect(signature(a)).toBe(signature(repeat));
      signatures.push(signature(a));
      features.push(a.userData.apartmentPilot.features);
      const bounds = new THREE.Box3().setFromObject(a);
      expect(bounds.min.x).toBeGreaterThanOrEqual(-b.width / 2 - 0.001);
      expect(bounds.max.x).toBeLessThanOrEqual(b.width / 2 + 0.001);
      expect(bounds.min.z).toBeGreaterThanOrEqual(-b.depth / 2 - 0.001);
      expect(bounds.max.z).toBeLessThanOrEqual(b.depth / 2 + 0.001);
      disposeGroup(a);
      disposeGroup(repeat);
    }
    expect(new Set(signatures).size).toBe(12);
    for (const key of ['windows', 'stairSide', 'stairScreen', 'canopy', 'roof', 'balconyScreens'])
      expect(new Set(features.map((f) => f[key])).size, key).toBeGreaterThan(1);
  } finally {
    art.dispose();
  }
});
it('household dressing is sparse and avoids laundry stacked on consecutive balconies', async () => {
  const { householdDressing } = await import('../web/src/render/architecture/dressing');
  let occupied = 0,
    total = 0;
  const profiles = new Set<string>();
  for (let seed = 0; seed < 20; seed++)
    for (let bay = 0; bay < 4; bay++)
      for (let floor = 1; floor < 11; floor++) {
        const p = householdDressing(seed, floor, bay);
        total++;
        if (p.laundry) {
          occupied++;
          profiles.add(`${p.items}/${p.offset}/${p.variant}`);
          expect(householdDressing(seed, floor + 1, bay).laundry).toBe(false);
        }
      }
  expect(occupied / total).toBeGreaterThan(0.08);
  expect(occupied / total).toBeLessThan(0.25);
  expect(profiles.size).toBeGreaterThan(20);
});
it('keeps exterior stairs outside the clear gallery and leaves openings at landings', async () => {
  const { circulationLayout } = await import('../web/src/render/architecture/circulation');
  for (const width of [5, 7.5, 12, 24])
    for (const depth of [8, 14, 24]) {
      const c = circulationLayout(width, depth);
      expect(c.stairZ + c.stairWidth / 2).toBeLessThan(c.galleryZ - c.galleryDepth / 2);
      expect(c.stairZ - c.stairWidth / 2).toBeGreaterThan(-depth / 2);
      if (c.external) {
        expect(c.stairLength / 18).toBeGreaterThan(0.24);
        expect(c.landingWidth).toBeGreaterThanOrEqual(c.stairWidth);
      }
    }
});
it('uses hysteresis and progressively cheaper geometry at camera distance', async () => {
  const { apartmentDetail } = await import('../web/src/render/architecture/detail');
  expect(apartmentDetail(94)).toBe('close');
  expect(apartmentDetail(120, 'close')).toBe('close');
  expect(apartmentDetail(140, 'close')).toBe('district');
  expect(apartmentDetail(361, 'district')).toBe('far');
  expect(apartmentDetail(320, 'far')).toBe('far');
  expect(apartmentDetail(290, 'far')).toBe('district');
  const resources = new ApartmentResources();
  try {
    for (const recipe of APARTMENT_RECIPE_IDS) {
      const groups = (['close', 'district', 'far'] as const).map((detail) =>
        buildApartmentBuilding(specimen(), resources.art, { recipe, detail })!,
      );
      expect(triangles(groups[2]), recipe).toBeLessThan(triangles(groups[1]));
      expect(triangles(groups[1]), recipe).toBeLessThan(triangles(groups[0]));
      groups.forEach(disposeGroup);
    }
  } finally {
    resources.dispose();
  }
});
it('batches the actual 24-building fixture with shader identity, picking and style variants intact', async () => {
  const { BuildingChunks } = await import('../web/src/render/building-chunks');
  const resources = new ApartmentResources(),
    chunks = new BuildingChunks(
      (m) => (resources.owns(m) ? resources.batch(m) : resources.art.buildingBatch(m)),
      (release) => release(),
    );
  let sourceMeshes = 0;
  try {
    for (const b of blockStudyBuildings()) {
      const createGroup = () => {
        const g =
          buildApartmentBuilding(b, resources.art, { recipe: b.recipe, detail: 'district' }) ??
          buildBlockStudyFallback(b);
        g.position.set(b.x, 0, b.z);
        g.rotation.y = b.rotation;
        return g;
      };
      // Only apartment sources belong to this material-owner integration test.
      if (b.kind !== 'apartment') continue;
      const g = createGroup();
      sourceMeshes += g.children.length;
      chunks.set(b.id, g, { createGroup, disposeGroup });
    }
    let updates = 0;
    while (chunks.pendingCount && updates++ < 10000) chunks.update();
    expect(chunks.pendingCount).toBe(0);
    const meshes: THREE.Mesh[] = [];
    chunks.root.traverse((o) => {
      if (o instanceof THREE.Mesh) meshes.push(o);
    });
    expect(meshes.length).toBeLessThan(sourceMeshes / 2);
    expect(meshes.some((m) => resources.owns(m.material as THREE.Material))).toBe(true);
    for (const mesh of meshes) {
      expect(mesh.userData.buildingSpans.length).toBeGreaterThan(0);
      const m = mesh.material as THREE.Material;
      if (resources.owns(m)) {
        expect(resources.variant(m, 'anime').userData.artKey).toBe(m.userData.artKey);
        if (m.userData.apartmentSurface && m.userData.apartmentRole !== 'glass')
          expect(mesh.geometry.getAttribute('architectureWear')).toBeDefined();
      }
    }
    console.log(
      `Architecture chunk check: ${sourceMeshes} source meshes -> ${meshes.length} chunk meshes; ${updates} bounded updates`,
    );
  } finally {
    chunks.clear();
    resources.dispose();
  }
  function buildBlockStudyFallback(_b: Building): THREE.Group {
    throw Error('Unexpected fallback');
  }
}, 20000);
it('replaces real apartment construction batches through cladding, completion and removal', async () => {
  const { ConstructionBatches } = await import('../web/src/render/construction-batches');
  const resources = new ApartmentResources(),
    chunks = new ConstructionBatches(
      (m) => (resources.owns(m) ? resources.batch(m) : resources.art.buildingBatch(m)),
      (release) => release(),
    );
  const drain = () => {
    let n = 0;
    while (chunks.pendingCount && n++ < 10000) chunks.update();
    expect(chunks.pendingCount).toBe(0);
  };
  try {
    for (const stage of ['structure', 'surface', 'open', 'renovation', 'open'] as const) {
      const b = { ...specimen(), stage, progress: stage === 'open' ? 1 : 0.76 };
      const createGroup = () =>
        buildApartmentBuilding(b, resources.art, { recipe: 'modern', detail: 'district' })!;
      const descriptor = { createGroup, disposeGroup };
      chunks.set(b.id, createGroup(), descriptor, stage !== 'open');
      drain();
      expect(chunks.isSourceVisible(b.id, descriptor)).toBe(true);
    }
    chunks.remove(1);
    drain();
    expect(chunks.hasVisibleSource(1)).toBe(false);
  } finally {
    chunks.clear();
    resources.dispose();
  }
}, 20000);
it('varies entrance positions coherently and keeps doors recessed behind the facade', async () => {
  const { entranceLayout } = await import('../web/src/render/architecture/character');
  const { APARTMENT_RECIPES } = await import('../web/src/render/architecture/recipes');
  const art = new ArtMaterials();
  try {
    for (const recipe of APARTMENT_RECIPE_IDS) {
      const positions = new Set<number>();
      for (let seed = 0; seed < 12; seed++) {
        const b = { ...specimen(), appearance_seed: seed },
          layout = entranceLayout(b, APARTMENT_RECIPES[recipe]);
        positions.add(layout.x);
        const group = buildApartmentBuilding(b, art, { recipe, detail: 'district' })!;
        expect(group.userData.apartmentPilot.entrance.x).toBe(layout.x);
        const front = group.userData.apartmentPilot.circulation.front;
        const hit = new THREE.Raycaster(
          new THREE.Vector3(layout.x, 1.8, b.depth / 2 + 2),
          new THREE.Vector3(0, 0, -1),
        ).intersectObject(group, true)[0];
        expect(hit, `${recipe}/${seed}`).toBeDefined();
        expect(hit.point.z, `${recipe}/${seed}`).toBeLessThan(front + 0.16);
        const bounds = new THREE.Box3().setFromObject(group);
        expect(bounds.min.x).toBeGreaterThanOrEqual(-b.width / 2 - 0.001);
        expect(bounds.max.x).toBeLessThanOrEqual(b.width / 2 + 0.001);
        disposeGroup(group);
      }
      expect(positions.size).toBe(3);
    }
    expect(entranceLayout(specimen(5), APARTMENT_RECIPES.gallery).x).toBe(0);
  } finally {
    art.dispose();
  }
}, 20000);
it('keeps seeded weather profiles on geometry through batching without allocating a material per building', async () => {
  const { buildingWeathering } = await import('../web/src/render/architecture/character');
  const { APARTMENT_RECIPES } = await import('../web/src/render/architecture/recipes');
  const resources = new ApartmentResources(),
    profiles = new Set<string>(),
    shaders = new Set<THREE.Material>();
  try {
    for (let seed = 0; seed < 20; seed++) {
      const b = { ...specimen(), appearance_seed: seed },
        weather = buildingWeathering(b, APARTMENT_RECIPES.gallery),
        group = buildApartmentBuilding(b, resources.art, { recipe: 'gallery' })!;
      profiles.add(weather.profile);
      group.traverse((o) => {
        if (!(o instanceof THREE.Mesh)) return;
        const material = o.material as THREE.Material;
        if (!material.userData.apartmentSurface || material.userData.apartmentRole === 'glass')
          return;
        const attr = o.geometry.getAttribute('architectureWear');
        expect(attr).toBeDefined();
        expect(attr.getX(0)).toBeCloseTo(weather.strength);
        expect(attr.getY(0)).toBe(weather.phase);
        if (material.userData.apartmentRole === 'plaster')
          shaders.add(resources.batch(material).material);
      });
      disposeGroup(group);
      expect(
        buildingWeathering({ ...b, stage: 'surface' as const }, APARTMENT_RECIPES.gallery).strength,
      ).toBeLessThan(0.03);
    }
    expect(profiles.size).toBe(4);
    expect(shaders.size).toBe(1);
  } finally {
    resources.dispose();
  }
});
it('gives contemporary recipes integrated circulation and restrained exposure profiles', async () => {
  const { APARTMENT_RECIPES } = await import('../web/src/render/architecture/recipes');
  const { buildingWeathering } = await import('../web/src/render/architecture/character');
  const art = new ArtMaterials();
  try {
    for (const recipe of ['modern', 'urban'] as const) {
      const group = buildApartmentBuilding(specimen(), art, { recipe })!;
      expect(group.userData.apartmentPilot.circulation.external).toBe(false);
      expect(group.userData.apartmentPilot.features.shutters).toBe(false);
      for (let appearance_seed = 0; appearance_seed < 32; appearance_seed++)
        expect(
          buildingWeathering({ ...specimen(), appearance_seed }, APARTMENT_RECIPES[recipe])
            .strength,
        ).toBeLessThan(0.2);
      disposeGroup(group);
    }
  } finally {
    art.dispose();
  }
});
