import { readFile, writeFile } from 'node:fs/promises';
import { resolve, dirname, relative } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const DEFAULT_CELL_SIZE = 100;
const DENOMINATOR = 'gross area of world.bounds, including water and protected land';
const CONSTRUCTION_STAGES = new Set([
  'foundation',
  'survey',
  'clearing',
  'structure',
  'surface',
  'finishing',
]);
const STATUS_NAMES = [
  'resident_occupied',
  'empty_housing',
  'active_commercial',
  'vacant_commercial',
  'commercial_unknown',
  'renovation',
  'construction',
  'unknown',
];
const HOUSING_KINDS = new Set(['home', 'apartment']);
const COMMERCIAL_KINDS = new Set(['shop', 'cafe', 'office']);

/**
 * Summarize rotated nominal parcel-envelope proxies and proxy FAR. Building
 * width/depth are parcel dimensions: the renderer sets structures back from
 * them and uses profile-specific massing. These are not rendered-footprint or
 * usable-floor-area measurements.
 */
export function summarizeDensity(snapshot, options = {}) {
  const bounds = snapshot?.bounds;
  if (
    !bounds ||
    ![bounds.min_x, bounds.max_x, bounds.min_z, bounds.max_z].every(Number.isFinite) ||
    bounds.max_x <= bounds.min_x ||
    bounds.max_z <= bounds.min_z
  ) {
    throw new TypeError('World snapshot requires finite, increasing bounds');
  }
  const cellSize = options.cellSize === undefined ? DEFAULT_CELL_SIZE : options.cellSize;
  if (cellSize !== null && (!Number.isFinite(cellSize) || cellSize <= 0)) {
    throw new TypeError('cellSize must be a positive finite number or null');
  }
  const domainAreaM2 = (bounds.max_x - bounds.min_x) * (bounds.max_z - bounds.min_z);
  const total = accumulator();
  const byStatus = Object.fromEntries(STATUS_NAMES.map((status) => [status, accumulator()]));
  const byKind = new Map();
  const cells = new Map();
  const invalidBuildingIds = [];
  let validBuildings = 0;

  for (const building of Array.isArray(snapshot.buildings) ? snapshot.buildings : []) {
    const { id, x, z, width, depth, floors } = building ?? {};
    const rotation = building?.rotation === undefined ? 0 : building.rotation;
    if (
      ![x, z, width, depth, floors, rotation].every(Number.isFinite) ||
      width <= 0 ||
      depth <= 0 ||
      floors <= 0
    ) {
      invalidBuildingIds.push(Number.isFinite(id) ? id : null);
      continue;
    }
    const envelope = clipPolygonToRect(orientedRectangle(x, z, width, depth, rotation), bounds);
    const envelopeM2 = polygonArea(envelope);
    if (!(envelopeM2 > 1e-8)) continue;
    const floorProxyM2 = envelopeM2 * floors;
    const status = classifyStatus(building);
    const kind =
      typeof building.kind === 'string' && building.kind.length > 0 ? building.kind : 'unknown';
    validBuildings++;
    add(total, envelopeM2, floorProxyM2);
    add(byStatus[status], envelopeM2, floorProxyM2);
    let kindStats = byKind.get(kind);
    if (!kindStats) byKind.set(kind, (kindStats = accumulator()));
    add(kindStats, envelopeM2, floorProxyM2);

    if (cellSize !== null) {
      const envelopeBounds = polygonBounds(envelope);
      const firstCellX = Math.floor(envelopeBounds.min_x / cellSize);
      const lastCellX = Math.ceil(envelopeBounds.max_x / cellSize) - 1;
      const firstCellZ = Math.floor(envelopeBounds.min_z / cellSize);
      const lastCellZ = Math.ceil(envelopeBounds.max_z / cellSize) - 1;
      for (let ix = firstCellX; ix <= lastCellX; ix++) {
        const cellMinX = Math.max(bounds.min_x, ix * cellSize);
        const cellMaxX = Math.min(bounds.max_x, (ix + 1) * cellSize);
        if (cellMaxX <= cellMinX) continue;
        for (let iz = firstCellZ; iz <= lastCellZ; iz++) {
          const cellMinZ = Math.max(bounds.min_z, iz * cellSize);
          const cellMaxZ = Math.min(bounds.max_z, (iz + 1) * cellSize);
          if (cellMaxZ <= cellMinZ) continue;
          const area = polygonArea(
            clipPolygonToRect(envelope, {
              min_x: cellMinX,
              max_x: cellMaxX,
              min_z: cellMinZ,
              max_z: cellMaxZ,
            }),
          );
          if (!(area > 1e-8)) continue;
          const key = `${ix},${iz}`;
          let cell = cells.get(key);
          if (!cell) {
            cell = {
              ix,
              iz,
              bounds: { min_x: cellMinX, max_x: cellMaxX, min_z: cellMinZ, max_z: cellMaxZ },
              areaM2: (cellMaxX - cellMinX) * (cellMaxZ - cellMinZ),
              total: accumulator(),
              byStatus: createStatusAccumulators(),
              byKind: new Map(),
            };
            cells.set(key, cell);
          }
          const cellFloorArea = area * floors;
          add(cell.total, area, cellFloorArea);
          add(cell.byStatus[status], area, cellFloorArea);
          let cellKind = cell.byKind.get(kind);
          if (!cellKind) cell.byKind.set(kind, (cellKind = accumulator()));
          add(cellKind, area, cellFloorArea);
        }
      }
    }
  }

  return {
    metrics: {
      area: 'nominalParcelEnvelopeM2',
      floorArea: 'nominalFloorAreaProxyM2',
      coverage: 'nominalParcelEnvelopeCoverage',
      grossFar: 'grossFloorAreaRatioProxy',
    },
    method: {
      buildingDimensions:
        'Building.width × depth are parcel-envelope dimensions. The renderer sets structures back and uses kind/profile-specific massing.',
      interpretation:
        'Coverage and FAR are nominal parcel-envelope proxies, not actual rendered building footprint or usable floor area.',
      aggregation:
        'Areas sum clipped nominal envelopes; overlapping envelopes are counted separately rather than unioned.',
      rotation:
        'Nominal parcel rectangles use Building.rotation (Three.js Y convention); missing rotation defaults to zero. Cell and world clipping use the rotated polygon.',
      occupancy:
        'Building.occupancy counts residents in housing; it does not determine commercial activity.',
      commercialActivity:
        'Open commercial buildings use business_active: true means active, false means vacant, and a missing value (as in legacy saves) means unknown.',
    },
    denominator: DENOMINATOR,
    domain: { ...bounds, areaM2: domainAreaM2 },
    buildings: {
      valid: validBuildings,
      invalid: invalidBuildingIds.length,
      invalidIds: invalidBuildingIds,
      total: measures(total, domainAreaM2),
      byStatus: Object.fromEntries(
        Object.entries(byStatus).map(([key, value]) => [key, measures(value, domainAreaM2)]),
      ),
      byKind: Object.fromEntries(
        [...byKind.entries()]
          .sort(([a], [b]) => a.localeCompare(b))
          .map(([key, value]) => [key, measures(value, domainAreaM2)]),
      ),
    },
    ...(cellSize === null
      ? {}
      : {
          cells: {
            sizeM: cellSize,
            origin: { x: 0, z: 0 },
            populated: [...cells.values()]
              .sort((a, b) => a.ix - b.ix || a.iz - b.iz)
              .map((cell) => ({
                ix: cell.ix,
                iz: cell.iz,
                bounds: cell.bounds,
                areaM2: cell.areaM2,
                total: measures(cell.total, cell.areaM2),
                byStatus: Object.fromEntries(
                  Object.entries(cell.byStatus).map(([key, value]) => [
                    key,
                    measures(value, cell.areaM2),
                  ]),
                ),
                byKind: Object.fromEntries(
                  [...cell.byKind.entries()]
                    .sort(([a], [b]) => a.localeCompare(b))
                    .map(([key, value]) => [key, measures(value, cell.areaM2)]),
                ),
              })),
          },
        }),
  };
}

function orientedRectangle(x, z, width, depth, rotation) {
  const c = Math.cos(rotation),
    s = Math.sin(rotation);
  return [
    { x: x - (width * c) / 2 - (depth * s) / 2, z: z + (width * s) / 2 - (depth * c) / 2 },
    { x: x + (width * c) / 2 - (depth * s) / 2, z: z - (width * s) / 2 - (depth * c) / 2 },
    { x: x + (width * c) / 2 + (depth * s) / 2, z: z - (width * s) / 2 + (depth * c) / 2 },
    { x: x - (width * c) / 2 + (depth * s) / 2, z: z + (width * s) / 2 + (depth * c) / 2 },
  ];
}

function clipPolygonToRect(polygon, rect) {
  let clipped = polygon;
  const edges = [
    { axis: 'x', value: rect.min_x, keepGreater: true },
    { axis: 'x', value: rect.max_x, keepGreater: false },
    { axis: 'z', value: rect.min_z, keepGreater: true },
    { axis: 'z', value: rect.max_z, keepGreater: false },
  ];
  for (const edge of edges) {
    if (clipped.length === 0) break;
    const output = [];
    let previous = clipped[clipped.length - 1];
    let previousInside = edge.keepGreater
      ? previous[edge.axis] >= edge.value
      : previous[edge.axis] <= edge.value;
    for (const current of clipped) {
      const currentInside = edge.keepGreater
        ? current[edge.axis] >= edge.value
        : current[edge.axis] <= edge.value;
      if (currentInside !== previousInside) {
        const delta = current[edge.axis] - previous[edge.axis];
        const t = (edge.value - previous[edge.axis]) / delta;
        output.push({
          x: previous.x + (current.x - previous.x) * t,
          z: previous.z + (current.z - previous.z) * t,
        });
      }
      if (currentInside) output.push(current);
      previous = current;
      previousInside = currentInside;
    }
    clipped = output;
  }
  return clipped;
}

function polygonArea(polygon) {
  if (polygon.length < 3) return 0;
  let twiceArea = 0;
  for (let i = 0; i < polygon.length; i += 1) {
    const current = polygon[i],
      next = polygon[(i + 1) % polygon.length];
    twiceArea += current.x * next.z - next.x * current.z;
  }
  return Math.abs(twiceArea) / 2;
}

function polygonBounds(polygon) {
  return polygon.reduce(
    (result, point) => ({
      min_x: Math.min(result.min_x, point.x),
      max_x: Math.max(result.max_x, point.x),
      min_z: Math.min(result.min_z, point.z),
      max_z: Math.max(result.max_z, point.z),
    }),
    { min_x: Infinity, max_x: -Infinity, min_z: Infinity, max_z: -Infinity },
  );
}

function accumulator() {
  return { count: 0, envelopeM2: 0, floorProxyM2: 0 };
}

function createStatusAccumulators() {
  return Object.fromEntries(STATUS_NAMES.map((status) => [status, accumulator()]));
}

function classifyStatus(building) {
  if (building.stage === 'renovation') return 'renovation';
  if (CONSTRUCTION_STAGES.has(building.stage)) return 'construction';
  if (building.stage !== 'open') return 'unknown';
  if (COMMERCIAL_KINDS.has(building.kind)) {
    if (building.business_active === true) return 'active_commercial';
    if (building.business_active === false) return 'vacant_commercial';
    return 'commercial_unknown';
  }
  if (
    !HOUSING_KINDS.has(building.kind) ||
    !Number.isFinite(building.occupancy) ||
    building.occupancy < 0
  )
    return 'unknown';
  return building.occupancy === 0 ? 'empty_housing' : 'resident_occupied';
}

function add(target, envelopeM2, floorProxyM2) {
  target.count++;
  target.envelopeM2 += envelopeM2;
  target.floorProxyM2 += floorProxyM2;
}

function measures(value, denominatorM2) {
  return {
    count: value.count,
    nominalParcelEnvelopeM2: value.envelopeM2,
    nominalFloorAreaProxyM2: value.floorProxyM2,
    nominalParcelEnvelopeCoverage: value.envelopeM2 / denominatorM2,
    grossFloorAreaRatioProxy: value.floorProxyM2 / denominatorM2,
  };
}

async function runCli(argv) {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  let inputPath = resolve(root, 'validation/mature-world.json');
  let outputPath = null;
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--output') {
      if (!argv[i + 1]) throw new Error('--output requires a path');
      outputPath = resolve(argv[++i]);
    } else if (argv[i] === '--help' || argv[i] === '-h') {
      process.stdout.write(
        'Usage: node scripts/audit/audit-density.mjs [save.json] [--output report.json]\n',
      );
      return;
    } else if (argv[i].startsWith('-')) {
      throw new Error(`Unknown option: ${argv[i]}`);
    } else {
      inputPath = resolve(argv[i]);
    }
  }

  const saveData = await readFile(inputPath, 'utf8');
  const wasmBytes = await readFile(resolve(root, 'public/wasm/city_engine_bg.wasm'));
  const wasmModule = await import(pathToFileURL(resolve(root, 'public/wasm/city_engine.js')).href);
  await wasmModule.default({ module_or_path: wasmBytes });
  const engine = wasmModule.CityEngine.restore(saveData);
  try {
    const snapshot = JSON.parse(engine.snapshot());
    const report = {
      source: {
        save: relative(root, inputPath),
        seed: snapshot.seed,
        tick: snapshot.tick,
        engineVersion: snapshot.engine_version,
      },
      ...summarizeDensity(snapshot),
    };
    const output = `${JSON.stringify(report, null, 2)}\n`;
    if (outputPath) await writeFile(outputPath, output);
    else process.stdout.write(output);
  } finally {
    engine.free?.();
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  runCli(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
