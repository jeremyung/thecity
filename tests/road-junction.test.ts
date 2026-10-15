import { describe, expect, it } from 'vitest';
import { junctionFootprint, type JunctionArm } from '../web/src/render/road-junction';
import type { Point } from '../web/src/types';

function assertConvex(points: Point[]) {
  expect(points.length).toBeGreaterThanOrEqual(3);
  for (let i = 0; i < points.length; i++) {
    const a = points[i], b = points[(i + 1) % points.length], c = points[(i + 2) % points.length];
    const turn = (b.x - a.x) * (c.z - b.z) - (b.z - a.z) * (c.x - b.x);
    expect(turn).toBeGreaterThan(-1e-7);
  }
}

function transformPoint(point: Point, angle: number, tx: number, tz: number): Point {
  return {
    x: point.x * Math.cos(angle) - point.z * Math.sin(angle) + tx,
    z: point.x * Math.sin(angle) + point.z * Math.cos(angle) + tz,
  };
}

function canonicalRelative(points: Point[], node: Point) {
  return points.map(point => [point.x - node.x, point.z - node.z]);
}

describe('junctionFootprint', () => {
  it('keeps the narrow side of a mixed 90-degree junction within its half-width', () => {
    const node = { x: 0, z: 0 };
    const footprint = junctionFootprint(node, [
      { end: { x: 60, z: 0 }, width: 16 },
      { end: { x: 0, z: 60 }, width: 6.5 },
    ]);

    expect(footprint.length).toBeGreaterThan(0);
    for (const point of footprint) {
      expect(Math.hypot(point.x, point.z)).toBeLessThanOrEqual(8 + 1e-7);
      if (point.z > 1e-7) expect(Math.abs(point.x)).toBeLessThanOrEqual(3.25 + 1e-7);
    }
  });

  it('returns no pad for collinear roads or degenerate arms', () => {
    expect(junctionFootprint({ x: 0, z: 0 }, [
      { end: { x: 50, z: 0 }, width: 6.5 },
      { end: { x: -50, z: 0 }, width: 16 },
    ])).toEqual([]);
    expect(junctionFootprint({ x: 0, z: 0 }, [
      { end: { x: 0, z: 0 }, width: 16 },
      { end: { x: 0, z: 0 }, width: 6.5 },
      { end: { x: 40, z: 0 }, width: 6.5 },
    ])).toEqual([]);
    expect(junctionFootprint({ x: 0, z: 0 }, [
      { end: { x: 40, z: 0 }, width: 0 },
      { end: { x: 0, z: 40 }, width: Number.NaN },
    ])).toEqual([]);
  });

  it('is invariant to arm ordering', () => {
    const node = { x: 4, z: -9 };
    const arms: JunctionArm[] = [
      { end: { x: 54, z: -9 }, width: 16 },
      { end: { x: 4, z: 41 }, width: 6.5 },
      { end: { x: -36, z: -9 }, width: 6.5 },
    ];
    expect(junctionFootprint(node, arms)).toEqual(junctionFootprint(node, [...arms].reverse()));
  });

  it.each([
    ['oblique T', [
      { end: { x: 70, z: 15 }, width: 16 },
      { end: { x: -15, z: 10 }, width: 6.5 },
      { end: { x: 8, z: -60 }, width: 6.5 },
    ] satisfies JunctionArm[]],
    ['oblique X', [
      { end: { x: 55, z: 32 }, width: 16 },
      { end: { x: -44, z: -24 }, width: 16 },
      { end: { x: -30, z: 53 }, width: 6.5 },
      { end: { x: 34, z: -60 }, width: 6.5 },
    ] satisfies JunctionArm[]],
  ])('returns finite convex vertices for a translated and rotated %s', (_name, baseArms) => {
    const origin = { x: 0, z: 0 };
    const angle = .713, tx = 127.5, tz = -83.25;
    const node = { x: tx, z: tz };
    const arms = baseArms.map(arm => ({ ...arm, end: transformPoint(arm.end, angle, tx, tz) }));
    const footprint = junctionFootprint(node, arms);

    assertConvex(footprint);
    expect(footprint.every(point => Number.isFinite(point.x) && Number.isFinite(point.z))).toBe(true);
    const maxHalfWidth = Math.max(...arms.map(arm => arm.width / 2));
    for (const [x, z] of canonicalRelative(footprint, node)) {
      expect(Math.hypot(x, z)).toBeLessThanOrEqual(maxHalfWidth + 1e-7);
    }
    const baseline = junctionFootprint(origin, baseArms);
    const transformedBaseline = baseline.map(point => transformPoint(point, angle, tx, tz));
    expect(footprint.length).toBe(transformedBaseline.length);
    for (const expected of transformedBaseline) {
      expect(footprint.some(point => Math.hypot(point.x - expected.x, point.z - expected.z) < 1e-6)).toBe(true);
    }
  });
});
