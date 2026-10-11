import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, relative, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const DEFAULT_CELL_SIZE = 0.5;
const DEFAULT_BLOCK_SIZE = 100;
const DEFAULT_MAX_GRID_CELLS = 4_000_000;
const PROJECTED_Y_THRESHOLD = 1;

/**
 * Bounded plan-view coverage raster. Coverage is a union of projected kit
 * geometry, so overlapping buildings and facade triangles are counted once.
 */
export function createCoverageGrid(bounds, cellSize = DEFAULT_CELL_SIZE, maxCells = DEFAULT_MAX_GRID_CELLS) {
  if (!bounds || ![bounds.min_x, bounds.max_x, bounds.min_z, bounds.max_z].every(Number.isFinite)
      || bounds.max_x <= bounds.min_x || bounds.max_z <= bounds.min_z) {
    throw new TypeError('Coverage bounds must be finite and increasing');
  }
  if (!Number.isFinite(cellSize) || cellSize <= 0) throw new RangeError('cellSize must be positive');
  const columns = Math.ceil((bounds.max_x - bounds.min_x) / cellSize);
  const rows = Math.ceil((bounds.max_z - bounds.min_z) / cellSize);
  if (!Number.isSafeInteger(maxCells) || maxCells <= 0 || columns * rows > maxCells) {
    throw new RangeError(`Coverage grid exceeds cell cap (${columns} × ${rows} > ${maxCells})`);
  }
  return {
    bounds: { ...bounds }, cellSize, columns, rows, maxCells,
    mask: new Uint8Array(columns * rows),
  };
}

function clipTriangleAboveY(vertices, threshold) {
  const clipped = [];
  for (let i = 0; i < vertices.length; i++) {
    const current = vertices[i], next = vertices[(i + 1) % vertices.length];
    const currentInside = current[1] >= threshold;
    const nextInside = next[1] >= threshold;
    if (currentInside) clipped.push(current);
    if (currentInside !== nextInside) {
      const t = (threshold - current[1]) / (next[1] - current[1]);
      clipped.push([
        current[0] + (next[0] - current[0]) * t,
        threshold,
        current[2] + (next[2] - current[2]) * t,
      ]);
    }
  }
  return clipped;
}

function pointInPolygon(x, z, polygon) {
  let inside = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const [xi, zi] = polygon[i], [xj, zj] = polygon[j];
    if ((zi > z) !== (zj > z) && x < ((xj - xi) * (z - zi)) / (zj - zi) + xi) inside = !inside;
  }
  return inside;
}

/** Rasterize a world-space XZ polygon by cell-center coverage. */
export function markProjectedPolygon(grid, polygon, channel = 'mask') {
  if (!Array.isArray(polygon) || polygon.length < 3) return 0;
  const area2 = polygon.reduce((sum, point, index) => {
    const next = polygon[(index + 1) % polygon.length];
    return sum + point[0] * next[1] - next[0] * point[1];
  }, 0);
  if (!Number.isFinite(area2) || Math.abs(area2) < 1e-9) return 0;
  const mask = channel === 'mask' ? grid.mask : grid[channel];
  if (!(mask instanceof Uint8Array) || mask.length !== grid.mask.length) {
    throw new TypeError(`Unknown coverage channel: ${channel}`);
  }
  const { min_x, max_x, min_z, max_z } = grid.bounds;
  const cell = grid.cellSize;
  const minI = Math.max(0, Math.floor((Math.min(...polygon.map(point => point[0])) - min_x) / cell));
  const maxI = Math.min(grid.columns - 1, Math.floor((Math.max(...polygon.map(point => point[0])) - min_x) / cell));
  const minJ = Math.max(0, Math.floor((Math.min(...polygon.map(point => point[1])) - min_z) / cell));
  const maxJ = Math.min(grid.rows - 1, Math.floor((Math.max(...polygon.map(point => point[1])) - min_z) / cell));
  if (minI > maxI || minJ > maxJ) return 0;
  let added = 0;
  for (let j = minJ; j <= maxJ; j++) {
    const z = Math.min(max_z, min_z + (j + 0.5) * cell);
    for (let i = minI; i <= maxI; i++) {
      const index = j * grid.columns + i;
      if (mask[index]) continue;
      const x = Math.min(max_x, min_x + (i + 0.5) * cell);
      if (pointInPolygon(x, z, polygon)) {
        mask[index] = 1;
        added++;
      }
    }
  }
  return added;
}

/** Clip local kit triangles above 1m, apply saved building placement, and rasterize their projection. */
export function markLocalTriangle(grid, vertices, placement, channel = 'mask', yThreshold = PROJECTED_Y_THRESHOLD) {
  if (!Array.isArray(vertices) || vertices.length !== 3 || !vertices.every(v => v.length >= 3 && v.slice(0, 3).every(Number.isFinite))) return 0;
  const clipped = clipTriangleAboveY(vertices, yThreshold);
  if (clipped.length < 3) return 0;
  const c = Math.cos(placement.rotation ?? 0), s = Math.sin(placement.rotation ?? 0);
  const projected = clipped.map(([x, _y, z]) => [
    (placement.x ?? 0) + c * x + s * z,
    (placement.z ?? 0) - s * x + c * z,
  ]);
  return markProjectedPolygon(grid, projected, channel);
}

function createChannels(bounds, cellSize, maxCells) {
  const grid = createCoverageGrid(bounds, cellSize, maxCells);
  grid.envelopeMask = new Uint8Array(grid.mask.length);
  return grid;
}

function gridArea(grid, mask) {
  const { min_x, max_x, min_z, max_z } = grid.bounds;
  const cell = grid.cellSize;
  let area = 0;
  for (let j = 0; j < grid.rows; j++) {
    const h = Math.max(0, Math.min(cell, max_z - (min_z + j * cell)));
    for (let i = 0; i < grid.columns; i++) {
      if (!mask[j * grid.columns + i]) continue;
      const w = Math.max(0, Math.min(cell, max_x - (min_x + i * cell)));
      area += w * h;
    }
  }
  return area;
}

function coverageCells(grid, blockSize) {
  const blocks = new Map();
  const { min_x, min_z, max_x, max_z } = grid.bounds;
  const cell = grid.cellSize;
  for (let j = 0; j < grid.rows; j++) {
    const z0 = min_z + j * cell;
    const h = Math.max(0, Math.min(cell, max_z - z0));
    for (let i = 0; i < grid.columns; i++) {
      const index = j * grid.columns + i;
      if (!grid.mask[index] && !grid.envelopeMask[index]) continue;
      const x0 = min_x + i * cell;
      const area = Math.max(0, Math.min(cell, max_x - x0)) * h;
      const blockX = Math.floor((x0 + cell / 2 - min_x) / blockSize);
      const blockZ = Math.floor((z0 + cell / 2 - min_z) / blockSize);
      const key = `${blockX},${blockZ}`;
      let entry = blocks.get(key);
      if (!entry) {
        entry = { x: min_x + blockX * blockSize, z: min_z + blockZ * blockSize, projectedKitGeometryM2: 0, nominalParcelEnvelopeM2: 0 };
        blocks.set(key, entry);
      }
      if (grid.mask[index]) entry.projectedKitGeometryM2 += area;
      if (grid.envelopeMask[index]) entry.nominalParcelEnvelopeM2 += area;
    }
  }
  return [...blocks.values()].sort((a, b) => a.z - b.z || a.x - b.x);
}

function markNominalEnvelope(grid, building) {
  const width = building.width, depth = building.depth, rotation = building.rotation ?? 0;
  const c = Math.cos(rotation), s = Math.sin(rotation);
  const points = [[-width / 2, -depth / 2], [width / 2, -depth / 2], [width / 2, depth / 2], [-width / 2, depth / 2]]
    .map(([x, z]) => [building.x + c * x + s * z, building.z - s * x + c * z]);
  markProjectedPolygon(grid, points, 'envelopeMask');
}

function geometryTriangles(mesh, placement, grid) {
  const geometry = mesh.geometry;
  const position = geometry?.getAttribute?.('position');
  if (!position || position.itemSize < 3) return;
  const index = geometry.index;
  const count = index ? index.count : position.count;
  const e = mesh.matrixWorld?.elements;
  if (!e || e.length !== 16) throw new TypeError('Kit mesh needs an updated world matrix');
  const transform = (at) => {
    const x = position.getX(at), y = position.getY(at), z = position.getZ(at);
    return [e[0] * x + e[4] * y + e[8] * z + e[12], e[1] * x + e[5] * y + e[9] * z + e[13], e[2] * x + e[6] * y + e[10] * z + e[14]];
  };
  for (let i = 0; i + 2 < count; i += 3) {
    const ids = index ? [index.getX(i), index.getX(i + 1), index.getX(i + 2)] : [i, i + 1, i + 2];
    const vertices = ids.map(transform);
    const clipped = clipTriangleAboveY(vertices, PROJECTED_Y_THRESHOLD);
    if (clipped.length < 3) continue;
    markProjectedPolygon(grid, clipped.map(([x, _y, z]) => [x, z]));
  }
}

/**
 * Audit a production kit through an injected create/dispose pair. The
 * injection keeps geometric self-tests independent from DOM and WebGPU.
 */
export function auditRenderedCoverage(snapshot, kit, { cellSize = DEFAULT_CELL_SIZE, blockSize = DEFAULT_BLOCK_SIZE, maxGridCells = DEFAULT_MAX_GRID_CELLS, cropBounds = null } = {}) {
  if (!snapshot || !Array.isArray(snapshot.buildings)) throw new TypeError('Snapshot requires a buildings array');
  if (!Number.isFinite(blockSize) || blockSize <= 0) throw new RangeError('blockSize must be positive');
  const worldBounds = snapshot.bounds;
  if (!worldBounds || ![worldBounds.min_x, worldBounds.max_x, worldBounds.min_z, worldBounds.max_z].every(Number.isFinite)
      || worldBounds.max_x <= worldBounds.min_x || worldBounds.max_z <= worldBounds.min_z) {
    throw new TypeError('Snapshot world bounds must be finite and increasing');
  }
  if (cropBounds !== null && (!cropBounds || ![cropBounds.min_x, cropBounds.max_x, cropBounds.min_z, cropBounds.max_z].every(Number.isFinite)
      || cropBounds.max_x <= cropBounds.min_x || cropBounds.max_z <= cropBounds.min_z
      || cropBounds.min_x < worldBounds.min_x || cropBounds.min_z < worldBounds.min_z
      || cropBounds.max_x > worldBounds.max_x || cropBounds.max_z > worldBounds.max_z)) {
    throw new RangeError('Crop bounds must be finite, increasing, and wholly inside snapshot world bounds');
  }
  const bounds = cropBounds ?? worldBounds;
  const grid = createChannels(bounds, cellSize, maxGridCells);
  const originalFootprints = new Map((Array.isArray(snapshot.building_original_footprints)
    ? snapshot.building_original_footprints : []).map(footprint => [footprint.building_id, footprint]));
  let renderedBuildingCount = 0;
  for (const building of snapshot.buildings) {
    if (![building.x, building.z, building.width, building.depth, building.rotation ?? 0].every(Number.isFinite)
        || building.width <= 0 || building.depth <= 0) continue;
    markNominalEnvelope(grid, building);
    const group = kit.create(building, originalFootprints.get(building.id));
    try {
      group.position?.set?.(building.x, 0, building.z);
      group.rotation && (group.rotation.y = building.rotation ?? 0);
      group.updateMatrixWorld?.(true);
      group.traverse?.((object) => {
        if (!object.isMesh) return;
        object.updateMatrixWorld?.(true);
        geometryTriangles(object, null, grid);
      });
      renderedBuildingCount++;
    } finally {
      kit.dispose(group);
    }
  }
  const domainArea = (bounds.max_x - bounds.min_x) * (bounds.max_z - bounds.min_z);
  const projectedArea = gridArea(grid, grid.mask);
  const nominalArea = gridArea(grid, grid.envelopeMask);
  return {
    methodology: {
      label: cropBounds ? 'projected kit geometry above 1m within cropped bounds' : 'projected kit geometry above 1m',
      cellSizeM: cellSize,
      comparison: 'rotated nominal parcel envelopes from saved building width/depth; envelope area is a union too',
      domainDenominator: cropBounds
        ? 'cropped world bounds, including any river, protected land, and undevelopable areas inside the crop'
        : 'full world bounds, including river, protected land, and undevelopable areas',
      yCriterion: 'clip production kit triangles at building-local y=1m before projection',
      limitations: 'Visual plan-view union of authored kit surfaces; includes roof/eaves/props above 1m. It is not a legal footprint, parcel coverage, floor area, or net developable-land ratio. Raster coverage is cell-center sampled and may miss features narrower than the cell size.',
    },
    source: { seed: snapshot.seed, tick: snapshot.tick, engineVersion: snapshot.engine_version },
    cropBounds: cropBounds ? { ...cropBounds } : null,
    grid: { columns: grid.columns, rows: grid.rows, maxCells: grid.maxCells, blockSizeM: blockSize },
    buildings: { snapshotCount: snapshot.buildings.length, renderedCount: renderedBuildingCount },
    domain: { areaM2: domainArea, projectedKitGeometryAreaM2: projectedArea, projectedKitGeometryPercent: projectedArea / domainArea * 100, nominalParcelEnvelopeUnionM2: nominalArea, nominalParcelEnvelopeUnionPercent: nominalArea / domainArea * 100, projectedVsEnvelopePercent: nominalArea > 0 ? projectedArea / nominalArea * 100 : null },
    cells: coverageCells(grid, blockSize),
  };
}

function installCanvasStub() {
  if (globalThis.document) return () => {};
  const makeContext = () => new Proxy({}, {
    get(target, key) {
      if (key === 'measureText') return () => ({ width: 0 });
      if (key === 'getImageData') return () => ({ data: new Uint8ClampedArray(4) });
      if (!(key in target)) target[key] = () => {};
      return target[key];
    },
    set(target, key, value) { target[key] = value; return true; },
  });
  globalThis.document = { createElement: (tag) => tag === 'canvas' ? { width: 1, height: 1, getContext: () => makeContext() } : {} };
  return () => { delete globalThis.document; };
}

async function loadKitRuntime() {
  const tempDir = await mkdtemp(resolve(tmpdir(), 'thecity-rendered-coverage-'));
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
  try {
    const bundleSha256 = createHash('sha256').update(await readFile(outfile)).digest('hex');
    return { module: await import(pathToFileURL(outfile).href), bundleSha256, cleanup: () => rm(tempDir, { recursive: true, force: true }), restoreCanvas };
  } catch (error) {
    restoreCanvas();
    await rm(tempDir, { recursive: true, force: true });
    throw error;
  }
}

async function loadSnapshot(savePath) {
  const saveData = await readFile(savePath, 'utf8');
  const wasmBytes = await readFile(resolve(root, 'public/wasm/city_engine_bg.wasm'));
  const wasmModule = await import(pathToFileURL(resolve(root, 'public/wasm/city_engine.js')).href);
  await wasmModule.default({ module_or_path: wasmBytes });
  const engine = wasmModule.CityEngine.restore(saveData);
  try {
    return { snapshot: JSON.parse(engine.snapshot()), inputSha256: createHash('sha256').update(saveData).digest('hex') };
  } finally {
    engine.free?.();
  }
}

async function auditFile(inputPath, outputPath, inputKind = 'save', compactMidrise = false, cropBounds = null) {
  let loaded;
  if (inputKind === 'snapshot') {
    const data = await readFile(inputPath, 'utf8');
    loaded = { snapshot: JSON.parse(data), inputSha256: createHash('sha256').update(data).digest('hex') };
  } else {
    loaded = await loadSnapshot(inputPath);
  }
  const { snapshot, inputSha256 } = loaded;
  if (!snapshot.bounds) throw new Error('Snapshot has no world bounds');
  const runtime = await loadKitRuntime();
  try {
    const art = new runtime.module.ArtMaterials();
    try {
      const report = auditRenderedCoverage(snapshot, {
        create: (building, original) => runtime.module.buildExtendedBuilding(building, original, art, {compactMidrise}),
        dispose: (group) => runtime.module.disposeGroup(group),
      }, { cropBounds });
      report.source = { ...report.source, [inputKind]: relative(root, inputPath), sha256: inputSha256, kitModule: 'web/src/render/extended-building.ts', kitBundleSha256: runtime.bundleSha256, compactMidriseReview: compactMidrise };
      const output = `${JSON.stringify(report, null, 2)}\n`;
      if (outputPath) await writeFile(outputPath, output);
      else process.stdout.write(output);
      process.stderr.write(`${basename(inputPath)}: ${report.buildings.renderedCount} buildings, ${report.domain.projectedKitGeometryAreaM2.toFixed(0)} m² projected coverage\n`);
    } finally {
      art.dispose();
    }
  } finally {
    runtime.restoreCanvas();
    await runtime.cleanup();
  }
}

async function runCli(argv) {
  let inputPath = null, outputPath = null, inputKind = 'save', compactMidrise = false, cropBounds = null;
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--input' || argv[i] === '--snapshot') {
      if (inputPath) throw new Error('Choose one save input or one snapshot input');
      inputKind = argv[i] === '--snapshot' ? 'snapshot' : 'save';
      if (!argv[i + 1]) throw new Error(`${argv[i]} requires a path`);
      inputPath = resolve(argv[++i]);
    } else if (argv[i] === '--output') {
      if (!argv[i + 1]) throw new Error('--output requires a report path');
      outputPath = resolve(argv[++i]);
    } else if (argv[i] === '--compact-midrise') {
      compactMidrise = true;
    } else if (argv[i] === '--crop') {
      if (!argv[i + 1]) throw new Error('--crop requires minX,minZ,maxX,maxZ');
      const values = argv[++i].split(',').map(Number);
      if (values.length !== 4 || !values.every(Number.isFinite)) throw new Error('--crop requires four finite comma-separated values: minX,minZ,maxX,maxZ');
      cropBounds = { min_x: values[0], min_z: values[1], max_x: values[2], max_z: values[3] };
    } else if (argv[i] === '--help' || argv[i] === '-h') {
      process.stdout.write('Usage: node scripts/audit-rendered-coverage.mjs (--input save.json | --snapshot snapshot.json) [--output report.json] [--compact-midrise] [--crop minX,minZ,maxX,maxZ]\n');
      return;
    } else if (argv[i].startsWith('-')) throw new Error(`Unknown option: ${argv[i]}`);
    else {
      if (inputPath) throw new Error('Choose one save input or one snapshot input');
      inputPath = resolve(argv[i]);
    }
  }
  if (!inputPath) throw new Error('Provide a verified save or snapshot path');
  await auditFile(inputPath, outputPath, inputKind, compactMidrise, cropBounds);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  runCli(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${error instanceof Error ? error.stack ?? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
