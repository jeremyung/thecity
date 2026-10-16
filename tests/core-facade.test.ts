import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingPalette } from '../web/src/render/building-palette';
import {
  buildingKit,
  disposeGroup,
  partyWallBodyDepth,
  partyWallBodyWidth,
} from '../web/src/render/kit';
import { midriseMassing } from '../web/src/render/building-massing';
import type { Building } from '../web/src/types';

const EPSILON = 1e-5;

function walkup(seed: number): Building {
  return {
    id: 1,
    parcel_id: 1,
    x: 0,
    z: 0,
    rotation: 0,
    width: 24,
    depth: 19,
    floors: 8,
    kind: 'apartment',
    stage: 'open',
    progress: 1,
    capacity: 32,
    occupancy: 20,
    business_active: false,
    built_day: 1,
    name: 'Walkup',
    appearance_seed: seed,
    history: [],
    anchor: false,
    vacant_days: 0,
  };
}

function materialPoints(group: THREE.Object3D, material: THREE.Material): THREE.Vector3[] {
  const points: THREE.Vector3[] = [];
  group.traverse((object) => {
    if (!(object instanceof THREE.Mesh) || object.material !== material) return;
    const position = object.geometry.getAttribute('position');
    for (let i = 0; i < position.count; i++) {
      points.push(new THREE.Vector3(position.getX(i), position.getY(i), position.getZ(i)));
    }
  });
  return points;
}

function hasBoxCorners(
  points: THREE.Vector3[],
  center: THREE.Vector3,
  size: THREE.Vector3,
): boolean {
  for (const sx of [-1, 1])
    for (const sy of [-1, 1])
      for (const sz of [-1, 1]) {
        const corner = new THREE.Vector3(
          center.x + (sx * size.x) / 2,
          center.y + (sy * size.y) / 2,
          center.z + (sz * size.z) / 2,
        );
        if (!points.some((point) => point.distanceTo(corner) < EPSILON)) return false;
      }
  return true;
}

describe('walkup core service windows', () => {
  it('places narrow windows only on the outward core side, within the parcel, in both styles', () => {
    const art = new ArtMaterials();
    for (const seed of [18, 19]) {
      const b = walkup(seed);
      const palette = buildingPalette(b, art.palette);
      const width = partyWallBodyWidth(b.width, b.anchor, b.floors, b.kind);
      const depth = partyWallBodyDepth(b.depth);
      const core = midriseMassing(b, width, depth)!.masses.find((mass) => mass.role === 'core')!;
      const side = Math.sign(core.x);
      const exteriorX = core.x + (side * core.width) / 2;
      const bayCount = Math.max(1, Math.floor((core.depth - 1.6) / 5));
      let animeVertices: number[][] | undefined;

      for (const style of ['anime', 'miniature'] as const) {
        art.setStyle(style);
        const backing = art.get('#344346');
        const glass = art.get('#55757a', 0.25);
        const mullion = art.get(palette.trim);
        const group = buildingKit(b, art);
        const bounds = new THREE.Box3().setFromObject(group);
        expect(bounds.min.x).toBeGreaterThanOrEqual(-b.width / 2 - EPSILON);
        expect(bounds.max.x).toBeLessThanOrEqual(b.width / 2 + EPSILON);
        expect(bounds.min.z).toBeGreaterThanOrEqual(-b.depth / 2 - EPSILON);
        expect(bounds.max.z).toBeLessThanOrEqual(b.depth / 2 + EPSILON);

        const darkPoints = materialPoints(group, backing);
        const glassPoints = materialPoints(group, glass);
        const ironPoints = materialPoints(group, mullion);
        for (let floor = 0; floor < b.floors; floor++) {
          const y = 0.58 + floor * 3.25 + 2.15;
          for (let bay = 0; bay < bayCount; bay++) {
            const z = ((bay + 0.5) * core.depth) / bayCount - core.depth / 2;
            const details = [
              {
                points: darkPoints,
                center: new THREE.Vector3(exteriorX + side * 0.045, y, z),
                size: new THREE.Vector3(0.12, 1.31, 0.84),
              },
              {
                points: glassPoints,
                center: new THREE.Vector3(exteriorX + side * 0.12, y, z),
                size: new THREE.Vector3(0.035, 1.15, 0.68),
              },
              {
                points: ironPoints,
                center: new THREE.Vector3(exteriorX + side * 0.15, y, z),
                size: new THREE.Vector3(0.035, 1.15, 0.045),
              },
            ];
            for (const { points, center, size } of details) {
              expect(
                hasBoxCorners(points, center, size),
                `seed ${seed}, ${style}, floor ${floor}, bay ${bay}`,
              ).toBe(true);
              const innerCenter = center.clone();
              innerCenter.x = 2 * core.x - center.x;
              expect(
                hasBoxCorners(points, innerCenter, size),
                `no matching inner-side window for seed ${seed}`,
              ).toBe(false);
            }
          }
        }

        const vertices: number[][] = [];
        group.traverse((object) => {
          if (!(object instanceof THREE.Mesh)) return;
          const position = object.geometry.getAttribute('position');
          for (let i = 0; i < position.count; i++)
            vertices.push([position.getX(i), position.getY(i), position.getZ(i)]);
        });
        if (style === 'anime') animeVertices = vertices;
        else expect(vertices).toEqual(animeVertices);
        disposeGroup(group);
      }
    }
    art.dispose();
  });
});
