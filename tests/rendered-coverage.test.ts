import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import {
  auditRenderedCoverage,
  createCoverageGrid,
  markLocalTriangle,
} from '../scripts/audit-rendered-coverage.mjs';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingKit, disposeGroup } from '../web/src/render/kit';
import { buildExtendedBuilding } from '../web/src/render/extended-building';

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

  it('measures the retained mass and additive annex, while legacy snapshots keep identical kit coverage', () => {
    const art = new ArtMaterials();
    const original = { building_id: 1, x: 0, z: 10, width: 20, depth: 20, rotation: 0 };
    const expanded = {
      id: 1, parcel_id: 1, x: 0, z: 0, rotation: 0, width: 20, depth: 40,
      floors: 2, kind: 'apartment', stage: 'open', progress: 1, capacity: 10,
      occupancy: 8, built_day: 1, name: 'Test', appearance_seed: 31,
      history: [], anchor: false, vacant_days: 0,
    };
    const baseSnapshot = {
      bounds: { min_x: -30, max_x: 30, min_z: -30, max_z: 30 },
      buildings: [expanded],
    };
    const measure = (snapshot: typeof baseSnapshot, create: (building: typeof expanded, footprint?: { building_id: number; x: number; z: number; width: number; depth: number; rotation: number }) => THREE.Group) =>
      auditRenderedCoverage(snapshot, { create, dispose: disposeGroup }, { cellSize: 0.25 });
    try {
      const legacy = measure(baseSnapshot, (building) => buildingKit(building, art));
      const legacyAdapter = measure(baseSnapshot, (building, footprint) => buildExtendedBuilding(building, footprint, art));
      expect(legacyAdapter.domain.projectedKitGeometryAreaM2).toBe(legacy.domain.projectedKitGeometryAreaM2);
      expect(legacyAdapter.cells).toEqual(legacy.cells);

      const extendedSnapshot = { ...baseSnapshot, building_original_footprints: [original] };
      const extended = measure(extendedSnapshot, (building, footprint) => buildExtendedBuilding(building, footprint, art));
      const retainedOnly = measure({ ...baseSnapshot, buildings: [{ ...expanded, ...original, id: 1 }] }, (building) => buildingKit(building, art));
      expect(extended.buildings.renderedCount).toBe(1);
      expect(extended.domain.projectedKitGeometryAreaM2).toBeGreaterThan(retainedOnly.domain.projectedKitGeometryAreaM2);
      // Ignoring persisted form data and stretching the old kit is a different
      // geometry, even though both would cover more than the retained-only kit.
      expect(extended.cells).not.toEqual(legacy.cells);
    } finally {
      art.dispose();
    }
  });

  it('clips crossing building unions and denominators to a validated crop', () => {
    const snapshot = {
      bounds: { min_x: 0, max_x: 10, min_z: 0, max_z: 10 },
      buildings: [
        { id: 1, x: 3, z: 4, rotation: 0, width: 4, depth: 2 },
        { id: 2, x: 9, z: 9, rotation: 0, width: 1, depth: 1 },
      ],
    };
    const vertices = [[-2, 2, -1], [2, 2, -1], [2, 2, 1], [-2, 2, -1], [2, 2, 1], [-2, 2, 1]];
    const position = {
      count: vertices.length,
      itemSize: 3,
      getX: (i: number) => vertices[i][0],
      getY: (i: number) => vertices[i][1],
      getZ: (i: number) => vertices[i][2],
    };
    const kit = {
      create: (building: typeof snapshot.buildings[number]) => {
        const mesh = {
          isMesh: true,
          geometry: { index: null, getAttribute: () => position },
          matrixWorld: { elements: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, building.x, 0, building.z, 1] },
          updateMatrixWorld() {},
        };
        return {
          position: { set() {} }, rotation: { y: 0 }, updateMatrixWorld() {},
          traverse(callback: (object: typeof mesh) => void) { callback(mesh); },
        };
      },
      dispose() {},
    };
    const cropBounds = { min_x: 2, min_z: 2, max_x: 6, max_z: 6 };
    const report = auditRenderedCoverage(snapshot, kit, { cellSize: 1, cropBounds });

    expect(report.cropBounds).toEqual(cropBounds);
    expect(report.buildings.snapshotCount).toBe(2);
    expect(report.buildings.renderedCount).toBe(2);
    expect(report.domain.areaM2).toBe(16);
    expect(report.domain.projectedKitGeometryAreaM2).toBe(6);
    expect(report.domain.nominalParcelEnvelopeUnionM2).toBe(6);
    expect(report.domain.projectedKitGeometryPercent).toBe(37.5);
    expect(report.methodology.label).toBe('projected kit geometry above 1m within cropped bounds');
    expect(report.methodology.domainDenominator).toMatch(/cropped world bounds/);
    expect(() => auditRenderedCoverage(snapshot, kit, { cropBounds: { min_x: -1, min_z: 2, max_x: 6, max_z: 6 } })).toThrow(/wholly inside/);
  });

  it('reports road-subtracted diagnostic coverage by raster union without changing gross coverage', () => {
    const snapshot = {
      bounds: { min_x: 0, max_x: 20, min_z: 0, max_z: 20 },
      nodes: [{ id: 1, x: 2, z: 10 }, { id: 2, x: 18, z: 10 }],
      roads: [
        { id: 1, from: 1, to: 2, kind: 'local', stage: 'open' },
        { id: 2, from: 1, to: 2, kind: 'local', stage: 'open' },
        { id: 3, from: 1, to: 2, kind: 'rail', stage: 'open' },
      ],
      buildings: [{ id: 1, x: 10, z: 10, width: 8, depth: 8 }],
    };
    const vertices = [
      [-4, 2, -4], [4, 2, -4], [4, 2, 4],
      [-4, 2, -4], [4, 2, 4], [-4, 2, 4],
    ];
    const position = {
      count: vertices.length, itemSize: 3,
      getX: (i: number) => vertices[i][0],
      getY: (i: number) => vertices[i][1],
      getZ: (i: number) => vertices[i][2],
    };
    const report = auditRenderedCoverage(snapshot, {
      create: () => {
        const mesh = {
          isMesh: true,
          geometry: { index: null, getAttribute: () => position },
          matrixWorld: { elements: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 10, 0, 10, 1] },
          updateMatrixWorld() {},
        };
        return { position: { set() {} }, rotation: { y: 0 }, updateMatrixWorld() {}, traverse(callback: (object: typeof mesh) => void) { callback(mesh); } };
      },
      dispose() {},
    }, { cellSize: 0.5 });

    expect(report.domain.projectedKitGeometryAreaM2).toBe(64);
    expect(report.openStreetAdjustedDiagnostic.openStreetAreaM2).toBe(16 * 6.5);
    expect(report.openStreetAdjustedDiagnostic.netDomainAreaM2).toBe(400 - 16 * 6.5);
    expect(report.openStreetAdjustedDiagnostic.netProjectedKitGeometryAreaM2).toBe(12);
    expect(report.openStreetAdjustedDiagnostic.netProjectedKitGeometryPercent).toBeCloseTo(12 / 296 * 100);
    expect(report.openStreetAdjustedDiagnostic.excludedKinds).toEqual(['rail', 'expressway']);
  });

  it('handles absent road data and returns null when open streets consume the whole denominator', () => {
    const bounds = { min_x: 0, max_x: 2, min_z: 0, max_z: 2 };
    const emptyKit = { create: () => ({ traverse() {} }), dispose() {} };
    const withoutRoadData = auditRenderedCoverage({ bounds, buildings: [] }, emptyKit);
    expect(withoutRoadData.openStreetAdjustedDiagnostic.openStreetAreaM2).toBe(0);
    expect(withoutRoadData.openStreetAdjustedDiagnostic.netDomainAreaM2).toBe(4);
    expect(withoutRoadData.openStreetAdjustedDiagnostic.netProjectedKitGeometryPercent).toBe(0);

    const fullyCovered = auditRenderedCoverage({
      bounds,
      buildings: [],
      nodes: [{ id: 1, x: -1, z: 1 }, { id: 2, x: 3, z: 1 }],
      roads: [{ from: 1, to: 2, kind: 'local', stage: 'open' }],
    }, emptyKit);
    expect(fullyCovered.openStreetAdjustedDiagnostic.netDomainAreaM2).toBe(0);
    expect(fullyCovered.openStreetAdjustedDiagnostic.netProjectedKitGeometryPercent).toBeNull();
  });
});
