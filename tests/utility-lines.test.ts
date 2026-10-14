import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { disposeGroup } from '../web/src/render/kit';
import { streetLightPlacements } from '../web/src/render/street-lighting';
import { MAX_POLES, POLE_SPACING, buildUtilityLines, poleOffset, poleSpans, sagPoints, utilityPolePlacements } from '../web/src/render/utility-lines';
import type { Building, World } from '../web/src/types';

type StreetWorld = Pick<World, 'nodes' | 'roads' | 'building_access' | 'bus_lines' | 'buildings'>;
function streetWorld(segments: Array<{ id: number; from: number; to: number; kind?: string; stage?: string; connected?: boolean }>, points: Array<{ id: number; x: number; z: number }>): StreetWorld {
  return {
    nodes: points.map(point => ({ ...point, kind: 'junction' })),
    roads: segments.map(segment => {
      const from = points.find(point => point.id === segment.from)!, to = points.find(point => point.id === segment.to)!;
      return { id: segment.id, from: segment.from, to: segment.to, kind: segment.kind ?? 'local', stage: segment.stage ?? 'open', progress: 1, connected: segment.connected ?? true, length: Math.hypot(to.x - from.x, to.z - from.z), lanes: 1 };
    }),
    buildings: [],
  };
}

describe('utility poles', () => {
  const points = [{ id: 1, x: 0, z: 0 }, { id: 2, x: 0, z: 200 }, { id: 3, x: 200, z: 200 }, { id: 4, x: 200, z: 400 }, { id: 5, x: 400, z: 400 }, { id: 6, x: 400, z: 600 }];

  it('stand on one side of open local streets and lanes only, about 30 m apart', () => {
    const world = streetWorld([
      { id: 1, from: 1, to: 2 }, { id: 2, from: 2, to: 3, kind: 'lane' }, { id: 3, from: 3, to: 4, kind: 'arterial' },
      { id: 4, from: 4, to: 5, stage: 'construction' }, { id: 5, from: 5, to: 6, connected: false },
    ], points);
    const poles = utilityPolePlacements(world);
    expect(poles).toEqual(utilityPolePlacements(world));
    expect(new Set(poles.map(p => p.roadId))).toEqual(new Set([1, 2]));
    const street = poles.filter(p => p.roadId === 1);
    // Road 1 runs along +z at x = 0: every pole shares one side at the curb offset.
    expect(new Set(street.map(p => Math.sign(p.x))).size).toBe(1);
    for (const p of street) expect(Math.abs(p.x)).toBeCloseTo(poleOffset('local'), 5);
    for (const p of poles.filter(p => p.roadId === 2)) expect(Math.abs(p.z - 200)).toBeCloseTo(poleOffset('lane'), 5);
    const along = street.map(p => p.along).sort((a, b) => a - b);
    for (let i = 1; i < along.length; i++) { expect(along[i] - along[i - 1]).toBeGreaterThan(POLE_SPACING - 7); expect(along[i] - along[i - 1]).toBeLessThan(POLE_SPACING + 7); }
    for (const lamp of streetLightPlacements(world)) for (const p of poles) expect(Math.hypot(lamp.x - p.x, lamp.z - p.z)).toBeGreaterThanOrEqual(2.5);
    expect(poles.some(p => p.transformer)).toBe(true);
  });

  it('keep entries, bus stops and crossing streets clear, sliding a blocked pole when they can', () => {
    const base = streetWorld([{ id: 1, from: 1, to: 2 }], points);
    const target = utilityPolePlacements(base)[2];
    const blocked: StreetWorld = { ...base, building_access: [{ building_id: 1, road_id: 1, points: [{ x: target.x * 2, z: target.z }, { x: target.x, z: target.z }] }] };
    const moved = utilityPolePlacements(blocked).filter(p => Math.abs(p.along - target.along) < 4);
    expect(moved).toHaveLength(1);
    expect(Math.abs(moved[0].along - target.along)).toBeCloseTo(3, 5);
    const stops: StreetWorld = { ...base, bus_lines: [{ id: 1, stops: [], route: [], active: true, stop_positions: [{ node_id: 1, x: target.x, z: target.z, heading: 0, direction: 1 }] }] };
    expect(utilityPolePlacements(stops).some(p => Math.hypot(p.x - target.x, p.z - target.z) < 4)).toBe(false);
    const crossed = streetWorld([{ id: 1, from: 1, to: 2 }, { id: 2, from: 7, to: 8, kind: 'arterial' }], [...points, { id: 7, x: -50, z: target.z }, { id: 8, x: 50, z: target.z }]);
    expect(utilityPolePlacements(crossed).some(p => p.roadId === 1 && Math.abs(p.z - target.z) < 9)).toBe(false);
  });

  it('respects the city-wide cap', () => {
    const many = Array.from({ length: 160 }, (_, i) => ({ id: i + 1, x: 0, z: i * 400 }));
    const world = streetWorld(many.slice(1).map((p, i) => ({ id: i + 1, from: i + 1, to: p.id })), many);
    expect(utilityPolePlacements(world)).toHaveLength(MAX_POLES);
  });
});

describe('overhead wires', () => {
  it('sag between attachment points and link neighboring poles on the same street', () => {
    const a = new THREE.Vector3(0, 9.5, 0), b = new THREE.Vector3(0, 9.5, 30);
    const sag = sagPoints(a, b);
    expect(sag[0]).toEqual(a); expect(sag.at(-1)).toEqual(b);
    expect(sag[4].y).toBeLessThan(9.5 - .4);
    expect(Math.min(...sag.map(p => p.y))).toBeGreaterThan(8.5);
    const poles = utilityPolePlacements(streetWorld([{ id: 1, from: 1, to: 2 }, { id: 2, from: 2, to: 3 }], [{ id: 1, x: 0, z: 0 }, { id: 2, x: 0, z: 200 }, { id: 3, x: 200, z: 200 }]));
    const spans = poleSpans(poles);
    expect(spans.every(([p, q]) => p.roadId === q.roadId && q.along > p.along)).toBe(true);
    expect(spans).toHaveLength(poles.length - 2);
  });

  it('keeps one side along a straight street split into short segments and wires through its junctions', () => {
    const nodes = Array.from({ length: 8 }, (_, i) => ({ id: i + 1, x: 10, z: i * 28 }));
    const world = streetWorld(nodes.slice(1).map((p, i) => ({ id: i + 1, from: i + 1, to: p.id })), nodes);
    const poles = utilityPolePlacements(world);
    expect(poles.length).toBeGreaterThanOrEqual(6);
    expect(new Set(poles.map(p => Math.sign(p.x - 10))).size).toBe(1);
    expect(new Set(poles.map(p => p.line)).size).toBe(1);
    expect(poleSpans(poles)).toHaveLength(poles.length - 1);
    // Reversing one segment's direction keeps the same line and side.
    const reversed = streetWorld([...nodes.slice(1).map((p, i) => ({ id: i + 1, from: i + 1, to: p.id })).slice(0, 3), { id: 4, from: 5, to: 4 }, ...nodes.slice(5).map((p, i) => ({ id: i + 5, from: i + 5, to: p.id }))], nodes);
    expect(new Set(utilityPolePlacements(reversed).map(p => Math.sign(p.x - 10))).size).toBe(1);
  });

  it('builds instanced poles, transformers, wires and service drops to houses', () => {
    const art = new ArtMaterials();
    const world = streetWorld([{ id: 1, from: 1, to: 2 }], [{ id: 1, x: 0, z: 0 }, { id: 2, x: 0, z: 200 }]);
    const side = Math.sign(utilityPolePlacements(world)[0].x);
    world.buildings = Array.from({ length: 12 }, (_, i): Building => ({
      id: i + 1, parcel_id: i + 1, x: side * 9, z: 10 + i * 15, rotation: side * -Math.PI / 2, width: 8, depth: 9, floors: 2, kind: 'home', stage: 'open', progress: 1,
      capacity: 4, occupancy: 2, built_day: 1, name: 'House', appearance_seed: i, history: [], anchor: false, vacant_days: 0,
    }));
    const empty = buildUtilityLines({ ...world, buildings: [] }, art), group = buildUtilityLines(world, art);
    const instanced = group.children.filter((c): c is THREE.InstancedMesh => c instanceof THREE.InstancedMesh);
    expect(instanced[0].count).toBe(utilityPolePlacements(world).length);
    const wires = (g: THREE.Group) => (g.getObjectByName('utility-wires') as THREE.Mesh).geometry.getIndex()!.count;
    expect(wires(group)).toBeGreaterThan(wires(empty));
    const box = new THREE.Box3().setFromObject(group.getObjectByName('utility-wires')!);
    expect(box.max.y).toBeLessThan(10); expect(box.min.y).toBeGreaterThan(3);
    disposeGroup(group); disposeGroup(empty);
    expect(buildUtilityLines(streetWorld([], []), art).children).toHaveLength(0);
  });
});
