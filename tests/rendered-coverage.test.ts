import { describe, expect, it } from 'vitest';
import {
  auditRenderedCoverage,
  createCoverageGrid,
  markLocalTriangle,
} from '../scripts/audit-rendered-coverage.mjs';

function area(grid: ReturnType<typeof createCoverageGrid>) {
  return grid.mask.reduce((sum, covered) => sum + covered, 0) * grid.cellSize ** 2;
}

describe('projected kit coverage raster', () => {
  it('preserves projected triangle area under rotation and unions overlaps once', () => {
    const bounds = { min_x: -2, max_x: 2, min_z: -2, max_z: 2 };
    const triangle = [[0, 2, 0], [2, 2, 0], [0, 2, 2]] as [number, number, number][];
    const plain = createCoverageGrid(bounds, 0.05);
    markLocalTriangle(plain, triangle, { x: 0, z: 0, rotation: 0 });
    const quarterTurn = createCoverageGrid(bounds, 0.05);
    markLocalTriangle(quarterTurn, triangle, { x: 0, z: 0, rotation: Math.PI / 2 });
    const union = createCoverageGrid(bounds, 0.05);
    markLocalTriangle(union, triangle, { x: 0, z: 0, rotation: 0 });
    const once = area(union);
    markLocalTriangle(union, triangle, { x: 0, z: 0, rotation: 0 });

    expect(area(plain)).toBeCloseTo(2, 1);
    expect(area(quarterTurn)).toBeCloseTo(area(plain), 1);
    expect(area(union)).toBe(once);
  });

  it('clips geometry below the one-metre visual threshold and rejects unbounded grids', () => {
    const grid = createCoverageGrid({ min_x: 0, max_x: 2, min_z: 0, max_z: 2 }, 0.1);
    markLocalTriangle(grid, [[0, 0.2, 0], [2, 0.2, 0], [0, 0.2, 2]], { x: 0, z: 0, rotation: 0 });
    expect(area(grid)).toBe(0);
    expect(() => createCoverageGrid({ min_x: 0, max_x: 100, min_z: 0, max_z: 100 }, 0.1, 10_000)).toThrow(/cell cap/);
  });

  it('disposes each production-like kit group after measuring it', () => {
    const snapshot = {
      seed: 12,
      tick: 400,
      engine_version: 'test',
      bounds: { min_x: -2, max_x: 2, min_z: -2, max_z: 2 },
      buildings: [{ id: 1, x: 0, z: 0, rotation: 0, width: 2, depth: 2 }],
    };
    const position = {
      count: 3,
      getX: (i: number) => [[0, 2, 0], [1, 2, 0], [0, 2, 1]][i][0],
      getY: (i: number) => [[0, 2, 0], [1, 2, 0], [0, 2, 1]][i][1],
      getZ: (i: number) => [[0, 2, 0], [1, 2, 0], [0, 2, 1]][i][2],
      itemSize: 3,
    };
    const mesh = {
      isMesh: true,
      geometry: { index: null, getAttribute: () => position },
      matrixWorld: { elements: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1] },
      updateMatrixWorld() {},
    };
    let disposed = false;
    const group = {
      position: { set() {} },
      rotation: { y: 0 },
      updateMatrixWorld() {},
      traverse(callback: (object: typeof mesh) => void) { callback(mesh); },
    };
    const report = auditRenderedCoverage(snapshot, {
      create: () => group,
      dispose: (value) => { expect(value).toBe(group); disposed = true; },
    });

    expect(disposed).toBe(true);
    expect(report.buildings.renderedCount).toBe(1);
    expect(report.domain.projectedKitGeometryAreaM2).toBeGreaterThan(0);
    expect(report.methodology.label).toBe('projected kit geometry above 1m');
  });
});
