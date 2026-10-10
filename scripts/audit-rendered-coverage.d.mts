export interface CoverageGrid {
  bounds: { min_x: number; max_x: number; min_z: number; max_z: number };
  cellSize: number;
  columns: number;
  rows: number;
  mask: Uint8Array;
  envelopeMask?: Uint8Array;
}

export interface CoverageSnapshot {
  seed?: number;
  tick?: number;
  engine_version?: string;
  bounds: CoverageGrid['bounds'];
  buildings: Array<{
    x: number;
    z: number;
    width: number;
    depth: number;
    rotation?: number;
    [key: string]: unknown;
  }>;
}

export function createCoverageGrid(
  bounds: CoverageGrid['bounds'],
  cellSize?: number,
  maxCells?: number,
): CoverageGrid;
export function markProjectedPolygon(grid: CoverageGrid, polygon: number[][], channel?: string): number;
export function markLocalTriangle(
  grid: CoverageGrid,
  vertices: number[][],
  placement: { x?: number; z?: number; rotation?: number },
  channel?: string,
  yThreshold?: number,
): number;
export function auditRenderedCoverage(
  snapshot: CoverageSnapshot,
  kit: { create(building: CoverageSnapshot['buildings'][number]): unknown; dispose(group: unknown): void },
  options?: { cellSize?: number; blockSize?: number; maxGridCells?: number },
): {
  methodology: Record<string, string | number>;
  source: { seed?: number; tick?: number; engineVersion?: string };
  grid: { columns: number; rows: number; maxCells: number; blockSizeM: number };
  buildings: { snapshotCount: number; renderedCount: number };
  domain: {
    areaM2: number;
    projectedKitGeometryAreaM2: number;
    projectedKitGeometryPercent: number;
    nominalParcelEnvelopeUnionM2: number;
    nominalParcelEnvelopeUnionPercent: number;
    projectedVsEnvelopePercent: number | null;
  };
  cells: Array<{ x: number; z: number; projectedKitGeometryM2: number; nominalParcelEnvelopeM2: number }>;
};
