import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { claddingFor, WINDOW_ATTRIBUTE } from '../web/src/render/building-surfaces';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingKit, disposeGroup, isModernHomeVariant } from '../web/src/render/kit';
import { lowriseBuildingWidth, lowriseMassing } from '../web/src/render/lowrise-massing';
import { buildingPalette } from '../web/src/render/building-palette';
import type { Building, Style } from '../web/src/types';

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

function building(
  kind: 'home' | 'shop' | 'cafe',
  stage: 'open' | 'construction' | 'renovation',
  overrides: Partial<Building> = {},
): Building {
  return {
    id: 45,
    parcel_id: 72,
    x: 12,
    z: -8,
    rotation: 0,
    width: 10,
    depth: 9,
    floors: 2,
    kind,
    stage,
    progress: 1,
    capacity: 16,
    occupancy: 8,
    built_day: 4,
    name: `${kind} sample`,
    appearance_seed: 451,
    history: [],
    anchor: false,
    vacant_days: 0,
    ...overrides,
  };
}

function geometryPoints(root: THREE.Object3D, material?: THREE.Material) {
  const points: THREE.Vector3[] = [];
  root.updateMatrixWorld(true);
  root.traverse((object) => {
    if (!(object instanceof THREE.Mesh) || (material && object.material !== material)) return;
    const attr = object.geometry.getAttribute('position');
    for (let index = 0; index < attr.count; index++) {
      const point = new THREE.Vector3(
        attr.getX(index),
        attr.getY(index),
        attr.getZ(index),
      ).applyMatrix4(object.matrixWorld);
      points.push(point);
    }
  });
  return points;
}

function parcelLocalPoints(root: THREE.Object3D) {
  root.updateMatrixWorld(true);
  const inverse = root.matrixWorld.clone().invert();
  return geometryPoints(root).map((point) => point.applyMatrix4(inverse));
}

function localGeometryPoints(root: THREE.Object3D, material: THREE.Material) {
  root.updateMatrixWorld(true);
  const inverse = root.matrixWorld.clone().invert();
  return geometryPoints(root, material).map((point) => point.applyMatrix4(inverse));
}

function expectFinite(points: THREE.Vector3[]) {
  expect(points.length).toBeGreaterThan(0);
  const invalid = points.find(
    (point) => !Number.isFinite(point.x) || !Number.isFinite(point.y) || !Number.isFinite(point.z),
  );
  expect(invalid, 'all geometry coordinates must be finite').toBeUndefined();
}

function signature(root: THREE.Object3D) {
  const parts: string[] = [];
  root.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    const p = object.geometry.getAttribute('position');
    parts.push(Array.from(p.array).join(','));
  });
  return parts.sort();
}

function vertexSignature(root: THREE.Object3D) {
  return parcelLocalPoints(root)
    .map((point) => `${point.x.toFixed(5)},${point.y.toFixed(5)},${point.z.toFixed(5)}`)
    .sort();
}

function materialGeometrySignature(root: THREE.Object3D, material: THREE.Material) {
  const parts: string[] = [];
  root.traverse((object) => {
    if (!(object instanceof THREE.Mesh) || object.material !== material) return;
    parts.push(Array.from(object.geometry.getAttribute('position').array).join(','));
  });
  return parts.sort();
}

describe('integrated low-rise families', () => {
  it('closes both fallback gable ends above the occupied wall', () => {
    const art = new ArtMaterials();
    const b = building('home', 'open', {
      anchor: true,
      x: 0,
      z: 0,
      rotation: 0,
      width: 10,
      depth: 12,
      floors: 2,
    });
    const root = buildingKit(b, art);
    root.updateMatrixWorld(true);
    try {
      const top = 0.58 + 2 * 3.25;
      for (const side of [-1, 1]) {
        const ray = new THREE.Raycaster(
          new THREE.Vector3(side * 12, top + 0.6, 0),
          new THREE.Vector3(-side, 0, 0),
        );
        const hits = ray.intersectObject(root, true);
        const wall = hits.find(
          (hit) =>
            hit.face &&
            Math.abs(hit.face.normal.x) > 0.99 &&
            Math.abs(hit.point.y - (top + 0.6)) < 0.001,
        );
        expect(wall, `outward-facing gable on side ${side}`).toBeDefined();
        expect(Math.abs(wall!.point.x)).toBeLessThan(b.width / 2);
        expect(Math.abs(wall!.point.x)).toBeGreaterThan(3);
      }
    } finally {
      disposeGroup(root);
      art.dispose();
    }
  });

  it('exposes shop canopies beyond the wall while staying inside the parcel', () => {
    const art = new ArtMaterials();
    try {
      for (const style of ['anime', 'miniature'] as const) {
        art.setStyle(style);
        for (const kind of ['shop', 'cafe'] as const)
          for (const depth of [8, 12]) {
            const b = building(kind, 'open', { x: 0, z: 0, width: 8, depth, floors: 2 });
            const group = buildingKit(b, art);
            group.updateMatrixWorld(true);
            const front = Math.max(6, depth * 0.84) / 2;
            const hit = new THREE.Raycaster(
              new THREE.Vector3(0, 6, front + 0.3),
              new THREE.Vector3(0, -1, 0),
            ).intersectObject(group, true)[0];
            expect(hit, 'projecting canopy covers the frontage').toBeDefined();
            expect(hit.point.y).toBeCloseTo(3.8, 5);
            expect((hit.object as THREE.Mesh).material === art.get('#527366')).toBe(true);
            for (const point of parcelLocalPoints(group))
              expect(point.z).toBeLessThanOrEqual(depth / 2 + 1e-5);
            disposeGroup(group);
          }
      }
    } finally {
      art.dispose();
    }
  });

  it('contains geometry on planner-admitted five-metre frontages through the lifecycle', () => {
    const art = new ArtMaterials();
    try {
      for (const kind of ['home', 'shop', 'cafe'] as const)
        for (const width of [5, 5.4, 5.99])
          for (const depth of [7, 12])
            for (const floors of [1, 2, 3])
              for (const seed of [1, 8, 23])
                for (const stage of ['open', 'construction', 'renovation'] as const) {
                  const b = building(kind, stage, {
                    width,
                    depth,
                    floors,
                    appearance_seed: seed,
                    rotation: 0.63,
                    progress: 0.65,
                  });
                  const group = buildingKit(b, art);
                  group.position.set(b.x, 0, b.z);
                  group.rotation.y = b.rotation;
                  const points = parcelLocalPoints(group);
                  expectFinite(points);
                  const maxX = Math.max(...points.map((p) => Math.abs(p.x)));
                  const maxZ = Math.max(...points.map((p) => Math.abs(p.z)));
                  disposeGroup(group);
                  const context = `${kind} ${width}x${depth} ${floors} floors seed ${seed} ${stage}`;
                  expect(maxX, context).toBeLessThanOrEqual(width / 2 + 1e-5);
                  expect(maxZ, context).toBeLessThanOrEqual(depth / 2 + 1e-5);
                }
    } finally {
      art.dispose();
    }
  });

  it('keeps real transformed geometry inside narrow rotated parcels across styles and roof profiles', () => {
    const art = new ArtMaterials();
    for (const kind of ['home', 'shop', 'cafe'] as const)
      for (const [width, depth] of [
        [6, 6],
        [8, 10],
        [10, 8],
        [13, 12],
        [8.6, 8],
        [11.65, 8],
        [14, 8],
      ] as const) {
        for (const floors of [1, 2, 3])
          for (const seed of [1, 8, 23]) {
            const b = building(kind, 'open', {
              width,
              depth,
              floors,
              appearance_seed: seed,
              rotation: 0.63,
            });
            const group = buildingKit(b, art);
            group.position.set(b.x, 0, b.z);
            group.rotation.y = b.rotation;
            group.updateMatrixWorld(true);
            const renderDepth = Math.max(6, depth * 0.84);
            const renderWidth = lowriseBuildingWidth(b, Math.max(6, width * 0.84), renderDepth);
            const layout = lowriseMassing(b, renderWidth, renderDepth)!;
            const frontage = layout.masses.find((mass) => mass.role === 'frontage')!;
            const service = layout.masses.find((mass) => mass.role === 'service')!;
            expect(
              frontage.width / width,
              `${kind} ${width}x${depth} frontage coverage`,
            ).toBeGreaterThanOrEqual(0.92);
            expect(
              Math.abs(service.x) + service.width / 2,
              `${kind} ${width}x${depth} service court`,
            ).toBeLessThanOrEqual(width / 2 - 0.7 + 1e-6);
            expect(
              frontage.z + frontage.depth / 2,
              `${kind} ${width}x${depth} access edge`,
            ).toBeCloseTo(renderDepth / 2, 8);
            const points = geometryPoints(group);
            expectFinite(points);
            const local = parcelLocalPoints(group);
            const context = `${kind} ${width}x${depth} ${floors} floors seed ${seed}`;
            // Scan every vertex, but report the actual extrema once per specimen.
            // Per-vertex assertion objects made this exhaustive check timeout under load.
            let minX = Infinity,
              maxX = -Infinity,
              minZ = Infinity,
              maxZ = -Infinity;
            for (const point of local) {
              minX = Math.min(minX, point.x);
              maxX = Math.max(maxX, point.x);
              minZ = Math.min(minZ, point.z);
              maxZ = Math.max(maxZ, point.z);
            }
            expect(minX, context).toBeGreaterThanOrEqual(-width / 2 - 1e-5);
            expect(maxX, context).toBeLessThanOrEqual(width / 2 + 1e-5);
            expect(minZ, context).toBeGreaterThanOrEqual(-depth / 2 - 1e-5);
            expect(maxZ, context).toBeLessThanOrEqual(depth / 2 + 1e-5);
            expect(Math.max(...points.map((point) => point.y)), context).toBeLessThanOrEqual(
              0.58 + floors * 3.25 + 2.75,
            );
            disposeGroup(group);
          }
      }
    art.dispose();
  }, 20_000);

  it('keeps a centered shop doorway separate from display glazing and inside the parcel', () => {
    const art = new ArtMaterials();
    for (const style of ['anime', 'miniature'] as const) {
      art.setStyle(style);
      for (const [width, depth] of [
        [6, 6],
        [8, 10],
        [12, 9],
      ] as const) {
        const b = building('shop', 'open', { width, depth, floors: 2, appearance_seed: 73 });
        const group = buildingKit(b, art);
        const pane = art.get('#55757a', 0.25);
        const points = localGeometryPoints(group, pane);
        const frontLimit = (depth * 0.84) / 2;
        const doorWidth = Math.min(
          1.35,
          lowriseBuildingWidth(b, Math.max(6, width * 0.84), Math.max(6, depth * 0.84)) * 0.17,
        );
        const doorMinX = -doorWidth / 2 - 0.04,
          doorMaxX = doorWidth / 2 + 0.04;
        const doorwayPanes = points.filter(
          (point) =>
            point.y >= 0.58 &&
            point.y <= 3.8 &&
            point.z > frontLimit - 0.15 &&
            point.x >= doorMinX &&
            point.x <= doorMaxX,
        );
        expect(doorwayPanes, `${width}x${depth}`).toHaveLength(0);
        const shopWindows = points.filter(
          (point) => point.y > 0.7 && point.y < 3.8 && point.z > frontLimit - 0.15,
        );
        expect(
          shopWindows.some((point) => point.x < -doorWidth / 2 - 0.2),
          `${style} ${width}x${depth} left display`,
        ).toBe(true);
        expect(
          shopWindows.some((point) => point.x > doorWidth / 2 + 0.2),
          `${style} ${width}x${depth} right display`,
        ).toBe(true);
        const wood = art.get(['#644e3e', '#755d49', '#56483d'][Math.abs(Math.trunc(73 >>> 5)) % 3]);
        const door = localGeometryPoints(group, wood);
        expect(
          door.some(
            (point) =>
              Math.abs(point.x) < doorWidth / 2 + 0.02 &&
              point.y > 1 &&
              point.y < 3.1 &&
              point.z > frontLimit - 0.18,
          ),
          `${width}x${depth} centered door`,
        ).toBe(true);
        const threshold = localGeometryPoints(group, art.get('#aaa99e'));
        expect(
          threshold.some(
            (point) =>
              Math.abs(point.x) < doorWidth / 2 + 0.12 &&
              point.y >= 0.57 &&
              point.y <= 0.74 &&
              point.z > frontLimit - 0.2,
          ),
          `${width}x${depth} centered threshold`,
        ).toBe(true);
        for (const point of parcelLocalPoints(group)) {
          expect(point.x).toBeGreaterThanOrEqual(-width / 2 - 1e-5);
          expect(point.x).toBeLessThanOrEqual(width / 2 + 1e-5);
          expect(point.z).toBeGreaterThanOrEqual(-depth / 2 - 1e-5);
          expect(point.z).toBeLessThanOrEqual(depth / 2 + 1e-5);
        }
        disposeGroup(group);
      }
    }
    art.dispose();
  });

  it('keeps shop glazing and its timber reveal on distinct parcel-safe depth planes', () => {
    const art = new ArtMaterials();
    for (const style of ['anime', 'miniature'] as const) {
      art.setStyle(style);
      const b = building('shop', 'open', { width: 8, depth: 8, floors: 2, appearance_seed: 24 });
      const group = buildingKit(b, art);
      const frontageEdge = (b.depth * 0.84) / 2;
      const panes = localGeometryPoints(group, art.get('#55757a', 0.25)).filter(
        (point) => point.y > 0.7 && point.y < 3.1 && point.z > frontageEdge - 0.35,
      );
      const reveals = localGeometryPoints(group, art.get('#344346')).filter(
        (point) => point.y > 0.7 && point.y < 3.1 && point.z > frontageEdge - 0.35,
      );
      for (const [side, minX, maxX] of [
        ['left', -3.2, -0.7],
        ['right', 0.7, 3.2],
      ] as const) {
        const pane = panes.filter((point) => point.x > minX && point.x < maxX);
        const reveal = reveals.filter((point) => point.x > minX && point.x < maxX);
        expect(pane.length, `${style} ${side} pane`).toBeGreaterThan(0);
        expect(reveal.length, `${style} ${side} frame`).toBeGreaterThan(0);
        const paneFront = Math.max(...pane.map((point) => point.z)),
          revealFront = Math.max(...reveal.map((point) => point.z));
        expect(paneFront, `${style} ${side} glass sits ahead of its frame`).toBeGreaterThan(
          revealFront + 0.005,
        );
        expect(paneFront, `${style} ${side} pane remains inside parcel`).toBeLessThanOrEqual(
          frontageEdge + 1e-5,
        );
        expect(revealFront, `${style} ${side} reveal remains inside parcel`).toBeLessThanOrEqual(
          frontageEdge + 1e-5,
        );
      }
      const doorWidth = Math.min(
        1.35,
        lowriseBuildingWidth(b, Math.max(6, b.width * 0.84), Math.max(6, b.depth * 0.84)) * 0.17,
      );
      expect(
        panes.some((point) => Math.abs(point.x) < doorWidth / 2 - 0.02),
        `${style} door remains clear of glazing`,
      ).toBe(false);
      for (const point of parcelLocalPoints(group)) {
        expect(point.x).toBeGreaterThanOrEqual(-b.width / 2 - 1e-5);
        expect(point.x).toBeLessThanOrEqual(b.width / 2 + 1e-5);
        expect(point.z).toBeGreaterThanOrEqual(-b.depth / 2 - 1e-5);
        expect(point.z).toBeLessThanOrEqual(b.depth / 2 + 1e-5);
      }
      disposeGroup(group);
    }
    art.dispose();
  });

  it('uses the same frontage and rear-wing positions through open, construction and renovation stages', () => {
    const art = new ArtMaterials();
    for (const kind of ['home', 'shop'] as const)
      for (const [width, depth] of [
        [12, 11],
        [8.6, 8],
        [11.65, 8],
        [14, 8],
      ] as const)
        for (const style of ['anime', 'miniature'] as const) {
          art.setStyle(style);
          const b = building(kind, 'open', {
            width,
            depth,
            floors: 3,
            appearance_seed: 15,
            rotation: 0.63,
          });
          const envelopeDepth = Math.max(6, b.depth * 0.84);
          const layout = lowriseMassing(
            b,
            lowriseBuildingWidth(b, Math.max(6, b.width * 0.84), envelopeDepth),
            envelopeDepth,
          )!;
          const open = buildingKit(b, art);
          const framing = buildingKit({ ...b, stage: 'construction', progress: 0.8 }, art);
          const renovation = buildingKit({ ...b, stage: 'renovation', progress: 0.7 }, art);
          for (const [name, group] of [
            ['open', open],
            ['construction', framing],
            ['renovation', renovation],
          ] as const) {
            group.position.set(b.x, 0, b.z);
            group.rotation.y = b.rotation;
            const points = geometryPoints(group);
            expectFinite(points);
            // Keep the full vertex scan and exact bounds; report the first
            // offending point without allocating four matchers for every vertex.
            const outside = parcelLocalPoints(group).find(
              (point) =>
                !(
                  point.x >= -b.width / 2 - 1e-5 &&
                  point.x <= b.width / 2 + 1e-5 &&
                  point.z >= -b.depth / 2 - 1e-5 &&
                  point.z <= b.depth / 2 + 1e-5
                ),
            );
            expect(outside, `${kind} ${name} stays within parcel bounds`).toBeUndefined();
          }
          const builtWall = localGeometryPoints(
            framing,
            art.get(buildingPalette(b, art.palette).wall, 0.85, false, 'ambient', claddingFor(b)),
          );
          expect(builtWall.some((point) => point.y > 1 && point.z > envelopeDepth * 0.25)).toBe(
            true,
          );
          for (const mass of layout.masses) {
            expect(mass.baseFloor).toBe(0); // both the frontage and rear wing are grounded
            expect(mass.z - mass.depth / 2).toBeGreaterThanOrEqual(-b.depth / 2);
            expect(mass.z + mass.depth / 2).toBeLessThanOrEqual(b.depth / 2);
          }
          expect(geometryPoints(framing, art.get('#8c8070', 0.72)).length).toBeGreaterThan(0);
          // Renovation remains at ground-level tiers rather than floating onto the roof.
          const scaffold = geometryPoints(renovation, art.get('#8c8070', 0.72));
          expect(Math.max(...scaffold.map((point) => point.y))).toBeLessThanOrEqual(6.6);
          [open, framing, renovation].forEach(disposeGroup);
        }
    art.dispose();
  });

  it('uses the finished facade family at the late construction stage with grounded worksite details', () => {
    const art = new ArtMaterials();
    for (const kind of ['home', 'shop', 'cafe'] as const)
      for (const style of ['anime', 'miniature'] as Style[]) {
        art.setStyle(style);
        const b = building(kind, 'open', { width: 12, depth: 11, floors: 3, appearance_seed: 73 });
        const opened = buildingKit(b, art);
        const late = buildingKit({ ...b, stage: 'construction', progress: 0.99 }, art);
        const wall = art.get(buildingPalette(b, art.palette).wall);
        expect(materialGeometrySignature(late, wall), `${kind} ${style} finished wall`).toEqual(
          materialGeometrySignature(opened, wall),
        );
        expect(
          geometryPoints(late, art.get('#8c8070', 0.72)).length,
          `${kind} ${style} scaffold`,
        ).toBeGreaterThan(0);
        expect(
          geometryPoints(late, art.get('#d6b25e')).length,
          `${kind} ${style} caution marker`,
        ).toBeGreaterThan(0);
        let emissive = false;
        late.traverse((object) => {
          if (!(object instanceof THREE.Mesh)) return;
          const materials = Array.isArray(object.material) ? object.material : [object.material];
          if (
            materials.some(
              (material) =>
                (material.userData.artBuildingClass as { emissive?: boolean } | undefined)
                  ?.emissive === true,
            )
          )
            emissive = true;
        });
        expect(emissive, `${kind} ${style} construction lighting`).toBe(false);
        for (const point of parcelLocalPoints(late)) {
          expect(point.x).toBeGreaterThanOrEqual(-b.width / 2 - 1e-5);
          expect(point.x).toBeLessThanOrEqual(b.width / 2 + 1e-5);
          expect(point.z).toBeGreaterThanOrEqual(-b.depth / 2 - 1e-5);
          expect(point.z).toBeLessThanOrEqual(b.depth / 2 + 1e-5);
        }
        disposeGroup(opened);
        disposeGroup(late);
      }
    art.dispose();
  });

  it('gives one- and multi-storey homes a centered door with clear flanking windows', () => {
    const art = new ArtMaterials();
    for (const floors of [1, 2, 3] as const) {
      const b = building('home', 'open', {
        width: 8,
        depth: 8,
        floors,
        appearance_seed: 19,
        rotation: 0.43,
      });
      const group = buildingKit(b, art);
      group.position.set(b.x, 0, b.z);
      group.rotation.y = b.rotation;
      const wood = art.get(
        ['#644e3e', '#755d49', '#56483d'][Math.abs(Math.trunc(b.appearance_seed >>> 5)) % 3],
      );
      const door = localGeometryPoints(group, wood);
      const frontageW =
        Math.min(
          lowriseBuildingWidth(b, Math.max(6, b.width * 0.84), Math.max(6, b.depth * 0.84)),
          b.width - 0.2,
        ) * 0.96;
      const doorWidth = floors === 1 ? 1.25 : Math.min(1.12, frontageW * 0.24);
      expect(
        door.some(
          (point) =>
            Math.abs(point.x) < doorWidth / 2 + 0.02 &&
            point.y > 0.8 &&
            point.y < 3.2 &&
            point.z > 2.8,
        ),
        `${floors} floors centered door`,
      ).toBe(true);
      const paneMaterials = [art.window('home')];
      const panes = paneMaterials
        .flatMap((material) => localGeometryPoints(group, material))
        .filter((point) => point.y >= 0.65 && point.y < 3.05 && point.z > 2.5);
      expect(
        panes.some((point) => point.x < -doorWidth / 2 - 0.1),
        `${floors} floors left window`,
      ).toBe(true);
      expect(
        panes.some((point) => point.x > doorWidth / 2 + 0.1),
        `${floors} floors right window`,
      ).toBe(true);
      expect(
        panes.some((point) => Math.abs(point.x) < doorWidth / 2 - 0.03),
        `${floors} floors window clear of door`,
      ).toBe(false);
      const threshold = localGeometryPoints(group, art.get('#aaa99e'));
      expect(
        threshold.some(
          (point) =>
            Math.abs(point.x) < doorWidth / 2 + 0.15 &&
            point.y >= 0.57 &&
            point.y <= 0.74 &&
            point.z > 2.8,
        ),
        `${floors} floors centered threshold`,
      ).toBe(true);
      for (const point of parcelLocalPoints(group)) {
        expect(point.x).toBeGreaterThanOrEqual(-b.width / 2 - 1e-5);
        expect(point.x).toBeLessThanOrEqual(b.width / 2 + 1e-5);
        expect(point.z).toBeGreaterThanOrEqual(-b.depth / 2 - 1e-5);
        expect(point.z).toBeLessThanOrEqual(b.depth / 2 + 1e-5);
      }
      disposeGroup(group);
    }
    art.dispose();
  });

  it('builds recessed sash depth and a supported sill into low-rise house windows in both styles', () => {
    const art = new ArtMaterials();
    for (const style of ['anime', 'miniature'] as const) {
      art.setStyle(style);
      const paneMaterial = art.window('home'),
        dark = art.get('#344346');
      const b = building('home', 'open', {
        width: 8,
        depth: 8,
        floors: 3,
        appearance_seed: 23,
        occupancy: 0,
      });
      const group = buildingKit(b, art);
      const front = (8 * 0.84) / 2;
      const levelY = 0.58 + 3.25 + 1.65;
      const pane = localGeometryPoints(group, paneMaterial).filter(
        (point) =>
          point.x > -3.2 &&
          point.x < -1 &&
          point.y > levelY - 0.72 &&
          point.y < levelY + 0.72 &&
          point.z > front - 0.35,
      );
      const reveal = localGeometryPoints(group, dark).filter(
        (point) =>
          point.x > -3.2 &&
          point.x < -1 &&
          point.y > levelY - 0.72 &&
          point.y < levelY + 0.72 &&
          point.z > front - 0.35,
      );
      const wood = art.get(
        ['#644e3e', '#755d49', '#56483d'][Math.abs(Math.trunc(b.appearance_seed >>> 5)) % 3],
      );
      const sash = localGeometryPoints(group, art.get('#d8cdb5')).filter(
        (point) =>
          point.x > -3.2 &&
          point.x < -1 &&
          point.y > levelY - 0.65 &&
          point.y < levelY + 0.65 &&
          point.z > front - 0.35,
      );
      const frame = localGeometryPoints(group, wood).filter(
        (point) =>
          point.x > -3.2 &&
          point.x < -1 &&
          point.y > levelY - 0.72 &&
          point.y < levelY + 0.72 &&
          point.z > front - 0.35,
      );
      expect(pane.length, `${style} recessed glazing`).toBeGreaterThan(0);
      expect(reveal.length, `${style} dark reveal`).toBeGreaterThan(0);
      const paneFront = Math.max(...pane.map((point) => point.z));
      const revealFront = Math.max(...reveal.map((point) => point.z));
      const frameFront = Math.max(...frame.map((point) => point.z));
      const sashFront = Math.max(...sash.map((point) => point.z));
      expect(paneFront, `${style} glass ahead of recessed backing`).toBeGreaterThan(
        revealFront + 0.015,
      );
      expect(sashFront, `${style} sash has its own forward plane`).toBeGreaterThan(
        paneFront + 0.01,
      );
      expect(frameFront, `${style} frame stays behind the sash`).toBeLessThan(sashFront);
      expect(paneFront).toBeLessThanOrEqual(front + 1e-5);
      expect(sashFront).toBeLessThanOrEqual(front + 1e-5);
      const sill = localGeometryPoints(group, art.get('#aaa99e')).filter(
        (point) =>
          point.x > -3.5 &&
          point.x < -1 &&
          point.y > levelY - 1 &&
          point.y < levelY - 0.65 &&
          point.z > front - 0.35,
      );
      expect(sill.length, `${style} sill under window`).toBeGreaterThan(0);

      // Kit merges all repeated detail by shared material, so the added depth
      // does not create one draw call per window component.
      const meshMaterials = new Set<THREE.Material>();
      for (const child of group.children) {
        expect(child).toBeInstanceOf(THREE.Mesh);
        const mesh = child as THREE.Mesh;
        expect(meshMaterials.has(mesh.material as THREE.Material)).toBe(false);
        meshMaterials.add(mesh.material as THREE.Material);
      }
      disposeGroup(group);
    }
    art.dispose();
  });

  it('flags occupied house windows as lived, while vacancies and construction stay dark', () => {
    // Rooms light individually in the shader from the hour; the kit only says whether anyone is in.
    const lived = (group: THREE.Object3D) => {
      const values: number[] = [];
      group.traverse((object) => {
        if (
          !(object instanceof THREE.Mesh) ||
          !(object.material as THREE.Material).userData.windowPane
        )
          return;
        const info = object.geometry.getAttribute(WINDOW_ATTRIBUTE);
        for (let i = 0; i < info.count; i++) values.push(info.getY(i));
      });
      return values;
    };
    const art = new ArtMaterials();
    for (const style of ['anime', 'miniature'] as const) {
      art.setStyle(style);
      for (const floors of [1, 2, 3] as const) {
        const b = building('home', 'open', {
          width: 10,
          depth: 10,
          floors,
          appearance_seed: 19,
          occupancy: 5,
          vacant_days: 0,
        });
        const occupied = buildingKit(b, art);
        const vacant = buildingKit({ ...b, occupancy: 0, vacant_days: 8 }, art);
        expect(
          lived(occupied).length,
          `${style} ${floors}-floor occupied home has panes`,
        ).toBeGreaterThan(0);
        expect(new Set(lived(occupied)), `${style} ${floors}-floor occupied home`).toEqual(
          new Set([1]),
        );
        expect(new Set(lived(vacant)), `${style} ${floors}-floor vacant home`).toEqual(
          new Set([0]),
        );
        expect(
          vertexSignature(occupied),
          `${style} ${floors}-floor occupancy does not alter geometry`,
        ).toEqual(vertexSignature(vacant));
        const construction = buildingKit({ ...b, stage: 'construction', progress: 0.99 }, art);
        expect(
          lived(construction).every((value) => value === 0),
          `${style} ${floors}-floor late construction`,
        ).toBe(true);
        for (const group of [occupied, vacant, construction]) disposeGroup(group);
      }
    }
    art.dispose();
  });

  it('adds seeded modern-home window guards and rear service details within the parcel', () => {
    const art = new ArtMaterials();
    const selected = building('home', 'open', {
      width: 12,
      depth: 12,
      floors: 2,
      appearance_seed: 19,
      occupancy: 4,
      rotation: 0.63,
    });
    const traditional = building('home', 'open', { ...selected, appearance_seed: 23 });
    expect(isModernHomeVariant(selected)).toBe(true);
    expect(isModernHomeVariant(traditional)).toBe(false);
    expect(isModernHomeVariant({ ...selected, anchor: true })).toBe(false);
    expect(isModernHomeVariant({ ...selected, floors: 1 })).toBe(false);
    expect(isModernHomeVariant({ ...selected, floors: 4 })).toBe(false);

    for (const style of ['anime', 'miniature'] as const) {
      art.setStyle(style);
      const open = buildingKit(selected, art);
      const plain = buildingKit(traditional, art);
      const late = buildingKit({ ...selected, stage: 'construction', progress: 0.99 }, art);
      const initialWidth = Math.max(6, selected.width * 0.84),
        depth = Math.max(6, selected.depth * 0.84);
      const width = lowriseBuildingWidth(selected, initialWidth, depth);
      const layout = lowriseMassing(selected, width, depth)!;
      const frontage = layout.masses.find((mass) => mass.role === 'frontage')!;
      const service = layout.masses.find((mass) => mass.role === 'service')!;
      const base = 0.58,
        floor = 3.25,
        upperY = base + floor + 1.65,
        frontEdge = frontage.z + frontage.depth / 2;
      const iron = art.get('#536164', 0.62),
        lightGlass = art.window('home'),
        pale = art.get('#c3b9a5');

      const guards = localGeometryPoints(open, iron).filter(
        (point) => point.y > upperY - 0.72 && point.y < upperY - 0.2 && point.z > frontEdge - 0.1,
      );
      const plainGuards = localGeometryPoints(plain, iron).filter(
        (point) => point.y > upperY - 0.72 && point.y < upperY - 0.2 && point.z > frontEdge - 0.1,
      );
      expect(guards.length, `${style} selected upper-window guard geometry`).toBeGreaterThan(0);
      expect(plainGuards, `${style} unselected home retains its simpler frontage`).toHaveLength(0);

      const rear = frontage.z - frontage.depth / 2;
      const rearPanes = localGeometryPoints(open, lightGlass).filter(
        (point) => point.y > upperY - 0.7 && point.y < upperY + 0.7 && point.z < rear - 0.05,
      );
      const plainRearPanes = localGeometryPoints(plain, lightGlass).filter(
        (point) => point.y > upperY - 0.7 && point.y < upperY + 0.7 && point.z < rear - 0.05,
      );
      expect(rearPanes.length, `${style} compact rear utility windows`).toBeGreaterThan(0);
      expect(
        plainRearPanes.length,
        `${style} both home variants have occupied rear rooms`,
      ).toBeGreaterThan(0);

      const side = layout.quietSides[0],
        acX = service.x + side * (service.width / 2 + 0.36),
        acY = base + 1.3,
        acZ = service.z + service.depth * 0.16;
      const unitVertex = new THREE.Vector3(acX + side * 0.36, acY + 0.28, acZ + 0.24);
      const hasUnit = (group: THREE.Object3D) =>
        localGeometryPoints(group, pale).some((point) => point.distanceTo(unitVertex) < 1e-5);
      expect(hasUnit(open), `${style} attached condenser corner`).toBe(true);
      const condenserPoints = localGeometryPoints(open, pale).filter(
        (point) =>
          point.y >= acY - 0.29 && point.y <= acY + 0.29 && Math.abs(point.z - acZ) <= 0.25,
      );
      const condenserOutermost = Math.max(...condenserPoints.map((point) => side * point.x));
      expect(
        selected.width / 2 - condenserOutermost,
        `${style} condenser parcel-side clearance`,
      ).toBeGreaterThanOrEqual(0.7 - 1e-5);
      expect(hasUnit(plain), `${style} plain family has no condenser`).toBe(false);
      expect(hasUnit(late), `${style} late construction retains its condenser`).toBe(true);
      expect(
        materialGeometrySignature(late, iron),
        `${style} late construction keeps the same window guards`,
      ).toEqual(materialGeometrySignature(open, iron));

      for (const group of [open, plain, late]) {
        group.position.set(selected.x, 0, selected.z);
        group.rotation.y = selected.rotation;
        for (const point of parcelLocalPoints(group)) {
          expect(point.x, `${style} modern detail x`).toBeGreaterThanOrEqual(
            -selected.width / 2 - 1e-5,
          );
          expect(point.x, `${style} modern detail x`).toBeLessThanOrEqual(
            selected.width / 2 + 1e-5,
          );
          expect(point.z, `${style} modern detail z`).toBeGreaterThanOrEqual(
            -selected.depth / 2 - 1e-5,
          );
          expect(point.z, `${style} modern detail z`).toBeLessThanOrEqual(
            selected.depth / 2 + 1e-5,
          );
        }
        disposeGroup(group);
      }
    }
    art.dispose();
  });

  it('keeps rear residential rooms above the attached service roof for homes and shop-houses', () => {
    const art = new ArtMaterials();
    try {
      for (const kind of ['home', 'shop', 'cafe'] as const)
        for (const style of ['anime', 'miniature'] as const) {
          art.setStyle(style);
          const b = building(kind, 'open', { floors: 3, width: 8, depth: 12 });
          const d = b.depth * 0.84,
            w = lowriseBuildingWidth(b, b.width - 0.9, d),
            layout = lowriseMassing(b, w, d)!;
          const main = layout.masses.find((x) => x.role === 'frontage')!,
            service = layout.masses.find((x) => x.role === 'service')!;
          const group = buildingKit(b, art);
          group.updateMatrixWorld(true);
          const rear = main.z - main.depth / 2;
          const panes = localGeometryPoints(group, art.window('home')).filter(
            (p) => p.z < rear - 0.05 && p.z > rear - 0.15,
          );
          expect(panes.length).toBeGreaterThan(0);
          expect(Math.min(...panes.map((p) => p.y))).toBeGreaterThan(
            0.58 + service.floors * 3.25 + 0.4,
          );
          const ray = new THREE.Raycaster(
            new THREE.Vector3(panes[0].x, 0.58 + 3.25 + 1.65, rear - 1),
            new THREE.Vector3(0, 0, 1),
          );
          const hits = ray.intersectObject(group, true);
          expect(hits.length).toBeGreaterThan(0);
          expect(hits[0].point.z).toBeLessThan(rear);
          disposeGroup(group);
        }
    } finally {
      art.dispose();
    }
  });

  it('keeps gable-edge trim attached to the roof eaves instead of parcel-clearance coordinates', () => {
    const art = new ArtMaterials();
    const b = building('home', 'open', { width: 14, depth: 14, floors: 2, appearance_seed: 1 });
    const d = Math.max(6, b.depth * 0.84),
      w = lowriseBuildingWidth(b, Math.max(6, b.width * 0.84), d);
    const layout = lowriseMassing(b, w, d)!;
    const frontage = layout.masses.find((mass) => mass.role === 'frontage')!;
    expect(frontage.roof).toBe('gable');
    const top = 0.58 + b.floors * 3.25;
    const group = buildingKit(b, art);
    const trim = localGeometryPoints(group, art.get('#d8cdb5'));
    const frontEave = frontage.z + frontage.depth / 2 + 0.2;
    expect(trim.some((point) => point.y > top + 0.08 && point.z > frontEave + 0.35)).toBe(false);
    disposeGroup(group);
    art.dispose();
  });

  it('keeps authored silhouettes identical under both material styles and leaves anchors on their established path', () => {
    const art = new ArtMaterials();
    const sample = building('home', 'open', { width: 13, depth: 12, appearance_seed: 8 });
    const groups: THREE.Group[] = [];
    for (const style of ['anime', 'miniature'] as Style[]) {
      art.setStyle(style);
      groups.push(buildingKit(sample, art));
    }
    expect(signature(groups[0])).toEqual(signature(groups[1]));
    const anchor = building('shop', 'open', { anchor: true, width: 22, depth: 16, floors: 2 });
    const anchorGroup = buildingKit(anchor, art);
    expect(new THREE.Box3().setFromObject(anchorGroup).max.y).toBeGreaterThan(
      0.58 + anchor.floors * 3.25,
    );
    [...groups, anchorGroup].forEach(disposeGroup);
    art.dispose();
  });
});

describe('broad home staged family', () => {
  it('keeps actual witness geometry within rotated parcels throughout construction and renovation', () => {
    const art = new ArtMaterials();
    try {
      for (const style of ['anime', 'miniature'] as const) {
        art.setStyle(style);
        for (const [width, depth] of [
          [14.746641, 24.086472],
          [10.633784, 24.086466],
        ]) {
          const b = building('home', 'open', {
            width,
            depth,
            floors: 3,
            appearance_seed: 1018519500,
            rotation: 1.8066885,
          });
          for (const stage of ['open', 'construction', 'renovation'] as const)
            for (const progress of stage === 'construction' ? [0.3, 0.7, 0.99] : [1]) {
              const group = buildingKit({ ...b, stage, progress }, art);
              group.position.set(b.x, 0, b.z);
              group.rotation.y = b.rotation;
              const points = parcelLocalPoints(group);
              expectFinite(points);
              const maxX = Math.max(...points.map((p) => Math.abs(p.x)));
              const maxZ = Math.max(...points.map((p) => Math.abs(p.z)));
              expect(maxX, `${width} ${style} ${stage}/${progress} X`).toBeLessThanOrEqual(
                width / 2 + 1e-5,
              );
              expect(maxZ, `${width} ${style} ${stage}/${progress} Z`).toBeLessThanOrEqual(
                depth / 2 + 1e-5,
              );
              disposeGroup(group);
            }
          const open = buildingKit(b, art),
            late = buildingKit({ ...b, stage: 'construction', progress: 0.99 }, art);
          open.updateMatrixWorld(true);
          const upperFront = Math.max(6, depth * 0.84) / 2 - 2.4;
          const doorHit = new THREE.Raycaster(
            new THREE.Vector3(0, 0.58 + 6.5 + 1.13, upperFront + 1),
            new THREE.Vector3(0, 0, -1),
          ).intersectObject(open, true)[0];
          expect(
            doorHit,
            'upper terrace has a visible door rather than an overlapping window',
          ).toBeDefined();
          expect((doorHit.object as THREE.Mesh).material).toBe(art.get('#55757a', 0.25));
          const wall = art.get(buildingPalette(b, art.palette).wall);
          expect(materialGeometrySignature(late, wall)).toEqual(
            materialGeometrySignature(open, wall),
          );
          disposeGroup(open);
          disposeGroup(late);
        }
      }
    } finally {
      art.dispose();
    }
  });
});

describe('closed domestic gables', () => {
  it('closes both roof ends with outward-facing wall geometry through late construction and renovation', () => {
    const art = new ArtMaterials();
    try {
      for (const style of ['anime', 'miniature'] as const) {
        art.setStyle(style);
        const b = building('home', 'open', {
          width: 8,
          depth: 14,
          floors: 2,
          appearance_seed: 451,
        });
        const d = b.depth * 0.84,
          w = lowriseBuildingWidth(b, 7.1, d);
        const mass = lowriseMassing(b, w, d)!.masses[0];
        expect(mass.roof).toBe('gable');
        for (const stage of ['open', 'construction', 'renovation'] as const) {
          const group = buildingKit(
            { ...b, stage, progress: stage === 'construction' ? 0.99 : 1 },
            art,
          );
          group.updateMatrixWorld(true);
          for (const side of [-1, 1]) {
            const hit = new THREE.Raycaster(
              new THREE.Vector3(side * (b.width / 2 + 1), 0.58 + b.floors * 3.25 + 0.6, mass.z),
              new THREE.Vector3(-side, 0, 0),
            ).intersectObject(group, true)[0];
            expect(hit, `${style} ${stage} gable ${side}`).toBeDefined();
            expect(hit.point.x).toBeCloseTo((side * mass.width) / 2, 4);
            expect(hit.face!.normal.x).toBe(side);
          }
          disposeGroup(group);
        }
      }
    } finally {
      art.dispose();
    }
  });
});
