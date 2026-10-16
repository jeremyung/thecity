import { describe, expect, it } from 'vitest';
// @ts-expect-error This executable .mjs utility intentionally has no TS build dependency.
import { summarizeDensity } from '../scripts/audit-density.mjs';
// @ts-expect-error This executable .mjs utility intentionally has no TS build dependency.
import { renderDensityMap } from '../scripts/render-density-map.mjs';

interface Measure {
  count: number;
  nominalParcelEnvelopeM2: number;
  nominalFloorAreaProxyM2: number;
  nominalParcelEnvelopeCoverage: number;
  grossFloorAreaRatioProxy: number;
}
interface Cell {
  ix: number;
  iz: number;
  areaM2: number;
  total: Measure;
}
interface DensityReport {
  domain: { areaM2: number };
  denominator: string;
  method: {
    interpretation: string;
    buildingDimensions: string;
    rotation: string;
    occupancy: string;
    commercialActivity: string;
  };
  buildings: {
    valid: number;
    invalid: number;
    invalidIds: Array<number | null>;
    total: Measure;
    byStatus: Record<string, Measure>;
    byKind: Record<string, Measure>;
  };
  cells?: { populated: Cell[] };
}
const summarize = summarizeDensity as (
  snapshot: unknown,
  options?: { cellSize?: number | null },
) => DensityReport;

function snapshot(buildings: unknown[], bounds = { min_x: 0, max_x: 200, min_z: 0, max_z: 100 }) {
  return { bounds, buildings };
}

function building(overrides: Record<string, unknown> = {}) {
  return {
    id: 1,
    x: 50,
    z: 50,
    width: 20,
    depth: 10,
    floors: 1,
    occupancy: 1,
    kind: 'home',
    stage: 'open',
    ...overrides,
  };
}

describe('urban density audit', () => {
  it('uses non-square lot area for footprint coverage and multistorey gross FAR', () => {
    const report = summarize(
      snapshot(
        [
          building({ width: 20, depth: 10, floors: 4 }),
          building({ id: 2, x: 150, width: 30, depth: 10, floors: 2, kind: 'office' }),
        ],
        { min_x: 0, max_x: 200, min_z: 0, max_z: 100 },
      ),
    );

    expect(report.domain.areaM2).toBe(20_000);
    expect(report.buildings.total.nominalParcelEnvelopeM2).toBe(500);
    expect(report.buildings.total.nominalFloorAreaProxyM2).toBe(1_400);
    expect(report.buildings.total.nominalParcelEnvelopeCoverage).toBe(0.025);
    expect(report.buildings.total.grossFloorAreaRatioProxy).toBe(0.07);
    expect(report.buildings.byKind.home.nominalFloorAreaProxyM2).toBe(800);
    expect(report.buildings.byKind.office.nominalParcelEnvelopeM2).toBe(300);
    expect(report.denominator).toMatch(/gross area.*water and protected land/);
    expect(report.method.interpretation).toMatch(/proxies, not actual rendered/);
    expect(report.method.buildingDimensions).toMatch(/sets structures back/);
    expect(report.method.occupancy).toMatch(/does not determine commercial activity/);
    expect(report.method.commercialActivity).toMatch(/business_active/);
  });

  it('clips each footprint into fixed cells and conserves area at cell boundaries', () => {
    const report = summarize(
      snapshot([building({ x: 100, z: 50, width: 40, depth: 20, floors: 3 })]),
    );
    const cells = report.cells!.populated;

    expect(cells.map((cell) => [cell.ix, cell.iz])).toEqual([
      [0, 0],
      [1, 0],
    ]);
    expect(cells.map((cell) => cell.total.nominalParcelEnvelopeM2)).toEqual([400, 400]);
    expect(cells.reduce((sum, cell) => sum + cell.total.nominalParcelEnvelopeM2, 0)).toBe(
      report.buildings.total.nominalParcelEnvelopeM2,
    );
    expect(cells.reduce((sum, cell) => sum + cell.total.nominalFloorAreaProxyM2, 0)).toBe(
      report.buildings.total.nominalFloorAreaProxyM2,
    );
    expect(cells[0].areaM2).toBe(10_000);
    expect(cells[0].total.nominalParcelEnvelopeCoverage).toBe(0.04);
  });

  it('clips a rotated parcel into 100m cells and conserves its nominal envelope area', () => {
    const report = summarize(
      snapshot([building({ x: 100, z: 100, width: 40, depth: 20, rotation: Math.PI / 4 })], {
        min_x: 0,
        max_x: 300,
        min_z: 0,
        max_z: 300,
      }),
    );
    const cells = report.cells!.populated;

    expect(cells.map((cell) => [cell.ix, cell.iz])).toEqual([
      [0, 0],
      [0, 1],
      [1, 0],
      [1, 1],
    ]);
    expect(report.buildings.total.nominalParcelEnvelopeM2).toBeCloseTo(800, 8);
    expect(cells.reduce((sum, cell) => sum + cell.total.nominalParcelEnvelopeM2, 0)).toBeCloseTo(
      800,
      8,
    );
    expect(cells.reduce((sum, cell) => sum + cell.total.nominalFloorAreaProxyM2, 0)).toBeCloseTo(
      800,
      8,
    );
    expect(report.method.rotation).toMatch(/Building\.rotation/);

    const area = new Map(
      cells.map((cell) => [`${cell.ix},${cell.iz}`, cell.total.nominalParcelEnvelopeM2]),
    );
    // +π/4 sends the long local X axis toward +X/-Z, so the NW/SE diagonal
    // carries more area than the NE/SW diagonal (Three.js Y rotation).
    expect(area.get('1,0')! + area.get('0,1')!).toBeGreaterThan(
      area.get('0,0')! + area.get('1,1')!,
    );
  });

  it('keeps zero-occupancy commercial uses out of empty-housing counts', () => {
    const report = summarize(
      snapshot([
        building({
          id: 1,
          width: 15,
          depth: 8,
          floors: 2,
          kind: 'cafe',
          stage: 'open',
          occupancy: 0,
          business_active: true,
        }),
        building({
          id: 2,
          x: 150,
          width: 12,
          depth: 9,
          floors: 5,
          kind: 'apartment',
          stage: 'open',
          occupancy: 0,
        }),
        building({
          id: 3,
          x: 50,
          width: 10,
          depth: 8,
          floors: 3,
          kind: 'home',
          stage: 'open',
          occupancy: 4,
        }),
        building({
          id: 4,
          x: 150,
          width: 10,
          depth: 8,
          floors: 4,
          kind: 'home',
          stage: 'renovation',
          occupancy: 0,
        }),
        building({
          id: 5,
          x: 50,
          width: 10,
          depth: 8,
          floors: 2,
          kind: 'office',
          stage: 'foundation',
        }),
        building({
          id: 6,
          x: 150,
          width: 10,
          depth: 8,
          floors: 1,
          kind: 'unknown_kind',
          stage: 'mystery',
        }),
        building({ id: 7, width: -4 }),
        building({ id: 8, floors: Number.NaN }),
      ]),
    );

    expect(report.buildings.valid).toBe(6);
    expect(report.buildings.invalid).toBe(2);
    expect(report.buildings.invalidIds).toEqual([7, 8]);
    expect(report.buildings.byStatus.active_commercial.count).toBe(1);
    expect(report.buildings.byStatus.active_commercial.nominalFloorAreaProxyM2).toBe(240);
    expect(report.buildings.byStatus.empty_housing.count).toBe(1);
    expect(report.buildings.byStatus.empty_housing.nominalFloorAreaProxyM2).toBe(540);
    expect(report.buildings.byStatus.resident_occupied.count).toBe(1);
    expect(report.buildings.byStatus.resident_occupied.nominalFloorAreaProxyM2).toBe(240);
    expect(report.buildings.byStatus.renovation.count).toBe(1);
    expect(report.buildings.byStatus.construction.count).toBe(1);
    expect(report.buildings.byStatus.unknown.count).toBe(1);
    expect(report.buildings.byKind.cafe.count).toBe(1);
    expect(report.buildings.byKind.apartment.count).toBe(1);
  });

  it('uses business_active for active and vacant commercial status, with legacy absence unknown', () => {
    const report = summarize(
      snapshot([
        building({ id: 1, kind: 'shop', occupancy: 0, business_active: true }),
        building({ id: 2, x: 100, kind: 'office', occupancy: 4, business_active: true }),
        building({ id: 3, kind: 'cafe', occupancy: 4, business_active: false }),
        building({ id: 4, x: 100, kind: 'shop', occupancy: 0, business_active: false }),
        building({ id: 5, kind: 'office', occupancy: 0 }),
        building({ id: 6, x: 100, kind: 'cafe', occupancy: 4 }),
      ]),
    );

    expect(report.buildings.byStatus.active_commercial.count).toBe(2);
    expect(report.buildings.byStatus.vacant_commercial.count).toBe(2);
    expect(report.buildings.byStatus.commercial_unknown.count).toBe(2);
    expect(report.buildings.byStatus.empty_housing.count).toBe(0);
    expect(report.buildings.byStatus.resident_occupied.count).toBe(0);
    expect(report.method.commercialActivity).toMatch(/legacy saves/);
  });

  it('marks commercial state explicitly in the map and legend, including legacy unknowns', () => {
    const svg = (renderDensityMap as (snapshot: unknown) => string)(
      snapshot([
        building({ id: 1, kind: 'shop', occupancy: 0, business_active: true }),
        building({ id: 2, x: 100, kind: 'cafe', occupancy: 4, business_active: false }),
        building({ id: 3, kind: 'office', occupancy: 0 }),
      ]),
    );

    expect(svg).toContain('class="building active_commercial"');
    expect(svg).toContain('class="building vacant_commercial"');
    expect(svg).toContain('class="building commercial_unknown"');
    expect(svg).toContain('data-business-active="true"');
    expect(svg).toContain('data-business-active="false"');
    expect(svg).toContain('data-business-active="unknown"');
    expect(svg).toContain('Active commercial');
    expect(svg).toContain('Vacant commercial');
    expect(svg).toContain('Commercial unknown');
    expect(svg).toContain('missing data in legacy saves is unknown');
  });

  it('clips buildings crossing the world edge and allows cell output to be disabled', () => {
    const report = summarize(
      snapshot([building({ x: 0, z: 50, width: 20, depth: 10, floors: 2 })], {
        min_x: 0,
        max_x: 200,
        min_z: 0,
        max_z: 100,
      }),
      { cellSize: null },
    );

    expect(report.buildings.total.nominalParcelEnvelopeM2).toBe(100);
    expect(report.buildings.total.nominalFloorAreaProxyM2).toBe(200);
    expect(report.cells).toBeUndefined();
  });

  it('clips a rotated parcel against the world bounds using its true polygon area', () => {
    const report = summarize(
      snapshot([building({ x: 0, z: 0, width: 2, depth: 2, rotation: Math.PI / 4 })], {
        min_x: 0,
        max_x: 10,
        min_z: 0,
        max_z: 10,
      }),
      { cellSize: null },
    );

    // A centered 2×2 square rotated 45° is a diamond. Its northeast quadrant
    // within the world domain is one triangle with area 1m².
    expect(report.buildings.total.nominalParcelEnvelopeM2).toBeCloseTo(1, 10);
    expect(report.buildings.total.nominalFloorAreaProxyM2).toBeCloseTo(1, 10);
  });

  it('rejects an explicitly non-finite rotation as invalid input', () => {
    const report = summarize(snapshot([building({ id: 44, rotation: Number.NaN })]));

    expect(report.buildings.valid).toBe(0);
    expect(report.buildings.invalid).toBe(1);
    expect(report.buildings.invalidIds).toEqual([44]);
    expect(report.buildings.total.nominalParcelEnvelopeM2).toBe(0);
  });

  it('rejects invalid domains and cell sizes instead of producing misleading ratios', () => {
    expect(() => summarize(snapshot([], { min_x: 2, max_x: 2, min_z: 0, max_z: 10 }))).toThrow(
      /bounds/,
    );
    expect(() => summarize(snapshot([]), { cellSize: 0 })).toThrow(/cellSize/);
  });
});
