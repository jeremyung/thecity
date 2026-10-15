import fixtureText from './fixtures/rail-alignment-native.json?raw';
import { sampledRail } from '../web/src/render/rail-path';
import type { RailAlignment } from '../web/src/types';
import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { Kit, disposeGroup } from '../web/src/render/kit';
import { addRailCorridor } from '../web/src/render/rail-corridor';
import { RAIL_HEAD_Y } from '../web/src/render/rail-profile';
import { buildTrainCar } from '../web/src/render/train-kit';
import type { Point, RailPlatform } from '../web/src/types';

const CONTACT_HEIGHT = 4.9425;
const CONTACT_Y = RAIL_HEAD_Y + CONTACT_HEIGHT;
const CORRIDOR_HALF_WIDTH = 3.55;

function collect(group: THREE.Group) {
  group.updateMatrixWorld(true);
  const vertices: Array<{ x: number; y: number; z: number }> = [];
  let triangles = 0;
  const materials = new Set<THREE.Material>();
  group.traverse(object => {
    if (!(object instanceof THREE.Mesh)) return;
    const mesh = object as THREE.Mesh;
    const position = mesh.geometry.getAttribute('position');
    triangles += mesh.geometry.index ? mesh.geometry.index.count / 3 : position.count / 3;
    for (const material of Array.isArray(mesh.material) ? mesh.material : [mesh.material]) materials.add(material);
    for (let i = 0; i < position.count; i++) {
      const point = new THREE.Vector3().fromBufferAttribute(position, i).applyMatrix4(mesh.matrixWorld);
      vertices.push({ x: point.x, y: point.y, z: point.z });
    }
  });
  return { vertices, triangles, materials };
}

function pointAt(a: Point, b: Point, along: number, lateral = 0): Point {
  const dx = b.x - a.x, dz = b.z - a.z, length = Math.hypot(dx, dz);
  return { x: a.x + dx / length * along + dz / length * lateral, z: a.z + dz / length * along - dx / length * lateral };
}

function expectPlatformEnvelopeClear(group: THREE.Group, platform: RailPlatform) {
  group.updateMatrixWorld(true);
  const inPlatformFrame = (x: number, z: number) => {
    const dx = x - platform.x, dz = z - platform.z;
    return { across: dx * platform.uz - dz * platform.ux, along: dx * platform.ux + dz * platform.uz };
  };
  group.traverse(object => {
    if (!(object instanceof THREE.Mesh)) return;
    const mesh = object as THREE.Mesh, position = mesh.geometry.getAttribute('position');
    const index = mesh.geometry.index;
    const count = index ? index.count : position.count;
    for (let i = 0; i < count; i += 3) {
      const points = [0, 1, 2].map(offset => {
        const vertex = index ? index.getX(i + offset) : i + offset;
        const p = new THREE.Vector3().fromBufferAttribute(position, vertex).applyMatrix4(mesh.matrixWorld);
        return { ...inPlatformFrame(p.x, p.z), y: p.y };
      });
      const low = (key: 'across' | 'along' | 'y') => Math.min(...points.map(p => p[key]));
      const high = (key: 'across' | 'along' | 'y') => Math.max(...points.map(p => p[key]));
      // Triangle AABBs conservatively test the station walking prism, including
      // vertical poles whose end vertices alone would miss its height interval.
      const overlaps = high('across') >= 1.65 && low('across') <= 7.8
        && high('along') >= 0 && low('along') <= 49
        && high('y') >= 8 && low('y') <= 11.5;
      expect(overlaps, `triangle in station ${platform.node_id} walking envelope`).toBe(false);
    }
  });
}

describe('commuter rail corridor geometry', () => {
  it('sweeps a finite bounded corridor with continuous contact wire in rotated and reversed frames', () => {
    const materials = new Map<string, THREE.Material>();
    const art = { get(color: string) { let material = materials.get(color); if (!material) { material = new THREE.MeshBasicNodeMaterial(); materials.set(color, material); } return material; } };
    try {
      const cases: Array<[Point, Point]> = [
        [{ x: 13, z: -29 }, { x: 113, z: -29 }],
        [{ x: 13, z: -29 }, { x: 73, z: 51 }],
        [{ x: 73, z: 51 }, { x: 13, z: -29 }],
      ];
      for (const [a, b] of cases) {
        const kit = new Kit();
        addRailCorridor(kit, a, b, art);
        const group = kit.finish();
        const { vertices, triangles, materials: usedMaterials } = collect(group);
        const dx = b.x - a.x, dz = b.z - a.z, length = Math.hypot(dx, dz);
        expect(vertices.length).toBeGreaterThan(0);
        expect(triangles).toBeLessThanOrEqual(20_000);
        expect(usedMaterials.size).toBeLessThanOrEqual(6);
        for (const p of vertices) {
          expect(Number.isFinite(p.x) && Number.isFinite(p.y) && Number.isFinite(p.z)).toBe(true);
          const x = p.x - a.x, z = p.z - a.z;
          const along = (x * dx + z * dz) / length;
          const lateral = (x * dz - z * dx) / length;
          expect(along).toBeGreaterThanOrEqual(-.08);
          expect(along).toBeLessThanOrEqual(length + .08);
          expect(Math.abs(lateral)).toBeLessThanOrEqual(CORRIDOR_HALF_WIDTH + .15);
        }

        // The contact wire runs above the track center through both endpoint planes.
        for (const along of [0, length]) {
          const endpoint = pointAt(a, b, along);
          const onWire = vertices.some(p => Math.hypot(p.x - endpoint.x, p.z - endpoint.z) < .09 && Math.abs(p.y - CONTACT_Y) < .025);
          expect(onWire).toBe(true);
        }
        expect(vertices.some(p => Math.abs(p.y - CONTACT_Y) < .025)).toBe(true);

        // Elevated supports occupy the negative side, and overhead steel clears the body and roof volume.
        const elevatedNegativeSide = vertices.filter(p => {
          const x = p.x - a.x, z = p.z - a.z;
          const lateral = (x * dz - z * dx) / length;
          return lateral < -2.5 && p.y > CONTACT_Y;
        });
        expect(elevatedNegativeSide.length).toBeGreaterThan(0);
        for (const p of vertices) {
          const x = p.x - a.x, z = p.z - a.z;
          const lateral = (x * dz - z * dx) / length;
          if (p.y > RAIL_HEAD_Y && p.y < RAIL_HEAD_Y + 4.245) {
            expect(Math.abs(lateral)).toBeGreaterThanOrEqual(1.55);
          }
        }
        disposeGroup(group);
      }
    } finally {
      for (const material of materials.values()) material.dispose();
    }
  });

  it('keeps a long corridor within the geometry and material budget', () => {
    const materials = new Map<string, THREE.Material>();
    const art = { get(color: string) { let material = materials.get(color); if (!material) { material = new THREE.MeshBasicNodeMaterial(); materials.set(color, material); } return material; } };
    try {
      const kit = new Kit();
      addRailCorridor(kit, { x: -400, z: 17 }, { x: 400, z: 17 }, art);
      const group = kit.finish();
      const { triangles, materials: usedMaterials } = collect(group);
      expect(triangles).toBeLessThanOrEqual(20_000);
      expect(usedMaterials.size).toBeLessThanOrEqual(6);
      disposeGroup(group);
    } finally {
      for (const material of materials.values()) material.dispose();
    }
  });

  it('places the contact wire at the authored middle-coach pantograph shoe', () => {
    const materials = new Map<string, THREE.Material>();
    const art = { get(color: string) { let material = materials.get(color); if (!material) { material = new THREE.MeshBasicNodeMaterial(); materials.set(color, material); } return material; } };
    try {
      const corridorKit = new Kit();
      addRailCorridor(corridorKit, { x: 0, z: 0 }, { x: 100, z: 0 }, art);
      const corridorGroup = corridorKit.finish();
      const corridor = collect(corridorGroup).vertices;
      const coachGeometry = buildTrainCar('commuter', 'motor');
      const coach = new THREE.Group();coach.add(new THREE.Mesh(coachGeometry,new THREE.MeshBasicMaterial()));
      const pantographMax = collect(coach).vertices.reduce((max, p) => Math.max(max, p.y), -Infinity);
      const contact = RAIL_HEAD_Y + CONTACT_HEIGHT;
      const contactWireY = corridor.filter(p => Math.abs(p.y - contact) < .02).map(p => p.y);
      expect(contactWireY.length).toBeGreaterThan(0);
      expect(Math.min(...contactWireY)).toBeCloseTo(RAIL_HEAD_Y + pantographMax, 3);
      disposeGroup(corridorGroup);
      disposeGroup(coach);
    } finally {
      for (const material of materials.values()) material.dispose();
    }
  });

  it('keeps both inward-facing terminal platform envelopes clear in either road storage direction', () => {
    const materials = new Map<string, THREE.Material>();
    const art = { get(color: string) { let material = materials.get(color); if (!material) { material = new THREE.MeshBasicNodeMaterial(); materials.set(color, material); } return material; } };
    const terminals: RailPlatform[] = [
      { node_id: 1, x: 0, z: 0, ux: 0, uz: 1, surface_y: 8, stair_x: 4.05, stair_bottom_z: -20, stair_top_z: -1, ground_y: .27, stair_steps: 44, ground_path: [{x:0,z:0},{x:7.5,z:-1.5},{x:7.5,z:-21.5},{x:4.05,z:-21.5},{x:4.05,z:-20}], ground_width: 1.6, construction_phase: 4 },
      { node_id: 2, x: 0, z: 200, ux: 0, uz: -1, surface_y: 8, stair_x: 4.05, stair_bottom_z: -20, stair_top_z: -1, ground_y: .27, stair_steps: 44, ground_path: [{x:0,z:0},{x:7.5,z:-1.5},{x:7.5,z:-21.5},{x:4.05,z:-21.5},{x:4.05,z:-20}], ground_width: 1.6, construction_phase: 4 },
    ];
    try {
      for (const [a, b] of [
        [{ x: 0, z: 0 }, { x: 0, z: 200 }],
        [{ x: 0, z: 200 }, { x: 0, z: 0 }],
      ] as Array<[Point, Point]>) {
        const kit = new Kit();
        addRailCorridor(kit, a, b, art, terminals);
        const group = kit.finish();
        for (const terminal of terminals) expectPlatformEnvelopeClear(group, terminal);
        disposeGroup(group);
      }
    } finally {
      for (const material of materials.values()) material.dispose();
    }
  });

  it('emits nothing for invalid or zero-length endpoints', () => {
    const materials = new Map<string, THREE.Material>();
    const art = { get(color: string) { let material = materials.get(color); if (!material) { material = new THREE.MeshBasicNodeMaterial(); materials.set(color, material); } return material; } };
    try {
      for (const [a, b] of [
        [{ x: 2, z: 3 }, { x: 2, z: 3 }],
        [{ x: Number.NaN, z: 0 }, { x: 1, z: 1 }],
        [{ x: 0, z: 0 }, { x: Number.POSITIVE_INFINITY, z: 0 }],
      ] as Array<[Point, Point]>) {
        const kit = new Kit();
        addRailCorridor(kit, a, b, art);
        expect(collect(kit.finish()).vertices).toHaveLength(0);
      }
    } finally {
      for (const material of materials.values()) material.dispose();
    }
  });
});

it('keeps curved corridor furniture within the authoritative reservation and contact wire on track', () => {
  const cases = (JSON.parse(fixtureText) as { cases: RailAlignment[] }).cases;
  const material = new THREE.MeshBasicNodeMaterial();
  for (const alignment of cases) {
    const path = sampledRail(alignment), kit = new Kit();
    addRailCorridor(kit, path.at(0), path.at(path.length), { get: () => material }, [], path);
    const group = kit.finish(), { vertices } = collect(group);
    for (const v of vertices) {
      const p = path.at(path.distanceAt(v));
      expect(Math.hypot(v.x - p.x, v.z - p.z)).toBeLessThan(3.75);
    }
    for (const d of [0, path.length * .5, path.length]) {
      const p = path.at(d);
      expect(vertices.some(v => Math.hypot(v.x-p.x, v.z-p.z)<1.1 && Math.abs(v.y-CONTACT_Y)<.04)).toBe(true);
    }
    disposeGroup(group);
  }
  material.dispose();
});
