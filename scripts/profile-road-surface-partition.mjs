// CPU-only profiling of the renderer's road ownership claims on fresh seed-0 worlds.
import { cpus, hostname, platform, release, arch } from 'node:os';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import assert from 'node:assert/strict';
import { build } from 'esbuild';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

const root = process.cwd();
const output = 'validation/road-surface-partition-cost.json';
const targets = [9_250, 100_000];
const developmentRate = 16;
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
await build({ stdin: { contents: "export {RoadSurfacePartition} from './web/src/render/road-surface-partition'; export {junctionFootprint} from './web/src/render/road-junction'; export {roadWidth} from './web/src/render/road-elevation';", resolveDir: root }, bundle: true, platform: 'node', format: 'esm', outfile: '.tools/road-surface-partition-profile.mjs' });
const { RoadSurfacePartition, junctionFootprint, roadWidth } = await import('../.tools/road-surface-partition-profile.mjs');
initSync({ module: await readFile('public/wasm/city_engine_bg.wasm') });

function rectangle(a, b, width) {
  const dx = b.x - a.x, dz = b.z - a.z, length = Math.hypot(dx, dz);
  if (!Number.isFinite(length) || length < 1e-6) return [];
  const nx = -dz / length * width / 2, nz = dx / length * width / 2;
  return [{ x: a.x + nx, z: a.z + nz }, { x: b.x + nx, z: b.z + nz },
    { x: b.x - nx, z: b.z - nz }, { x: a.x - nx, z: a.z - nz }];
}
const eligible = kind => kind !== 'bridge' && kind !== 'rail' && kind !== 'expressway';

function claimWorld(world, { construction = false, partition = new RoadSurfacePartition(), completedAlready = false, capture = false } = {}) {
  const nodes = new Map(world.nodes.map(n => [n.id, n]));
  const counts = { completedRoads: 0, junctionPads: 0, constructionRoads: 0, constructionProjects: 0,
    emittedRoadPieces: 0, emittedPadPieces: 0, emittedConstructionPieces: 0, emptyClaims: 0 };
  const emitted = capture ? { completed: [], construction: [] } : null;
  const claim = (poly, bucket) => {
    if (!poly.length) return;
    const pieces = partition.claim(poly);
    if (emitted) emitted[bucket === 'completedRoads' || bucket === 'junctionPads' ? 'completed' : 'construction'].push(...pieces);
    counts[bucket]++;
    if (!pieces.length) counts.emptyClaims++;
    if (bucket === 'completedRoads') counts.emittedRoadPieces += pieces.length;
    else if (bucket === 'junctionPads') counts.emittedPadPieces += pieces.length;
    else counts.emittedConstructionPieces += pieces.length;
  };
  const roads = [...world.roads].sort((a, b) => a.id - b.id);
  if (!completedAlready) {
    for (const road of roads) {
      if (road.stage !== 'open' || !eligible(road.kind)) continue;
      const a = nodes.get(road.from), b = nodes.get(road.to);
      if (!a || !b) continue;
      claim(rectangle(a, b, roadWidth(road.kind)), 'completedRoads');
    }
    const arms = new Map();
    for (const road of world.roads) {
      if (road.stage !== 'open' || !eligible(road.kind)) continue;
      const a = nodes.get(road.from), b = nodes.get(road.to);
      if (!a || !b) continue;
      for (const [node, end] of [[a, b], [b, a]]) {
        const incident = arms.get(node.id) ?? [];
        incident.push({ end, width: roadWidth(road.kind) });
        arms.set(node.id, incident);
      }
    }
    for (const [id, junctionArms] of arms) {
      const pad = junctionFootprint(nodes.get(id), junctionArms);
      if (pad.length) claim(pad, 'junctionPads');
    }
  }
  if (!construction) return { counts, partition, emitted };
  const constructionStarted = performance.now();
  for (const road of roads) {
    if (road.stage === 'open' || !eligible(road.kind)) continue;
    const a = nodes.get(road.from), fullB = nodes.get(road.to);
    if (!a || !fullB) continue;
    const fraction = Math.max(0, Math.min(1, ((road.progress ?? 0) - .12) / .88));
    if (fraction < .015) continue;
    claim(rectangle(a, { x: a.x + (fullB.x - a.x) * fraction, z: a.z + (fullB.z - a.z) * fraction }, roadWidth(road.kind)), 'constructionRoads');
  }
  for (const project of world.projects.filter(p => p.kind === 'road' && p.progress < 1)) {
    const a = nodes.get(project.from), b = nodes.get(project.to);
    if (!a || !b || roads.some(r => r.from === a.id && r.to === b.id && r.stage !== 'open')) continue;
    const match = roads.find(r => r.from === a.id && r.to === b.id || r.from === b.id && r.to === a.id);
    const kind = match?.kind ?? (Math.abs(a.x - b.x) > 100 ? 'bridge' : 'local');
    if (!eligible(kind)) continue;
    const fraction = Math.max(0, Math.min(1, (project.progress - .12) / .88));
    if (fraction < .015) continue;
    claim(rectangle(a, { x: a.x + (b.x - a.x) * fraction, z: a.z + (b.z - a.z) * fraction }, roadWidth(kind)), 'constructionProjects');
  }
  counts.constructionClaimElapsedMs = performance.now() - constructionStarted;
  return { counts, partition, emitted };
}

function summarize(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const percentile = p => sorted[Math.min(sorted.length - 1, Math.floor((sorted.length - 1) * p))];
  return { samples: values.length, medianMs: percentile(.5), p95Ms: percentile(.95), maxMs: Math.max(...values) };
}
function timedPass(world, construction, options = {}) {
  const start = performance.now();
  const { counts, partition, emitted } = claimWorld(world, { construction, ...options });
  const { constructionClaimElapsedMs = 0, ...claimCounts } = counts;
  return { elapsedMs: performance.now() - start, constructionClaimElapsedMs, counts: claimCounts, partition, emitted };
}

const engine = new CityEngine(0), snapshots = [];
try {
  engine.set_development_rate(developmentRate);
  for (const target of targets) {
    let world = JSON.parse(engine.snapshot());
    while (world.tick < target) {
      engine.step(Math.min(2_000, target - world.tick));
      world = JSON.parse(engine.snapshot());
    }
    snapshots.push(world);
  }
} finally { engine.free(); }

const measurements = [];
for (const world of snapshots) {
  const initial = claimWorld(world, { construction: true });
  const { constructionClaimElapsedMs: _initialConstructionTiming, ...counts } = initial.counts;
  const completedCache = claimWorld(world, { construction: false }).partition;
  // Warm the module/JIT and hash lookups, then measure fresh complete ownership
  // passes and the same full pass including construction prefixes/projects.
  for (let i = 0; i < 3; i++) { timedPass(world, false); timedPass(world, true); }
  const completed = [], withConstruction = [], constructionOnly = [], forkCosts = [], forkConstruction = [];
  for (let i = 0; i < 15; i++) {
    completed.push(timedPass(world, false).elapsedMs);
    const full = timedPass(world, true, { capture: true });
    withConstruction.push(full.elapsedMs);
    constructionOnly.push(full.constructionClaimElapsedMs);
    const forkStart = performance.now();
    const fork = completedCache.fork();
    const forkElapsedMs = performance.now() - forkStart;
    forkCosts.push(forkElapsedMs);
    const forkResult = claimWorld(world, { construction: true, partition: fork, completedAlready: true, capture: true });
    forkConstruction.push(performance.now() - forkStart);
    assert.deepEqual(forkResult.emitted.construction, full.emitted.construction,
      `fork and replay construction output differ at tick ${world.tick}, sample ${i}`);
  }
  measurements.push({
    tick: world.tick,
    world: { roads: world.roads.length, openRoads: world.roads.filter(r => r.stage === 'open').length,
      activeRoadProjects: world.projects.filter(p => p.kind === 'road' && p.progress < 1).length,
      nodes: world.nodes.length, buildings: world.buildings.length },
    ownershipClaims: counts,
    freshPartitionPass: { completedRoadsAndPads: summarize(completed), completedPlusConstruction: summarize(withConstruction),
      constructionClaimsOnlyAfterCompletedReplay: summarize(constructionOnly),
      constructionReplayIncrementMedianMs: +(summarize(withConstruction).medianMs - summarize(completed).medianMs).toFixed(4) },
    cachedFork: { forkOnly: summarize(forkCosts), forkPlusConstructionClaims: summarize(forkConstruction),
      constructionOutputDeepEqualToReplay: true, comparisonSamples: forkConstruction.length },
    gpuMeasured: false,
  });
}

const report = {
  schemaVersion: 1,
  source: 'Fresh seed 0 CityEngine using public WASM API; dev rate 16; renderer-equivalent open road ID order, junction pads, then construction prefixes/projects. Fresh replay timings create a partition; cached timings fork a previously populated completed partition.',
  environment: { node: process.version, platform: `${platform()} ${release()} ${arch()}`, hostname: hostname(), cpu: cpus()[0]?.model ?? 'unknown', logicalCpus: cpus().length,
    wasmSha256: sha(await readFile('public/wasm/city_engine_bg.wasm')), gpuMeasured: false },
  method: { stepping: 'advance each target in steps no larger than 2,000 ticks, with JSON snapshot outside claim timings', warmupPasses: 3,
    measuredPasses: 15, polygon: 'roadWidth rectangles with renderer construction endpoint clamp((progress-.12)/.88), junctionFootprint pads for open non-bridge/non-rail/non-expressway roads',
    timing: 'Node process performance.now CPU elapsed time; fresh replay per pass, plus separately timed cached completed-partition fork and construction claims; no GPU or mesh extrusion measured', limitations: 'No browser frame scheduling or GPU claim. Fork construction output is deep-compared against the same construction claim sequence following a completed replay on every measured sample.' },
  snapshots: measurements,
};
await writeFile(output, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ output, environment: report.environment, snapshots: report.snapshots }, null, 2));
