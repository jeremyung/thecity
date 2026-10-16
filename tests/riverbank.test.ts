import { describe, expect, it } from 'vitest';
import { createRiverbankGeometry } from '../web/src/render/riverbank';

describe('continuous riverbank sweep', () => {
  const points = [
    { x: 0, z: 0 },
    { x: 8, z: 10 },
    { x: -5, z: 20 },
  ];

  it('keeps every section on the same world-X water edge offset, including sharp bends', () => {
    const g = createRiverbankGeometry(points, 10, 1, 0, 4, -2, 1);
    const p = g.getAttribute('position');
    const stationXs = new Set<number>();
    for (let i = 0; i < p.count; i++) stationXs.add(p.getX(i));
    expect(stationXs.has(10)).toBe(true);
    expect(stationXs.has(14)).toBe(true);
    expect(stationXs.has(18)).toBe(true);
    expect(stationXs.has(22)).toBe(true);
    expect(stationXs.has(5)).toBe(true);
    expect(stationXs.has(9)).toBe(true);
  });

  it('has outward triangle winding, upward top faces and no water intrusion', () => {
    for (const side of [-1, 1] as const) {
      const g = createRiverbankGeometry(points, 10, side, 0, 3, -2, 1);
      const p = g.getAttribute('position'),
        n = g.getAttribute('normal'),
        index = g.getIndex()!;
      const faceNormals = new Set<string>();
      for (let t = 0; t < index.count; t += 3) {
        const ia = index.getX(t),
          ib = index.getX(t + 1),
          ic = index.getX(t + 2);
        const ax = p.getX(ia),
          ay = p.getY(ia),
          az = p.getZ(ia);
        const ab = [p.getX(ib) - ax, p.getY(ib) - ay, p.getZ(ib) - az];
        const ac = [p.getX(ic) - ax, p.getY(ic) - ay, p.getZ(ic) - az];
        const cross = [
          ab[1] * ac[2] - ab[2] * ac[1],
          ab[2] * ac[0] - ab[0] * ac[2],
          ab[0] * ac[1] - ab[1] * ac[0],
        ];
        const ni = [n.getX(ia), n.getY(ia), n.getZ(ia)];
        expect(cross[0] * ni[0] + cross[1] * ni[1] + cross[2] * ni[2]).toBeGreaterThan(0);
        if (Math.abs(ni[1]) > 0.99) faceNormals.add(`${Math.sign(ni[1])}`);
      }
      expect(faceNormals.has('1')).toBe(true);
      const pos = g.getAttribute('position');
      for (let i = 0; i < pos.count; i++) {
        const edgeX = points.find((q) => q.z === pos.getZ(i))?.x;
        if (edgeX !== undefined)
          expect(side * (pos.getX(i) - edgeX)).toBeGreaterThanOrEqual(10 - 1e-5);
      }
    }
  });

  it('caps only the two global ends and produces finite bounds and UVs', () => {
    const g = createRiverbankGeometry(points, 10, -1, 0, 4, -2, 1);
    const p = g.getAttribute('position'),
      uv = g.getAttribute('uv'),
      index = g.getIndex()!;
    const constantZTriangles = new Map<number, number>();
    for (let t = 0; t < index.count; t += 3) {
      const zs = [0, 1, 2].map((k) => p.getZ(index.getX(t + k)));
      if (zs.every((z) => z === zs[0])) {
        constantZTriangles.set(zs[0], (constantZTriangles.get(zs[0]) ?? 0) + 1);
        const n = g.getAttribute('normal');
        const expectedZ = zs[0] === points[0].z ? -1 : 1;
        expect(n.getZ(index.getX(t))).toBe(expectedZ);
      }
    }
    expect(constantZTriangles.get(points[0].z)).toBe(2);
    expect(constantZTriangles.get(points.at(-1)!.z)).toBe(2);
    expect(constantZTriangles.has(points[1].z)).toBe(false);
    expect(g.boundingBox).not.toBeNull();
    expect(
      [
        ...p.array,
        ...uv.array,
        ...g.boundingBox!.min.toArray(),
        ...g.boundingBox!.max.toArray(),
      ].every(Number.isFinite),
    ).toBe(true);
    expect(index.count).toBeGreaterThan(0);
  });
});

it('forms a closed manifold after welding coincident face vertices on both banks', () => {
  for (const side of [-1, 1] as const) {
    const g = createRiverbankGeometry(
      [
        { x: 0, z: -20 },
        { x: 17, z: 0 },
        { x: -9, z: 25 },
      ],
      32,
      side,
      0,
      1.4,
      -1.3,
      0.04,
    );
    const p = g.getAttribute('position'),
      indices = g.getIndex()!;
    const key = (i: number) => `${p.getX(i)},${p.getY(i)},${p.getZ(i)}`;
    const edges = new Map<string, number>();
    for (let i = 0; i < indices.count; i += 3)
      for (const [a, b] of [
        [0, 1],
        [1, 2],
        [2, 0],
      ]) {
        const k = [key(indices.getX(i + a)), key(indices.getX(i + b))].sort().join('|');
        edges.set(k, (edges.get(k) ?? 0) + 1);
      }
    expect([...edges.values()].every((count) => count === 2)).toBe(true);
    g.dispose();
  }
});
