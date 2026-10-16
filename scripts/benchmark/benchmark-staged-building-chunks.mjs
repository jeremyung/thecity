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
const outputPath = resolve(process.argv[3] ?? 'validation/staged-building-chunks-benchmark.json');
const markdownPath = resolve(process.argv[4] ?? outputPath.replace(/\.json$/i, '.md'));
const cache = resolve(root, 'node_modules/.cache', `thecity-staged-chunks-${process.pid}`);
const inputBytes = await readFile(inputPath);
const world = JSON.parse(inputBytes.toString('utf8'));
if (!Array.isArray(world.buildings) || world.buildings.length !== 850)
  throw new Error(
    `Expected the mature 850-building save; got ${world.buildings?.length ?? 'no buildings'}`,
  );
const sha256 = (data) => createHash('sha256').update(data).digest('hex');

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

const codeFiles = [
  'web/src/render/kit.ts',
  'web/src/render/building-chunks.ts',
  'web/src/render/incremental-geometry-merge.ts',
  'web/src/render/incremental-geometry-transform.ts',
  'web/src/render/materials.ts',
];
await rm(cache, { recursive: true, force: true });
await mkdir(cache, { recursive: true });
try {
  const bundle = await build({
    entryPoints: codeFiles.map((file) => resolve(root, file)),
    outdir: cache,
    bundle: true,
    format: 'esm',
    platform: 'node',
    packages: 'external',
    logLevel: 'silent',
    metafile: true,
  });
  const THREE = await import('three/webgpu');
  const [
    { buildingKit, disposeGroup: disposeKitGroup },
    { ArtMaterials },
    { BuildingChunks, buildingChunkKey },
  ] = await Promise.all([
    import(pathToFileURL(resolve(cache, 'kit.js')).href),
    import(pathToFileURL(resolve(cache, 'materials.js')).href),
    import(pathToFileURL(resolve(cache, 'building-chunks.js')).href),
  ]);

  const art = new ArtMaterials();
  const chunks = new BuildingChunks(
    (material) => art.buildingBatch(material),
    (dispose) => dispose(),
    art.contours,
  );
  const factoryCounts = new Map();
  const initialSetMs = [];
  let sourceMeshes = 0,
    sourceVertices = 0,
    sourceIndices = 0;
  let maxSingleMeshVertices = 0,
    oversizedSourceMeshes = 0;
  let factoryGenerationMs = 0;

  function descriptorFor(building) {
    const snapshot = { ...building, history: [...(building.history ?? [])] };
    return {
      createGroup() {
        factoryCounts.set(snapshot.id, (factoryCounts.get(snapshot.id) ?? 0) + 1);
        const started = performance.now();
        const group = buildingKit(snapshot, art);
        group.position.set(snapshot.x, 0, snapshot.z);
        group.rotation.y = snapshot.rotation;
        factoryGenerationMs += performance.now() - started;
        return group;
      },
      disposeGroup: disposeKitGroup,
    };
  }

  function makeInitialGroup(building) {
    const group = buildingKit(building, art);
    group.position.set(building.x, 0, building.z);
    group.rotation.y = building.rotation;
    return group;
  }

  function inspectSourceGroup(group) {
    group.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return;
      const geometry = object.geometry,
        positions = geometry.getAttribute('position');
      sourceMeshes++;
      sourceVertices += positions.count;
      sourceIndices += geometry.index?.count ?? positions.count;
      maxSingleMeshVertices = Math.max(maxSingleMeshVertices, positions.count);
      if (positions.count > 12_000) oversizedSourceMeshes++;
    });
  }

  function batchStats() {
    let meshes = 0,
      vertices = 0,
      indices = 0,
      triangles = 0;
    chunks.root.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return;
      const position = object.geometry.getAttribute('position'),
        index = object.geometry.index;
      meshes++;
      vertices += position.count;
      indices += index?.count ?? position.count;
      triangles += (index?.count ?? position.count) / 3;
    });
    return { meshes, vertices, indices, triangles };
  }

  function sourceReferenceStats() {
    const sources = [...chunks.sources.values()];
    return {
      sourceRecords: sources.length,
      recordsWithPendingGroup: sources.filter((source) => source.pendingGroup !== null).length,
      recordsWithLegacyGroup: sources.filter((source) => source.legacyGroup !== null).length,
      activeBuild: chunks.active !== null,
      // The descriptor records retain immutable building data and contours;
      // source mesh arrays are only reachable through a group reference.
      retainedSourceGroupRefs: sources.filter(
        (source) => source.pendingGroup !== null || source.legacyGroup !== null,
      ).length,
    };
  }

  function quantiles(values) {
    const ordered = [...values].sort((a, b) => a - b);
    const q = (p) => ordered[Math.min(ordered.length - 1, Math.ceil(ordered.length * p) - 1)] ?? 0;
    return {
      calls: values.length,
      mean: values.reduce((sum, value) => sum + value, 0) / Math.max(1, values.length),
      p50: q(0.5),
      p95: q(0.95),
      max: q(1),
    };
  }

  let initialKitGenerationMs = 0,
    initialSetOutlineMs = 0;
  for (const building of world.buildings) {
    const generationStarted = performance.now();
    const group = makeInitialGroup(building);
    initialKitGenerationMs += performance.now() - generationStarted;
    inspectSourceGroup(group);
    const start = performance.now();
    chunks.set(building.id, group, descriptorFor(building));
    const elapsed = performance.now() - start;
    initialSetMs.push(elapsed);
    initialSetOutlineMs += elapsed;
  }

  function drain(label) {
    const perUpdateMs = [],
      factoryCallsPerUpdate = [];
    let updateCalls = 0;
    const started = performance.now();
    while (chunks.pendingCount > 0) {
      if (updateCalls >= 50_000)
        throw new Error(`${label}: staged chunk update failed to drain after 50000 calls`);
      const before = [...factoryCounts.values()].reduce((sum, count) => sum + count, 0);
      const frameStart = performance.now();
      chunks.update();
      perUpdateMs.push(performance.now() - frameStart);
      const after = [...factoryCounts.values()].reduce((sum, count) => sum + count, 0);
      factoryCallsPerUpdate.push(after - before);
      updateCalls++;
    }
    return {
      totalMs: performance.now() - started,
      updateCalls,
      perUpdate: quantiles(perUpdateMs),
      factoryCallsPerUpdate,
      factoryCallMaxPerUpdate: Math.max(0, ...factoryCallsPerUpdate),
      factoryCalls: factoryCallsPerUpdate.reduce((sum, count) => sum + count, 0),
    };
  }

  const initialDrain = drain('Initial drain');
  const initialStats = batchStats();
  const retainedAfterInitial = sourceReferenceStats();
  if (
    retainedAfterInitial.recordsWithPendingGroup ||
    retainedAfterInitial.recordsWithLegacyGroup ||
    retainedAfterInitial.activeBuild
  )
    throw new Error(
      `Initial descriptor drain retained source groups: ${JSON.stringify(retainedAfterInitial)}`,
    );

  // Replace an existing source at the same location with an identical immutable
  // building snapshot. This forces the production descriptor path to regenerate
  // its peers and consume the replacement's one-time pending group.
  const replaced = world.buildings[0];
  const replacement = { ...replaced, history: [...(replaced.history ?? [])] };
  const replacementGroupStarted = performance.now();
  const replacementGroup = makeInitialGroup(replacement);
  const replacementGenerationMs = performance.now() - replacementGroupStarted;
  const replacementQueueStarted = performance.now();
  chunks.set(replacement.id, replacementGroup, descriptorFor(replacement));
  const replacementSetMs = performance.now() - replacementQueueStarted;
  const replacementChunkKey = buildingChunkKey(replacement.x, replacement.z);
  const replacementChunkBuildings = world.buildings.filter(
    (building) => buildingChunkKey(building.x, building.z) === replacementChunkKey,
  ).length;
  const replacementDrain = drain('Replacement drain');
  const replacementStats = batchStats();
  const retainedAfterReplacement = sourceReferenceStats();
  if (
    retainedAfterReplacement.recordsWithPendingGroup ||
    retainedAfterReplacement.recordsWithLegacyGroup ||
    retainedAfterReplacement.activeBuild
  )
    throw new Error(
      `Replacement drain retained source groups: ${JSON.stringify(retainedAfterReplacement)}`,
    );
  if (replacementDrain.factoryCallMaxPerUpdate > 1)
    throw new Error(
      `Regenerated more than one source group in an update: ${replacementDrain.factoryCallMaxPerUpdate}`,
    );
  if (initialStats.triangles !== replacementStats.triangles)
    throw new Error(
      `Replacement changed triangle total: ${initialStats.triangles} -> ${replacementStats.triangles}`,
    );

  const inputHash = sha256(inputBytes);
  const report = {
    title: 'Descriptor-based staged building chunk benchmark',
    measuredAt: new Date().toISOString(),
    input: {
      file: inputPath.replace(`${root}/`, ''),
      sha256: inputHash,
      bytes: inputBytes.byteLength,
      seed: world.seed,
      tick: world.tick,
      buildings: world.buildings.length,
    },
    code: Object.fromEntries(
      await Promise.all(
        Object.keys(bundle.metafile.inputs)
          .sort()
          .map(async (file) => [file, sha256(await readFile(resolve(root, file)))]),
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
      note: 'Node CPU-only wall-clock measurements; other host activity was not isolated. No browser/GPU timing or renderer frame pacing.',
    },
    source: {
      buildings: world.buildings.length,
      meshes: sourceMeshes,
      vertices: sourceVertices,
      indices: sourceIndices,
      maxAuthoredMeshVertices: maxSingleMeshVertices,
      authoredMeshesExceedingVertexQuota: oversizedSourceMeshes,
      initialBuildingKitGenerationMs: initialKitGenerationMs,
      initialSetOutlineMs: initialSetOutlineMs,
      initialSetOutlinePerBuilding: quantiles(initialSetMs),
    },
    initialDrain: {
      ...initialDrain,
      batch: initialStats,
      retainedSourceReferencesAfterDrain: retainedAfterInitial,
    },
    replacement: {
      buildingId: replacement.id,
      chunkKey: replacementChunkKey,
      buildingsInAffectedChunk: replacementChunkBuildings,
      replacementAuthoredGroupGenerationMs: replacementGenerationMs,
      setIncludingOutlineExtractionMs: replacementSetMs,
      regeneratedFactoryGenerationMs: factoryGenerationMs,
      ...replacementDrain,
      batch: replacementStats,
      retainedSourceReferencesAfterDrain: retainedAfterReplacement,
      trianglesUnchanged: initialStats.triangles === replacementStats.triangles,
      factoryCallsByBuilding: Object.fromEntries([...factoryCounts].sort((a, b) => a[0] - b[0])),
    },
    limits: {
      chunkSizeMeters: 200,
      maxSourceGroupFactoriesPerUpdate: 1,
      preparedMeshCapPerUpdate: 16,
      preparedVertexQuotaPerUpdate: 12_000,
      preparedIndexQuotaPerUpdate: 36_000,
      contourFloatQuotaPerUpdate: 24_000,
      sourceGroupArrayRetainedAfterDrain: false,
    },
    interpretation: [
      'Initial source groups are generated up front as in the city building queue, passed to BuildingChunks.set() with immutable descriptors, then consumed and disposed during the staged initial drain. Descriptor source records are inspected directly after each drain; no source group refs remain in pendingGroup or legacyGroup.',
      'The replacement uses an identical building snapshot at the same location, so the reported triangle total must remain unchanged. Existing peers in the affected chunk are regenerated through their descriptors; the replacement group is consumed from pendingGroup. Factory calls per update are recorded and checked against the one-group cap.',
      'This measures Node CPU wall time and structural references only. It does not measure garbage collection, browser heap/RSS, transient allocations, renderer uploads, GPU memory, frame rate, or perceived responsiveness.',
      'Initial per-update p50/p95/max and replacement per-update p50/p95/max come from one drain each, not independent repeated runs. Source buildingKit generation plus synchronous set/outline work are reported separately from staged update drains.',
      'The replacement does not change city state and is only a reproducible benchmark of the production descriptor contract. Detailed rendering equivalence is covered by building chunk and source ownership tests.',
    ],
  };

  const markdown =
    `# Descriptor-based staged building chunk benchmark\n\n` +
    `Input: \`${report.input.file}\` (seed ${world.seed}, tick ${world.tick}, ${world.buildings.length} buildings), SHA-256 \`${inputHash}\`.\n\n` +
    `The initial ${sourceMeshes.toLocaleString()}-mesh source queue used **${initialKitGenerationMs.toFixed(1)} ms** for buildingKit generation and **${initialSetOutlineMs.toFixed(1)} ms** in set/outline preparation. The descriptor-based staged drain took **${initialDrain.updateCalls.toLocaleString()} updates** (${initialDrain.totalMs.toFixed(1)} ms); per-update p50/p95/max was ${initialDrain.perUpdate.p50.toFixed(3)}/${initialDrain.perUpdate.p95.toFixed(3)}/${initialDrain.perUpdate.max.toFixed(3)} ms.\n\n` +
    `Replacing building ${replacement.id} in chunk \`${replacementChunkKey}\` regenerated ${replacementDrain.factoryCalls} neighboring source groups across ${replacementDrain.updateCalls.toLocaleString()} updates. Factory calls per update peaked at **${replacementDrain.factoryCallMaxPerUpdate}**; per-update p50/p95/max was ${replacementDrain.perUpdate.p50.toFixed(3)}/${replacementDrain.perUpdate.p95.toFixed(3)}/${replacementDrain.perUpdate.max.toFixed(3)} ms. The replacement added no triangle difference (${initialStats.triangles.toLocaleString()} before and after).\n\n` +
    `After both drains, ${retainedAfterInitial.sourceRecords} source records had zero pending or legacy group references; no active build remained. This is a structural reference check, not evidence that garbage collection reclaimed a particular number of bytes. Timings are single-run Node CPU observations on ${report.machine.cpuModel}; they do not represent browser frame pacing, GPU upload, or memory. See the JSON for hashes, source counts, per-update distributions and limits.\n`;

  await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
  await writeFile(markdownPath, markdown);
  process.stdout.write(
    `${JSON.stringify({ outputPath, markdownPath, input: report.input, source: report.source, initialDrain: { totalMs: initialDrain.totalMs, updateCalls: initialDrain.updateCalls, perUpdate: initialDrain.perUpdate, batch: initialStats }, replacement: { buildingId: replacement.id, chunk: replacementChunkKey, regeneratedFactories: replacementDrain.factoryCalls, factoryCallMaxPerUpdate: replacementDrain.factoryCallMaxPerUpdate, updateCalls: replacementDrain.updateCalls, totalMs: replacementDrain.totalMs, perUpdate: replacementDrain.perUpdate, trianglesUnchanged: true, retainedSourceReferences: retainedAfterReplacement }, machine: report.machine }, null, 2)}\n`,
  );
  chunks.clear();
  art.dispose();
} finally {
  await rm(cache, { recursive: true, force: true });
}
