import { createHash } from 'node:crypto';
import { build } from 'esbuild';
import { cpus, freemem, platform, release, totalmem } from 'node:os';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';

const root = resolve(import.meta.dirname, '..');
const inputPath = resolve(process.argv[2] ?? 'validation/early-reservation-wasm-87231-snapshot.json');
const outputPath = resolve(process.argv[3] ?? 'validation/staged-building-chunks-benchmark.json');
const cache = resolve(root, 'node_modules/.cache', `thecity-staged-chunks-${process.pid}`);
const inputBytes = await readFile(inputPath);
const world = JSON.parse(inputBytes.toString('utf8'));
if (!Array.isArray(world.buildings) || world.buildings.length !== 850) throw new Error(`Expected the mature 850-building save; got ${world.buildings?.length ?? 'no buildings'}`);
const sha256 = (data) => createHash('sha256').update(data).digest('hex');

globalThis.document = { createElement(name) {
  if (name !== 'canvas') throw new Error(`Unexpected DOM element request: ${name}`);
  return { width: 0, height: 0, getContext: () => ({ fillRect() {}, strokeRect() {}, fillText() {}, beginPath() {}, ellipse() {}, fill() {} }) };
} };

const codeFiles = ['web/src/render/kit.ts', 'web/src/render/building-chunks.ts', 'web/src/render/incremental-geometry-merge.ts', 'web/src/render/materials.ts'];
await rm(cache, { recursive: true, force: true });
await mkdir(cache, { recursive: true });
try {
  await build({ entryPoints: codeFiles.map((file) => resolve(root, file)), outdir: cache, bundle: true, format: 'esm', platform: 'node', packages: 'external', logLevel: 'silent' });
  const THREE = await import('three/webgpu');
  const [{ buildingKit, disposeGroup }, { ArtMaterials }, { BuildingChunks, buildingChunkKey }] = await Promise.all([
    import(pathToFileURL(resolve(cache, 'kit.js')).href),
    import(pathToFileURL(resolve(cache, 'materials.js')).href),
    import(pathToFileURL(resolve(cache, 'building-chunks.js')).href),
  ]);

  const art = new ArtMaterials();
  const chunks = new BuildingChunks((material) => art.buildingBatch(material), (dispose) => dispose(), art.contours);
  const sourceGroups = [];
  const perUpdateMs = [];
  const started = performance.now();
  let generationMs = 0, sourceQueueMs = 0, sourceMeshes = 0, sourceVertices = 0, sourceIndices = 0;
  let maxSingleMeshVertices = 0, oversizedSourceMeshes = 0;
  for (const building of world.buildings) {
    const generateStarted = performance.now();
    const group = buildingKit(building, art);
    generationMs += performance.now() - generateStarted;
    const queueStarted = performance.now();
    group.position.set(building.x, 0, building.z); group.rotation.y = building.rotation;
    group.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return;
      const geometry = object.geometry, positions = geometry.getAttribute('position');
      sourceMeshes++; sourceVertices += positions.count; sourceIndices += geometry.index?.count ?? positions.count;
      maxSingleMeshVertices = Math.max(maxSingleMeshVertices, positions.count);
      if (positions.count > 12_000) oversizedSourceMeshes++;
    });
    sourceGroups.push(group);
    chunks.set(building.id, group);
    sourceQueueMs += performance.now() - queueStarted;
  }

  const updateStarted = performance.now();
  let updateCalls = 0, maxUpdateMs = 0;
  while (chunks.pendingCount > 0) {
    if (updateCalls >= 50_000) throw new Error('Staged chunk update failed to drain after 50000 calls');
    const frameStart = performance.now();
    chunks.update();
    const elapsed = performance.now() - frameStart;
    perUpdateMs.push(elapsed); maxUpdateMs = Math.max(maxUpdateMs, elapsed); updateCalls++;
  }
  const chunkUpdateMs = performance.now() - updateStarted;
  const totalMeasuredMs = performance.now() - started;
  const outputs = [];
  chunks.root.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    outputs.push({ vertices: object.geometry.getAttribute('position').count, indices: object.geometry.index?.count ?? 0 });
  });
  const ordered = [...perUpdateMs].sort((a, b) => a - b);
  const quantile = (p) => ordered[Math.min(ordered.length - 1, Math.ceil(ordered.length * p) - 1)] ?? 0;
  const report = {
    title: 'Staged production BuildingChunks update timing',
    measuredAt: new Date().toISOString(),
    input: { file: inputPath.replace(`${root}/`, ''), sha256: sha256(inputBytes), bytes: inputBytes.byteLength, seed: world.seed, tick: world.tick, buildings: world.buildings.length },
    code: Object.fromEntries(await Promise.all(codeFiles.map(async (file) => [file, sha256(await readFile(resolve(root, file)))]))),
    machine: { platform: `${platform()} ${release()}`, architecture: process.arch, cpuModel: cpus()[0]?.model ?? 'unknown', logicalCpus: cpus().length, totalRamBytes: totalmem(), freeRamBeforeRunBytes: freemem(), node: process.version, note: 'Node CPU-only wall-clock measurements; other host activity was not isolated. No browser/GPU timing or renderer frame pacing.' },
    workload: { chunkSizeMeters: 200, uniqueChunks: new Set(world.buildings.map((building) => buildingChunkKey(building.x, building.z))).size, sourceBuildings: world.buildings.length, sourceMeshes, sourceVertices, sourceIndices, batchMeshes: outputs.length, batchVertices: outputs.reduce((sum, output) => sum + output.vertices, 0), batchIndices: outputs.reduce((sum, output) => sum + output.indices, 0), inputQueue: 'All 850 buildingKit groups were generated and passed to BuildingChunks.set() before the first update().' },
    timingMs: {
      buildingKitGeneration: generationMs,
      sourceQueueIncludingOutlines: sourceQueueMs,
      chunkUpdateDrainWallTime: chunkUpdateMs,
      endToEnd: totalMeasuredMs,
      perUpdate: { calls: updateCalls, mean: perUpdateMs.reduce((sum, value) => sum + value, 0) / Math.max(1, perUpdateMs.length), p50: quantile(.5), p95: quantile(.95), max: maxUpdateMs },
    },
    preparationLimit: { preparedMeshCapPerUpdate: 16, preparedVertexQuotaPerUpdate: 12_000, preparedIndexQuotaPerUpdate: 36_000, oneOversizedMeshMayBeSynchronous: false, maxSingleAuthoredMeshVertices: maxSingleMeshVertices, authoredMeshesExceedingVertexQuota: oversizedSourceMeshes },
    interpretation: [
      'buildingKitGeneration is measured separately from sourceQueueIncludingOutlines; the latter includes group traversal and exact outline extraction performed synchronously by BuildingChunks.set().',
      'chunkUpdateDrainWallTime and the perUpdate distribution measure only calls to staged BuildingChunks.update() after the full fixture has been queued. These do not include renderer-side GPU upload.',
      'Per-update work is bounded by mesh/vertex preparation caps, merger vertex/index quotas and a contour-copy quota. Oversized authored meshes continue across updates under the same quotas; output array allocation and source outline extraction remain synchronous.',
      'Per-update p95/max are samples from one complete fixture drain, not repeated independent runs or a frame-rate estimate. This audit does not measure memory or rendering quality.',
    ],
  };

  await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify({ outputPath, input: report.input, workload: report.workload, timingMs: report.timingMs, preparationLimit: report.preparationLimit, machine: report.machine }, null, 2)}\n`);
  chunks.clear(); for (const group of sourceGroups) disposeGroup(group); art.dispose();
} finally { await rm(cache, { recursive: true, force: true }); }
