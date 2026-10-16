import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { Kit, disposeGroup } from '../web/src/render/kit';
import {
  addNeighborhoodShrine,
  addPocketPark,
  addScrambleCrossing,
  addShrineGrove,
  addStationPlaza,
  openSpaceKind,
  openSpaces,
  parkSignature,
  pocketParks,
  streetTreePlacements,
  STREET_TREE_OFFSET,
  STREET_TREE_SPACING,
  MAX_STREET_TREES,
} from '../web/src/render/parks';
import { streetLightPlacements } from '../web/src/render/street-lighting';
import type { Building, BuildingAccess, Parcel, World } from '../web/src/types';

type StreetWorld = Pick<World, 'nodes' | 'roads' | 'building_access' | 'bus_lines'>;
function streetWorld(
  segments: Array<{
    id: number;
    from: number;
    to: number;
    kind?: string;
    stage?: string;
    connected?: boolean;
  }>,
  points: Array<{ id: number; x: number; z: number }>,
): StreetWorld {
  return {
    nodes: points.map((point) => ({ ...point, kind: 'junction' })),
    roads: segments.map((segment) => {
      const from = points.find((point) => point.id === segment.from)!,
        to = points.find((point) => point.id === segment.to)!;
      return {
        id: segment.id,
        from: segment.from,
        to: segment.to,
        kind: segment.kind ?? 'arterial',
        stage: segment.stage ?? 'open',
        progress: 1,
        connected: segment.connected ?? true,
        length: Math.hypot(to.x - from.x, to.z - from.z),
        lanes: 2,
      };
    }),
  };
}
const parcel = (id: number, overrides: Partial<Parcel> = {}): Parcel => ({
  id,
  x: 0,
  z: 0,
  width: 12,
  depth: 16,
  rotation: 0,
  access_node: 1,
  suitability: 0.8,
  protected: true,
  reservation: null,
  ...overrides,
});

describe('street trees', () => {
  it('lines both sidewalks of open connected arterials only, deterministically and clear of lamps', () => {
    const world = streetWorld(
      [
        { id: 1, from: 1, to: 2 },
        { id: 2, from: 2, to: 3, kind: 'local' },
        { id: 3, from: 3, to: 4, stage: 'construction' },
        { id: 4, from: 4, to: 5, connected: false },
      ],
      [
        { id: 1, x: 0, z: 0 },
        { id: 2, x: 200, z: 0 },
        { id: 3, x: 200, z: 200 },
        { id: 4, x: 400, z: 200 },
        { id: 5, x: 400, z: 400 },
      ],
    );
    const trees = streetTreePlacements(world);
    expect(trees).toEqual(streetTreePlacements(world));
    expect(trees.length).toBeGreaterThan(30);
    for (const tree of trees) {
      expect(Math.abs(tree.z)).toBeCloseTo(STREET_TREE_OFFSET, 5);
      // Node 1 is a dead end; node 2 is a T-junction whose local street runs toward +z.
      expect(tree.x).toBeGreaterThanOrEqual(12 - 1e-6);
      if (tree.z > 0) expect(200 - tree.x).toBeGreaterThanOrEqual(6.5 / 2 + 4);
      else expect(tree.x).toBeLessThanOrEqual(198 + 1e-6);
      expect(tree.s).toBeGreaterThan(1.3);
      expect(tree.s).toBeLessThan(1.7);
    }
    // The side away from the T-junction's street runs on past the corner.
    expect(Math.max(...trees.filter((tree) => tree.z < 0).map((tree) => tree.x))).toBeGreaterThan(
      Math.max(...trees.filter((tree) => tree.z > 0).map((tree) => tree.x)),
    );
    const north = trees
      .filter((tree) => tree.z > 0)
      .map((tree) => tree.x)
      .sort((a, b) => a - b);
    for (let i = 1; i < north.length; i++)
      expect(north[i] - north[i - 1]).toBeGreaterThanOrEqual(STREET_TREE_SPACING - 1e-6);
    for (const lamp of streetLightPlacements(world))
      for (const tree of trees)
        expect(Math.hypot(lamp.x - tree.x, lamp.z - tree.z)).toBeGreaterThanOrEqual(3);
  });

  it('keeps entries, bus stops, cross streets and viaducts clear and respects the cap', () => {
    const base = streetWorld(
      [{ id: 1, from: 1, to: 2 }],
      [
        { id: 1, x: 0, z: 0 },
        { id: 2, x: 200, z: 0 },
      ],
    );
    const all = streetTreePlacements(base);
    const target = all[Math.floor(all.length / 2)];
    const blocked: StreetWorld = {
      ...base,
      building_access: [
        {
          building_id: 9,
          road_id: 1,
          points: [
            { x: target.x, z: target.z + Math.sign(target.z) * 3 },
            { x: target.x, z: target.z },
          ],
        },
      ],
      bus_lines: [
        {
          id: 1,
          stops: [],
          route: [],
          active: true,
          stop_positions: [{ node_id: 1, x: 60, z: STREET_TREE_OFFSET, heading: 0, direction: 1 }],
        },
      ],
    };
    const kept = streetTreePlacements(blocked);
    expect(kept.some((tree) => tree.x === target.x && tree.z === target.z)).toBe(false);
    expect(kept.some((tree) => Math.hypot(tree.x - 60, tree.z - STREET_TREE_OFFSET) < 7)).toBe(
      false,
    );

    const crossed = streetWorld(
      [
        { id: 1, from: 1, to: 2 },
        { id: 2, from: 3, to: 4, kind: 'local' },
        { id: 3, from: 5, to: 6, kind: 'rail' },
      ],
      [
        { id: 1, x: 0, z: 0 },
        { id: 2, x: 200, z: 0 },
        { id: 3, x: 100, z: -30 },
        { id: 4, x: 100, z: 30 },
        { id: 5, x: 150, z: -30 },
        { id: 6, x: 150, z: 30 },
      ],
    );
    const near = streetTreePlacements(crossed).filter(
      (tree) => Math.abs(tree.x - 100) < 6.5 / 2 + 4 || Math.abs(tree.x - 150) < 7 / 2 + 3,
    );
    expect(near).toEqual([]);

    // An expressway deck straight overhead keeps the rows; only its pier line is cleared.
    const covered = streetWorld(
      [
        { id: 1, from: 1, to: 2 },
        { id: 2, from: 3, to: 4, kind: 'expressway' },
      ],
      [
        { id: 1, x: 0, z: 0 },
        { id: 2, x: 200, z: 0 },
        { id: 3, x: 0, z: STREET_TREE_OFFSET },
        { id: 4, x: 200, z: STREET_TREE_OFFSET },
      ],
    );
    const underDeck = streetTreePlacements(covered);
    expect(underDeck.filter((tree) => tree.z < 0)).toEqual(
      streetTreePlacements(base).filter((tree) => tree.z < 0),
    );
    expect(underDeck.some((tree) => tree.z > 0)).toBe(false);

    const points = Array.from({ length: 120 }, (_, index) => ({
      id: index + 1,
      x: 0,
      z: index * 400,
    }));
    const long = streetWorld(
      points.slice(1).map((point, index) => ({ id: index + 1, from: index + 1, to: point.id })),
      points,
    );
    expect(streetTreePlacements(long)).toHaveLength(MAX_STREET_TREES);
  });
});

describe('pocket parks', () => {
  it('are protected lots with no building', () => {
    const building = { parcel_id: 2 } as Building;
    const parks = pocketParks({
      parcels: [parcel(3), parcel(1, { protected: false }), parcel(2), parcel(4, { width: 3 })],
      buildings: [building],
    });
    expect(parks.map((park) => park.id)).toEqual([3]);
  });

  it('keeps furniture and trees inside the rotated lot with a street-facing entry', () => {
    const art = new ArtMaterials();
    for (const lot of [
      parcel(1, { x: 40, z: -20, rotation: 0.7 }),
      parcel(2, { x: -10, z: 30, width: 7, depth: 8, rotation: Math.PI }),
    ]) {
      const kit = new Kit(),
        trees = addPocketPark(kit, lot, art),
        group = kit.finish();
      const inverse = (x: number, z: number) => {
        const dx = x - lot.x,
          dz = z - lot.z,
          c = Math.cos(lot.rotation!),
          s = Math.sin(lot.rotation!);
        return { x: c * dx - s * dz, z: s * dx + c * dz };
      };
      expect(trees.length).toBeGreaterThanOrEqual(2);
      for (const tree of trees) {
        const local = inverse(tree.x, tree.z);
        expect(Math.abs(local.x)).toBeLessThan(lot.width / 2);
        expect(Math.abs(local.z)).toBeLessThan(lot.depth / 2);
      }
      const box = new THREE.Box3();
      group.traverse((object) => {
        if (!(object instanceof THREE.Mesh)) return;
        const position = object.geometry.getAttribute('position');
        for (let i = 0; i < position.count; i++) {
          const local = inverse(position.getX(i), position.getZ(i));
          box.expandByPoint(new THREE.Vector3(local.x, position.getY(i), local.z));
        }
      });
      expect(box.min.x).toBeGreaterThanOrEqual(-lot.width / 2 - 1e-3);
      expect(box.max.x).toBeLessThanOrEqual(lot.width / 2 + 1e-3);
      expect(box.min.z).toBeGreaterThanOrEqual(-lot.depth / 2 - 1e-3);
      expect(box.max.z).toBeLessThanOrEqual(lot.depth / 2 + 1e-3);
      expect(box.min.y).toBeGreaterThanOrEqual(-1e-4);
      disposeGroup(group);
    }
  });

  it('plants a dense shrine grove that leaves the approach open toward the city', () => {
    const kit = new Kit(),
      trees = addShrineGrove(
        kit,
        [
          { x: -350, z: -350 },
          { x: -260, z: -350 },
          { x: -260, z: -270 },
          { x: -350, z: -270 },
        ],
        new ArtMaterials(),
      );
    expect(trees.length).toBeGreaterThan(80);
    expect(
      trees.every((tree) => tree.x > -350 && tree.x < -260 && tree.z > -350 && tree.z < -270),
    ).toBe(true);
    // The longer edge nearest the city center (+z here) holds the torii; its approach stays clear.
    expect(trees.some((tree) => Math.abs(tree.x + 305) < 4 && tree.z > -305)).toBe(false);
    expect(
      addShrineGrove(
        new Kit(),
        [
          { x: 0, z: 0 },
          { x: 1, z: 1 },
        ],
        new ArtMaterials(),
      ),
    ).toEqual([]);
    disposeGroup(kit.finish());
  });
});

describe('station plaza, shrines and the scramble crossing', () => {
  const localBounds = (group: THREE.Group, lot: Parcel) => {
    const c = Math.cos(lot.rotation ?? 0),
      sn = Math.sin(lot.rotation ?? 0),
      box = new THREE.Box3();
    group.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return;
      const position = object.geometry.getAttribute('position');
      for (let i = 0; i < position.count; i++) {
        const dx = position.getX(i) - lot.x,
          dz = position.getZ(i) - lot.z;
        box.expandByPoint(new THREE.Vector3(c * dx - sn * dz, position.getY(i), sn * dx + c * dz));
      }
    });
    return box;
  };

  it('classifies open spaces and keeps legacy parks as parks', () => {
    expect(openSpaceKind({})).toBe('park');
    expect(openSpaceKind({ open_space: 'shrine' })).toBe('shrine');
    expect(openSpaceKind({ open_space: 'plaza' })).toBe('plaza');
    const world = {
      parcels: [
        parcel(1),
        parcel(2, { open_space: 'shrine' }),
        parcel(3, { open_space: 'plaza' }),
        parcel(4, { open_space: 'park' }),
      ],
      buildings: [],
    };
    expect(openSpaces(world).map((p) => p.id)).toEqual([1, 2, 3, 4]);
    expect(pocketParks(world).map((p) => p.id)).toEqual([1, 4]);
  });

  it('keeps plaza and shrine furniture and trees inside their rotated lots', () => {
    const art = new ArtMaterials();
    for (const [build, lot] of [
      [
        addStationPlaza,
        parcel(1, { x: 30, z: 10, width: 20, depth: 14, rotation: 0.4, open_space: 'plaza' }),
      ],
      [
        addNeighborhoodShrine,
        parcel(2, { x: -40, z: 5, width: 13, depth: 18, rotation: 2.1, open_space: 'shrine' }),
      ],
    ] as const) {
      const kit = new Kit(),
        trees = build(kit, lot, art),
        group = kit.finish();
      const box = localBounds(group, lot);
      expect(box.min.x).toBeGreaterThanOrEqual(-lot.width / 2 - 1e-3);
      expect(box.max.x).toBeLessThanOrEqual(lot.width / 2 + 1e-3);
      expect(box.min.z).toBeGreaterThanOrEqual(-lot.depth / 2 - 1e-3);
      expect(box.max.z).toBeLessThanOrEqual(lot.depth / 2 + 1e-3);
      expect(box.min.y).toBeGreaterThanOrEqual(-1e-4);
      expect(trees.length).toBeGreaterThanOrEqual(1);
      const c = Math.cos(lot.rotation!),
        sn = Math.sin(lot.rotation!);
      for (const tree of trees) {
        const dx = tree.x - lot.x,
          dz = tree.z - lot.z;
        expect(Math.abs(c * dx - sn * dz)).toBeLessThan(lot.width / 2);
        expect(Math.abs(sn * dx + c * dz)).toBeLessThan(lot.depth / 2);
      }
      disposeGroup(group);
    }
  });

  it('marks the busiest arterial junction near the station and nothing without one', () => {
    const art = new ArtMaterials();
    const points = [
      { id: 1, x: 0, z: 0 },
      { id: 2, x: 80, z: 0 },
      { id: 3, x: -80, z: 0 },
      { id: 4, x: 0, z: 80 },
      { id: 5, x: 0, z: -80 },
      { id: 6, x: 400, z: 400 },
      { id: 7, x: 40, z: 60 },
    ];
    const world = streetWorld(
      [
        { id: 1, from: 1, to: 2 },
        { id: 2, from: 1, to: 3 },
        { id: 3, from: 1, to: 4, kind: 'local' },
        { id: 4, from: 1, to: 5, kind: 'local' },
      ],
      points,
    );
    world.nodes.push({ id: 99, x: 30, z: 30, kind: 'station' });
    const kit = new Kit();
    expect(addScrambleCrossing(kit, world, art)).toBe(1);
    const group = kit.finish();
    let bars = 0;
    group.traverse((object) => {
      if (object instanceof THREE.Mesh) bars += object.geometry.getAttribute('position').count / 24;
    });
    expect(bars).toBeGreaterThan(20);
    disposeGroup(group);
    const quiet = streetWorld(
      [
        { id: 1, from: 1, to: 2, kind: 'local' },
        { id: 2, from: 1, to: 3, kind: 'local' },
        { id: 3, from: 1, to: 4, kind: 'local' },
      ],
      points,
    );
    quiet.nodes.push({ id: 99, x: 30, z: 30, kind: 'station' });
    expect(addScrambleCrossing(new Kit(), quiet, art)).toBeNull();
    expect(
      addScrambleCrossing(new Kit(), streetWorld([{ id: 1, from: 1, to: 2 }], points), art),
    ).toBeNull();
  });
});

describe('park layer invalidation', () => {
  const world = (overrides: Partial<World> = {}) =>
    ({
      nodes: [
        { id: 1, x: 0, z: 0, kind: 'junction' },
        { id: 2, x: 200, z: 0, kind: 'junction' },
      ],
      roads: [
        {
          id: 1,
          from: 1,
          to: 2,
          kind: 'arterial',
          stage: 'open',
          progress: 1,
          connected: true,
          length: 200,
          lanes: 2,
        },
      ],
      parcels: [],
      buildings: [],
      building_access: [
        {
          building_id: 4,
          road_id: 1,
          points: [
            { x: 100, z: 14 },
            { x: 100, z: 9 },
          ] as const,
        },
      ],
      bus_lines: [
        {
          id: 1,
          stops: [1],
          route: [1, 2],
          active: true,
          stop_positions: [{ node_id: 1, x: 60, z: 9.5, heading: 0, direction: 1 as const }],
        },
      ],
      ...overrides,
    }) as unknown as World;

  it('changes when an entrance or stop moves without changing the count', () => {
    // Put the entrance on a tree that would otherwise stand on the +z sidewalk.
    const tree = streetTreePlacements(world({ building_access: [] })).filter((t) => t.z > 0)[3];
    const entrance = (x: number): BuildingAccess[] => [
      {
        building_id: 4,
        road_id: 1,
        points: [
          { x, z: tree.z + 5 },
          { x, z: tree.z - 0.5 },
        ],
      },
    ];
    const base = world({ building_access: entrance(tree.x) });
    expect(parkSignature(world({ building_access: entrance(tree.x) }))).toBe(parkSignature(base));
    const movedEntrance = world({ building_access: entrance(tree.x + STREET_TREE_SPACING / 2) });
    const movedStop = world({
      building_access: entrance(tree.x),
      bus_lines: [
        {
          id: 1,
          stops: [1],
          route: [1, 2],
          active: true,
          stop_positions: [{ node_id: 1, x: 150, z: 9.5, heading: 0, direction: 1 }],
        },
      ],
    });
    // The move really changes the trees this key guards, with the same entrance count.
    expect(streetTreePlacements(movedEntrance)).not.toEqual(streetTreePlacements(base));
    expect(parkSignature(movedEntrance)).not.toBe(parkSignature(base));
    expect(parkSignature(movedStop)).not.toBe(parkSignature(base));
    const reassigned = world({ building_access: [{ ...entrance(tree.x)[0], building_id: 5 }] });
    expect(parkSignature(reassigned)).not.toBe(parkSignature(base));
  });
});
