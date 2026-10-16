import { createHash } from 'node:crypto';
import { build } from 'esbuild';
import { cpus, freemem, platform, release, totalmem } from 'node:os';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';

const root = resolve(import.meta.dirname, '..');
const inputPath = resolve(
  process.argv[2] ?? 'validation/early-reservation-wasm-87231-snapshot.json',
);
const outputPath = resolve(process.argv[3] ?? 'validation/building-regeneration-benchmark.json');
const cache = resolve(root, 'node_modules/.cache', `thecity-building-regeneration-${process.pid}`);
const inputBytes = await readFile(inputPath);
const world = JSON.parse(inputBytes.toString('utf8'));
if (!Array.isArray(world.buildings) || world.buildings.length !== 850) {
  throw new Error(
    `Expected the current 850-building snapshot; found ${world.buildings?.length ?? 'no building list'}`,
  );
}
const sha256 = (data) => createHash('sha256').update(data).digest('hex');

function installCanvasStub() {
  globalThis.document = {
    createElement(name) {
      if (name !== 'canvas') throw new Error(`Unexpected DOM element request: ${name}`);
      return {
        width: 0,
        height: 0,
        getContext: () => ({
          fillRect() {},
          strokeRect() {},
          fillText() {},
          beginPath() {},
          ellipse() {},
          fill() {},
        }),
      };
    },
  };
}

function percentile(values, p) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * p) - 1)];
}

const codeFiles = [
  'web/src/render/kit.ts',
  'web/src/render/building-chunks.ts',
  'web/src/render/materials.ts',
  'web/src/render/building-massing.ts',
  'web/src/render/lowrise-massing.ts',
  'web/src/render/building-palette.ts',
  'web/src/render/tower-kit.ts',
  'web/src/render/painted-surface.ts',
];
await rm(cache, { recursive: true, force: true });
await mkdir(cache, { recursive: true });
try {
  await build({
    entryPoints: codeFiles.map((file) => resolve(root, file)),
    outdir: cache,
    bundle: true,
    format: 'esm',
    platform: 'node',
    packages: 'external',
    logLevel: 'silent',
  });
  const THREE = await import('three/webgpu');
  installCanvasStub();
  const [{ buildingKit, disposeGroup }, { ArtMaterials }, { BuildingChunks, buildingChunkKey }] =
    await Promise.all([
      import(pathToFileURL(resolve(cache, 'kit.js')).href),
      import(pathToFileURL(resolve(cache, 'materials.js')).href),
      import(pathToFileURL(resolve(cache, 'building-chunks.js')).href),
    ]);
  const art = new ArtMaterials();
  const byChunk = new Map();
  for (const building of world.buildings) {
    const key = buildingChunkKey(building.x, building.z);
    const list = byChunk.get(key) ?? [];
    list.push(building);
    byChunk.set(key, list);
  }

  function oneSample(buildings) {
    const groups = [];
    const chunks = new BuildingChunks(
      (material) => art.buildingBatch(material),
      (dispose) => dispose(),
      art.contours,
    );
    let generationMs = 0;
    let sourcePreparationMs = 0;
    try {
      for (const building of buildings) {
        const start = performance.now();
        const group = buildingKit(building, art);
        generationMs += performance.now() - start;
        const prepareStart = performance.now();
        group.position.set(building.x, 0, building.z);
        group.rotation.y = building.rotation;
        groups.push(group);
        chunks.set(building.id, group);
        sourcePreparationMs += performance.now() - prepareStart;
      }
      const mergeStart = performance.now();
      while (chunks.pendingCount > 0) chunks.update();
      const batchMergeMs = performance.now() - mergeStart;
      const triangles = chunks.root.children.reduce((sum, group) => {
        let count = 0;
        group.traverse((object) => {
          if (object instanceof THREE.Mesh)
            count +=
              (object.geometry.index?.count ?? object.geometry.getAttribute('position').count) / 3;
        });
        return sum + count;
      }, 0);
      return {
        generationMs,
        sourcePreparationMs,
        batchMergeMs,
        totalMs: generationMs + sourcePreparationMs + batchMergeMs,
        triangles,
      };
    } finally {
      chunks.clear();
      for (const group of groups) disposeGroup(group);
    }
  }

  const chunks = [...byChunk.entries()].sort(([a], [b]) => a.localeCompare(b));
  // One full pass warms the kit/material caches and the batch merger.
  let warmupTriangles = 0;
  for (const [, buildings] of chunks) warmupTriangles += oneSample(buildings).triangles;

  const REPEATS = 4;
  const measured = [];
  for (const [key, buildings] of chunks) {
    const generation = [],
      sourcePreparation = [],
      batchMerge = [],
      total = [];
    let triangles = 0;
    for (let repeat = 0; repeat < REPEATS; repeat++) {
      const sample = oneSample(buildings);
      generation.push(sample.generationMs);
      sourcePreparation.push(sample.sourcePreparationMs);
      batchMerge.push(sample.batchMergeMs);
      total.push(sample.totalMs);
      triangles = sample.triangles;
    }
    measured.push({
      key,
      buildings: buildings.length,
      triangles,
      generationMs: {
        median: percentile(generation, 0.5),
        p95: percentile(generation, 0.95),
        max: Math.max(...generation),
        samples: generation,
      },
      sourcePreparationMs: {
        median: percentile(sourcePreparation, 0.5),
        p95: percentile(sourcePreparation, 0.95),
        max: Math.max(...sourcePreparation),
        samples: sourcePreparation,
      },
      batchMergeMs: {
        median: percentile(batchMerge, 0.5),
        p95: percentile(batchMerge, 0.95),
        max: Math.max(...batchMerge),
        samples: batchMerge,
      },
      totalMs: {
        median: percentile(total, 0.5),
        p95: percentile(total, 0.95),
        max: Math.max(...total),
        samples: total,
      },
    });
  }

  const aggregate = (field) => {
    const values = measured.flatMap((row) => row[field].samples);
    return {
      medianMs: percentile(values, 0.5),
      p95Ms: percentile(values, 0.95),
      maxMs: Math.max(...values),
      sampleCount: values.length,
    };
  };
  const report = {
    title: 'Per-chunk authored building regeneration cost',
    measuredAt: new Date().toISOString(),
    input: {
      file: inputPath.replace(`${root}/`, ''),
      sha256: sha256(inputBytes),
      bytes: inputBytes.byteLength,
      seed: world.seed,
      tick: world.tick,
      buildings: world.buildings.length,
    },
    code: Object.fromEntries(
      await Promise.all(
        codeFiles.map(async (file) => [file, sha256(await readFile(resolve(root, file)))]),
      ),
    ),
    machine: {
      platform: `${platform()} ${release()}`,
      architecture: process.arch,
      cpuModel: cpus()[0]?.model ?? 'unknown',
      logicalCpus: cpus().length,
      totalRamBytes: totalmem(),
      freeRamBeforeRunBytes: freemem(),
      node: process.version,
      note: 'Wall-clock timing from Node CPU-side generation/merge; host was not isolated from other processes. No browser, GPU, or frame-rate timing.',
    },
    protocol: {
      warmup: 'One complete pass over every fixture chunk before collecting samples.',
      measuredRepeatsPerChunk: REPEATS,
      chunks: chunks.length,
      generation: 'Time inside production buildingKit() per building, summed for that chunk.',
      sourcePreparation:
        'Time to apply the building translation/rotation and call production BuildingChunks.set(); includes exact production outline extraction and recording the source.',
      batchMerge:
        'Time inside production BuildingChunks.update() commits; includes world-transform/geometry clones, per-vertex color creation, and mergeGeometries(), but excludes BuildingChunks.set() outline extraction.',
      total:
        'Sum of generation, sourcePreparation, and batchMerge samples for one complete chunk rebuild.',
      disposal:
        'Each measured sample clears its batch and calls production disposeGroup() on all authored groups before the next sample.',
      warmupTriangles,
    },
    allChunkGeneration: aggregate('generationMs'),
    allChunkSourcePreparation: aggregate('sourcePreparationMs'),
    allChunkBatchMerge: aggregate('batchMergeMs'),
    allChunkTotal: aggregate('totalMs'),
    chunks: measured,
    interpretation: [
      'This benchmark measures per-chunk work when regenerating all authored source groups from the saved snapshot. Generation, outline/source preparation, and batch transformation/merge are timed independently. It does not measure browser frame scheduling or asynchronous GPU upload.',
      'The generation and batch merge stages are synchronous and can exceed a frame budget for a large chunk; results inform whether regeneration needs incremental staging.',
      'The canvas stub enables production material construction for geometry generation; it does not validate the visual contents of textures or shader compilation.',
    ],
  };
  await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(
    `${JSON.stringify(
      {
        outputPath,
        input: report.input,
        machine: report.machine,
        protocol: report.protocol,
        allChunkGeneration: report.allChunkGeneration,
        allChunkSourcePreparation: report.allChunkSourcePreparation,
        allChunkBatchMerge: report.allChunkBatchMerge,
        allChunkTotal: report.allChunkTotal,
        largestChunks: [...measured]
          .sort((a, b) => b.buildings - a.buildings)
          .slice(0, 5)
          .map(
            ({
              key,
              buildings,
              triangles,
              generationMs,
              sourcePreparationMs,
              batchMergeMs,
              totalMs,
            }) => ({
              key,
              buildings,
              triangles,
              generationMs: {
                median: generationMs.median,
                p95: generationMs.p95,
                max: generationMs.max,
              },
              sourcePreparationMs: {
                median: sourcePreparationMs.median,
                p95: sourcePreparationMs.p95,
                max: sourcePreparationMs.max,
              },
              batchMergeMs: {
                median: batchMergeMs.median,
                p95: batchMergeMs.p95,
                max: batchMergeMs.max,
              },
              totalMs: { median: totalMs.median, p95: totalMs.p95, max: totalMs.max },
            }),
          ),
      },
      null,
      2,
    )}\n`,
  );
  art.dispose();
} finally {
  await rm(cache, { recursive: true, force: true });
}
