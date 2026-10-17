import { describe, expect, it } from 'vitest';
import type { Road, WorldNode } from '../web/src/types';
import {
  arterialSidewalkPolygons,
  arterialSidewalkSteps,
} from '../web/src/render/arterial-sidewalks';

const nodes = (rows: [number, number, number][]) =>
  rows.map(([id, x, z]) => ({ id, x, z, kind: 'junction' }) as WorldNode);
const road = (id: number, from: number, to: number, kind = 'arterial', stage = 'open') =>
  ({ id, from, to, kind, stage, progress: 1, connected: true, length: 1, lanes: 2 }) as Road;
const world = (ns: WorldNode[], rs: Road[]) => ({ nodes: ns, roads: rs });
const cross = (
  a: { x: number; z: number },
  b: { x: number; z: number },
  p: { x: number; z: number },
) => (b.x - a.x) * (p.z - a.z) - (b.z - a.z) * (p.x - a.x);
function contains(poly: { x: number; z: number }[], p: { x: number; z: number }) {
  return (
    poly.every((v, i) => cross(v, poly[(i + 1) % poly.length], p) >= -1e-6) ||
    poly.every((v, i) => cross(v, poly[(i + 1) % poly.length], p) <= 1e-6)
  );
}
function area(p: { x: number; z: number }[]) {
  return Math.abs(
    p.reduce((s, a, i) => s + a.x * p[(i + 1) % p.length].z - p[(i + 1) % p.length].x * a.z, 0) / 2,
  );
}
function overlapArea(a: { x: number; z: number }[], b: { x: number; z: number }[]) {
  let p = a;
  for (let i = 0; i < b.length; i++) {
    const u = b[i],
      v = b[(i + 1) % b.length];
    const signed = (q: { x: number; z: number }) =>
      cross(u, v, q) * (cross(u, v, b[(i + 2) % b.length]) >= 0 ? 1 : -1);
    const out: typeof p = [];
    for (let j = 0; j < p.length; j++) {
      const s = p[j],
        e = p[(j + 1) % p.length],
        cs = signed(s),
        ce = signed(e);
      if (cs >= -1e-8) out.push(s);
      if (cs >= 0 !== ce >= 0) {
        const t = cs / (cs - ce);
        out.push({ x: s.x + (e.x - s.x) * t, z: s.z + (e.z - s.z) * t });
      }
    }
    p = out;
  }
  return p.length >= 3 ? area(p) : 0;
}
const has = (polys: { x: number; z: number }[][], p: { x: number; z: number }) =>
  polys.some((q) => contains(q, p));
function drainInQuota(world: { nodes: WorldNode[]; roads: Road[] }, quota: number) {
  const steps = arterialSidewalkSteps(world);
  let result = steps.next();
  while (!result.done) for (let i = 0; i < quota && !result.done; i++) result = steps.next();
  return result.value;
}
function roadPoly(a: { x: number; z: number }, b: { x: number; z: number }, width: number) {
  const dx = b.x - a.x,
    dz = b.z - a.z,
    l = Math.hypot(dx, dz),
    nx = -dz / l,
    nz = dx / l;
  return [
    { x: a.x + (nx * width) / 2, z: a.z + (nz * width) / 2 },
    { x: b.x + (nx * width) / 2, z: b.z + (nz * width) / 2 },
    { x: b.x - (nx * width) / 2, z: b.z - (nz * width) / 2 },
    { x: a.x - (nx * width) / 2, z: a.z - (nz * width) / 2 },
  ];
}

describe('arterial sidewalk polygons', () => {
  it('paves the rounded outer corner while leaving the junction carriageway open', () => {
    const ps = arterialSidewalkPolygons(
      world(
        nodes([
          [1, 0, 0],
          [2, 0, 40],
          [3, 40, 0],
        ]),
        [road(1, 1, 2), road(2, 1, 3)],
      ),
    );
    expect(has(ps, { x: -6.5, z: -6.5 })).toBe(true); // inside outer radius 10.5m, outside the road and junction pad
    expect(has(ps, { x: -4, z: -4 })).toBe(false); // inside the shared road junction
    expect(ps.every((p) => area(p) > 0)).toBe(true);
    for (let i = 0; i < ps.length; i++)
      for (let j = i + 1; j < ps.length; j++) expect(overlapArea(ps[i], ps[j])).toBeLessThan(1e-5);
  });

  it('keeps the rounded sidewalk corner continuous through an oblique bend', () => {
    const angle = (2 * Math.PI) / 3,
      radius = 9.25;
    const corner = {
      x: radius * Math.cos((4 * Math.PI) / 3),
      z: radius * Math.sin((4 * Math.PI) / 3),
    };
    const ps = arterialSidewalkPolygons(
      world(
        nodes([
          [1, 0, 0],
          [2, 40, 0],
          [3, 40 * Math.cos(angle), 40 * Math.sin(angle)],
        ]),
        [road(1, 1, 2), road(2, 1, 3)],
      ),
    );
    expect(has(ps, corner)).toBe(true);
    expect(ps.some((p) => p.some((q) => !Number.isFinite(q.x) || !Number.isFinite(q.z)))).toBe(
      false,
    );
    for (let i = 0; i < ps.length; i++)
      for (let j = i + 1; j < ps.length; j++) expect(overlapArea(ps[i], ps[j])).toBeLessThan(1e-5);
  });

  it('reserves a construction lane at the junction while keeping the opposite arterial corner paved', () => {
    const ps = arterialSidewalkPolygons(
      world(
        nodes([
          [1, 0, 0],
          [2, 0, 40],
          [3, 40, 0],
          [4, -30, 30],
        ]),
        [road(1, 1, 2), road(2, 1, 3), road(3, 1, 4, 'lane', 'construction')],
      ),
    );
    expect(has(ps, { x: -6.5, z: -6.5 })).toBe(true);
    expect(has(ps, { x: -5, z: 5 })).toBe(false); // reserved lane corridor adjoining the sidewalk ring
    for (let i = 0; i < ps.length; i++)
      for (let j = i + 1; j < ps.length; j++) expect(overlapArea(ps[i], ps[j])).toBeLessThan(1e-5);
  });

  it('keeps a full-length shared strip through a straight two-arm junction without positive-area overlap', () => {
    const ps = arterialSidewalkPolygons(
      world(
        nodes([
          [1, 0, -40],
          [2, 0, 0],
          [3, 0, 40],
        ]),
        [road(2, 2, 3), road(1, 1, 2)],
      ),
    );
    expect(has(ps, { x: 9, z: 0 })).toBe(true);
    expect(has(ps, { x: 9, z: 35 })).toBe(true);
    expect(ps.every((p) => area(p) > 0)).toBe(true);
    for (let i = 0; i < ps.length; i++)
      for (let j = i + 1; j < ps.length; j++) expect(overlapArea(ps[i], ps[j])).toBeLessThan(1e-5);
  });
  it('retains mixed T-junction sidewalk beyond the former eight metre trim', () => {
    const ps = arterialSidewalkPolygons(
      world(
        nodes([
          [1, 0, 0],
          [2, 0, 40],
          [3, 30, 0],
        ]),
        [road(1, 1, 2), road(2, 1, 3, 'local')],
      ),
    );
    expect(has(ps, { x: 9, z: 4 })).toBe(true);
    expect(has(ps, { x: 9, z: 12 })).toBe(true);
    expect(has(ps, { x: 9, z: 2 })).toBe(false); // inside both sidewalk strip and local carriageway
  });
  it('clips all arms at a four-way junction and handles oblique approaches', () => {
    const ps = arterialSidewalkPolygons(
      world(
        nodes([
          [1, 0, -40],
          [2, 0, 0],
          [3, 0, 40],
          [4, -40, 0],
          [5, 40, 0],
          [6, 30, 30],
        ]),
        [road(1, 1, 2), road(2, 2, 3), road(3, 4, 2), road(4, 2, 5), road(5, 2, 6, 'local')],
      ),
    );
    expect(has(ps, { x: 9, z: 14 })).toBe(true);
    expect(has(ps, { x: 2, z: 0 })).toBe(false);
    expect(ps.some((p) => p.some((q) => !Number.isFinite(q.x) || !Number.isFinite(q.z)))).toBe(
      false,
    );
    for (let i = 0; i < ps.length; i++)
      for (let j = i + 1; j < ps.length; j++) expect(overlapArea(ps[i], ps[j])).toBeLessThan(1e-5);
    const map = new Map([
      [1, { x: 0, z: -40 }],
      [2, { x: 0, z: 0 }],
      [3, { x: 0, z: 40 }],
      [4, { x: -40, z: 0 }],
      [5, { x: 40, z: 0 }],
      [6, { x: 30, z: 30 }],
    ]);
    for (const p of ps)
      for (const r of [
        road(1, 1, 2),
        road(2, 2, 3),
        road(3, 4, 2),
        road(4, 2, 5),
        road(5, 2, 6, 'local'),
      ]) {
        const width =
          r.kind === 'bridge' ? 18.8 : r.kind === 'arterial' ? 16 : r.kind === 'local' ? 6.5 : 4;
        expect(overlapArea(p, roadPoly(map.get(r.from)!, map.get(r.to)!, width))).toBeLessThan(
          1e-5,
        );
      }
  });
  it('ignores zero-length arterials, emits no construction sidewalk, and reserves its corridor', () => {
    const ps = arterialSidewalkPolygons(
      world(
        nodes([
          [1, 0, 0],
          [2, 0, 0],
          [3, 0, 30],
          [4, 8.5, 0],
          [5, 8.5, 30],
        ]),
        [road(1, 1, 2), road(2, 1, 3), road(3, 4, 5, 'local', 'construction')],
      ),
    );
    expect(ps.length).toBeGreaterThan(0);
    expect(has(ps, { x: 9, z: 15 })).toBe(false);
    expect(ps.every((p) => area(p) > 1e-4)).toBe(true);
  });
  it('reserves the full bridge approach deck width, including its outer side structures', () => {
    const ps = arterialSidewalkPolygons(
      world(
        nodes([
          [1, 0, 0],
          [2, 0, 40],
          [3, 30, 0],
        ]),
        [road(1, 1, 2), road(2, 1, 3, 'bridge')],
      ),
    );
    expect(has(ps, { x: 9, z: 9.1 })).toBe(false); // beyond the nominal 8 m half-width, inside the 9.4 m bridge deck
  });
  it('does not treat grade-separated rail or expressway as ground carriageways', () => {
    const ps = arterialSidewalkPolygons(
      world(
        nodes([
          [1, 0, 0],
          [2, 0, 40],
          [3, -30, 0],
          [4, 30, 0],
        ]),
        [road(1, 1, 2), road(2, 3, 4, 'rail'), road(3, 3, 4, 'expressway')],
      ),
    );
    expect(has(ps, { x: 9, z: 1 })).toBe(true);
  });
  it('is invariant to input road ordering', () => {
    const ns = nodes([
        [1, 0, -40],
        [2, 0, 0],
        [3, 0, 40],
        [4, 30, 0],
      ]),
      rs = [road(1, 1, 2), road(2, 2, 3), road(3, 2, 4, 'local')];
    expect(arterialSidewalkPolygons(world(ns, rs))).toEqual(
      arterialSidewalkPolygons(world(ns, [...rs].reverse())),
    );
  });
  it('returns the exact synchronous output when drained at different yield quotas', () => {
    const ns = nodes([
        [1, 0, -40],
        [2, 0, 0],
        [3, 0, 40],
        [4, -40, 0],
        [5, 40, 0],
        [6, 30, 30],
      ]),
      rs = [road(1, 1, 2), road(2, 2, 3), road(3, 4, 2), road(4, 2, 5), road(5, 2, 6, 'local')],
      input = world(ns, rs),
      expected = arterialSidewalkPolygons(input);
    expect(drainInQuota(input, 1)).toEqual(expected);
    expect(drainInQuota(input, 7)).toEqual(expected);
    expect(drainInQuota(input, 48)).toEqual(expected);
  });
  it('cancels before completion without publishing partial polygons', () => {
    const steps = arterialSidewalkSteps(
      world(
        nodes([
          [1, 0, 0],
          [2, 0, 40],
        ]),
        [road(1, 1, 2)],
      ),
    );
    expect(steps.next().done).toBe(false);
    const cancelled = (
      steps as unknown as { return(): IteratorResult<void, { x: number; z: number }[][]> }
    ).return();
    expect(cancelled.done).toBe(true);
    expect(cancelled.value).toBeUndefined();
  });
});
