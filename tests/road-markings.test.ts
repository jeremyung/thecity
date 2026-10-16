import { describe, expect, it } from 'vitest';
import type { Road, WorldNode } from '../web/src/types';
import { roadMarkingTrims } from '../web/src/render/road-markings';

const nodes = (rows: [number, number, number][]) =>
  rows.map(([id, x, z]) => ({ id, x, z, kind: 'street' }) as WorldNode);
const road = (id: number, from: number, to: number, kind = 'local', stage = 'open') =>
  ({ id, from, to, kind, stage, progress: 1, connected: true, length: 1, lanes: 1 }) as Road;
const world = (ns: WorldNode[], roads: Road[]) => ({ nodes: ns, roads });

describe('roadMarkingTrims', () => {
  it('clears an orthogonal arterial approach by 8.2m and accounts for each road width', () => {
    const trims = roadMarkingTrims(
      world(
        nodes([
          [1, 0, 0],
          [2, 0, 60],
          [3, 60, 0],
        ]),
        [road(10, 1, 2, 'local'), road(11, 1, 3, 'arterial')],
      ),
    );
    expect(trims.get(10)?.start).toBeCloseTo(8.2, 7);
    expect(trims.get(10)?.end).toBe(0);
    expect(trims.get(11)?.start).toBeCloseTo(3.45, 7);
    expect(trims.get(11)?.end).toBe(0);
  });

  it('moves the same clearance to the reversed road endpoint', () => {
    const trims = roadMarkingTrims(
      world(
        nodes([
          [1, 0, 0],
          [2, 0, 60],
          [3, 60, 0],
        ]),
        [road(10, 2, 1, 'local'), road(11, 1, 3, 'arterial')],
      ),
    );
    expect(trims.get(10)?.start).toBe(0);
    expect(trims.get(10)?.end).toBeCloseTo(8.2, 7);
  });

  it('ignores staged and grade-separated neighbors', () => {
    const ns = nodes([
      [1, 0, 0],
      [2, 0, 60],
      [3, 60, 0],
      [4, -60, 0],
      [5, 0, -60],
    ]);
    const alone = roadMarkingTrims(world(ns, [road(10, 1, 2, 'local')]));
    const withIgnored = roadMarkingTrims(
      world(ns, [
        road(10, 1, 2, 'local'),
        road(11, 1, 3, 'arterial', 'construction'),
        road(12, 1, 4, 'rail'),
        road(13, 1, 5, 'expressway'),
      ]),
    );
    expect(withIgnored.get(10)).toEqual(alone.get(10));
    expect(withIgnored.get(10)).toEqual({ start: 0, end: 0 });
  });

  it('bounds an acute short approach by its finite rectangle', () => {
    const angle = 0.02,
      length = 3;
    const trims = roadMarkingTrims(
      world(
        nodes([
          [1, 0, 0],
          [2, 100, 0],
          [3, length * Math.cos(angle), length * Math.sin(angle)],
        ]),
        [road(10, 1, 2, 'local'), road(11, 1, 3, 'arterial')],
      ),
    );
    const start = trims.get(10)?.start;
    expect(start).toBeDefined();
    expect(Number.isFinite(start)).toBe(true);
    expect(start).toBeGreaterThan(0);
    expect(start).toBeLessThanOrEqual(length + 8 * Math.sin(angle) + 0.2 + 1e-7);
  });

  it('is deterministic when road input order changes', () => {
    const ns = nodes([
      [1, 0, 0],
      [2, 0, 60],
      [3, 60, 0],
      [4, -40, 20],
    ]);
    const roads = [road(10, 1, 2, 'local'), road(11, 1, 3, 'arterial'), road(12, 1, 4, 'bridge')];
    const serialize = (trims: Map<number, { start: number; end: number }>) =>
      [...trims].sort(([a], [b]) => a - b);
    expect(serialize(roadMarkingTrims(world(ns, roads)))).toEqual(
      serialize(roadMarkingTrims(world(ns, [...roads].reverse()))),
    );
  });
});
