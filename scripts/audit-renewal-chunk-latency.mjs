import { createHash } from 'node:crypto';
import { build } from 'esbuild';
import { cpus, freemem, platform, release, totalmem } from 'node:os';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';

const root = resolve(import.meta.dirname, '..');
const inputPath = resolve(process.argv[2] ?? '.tools/height-redevelopment/construction-surface.json');
const finishPath = resolve(process.argv[3] ?? '.tools/height-redevelopment/construction-finishing.json');
const outputPath = resolve(process.argv[4] ?? 'validation/renewal-chunk-latency.json');
const targetId = Number(process.argv[5] ?? 1243);
const constructionBatches = process.argv.includes('--construction-batches');
const isolatedTarget = process.argv.includes('--isolated-target');
const cache = resolve(root, 'node_modules/.cache', `thecity-renewal-chunk-latency-${process.pid}`);
const inputBytes = await readFile(inputPath), finishBytes = await readFile(finishPath);
const world = JSON.parse(inputBytes.toString('utf8')), finishWorld = JSON.parse(finishBytes.toString('utf8'));
if (!Array.isArray(world.buildings) || !Array.isArray(finishWorld.buildings)) throw new Error('Both inputs must be normal saved worlds with buildings arrays');
const sha256 = data => createHash('sha256').update(data).digest('hex');
const target = world.buildings.find(building => building.id === targetId);
const finishedTarget = finishWorld.buildings.find(building => building.id === targetId);
if (!target || !finishedTarget) throw new Error(`Building ${targetId} must exist in both input saves`);
if (target.x !== finishedTarget.x || target.z !== finishedTarget.z) throw new Error('Target location changed between snapshots; expected in-place renewal');

// Minimal art canvas used by the established Node BuildingChunks benchmark harness.
globalThis.document = { createElement(name) {
  if (name !== 'canvas') throw new Error(`Unexpected DOM element request: ${name}`);
  return { width: 0, height: 0, getContext: () => ({ fillRect() {}, strokeRect() {}, fillText() {}, beginPath() {}, ellipse() {}, fill() {} }) };
} };

const codeFiles = ['web/src/render/construction-batches.ts', 'web/src/render/extended-building.ts', 'web/src/render/kit.ts', 'web/src/render/building-chunks.ts', 'web/src/render/incremental-geometry-merge.ts', 'web/src/render/incremental-geometry-transform.ts', 'web/src/render/materials.ts'];
await rm(cache, { recursive: true, force: true });
await mkdir(cache, { recursive: true });
try {
  const bundle = await build({ entryPoints: codeFiles.map(file => resolve(root, file)), outdir: cache, bundle: true, format: 'esm', platform: 'node', packages: 'external', logLevel: 'silent', metafile: true });
  const THREE = await import('three/webgpu');
  const [{ disposeGroup: disposeKitGroup }, { ArtMaterials }, { BuildingChunks, buildingChunkKey }] = await Promise.all([
    import(pathToFileURL(resolve(cache, 'kit.js')).href),
    import(pathToFileURL(resolve(cache, 'materials.js')).href),
    import(pathToFileURL(resolve(cache, 'building-chunks.js')).href),
  ]);
  const { buildExtendedBuilding } = await import(pathToFileURL(resolve(cache, 'extended-building.js')).href);
  const originals = new Map((world.building_original_footprints ?? []).map(value => [value.building_id, value]));
  const finishOriginals = new Map((finishWorld.building_original_footprints ?? []).map(value => [value.building_id, value]));
  const art = new ArtMaterials();
  const { ConstructionBatches } = await import(pathToFileURL(resolve(cache, 'construction-batches.js')).href);
  const BatchClass = constructionBatches ? ConstructionBatches : BuildingChunks;
  const chunks = new BatchClass(material => art.buildingBatch(material), dispose => dispose(), art.contours);
  const factoryCounts = new Map();
  const initialDescriptors = new Map();
  let factoryGenerationMs = 0;
  const descriptorFor = (building, footprintMap = originals) => {
    const snapshot = { ...building, history: [...(building.history ?? [])] };
    const original = footprintMap.get(building.id);
    const originalSnapshot = original ? { ...original } : undefined;
    return {
      createGroup() {
        factoryCounts.set(snapshot.id, (factoryCounts.get(snapshot.id) ?? 0) + 1);
        const started = performance.now();
        const group = buildExtendedBuilding(snapshot, originalSnapshot, art, { compactMidrise: false });
        group.position.set(snapshot.x, 0, snapshot.z);
        group.rotation.y = snapshot.rotation;
        factoryGenerationMs += performance.now() - started;
        return group;
      },
      disposeGroup: disposeKitGroup,
    };
  };
  const makeGroup = (building, footprintMap = originals) => {
    const group = buildExtendedBuilding(building, footprintMap.get(building.id), art, { compactMidrise: false });
    group.position.set(building.x, 0, building.z);
    group.rotation.y = building.rotation;
    return group;
  };
  const quantiles = values => {
    const ordered = [...values].sort((a, b) => a - b);
    const q = p => ordered[Math.min(ordered.length - 1, Math.ceil(ordered.length * p) - 1)] ?? 0;
    return { calls: values.length, mean: values.reduce((sum, value) => sum + value, 0) / Math.max(1, values.length), p50: q(.5), p95: q(.95), max: q(1) };
  };
  const totalFactoryCalls = () => [...factoryCounts.values()].reduce((sum, count) => sum + count, 0);
  const initialSetMs = [];
  let initialKitGenerationMs = 0, initialSetIncludingOutlinesMs = 0;
  const allStart = performance.now();
  const sourceBuildings = isolatedTarget ? [target] : world.buildings;
  for (const building of sourceBuildings) {
    const generationStart = performance.now();
    const group = makeGroup(building);
    initialKitGenerationMs += performance.now() - generationStart;
    const descriptor = descriptorFor(building);
    initialDescriptors.set(building.id, descriptor);
    const setStart = performance.now();
    chunks.set(building.id, group, descriptor, building.stage !== 'open');
    const elapsed = performance.now() - setStart;
    initialSetMs.push(elapsed);
    initialSetIncludingOutlinesMs += elapsed;
  }
  const targetChunkKey = buildingChunkKey(target.x, target.z);
  const chunkBuildings = sourceBuildings.filter(building => buildingChunkKey(building.x, building.z) === targetChunkKey);
  const initialUpdateMs = [];
  let initialUpdateCalls = 0;
  while (chunks.pendingCount > 0) {
    if (initialUpdateCalls >= 100_000) throw new Error('Initial chunk build failed to drain after 100000 updates');
    const started = performance.now();
    chunks.update();
    initialUpdateMs.push(performance.now() - started);
    initialUpdateCalls++;
  }
  const initialLoadWallMs = performance.now() - allStart;
  const initialTargetVisible = chunks.isSourceVisible(targetId, initialDescriptors.get(targetId));
  if (!initialTargetVisible) throw new Error(`Initial target ${targetId} descriptor did not become visible after full drain`);
  const initialFactoryCalls = totalFactoryCalls();
  const factoryCountsBeforeReplacement = new Map(factoryCounts);

  // This follows City.update's signature-change path for the actual target record
  // in construction-finishing.json: generate one new source group, set it, then
  // call BuildingChunks.update until that exact descriptor is committed visibly.
  const replacementDescriptor = descriptorFor(finishedTarget, finishOriginals);
  const replacementGroupStart = performance.now();
  const replacementGroup = makeGroup(finishedTarget, finishOriginals);
  const replacementGroupGenerationMs = performance.now() - replacementGroupStart;
  const replacementSetStart = performance.now();
  chunks.set(targetId, replacementGroup, replacementDescriptor, finishedTarget.stage !== 'open');
  const replacementSetIncludingOutlinesMs = performance.now() - replacementSetStart;
  const visibleAtStart = chunks.isSourceVisible(targetId, replacementDescriptor);
  const replacementUpdateMs = [];
  const factoriesByUpdate = [];
  const replacementStart = performance.now();
  while (!chunks.isSourceVisible(targetId, replacementDescriptor)) {
    if (replacementUpdateMs.length >= 100_000) throw new Error('Replacement descriptor failed to become visible after 100000 updates');
    const before = totalFactoryCalls();
    const started = performance.now();
    chunks.update();
    replacementUpdateMs.push(performance.now() - started);
    factoriesByUpdate.push(totalFactoryCalls() - before);
  }
  const replacementWallMs = performance.now() - replacementStart;
  const replacementFactoryCounts = new Map([...factoryCounts].map(([id, count]) => [id, count - (factoryCountsBeforeReplacement.get(id) ?? 0)]).filter(([, count]) => count > 0));
  const regeneratedPeerIds = [...replacementFactoryCounts.keys()].filter(id => id !== targetId).sort((a, b) => a - b);
  const replacementFactoryTotal = [...replacementFactoryCounts.values()].reduce((sum, count) => sum + count, 0);
  const factoryCallMaxPerUpdate = Math.max(0, ...factoriesByUpdate);
  const replacementChunkIds = chunkBuildings.map(building => building.id).sort((a, b) => a - b);
  let backgroundUpdates = 0, backgroundFactoryMax = 0;
  while (chunks.pendingCount > 0) {
    if (++backgroundUpdates > 100_000) throw new Error('Background handoff did not drain');
    const before = totalFactoryCalls();
    chunks.update();
    backgroundFactoryMax = Math.max(backgroundFactoryMax, totalFactoryCalls() - before);
    if (!chunks.isSourceVisible(targetId, replacementDescriptor)) throw new Error('Latest source disappeared during background handoff');
  }
  const bundleCode = Object.fromEntries(await Promise.all(Object.keys(bundle.metafile.inputs).sort().map(async file => [file, sha256(await readFile(resolve(root, file)))])));
  const report = {
    title: 'Height-redevelopment building chunk visibility latency audit',
    measuredAt: new Date().toISOString(),
    input: {
      file: inputPath.replace(`${root}/`, ''), sha256: sha256(inputBytes), bytes: inputBytes.byteLength,
      seed: world.seed, tick: world.tick, buildings: world.buildings.length,
      buildingFactory: 'buildExtendedBuilding', compactMidrise: false, originalFootprints: originals.size,
      constructionBatches, isolatedTarget, submittedBuildings: sourceBuildings.length,
      targetUpdateFile: finishPath.replace(`${root}/`, ''), targetUpdateSha256: sha256(finishBytes), targetUpdateTick: finishWorld.tick,
    },
    target: {
      buildingId: targetId,
      initial: { floors: target.floors, stage: target.stage, progress: target.progress },
      update: { floors: finishedTarget.floors, stage: finishedTarget.stage, progress: finishedTarget.progress },
      chunkKey: targetChunkKey, chunkSizeMeters: 200,
      buildingsInSameChunk: chunkBuildings.length, peerBuildings: chunkBuildings.length - 1,
      chunkBuildingIds: replacementChunkIds,
    },
    code: bundleCode,
    machine: { platform: `${platform()} ${release()}`, architecture: process.arch, cpuModel: cpus()[0]?.model ?? 'unknown', logicalCpus: cpus().length, totalRamBytes: totalmem(), freeRamBeforeRunBytes: freemem(), node: process.version, note: 'Synthetic Node CPU-only update-call drain; host activity is not isolated. No browser frame rate, GPU, or presentation-thread measurement.' },
    initialLoad: {
      updateCalls: initialUpdateCalls,
      updateWallMs: initialLoadWallMs,
      perUpdateMs: quantiles(initialUpdateMs),
      authoredGroupGenerationMs: initialKitGenerationMs,
      setIncludingOutlineExtractionMs: initialSetIncludingOutlinesMs,
      setPerBuildingMs: quantiles(initialSetMs),
      descriptorFactoriesDuringInitialDrain: initialFactoryCalls,
      targetDescriptorVisibleAfterDrain: initialTargetVisible,
    },
    backgroundHandoff: { updateCalls: backgroundUpdates, maxFactoriesPerUpdate: backgroundFactoryMax, pendingAfterDrain: chunks.pendingCount },
    replacement: {
      updateCallsUntilIsSourceVisible: replacementUpdateMs.length,
      descriptorVisibleBeforeDrain: visibleAtStart,
      drainWallMs: replacementWallMs,
      perUpdateMs: quantiles(replacementUpdateMs),
      updateWallMs: replacementUpdateMs,
      targetAuthoredGroupGenerationMs: replacementGroupGenerationMs,
      targetSetIncludingOutlineExtractionMs: replacementSetIncludingOutlinesMs,
      regeneratedPeerFactories: replacementFactoryTotal,
      regeneratedPeerCount: regeneratedPeerIds.length,
      regeneratedPeerIds,
      factoryCallsByBuilding: Object.fromEntries([...replacementFactoryCounts].sort((a, b) => a[0] - b[0])),
      factoryCallsPerUpdate: factoriesByUpdate,
      factoryCallMaxPerUpdate,
      descriptorVisibleAtEnd: chunks.isSourceVisible(targetId, replacementDescriptor),
      initialTargetChunkSourceCount: chunkBuildings.length,
      expectedPeerFactoryCount: chunkBuildings.length - 1,
      allExpectedPeersRegeneratedOnce: regeneratedPeerIds.length === chunkBuildings.length - 1 && regeneratedPeerIds.every(id => replacementFactoryCounts.get(id) === 1),
      totalFactoryGenerationMsAcrossRun: factoryGenerationMs,
    },
    limits: { chunkSizeMeters: 200, maxSourceGroupFactoriesPerUpdate: 1, preparedMeshCapPerUpdate: 16, preparedVertexQuotaPerUpdate: 12000, preparedIndexQuotaPerUpdate: 36000, contourFloatQuotaPerUpdate: 24000 },
    interpretation: [
      'With --construction-batches the candidate construction/static wrapper replaces the baseline BuildingChunks scheduler.',
      'With --isolated-target only the target is submitted: this estimates isolated batching cost, not an implemented dynamic/static handoff or city performance.',
      'Geometry uses the production buildExtendedBuilding factory with persisted original footprints and compactMidrise disabled.',
      'Inputs are normal saved worlds. The initial load calls BuildingChunks.set for every source building, then drains production BuildingChunks.update calls until all dirty chunks commit.',
      'The replacement uses building 1243 from the actual construction-finishing save, at its saved location. It follows the city signature-change path by setting one new descriptor, then drains update calls until isSourceVisible returns true for that exact descriptor.',
      'The replacement regenerates the affected chunk peers from immutable descriptors while consuming the target group supplied to set. The report includes each update duration and per-building factory calls.',
      'This is a synthetic Node drain benchmark, not browser FPS or a visual acceptance test. It omits browser scheduling, renderer/GPU work, network/UI delays, and video capture timing; update-call wall time cannot be directly equated with the reported roughly 12-second video delay.',
    ],
  };
  await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify({ outputPath, input: report.input, target: report.target, initialLoad: { updateCalls: initialUpdateCalls, updateWallMs: initialLoadWallMs, perUpdateMs: report.initialLoad.perUpdateMs, authoredGroupGenerationMs: initialKitGenerationMs }, replacement: { updateCallsUntilIsSourceVisible: replacementUpdateMs.length, drainWallMs: replacementWallMs, perUpdateMs: report.replacement.perUpdateMs, regeneratedPeerCount: regeneratedPeerIds.length, regeneratedPeerFactories: replacementFactoryTotal, factoryCallMaxPerUpdate, descriptorVisibleAtEnd: report.replacement.descriptorVisibleAtEnd, allExpectedPeersRegeneratedOnce: report.replacement.allExpectedPeersRegeneratedOnce } }, null, 2)}\n`);
  chunks.clear();
  art.dispose();
} finally {
  await rm(cache, { recursive: true, force: true });
}
