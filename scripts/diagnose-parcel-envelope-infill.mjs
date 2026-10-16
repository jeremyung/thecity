import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createCoverageGrid, markProjectedPolygon } from './audit-rendered-coverage.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const CELL = 0.25;
const Y_THRESHOLD = 1;
const FIRST_FACE_BLOCKERS = [
  2608, 2614, 2627, 2730, 2762, 2763, 2765, 2767, 2768, 2770, 2812, 2825, 2827,
];

function installCanvasStub() {
  if (globalThis.document) return () => {};
  const context = () =>
    new Proxy(
      {},
      {
        get(target, key) {
          if (key === 'measureText') return () => ({ width: 0 });
          if (!(key in target)) target[key] = () => {};
          return target[key];
        },
        set(target, key, value) {
          target[key] = value;
          return true;
        },
      },
    );
  globalThis.document = {
    createElement: (tag) =>
      tag === 'canvas' ? { width: 1, height: 1, getContext: () => context() } : {},
  };
  return () => {
    delete globalThis.document;
  };
}

function clipAboveY(vertices, threshold) {
  const clipped = [];
  for (let i = 0; i < vertices.length; i++) {
    const a = vertices[i],
      b = vertices[(i + 1) % vertices.length];
    const ain = a[1] >= threshold,
      bin = b[1] >= threshold;
    if (ain) clipped.push(a);
    if (ain !== bin) {
      const t = (threshold - a[1]) / (b[1] - a[1]);
      clipped.push([a[0] + (b[0] - a[0]) * t, threshold, a[2] + (b[2] - a[2]) * t]);
    }
  }
  return clipped;
}

function pointInPolygon(point, polygon) {
  let inside = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const [xi, zi] = polygon[i],
      [xj, zj] = polygon[j];
    if (
      zi > point[1] !== zj > point[1] &&
      point[0] < ((xj - xi) * (point[1] - zi)) / (zj - zi) + xi
    )
      inside = !inside;
  }
  return inside;
}

function subdivisionOpportunity(grid, parcel, parcelCorners) {
  const parent = new Uint8Array(grid.mask.length);
  for (let j = 0; j < grid.rows; j++) {
    const z = grid.bounds.min_z + (j + 0.5) * CELL;
    for (let i = 0; i < grid.columns; i++) {
      const x = grid.bounds.min_x + (i + 0.5) * CELL;
      if (pointInPolygon([x, z], parcelCorners)) parent[j * grid.columns + i] = 1;
    }
  }
  const radius = Math.ceil(2.2 / CELL);
  const blocked = new Uint8Array(grid.mask.length);
  for (let j = 0; j < grid.rows; j++)
    for (let i = 0; i < grid.columns; i++) {
      if (!grid.mask[j * grid.columns + i]) continue;
      for (let dz = -radius; dz <= radius; dz++)
        for (let dx = -radius; dx <= radius; dx++) {
          if (dx * dx + dz * dz > radius * radius) continue;
          const x = i + dx,
            z = j + dz;
          if (x >= 0 && x < grid.columns && z >= 0 && z < grid.rows)
            blocked[z * grid.columns + x] = 1;
        }
    }
  const prefix = new Uint32Array((grid.rows + 1) * (grid.columns + 1));
  for (let j = 0; j < grid.rows; j++)
    for (let i = 0; i < grid.columns; i++) {
      const empty = parent[j * grid.columns + i] && !blocked[j * grid.columns + i] ? 1 : 0;
      const p = (j + 1) * (grid.columns + 1) + i + 1;
      prefix[p] =
        empty + prefix[p - 1] + prefix[p - grid.columns - 1] - prefix[p - grid.columns - 2];
    }
  const count = (x, z, width, depth) =>
    prefix[(z + depth) * (grid.columns + 1) + x + width] -
    prefix[z * (grid.columns + 1) + x + width] -
    prefix[(z + depth) * (grid.columns + 1) + x] +
    prefix[z * (grid.columns + 1) + x];
  const dimensions = [
    [Math.ceil(8 / CELL), Math.ceil(12 / CELL)],
    [Math.ceil(12 / CELL), Math.ceil(8 / CELL)],
  ];
  const placements = [];
  for (const [width, depth] of dimensions) {
    for (let z = 0; z + depth <= grid.rows; z++)
      for (let x = 0; x + width <= grid.columns; x++) {
        if (count(x, z, width, depth) === width * depth) {
          placements.push({
            x: Number((grid.bounds.min_x + (x + width / 2) * CELL).toFixed(2)),
            z: Number((grid.bounds.min_z + (z + depth / 2) * CELL).toFixed(2)),
            width_m: Number((width * CELL).toFixed(2)),
            depth_m: Number((depth * CELL).toFixed(2)),
          });
          if (placements.length >= 8)
            return {
              minimum_8x12_rectangles_after_2_2m_buffer: placements,
              search:
                'axis-aligned raster placements inside original envelope; no road/frontage path check',
            };
        }
      }
  }
  return {
    minimum_8x12_rectangles_after_2_2m_buffer: placements,
    search: 'axis-aligned raster placements inside original envelope; no road/frontage path check',
  };
}

function buildingGeometry(building, parcel, kitModule, art) {
  const group = kitModule.buildingKit(building, art);
  group.position.set(building.x, 0, building.z);
  group.rotation.y = building.rotation ?? 0;
  group.updateMatrixWorld(true);
  const rotation = parcel.rotation ?? 0;
  const c = Math.cos(rotation),
    s = Math.sin(rotation);
  const corners = [
    [-parcel.width / 2, -parcel.depth / 2],
    [parcel.width / 2, -parcel.depth / 2],
    [parcel.width / 2, parcel.depth / 2],
    [-parcel.width / 2, parcel.depth / 2],
  ].map(([x, z]) => [parcel.x + c * x + s * z, parcel.z - s * x + c * z]);
  const grid = createCoverageGrid(
    {
      min_x: Math.min(...corners.map(([x]) => x)),
      max_x: Math.max(...corners.map(([x]) => x)),
      min_z: Math.min(...corners.map(([, z]) => z)),
      max_z: Math.max(...corners.map(([, z]) => z)),
    },
    CELL,
    100_000,
  );
  let minX = Infinity,
    maxX = -Infinity,
    minZ = Infinity,
    maxZ = -Infinity;
  group.traverse((object) => {
    if (!object.isMesh) return;
    object.updateMatrixWorld(true);
    const position = object.geometry?.getAttribute?.('position');
    if (!position) return;
    const index = object.geometry.index;
    const count = index ? index.count : position.count;
    const e = object.matrixWorld.elements;
    const transform = (at) => {
      const x = position.getX(at),
        y = position.getY(at),
        z = position.getZ(at);
      return [
        e[0] * x + e[4] * y + e[8] * z + e[12],
        e[1] * x + e[5] * y + e[9] * z + e[13],
        e[2] * x + e[6] * y + e[10] * z + e[14],
      ];
    };
    for (let i = 0; i + 2 < count; i += 3) {
      const ids = index ? [index.getX(i), index.getX(i + 1), index.getX(i + 2)] : [i, i + 1, i + 2];
      const world = ids.map(transform);
      const clipped = clipAboveY(world, Y_THRESHOLD);
      for (const [x, , z] of clipped) {
        minX = Math.min(minX, x);
        maxX = Math.max(maxX, x);
        minZ = Math.min(minZ, z);
        maxZ = Math.max(maxZ, z);
      }
      if (clipped.length >= 3)
        markProjectedPolygon(
          grid,
          clipped.map(([x, , z]) => [x, z]),
        );
    }
  });
  const projectedCellsInsideParcel = grid.mask.reduce((sum, value) => sum + value, 0);
  const parcelArea = parcel.width * parcel.depth;
  const result = {
    projected_kit_bbox_above_1m: [minX, minZ, maxX, maxZ].map((value) =>
      Number.isFinite(value) ? Number(value.toFixed(3)) : null,
    ),
    projected_kit_bbox_area_m2: Number(((maxX - minX) * (maxZ - minZ)).toFixed(2)),
    projected_kit_area_inside_parent_parcel_m2_cell_estimate: Number(
      (projectedCellsInsideParcel * CELL * CELL).toFixed(2),
    ),
    parent_parcel_area_m2: Number(parcelArea.toFixed(2)),
    uncovered_parent_area_m2_cell_estimate: Number(
      (parcelArea - projectedCellsInsideParcel * CELL * CELL).toFixed(2),
    ),
    projected_vs_parent_area_percent: Number(
      (((projectedCellsInsideParcel * CELL * CELL) / parcelArea) * 100).toFixed(1),
    ),
    subdivision_probe: subdivisionOpportunity(grid, parcel, corners),
    y_threshold_m: Y_THRESHOLD,
    raster_cell_m: CELL,
    caveat:
      'Production buildingKit meshes projected above 1m, rasterized at 0.25m. Includes eaves, facade details and small site objects; it is not a legal building footprint or proof that the uncovered cells form a usable lot.',
  };
  kitModule.disposeGroup(group);
  return result;
}

function roadSummary(save, parcel, frontage) {
  const road = save.roads.find((item) => item.id === frontage?.road_id);
  if (!road) return null;
  const byId = new Map(save.nodes.map((node) => [node.id, node]));
  const a = byId.get(road.from),
    b = byId.get(road.to);
  const connectedRoads = save.roads.filter(
    (item) =>
      item.stage === 'open' &&
      item.connected &&
      (item.from === parcel.access_node || item.to === parcel.access_node),
  );
  return {
    frontage_road_id: road.id,
    road_kind: road.kind,
    road_stage: road.stage,
    road_connected: road.connected,
    road_endpoints: [a, b].map((node) => (node ? { id: node.id, x: node.x, z: node.z } : null)),
    frontage_along: frontage.along,
    frontage_side: frontage.side,
    parcel_access_node: parcel.access_node,
    access_node_connected: save.roads.some(
      (item) =>
        item.stage === 'open' &&
        item.connected &&
        (item.from === parcel.access_node || item.to === parcel.access_node),
    ),
    connected_open_roads_at_access_node: connectedRoads.map((item) => item.id),
  };
}

async function main() {
  const inputPath = resolve(process.argv[2] ?? 'validation/early-reservation-wasm-87231-save.json');
  const outputPath = resolve(
    process.argv[3] ?? 'validation/parcel-envelope-infill-feasibility.json',
  );
  const saveText = await readFile(inputPath, 'utf8');
  const save = JSON.parse(saveText);
  const snapshotPath = inputPath.replace(/-save\.json$/, '-snapshot.json');
  const snapshot = JSON.parse(await readFile(snapshotPath, 'utf8'));
  const wanted = new Set([707, 68, ...FIRST_FACE_BLOCKERS]);
  const buildingsByParcel = new Map(
    save.buildings.map((building) => [building.parcel_id, building]),
  );
  const frontagesByParcel = new Map(
    save.parcel_frontages.map((frontage) => [frontage.parcel_id, frontage]),
  );
  const parcels = save.parcels.filter((parcel) => wanted.has(parcel.id));
  const tempDir = await mkdtemp(resolve(tmpdir(), 'thecity-parcel-infill-'));
  const outfile = resolve(tempDir, 'kit-bundle.mjs');
  const esbuild = await import('esbuild');
  await esbuild.build({
    entryPoints: [resolve(root, 'scripts/rendered-coverage-runtime.ts')],
    bundle: true,
    platform: 'node',
    format: 'esm',
    target: 'node22',
    outfile,
    logLevel: 'silent',
  });
  const restoreCanvas = installCanvasStub();
  const runtime = await import(pathToFileURL(outfile).href);
  const art = new runtime.ArtMaterials();
  const rows = [];
  try {
    for (const parcel of parcels) {
      const building = buildingsByParcel.get(parcel.id);
      const frontage = frontagesByParcel.get(parcel.id);
      const snapshotBuilding = snapshot.buildings.find((item) => item.id === building?.id);
      rows.push({
        parcel_id: parcel.id,
        source_group:
          parcel.id === 707
            ? 'west-north named blocker'
            : parcel.id === 68
              ? 'west-central named blocker'
              : 'east-north face candidate blocker',
        parcel: {
          x: parcel.x,
          z: parcel.z,
          width: parcel.width,
          depth: parcel.depth,
          stored_rotation: parcel.rotation ?? null,
          legal_envelope_area_m2: parcel.width * parcel.depth,
        },
        building: building
          ? {
              id: building.id,
              kind: building.kind,
              floors: building.floors,
              stage: building.stage,
              rotation: building.rotation,
              width: building.width,
              depth: building.depth,
              capacity: building.capacity,
              occupancy: building.occupancy,
              business_active: building.business_active,
            }
          : null,
        actual_render_snapshot_building: snapshotBuilding
          ? {
              id: snapshotBuilding.id,
              kind: snapshotBuilding.kind,
              floors: snapshotBuilding.floors,
              width: snapshotBuilding.width,
              depth: snapshotBuilding.depth,
              rotation: snapshotBuilding.rotation,
            }
          : null,
        rendered_kit_measurement: building
          ? buildingGeometry(building, parcel, runtime, art)
          : null,
        access: roadSummary(save, parcel, frontage),
      });
    }
  } finally {
    art.dispose();
    restoreCanvas();
    await rm(tempDir, { recursive: true, force: true });
  }
  const output = {
    source: {
      save: inputPath,
      save_sha256: createHash('sha256').update(saveText).digest('hex'),
      snapshot: snapshotPath,
      seed: save.seed,
      tick: save.tick,
      kit_module: 'web/src/render/kit.ts',
    },
    methodology:
      'Selected existing parcels that blocked production-generated candidates in the three prior face diagnostics. Rendered kit union above 1m is rasterized inside the stored parent envelope using the same production buildingKit bundle. Parent envelopes are legal collision reservations, not remaining empty buildable land. Kit projection includes detail/eaves/site objects and is not a legal footprint.',
    rows,
  };
  await writeFile(outputPath, `${JSON.stringify(output, null, 2)}\n`);
  process.stdout.write(`${basename(outputPath)}: measured ${rows.length} blockers\n`);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
