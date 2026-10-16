import { describe, expect, it } from 'vitest';
import type { Building, Point, RearExtensionView, Road, WorldNode } from '../web/src/types';
import { createTreeClearanceGrid } from '../web/src/render/tree-clearance-grid';

const building = (
  x: number,
  z: number,
  rotation: number,
  width: number,
  depth: number,
  anchor = false,
) => ({ x, z, rotation, width, depth, anchor }) as Building;
const road = (id: number, from: number, to: number, kind = 'local') =>
  ({ id, from, to, kind }) as Road;
const extension = (
  x: number,
  z: number,
  rotation: number,
  width: number,
  depth: number,
  stage = 'active',
) => ({ x, z, rotation, width, depth, stage }) as RearExtensionView;

function bruteBuildings(p: Point & { s: number }, buildings: readonly Building[]) {
  return buildings.filter((b) => {
    const dx = p.x - b.x,
      dz = p.z - b.z,
      c = Math.cos(b.rotation),
      s = Math.sin(b.rotation),
      lx = c * dx - s * dz,
      lz = s * dx + c * dz,
      canopy = p.s * 1.7 + 2;
    return (
      Math.abs(lx) < b.width / 2 + canopy &&
      lz > -b.depth / 2 - canopy &&
      lz < b.depth / 2 + canopy + (b.anchor ? 4 : 0)
    );
  });
}
function bruteExtensions(p: Point & { s: number }, extensions: readonly RearExtensionView[]) {
  return extensions.slice(0, 2).filter((e) => {
    if (e.stage === 'survey') return false;
    const dx = p.x - e.x,
      dz = p.z - e.z,
      c = Math.cos(e.rotation),
      s = Math.sin(e.rotation),
      canopy = p.s * 1.7 + 2;
    return (
      Math.abs(c * dx - s * dz) < e.width / 2 + canopy &&
      Math.abs(s * dx + c * dz) < e.depth / 2 + canopy
    );
  });
}
function distanceToSegment(p: Point, a: Point, b: Point) {
  const dx = b.x - a.x,
    dz = b.z - a.z,
    t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.z - a.z) * dz) / (dx * dx + dz * dz || 1)));
  return Math.hypot(p.x - a.x - dx * t, p.z - a.z - dz * t);
}

describe('tree clearance broad-phase grid', () => {
  it('matches brute-force exact tests for rotated anchor footprints, varying canopy scales, and extensions', () => {
    const buildings = [
      building(24, 24, Math.PI / 4, 3, 13, true), // narrow rotated footprint with an extended rear
      building(-48, -24, -0.71, 18, 5),
      building(72, 48, 0.2, 8, 8),
    ];
    const extensions = [
      extension(24, 48, Math.PI / 3, 4, 16),
      extension(-48, -24, 0.4, 9, 5),
      extension(0, 0, 0, 100, 100, 'survey'),
    ];
    const find = createTreeClearanceGrid(
      buildings,
      [],
      extensions,
      [],
      Math.max(1.5, 4.3) * 1.7 + 2,
      () => 0,
    );
    const points = [
      { x: 24, z: 40, s: 4.3 },
      { x: 27, z: 29, s: 1.5 },
      { x: -44, z: -27, s: 3.9 },
      { x: 73, z: 48, s: 2.1 },
      { x: 47.999, z: 24, s: 4.3 },
      { x: -24, z: -24, s: 1.5 },
    ];
    for (const p of points)
      expect(find(p).buildings.filter((b) => bruteBuildings(p, [b]).length)).toEqual(
        bruteBuildings(p, buildings),
      );
    for (const p of points)
      expect(find(p).rearExtensions.filter((e) => bruteExtensions(p, [e]).length)).toEqual(
        bruteExtensions(p, extensions),
      );
  });

  it('covers diagonal and negative-coordinate roads across cell boundaries, retaining source order and excluding expressways', () => {
    const nodes: WorldNode[] = [
      { id: 1, x: -51, z: -49, kind: 'road' },
      { id: 2, x: 51, z: 49, kind: 'road' },
      { id: 3, x: 24, z: -51, kind: 'road' },
      { id: 4, x: 24, z: 51, kind: 'road' },
    ];
    const roads = [
      road(1, 1, 2),
      road(2, 3, 4, 'lane'),
      road(3, 1, 2, 'expressway'),
      road(4, 100, 101),
    ];
    const reach = (kind: string) => (kind === 'lane' ? 4 : 3);
    const find = createTreeClearanceGrid([], roads, [], nodes, 0, reach);
    const points = [
      { x: 0, z: 0 },
      { x: 23.99, z: -24.01 },
      { x: -24, z: -24 },
      { x: 24, z: 48 },
      { x: 55, z: 0 },
    ];
    for (const p of points) {
      const expected = roads.filter((r) => {
        if (r.kind === 'expressway') return false;
        const a = nodes.find((n) => n.id === r.from),
          b = nodes.find((n) => n.id === r.to);
        return !!a && !!b && distanceToSegment(p, a, b) < reach(r.kind);
      });
      const broad = find(p).roads;
      expect(broad.filter((r) => expected.includes(r))).toEqual(expected);
      expect(broad).not.toContain(roads[2]);
    }
  });
});

it('includes trees near a rail bulge outside the endpoint chord bounds', () => {
  const rail = road(9, 1, 2, 'rail');
  const nodes = [
    { id: 1, x: 0, z: 0 },
    { id: 2, x: 100, z: 0 },
  ] as WorldNode[];
  const find = createTreeClearanceGrid([], [rail], [], nodes, 4, () => 7, [
    {
      road_id: 9,
      samples: [
        [0, 0, 0, 1, 0],
        [60, 50, 40, 1, 0],
        [120, 100, 0, 1, 0],
      ],
    },
  ]);
  expect(find({ x: 50, z: 42 }).roads).toContain(rail);
  expect(find({ x: 50, z: 100 }).roads).not.toContain(rail);
});
