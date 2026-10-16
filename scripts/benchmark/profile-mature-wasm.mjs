import { readFileSync, writeFileSync } from 'node:fs';
import { cpus, totalmem } from 'node:os';
import { createHash } from 'node:crypto';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import inspector from 'node:inspector';
import { CityEngine, initSync } from '../../public/wasm/city_engine.js';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const wasmPath = resolve(root, 'public/wasm/city_engine_bg.wasm');
const wasmBytes = readFileSync(wasmPath);
const wasm = initSync({ module: wasmBytes });
const seed = 1_204;
const developmentRate = 60;
const batchCount = 150;
const batchTicks = 2_000;
const sampleEveryBatches = 10;
const profileFirstBatch = 61;
const profileLastBatch = 150;
const profilerIntervalUs = 10_000;
const engine = new CityEngine(seed);
const batchDurations = [];
const snapshots = [];
let peakRssBytes = process.memoryUsage().rss;
const wasmStartBytes = wasm.memory.buffer.byteLength;

const inspectorSession = new inspector.Session();
inspectorSession.connect();
function inspectorPost(method, params = {}) {
  return new Promise((resolvePromise, rejectPromise) => {
    inspectorSession.post(method, params, (error, result) =>
      error ? rejectPromise(error) : resolvePromise(result),
    );
  });
}

function summarizeCpuProfile(profile) {
  const nodes = new Map(profile.nodes.map((node) => [node.id, node]));
  const parents = new Map();
  for (const node of profile.nodes)
    for (const child of node.children ?? []) parents.set(child, node.id);
  const self = new Map();
  const inclusive = new Map();
  let totalSampledUs = 0;
  for (let index = 0; index < profile.samples.length; index++) {
    const leafId = profile.samples[index];
    const delta = profile.timeDeltas[index] ?? 0;
    totalSampledUs += delta;
    const leafStats = self.get(leafId) ?? { timeUs: 0, samples: 0 };
    leafStats.timeUs += delta;
    leafStats.samples++;
    self.set(leafId, leafStats);
    let id = leafId;
    while (id !== undefined) {
      const stats = inclusive.get(id) ?? { timeUs: 0, samples: 0 };
      stats.timeUs += delta;
      stats.samples++;
      inclusive.set(id, stats);
      id = parents.get(id);
    }
  }
  const frame = (id, stats) => {
    const callFrame = nodes.get(id)?.callFrame ?? {};
    const url = callFrame.url ?? '';
    const name = callFrame.functionName || '(anonymous)';
    let source = url || null;
    let sourceReference = null;
    const references = [
      [/route_avoiding/, 'engine/src/lib.rs:8617'],
      [/geometry::rects_overlap/, 'engine/src/geometry.rs:96'],
      [/geometry::segments_distance_coords/, 'engine/src/geometry.rs:182'],
      [/try_create_neighborhood_plan/, 'engine/src/lib.rs:5061'],
      [/plan_road_with_policy/, 'engine/src/lib.rs:6631'],
      [/oriented_parcel_geometry_clear_with_context_mode/, 'engine/src/lib.rs:1775'],
      [/schedule_bus_trip/, 'engine/src/lib.rs:8064'],
      [/step_one/, 'engine/src/lib.rs:2066'],
      [/try_plan_local_chord/, 'engine/src/lib.rs:5500'],
      [/propose_interior_access/, 'engine/src/lib.rs:5827'],
      [/road_corridor_clear_of_parcels_with_context/, 'engine/src/lib.rs:1754'],
      [/neighborhood_candidate_clear_with_layout/, 'engine/src/lib.rs:4654'],
      [/rect_segment_distance/, 'engine/src/geometry.rs:130'],
    ];
    sourceReference = references.find(([pattern]) => pattern.test(name))?.[1] ?? null;
    if (/compiler_builtins|hypotf/.test(name))
      source = 'Rust compiler_builtins/libm (WASM symbol; source line not available)';
    else if (sourceReference) source = `${sourceReference} (WASM symbol)`;
    else if (/wasm-function/.test(name)) source = 'WASM (unresolved symbol)';
    else if (/wasm:\/\//.test(url)) source = 'WASM symbol (source line not available)';
    return {
      function: name,
      url: url || null,
      line: callFrame.lineNumber == null ? null : callFrame.lineNumber + 1,
      column: callFrame.columnNumber == null ? null : callFrame.columnNumber + 1,
      source,
      sourceReference,
      samples: stats.samples,
      timeMs: stats.timeUs / 1000,
      shareOfProfilePercent: totalSampledUs ? (stats.timeUs * 100) / totalSampledUs : 0,
    };
  };
  const rank = (map) =>
    [...map.entries()].map(([id, stats]) => frame(id, stats)).sort((a, b) => b.timeMs - a.timeMs);
  return {
    sampleCount: profile.samples.length,
    totalSampledMs: totalSampledUs / 1000,
    intervalUs: profilerIntervalUs,
    selfTimeTop: rank(self).slice(0, 25),
    inclusiveTimeTop: rank(inclusive).slice(0, 25),
  };
}

function countSummary(world, state) {
  return {
    tick: world.tick,
    day: world.day,
    stage: world.stage,
    population: world.stats.population,
    buildings: world.buildings.length,
    roads: world.roads.length,
    parcels: world.parcels.length,
    projects: state.projects.length,
    pendingProjects: state.projects.filter((project) => project.progress < 1).length,
    travelers: state.travelers.length,
    residents: state.residents.length,
    events: world.events.length,
    activeTravelers: world.stats.active_travelers,
  };
}

function percentile(sorted, value) {
  return sorted[Math.max(0, Math.ceil(sorted.length * value) - 1)];
}

function distribution(values) {
  const sorted = [...values].sort((a, b) => a - b);
  return {
    samples: sorted.length,
    p50Ms: percentile(sorted, 0.5),
    p90Ms: percentile(sorted, 0.9),
    p95Ms: percentile(sorted, 0.95),
    p99Ms: percentile(sorted, 0.99),
    maxMs: sorted.at(-1),
    meanMs: sorted.reduce((sum, value) => sum + value, 0) / sorted.length,
  };
}

function batchPhase(name, rows) {
  return {
    name,
    firstBatch: rows[0]?.batch ?? null,
    lastBatch: rows.at(-1)?.batch ?? null,
    ...distribution(rows.map((row) => row.elapsedMs)),
  };
}

engine.set_development_rate(developmentRate);
const runStartedAt = performance.now();
let cpuProfile;
let profilerStartedAt;
let profilerStoppedAt;
for (let batch = 1; batch <= batchCount; batch++) {
  if (batch === profileFirstBatch) {
    await inspectorPost('Profiler.enable');
    await inspectorPost('Profiler.setSamplingInterval', { interval: profilerIntervalUs });
    await inspectorPost('Profiler.start');
    profilerStartedAt = performance.now();
  }
  const startedAt = performance.now();
  engine.step(batchTicks);
  const elapsedMs = performance.now() - startedAt;
  batchDurations.push(elapsedMs);
  peakRssBytes = Math.max(peakRssBytes, process.memoryUsage().rss);

  if (batch === profileLastBatch) {
    const stopped = await inspectorPost('Profiler.stop');
    cpuProfile = stopped.profile;
    profilerStoppedAt = performance.now();
    await inspectorPost('Profiler.disable');
  }

  if (batch % sampleEveryBatches === 0 || batch === 1 || batch === batchCount) {
    const snapshotStartedAt = performance.now();
    const world = JSON.parse(engine.snapshot());
    const save = JSON.parse(engine.save());
    const sampleCostMs = performance.now() - snapshotStartedAt;
    snapshots.push({
      batch,
      stepElapsedMs: batchDurations.slice(0, batch).reduce((sum, value) => sum + value, 0),
      snapshotAndSaveReadMs: sampleCostMs,
      counts: countSummary(world, save),
    });
  }
}
inspectorSession.disconnect();
const simulationElapsedMs = performance.now() - runStartedAt;

const finalSnapshotStartedAt = performance.now();
const finalSnapshotText = engine.snapshot();
const finalSnapshotMs = performance.now() - finalSnapshotStartedAt;
const saveStartedAt = performance.now();
const savedText = engine.save();
const saveMs = performance.now() - saveStartedAt;
const finalWorld = JSON.parse(finalSnapshotText);
const finalState = JSON.parse(savedText);
const restoreStartedAt = performance.now();
const restored = CityEngine.restore(savedText);
const restoreMs = performance.now() - restoreStartedAt;
const restoreSaveEqual = restored.save() === savedText;
const restoreSnapshotEqual = restored.snapshot() === finalSnapshotText;
const restoreMovementEqual =
  Array.from(restored.movements()).join(',') === Array.from(engine.movements()).join(',');

const queuedResidents = finalState.residents.filter(
  (resident) =>
    resident.transfer === 'waiting_station' ||
    resident.transfer === 'waiting_gateway' ||
    resident.transfer === 'waiting_bus',
).length;
const counts = countSummary(finalWorld, finalState);
const invariants = {
  buildingsWithinCap: counts.buildings <= 2_000,
  roadsWithinCap: counts.roads <= 5_000,
  parcelsWithinCap: counts.parcels <= 4_000,
  projectsWithinCap: counts.projects <= 76,
  pendingProjectsWithinCap: counts.pendingProjects <= 12,
  travelersWithinCap: counts.travelers <= 2_000,
  eventsWithinCap: counts.events <= 10_000,
  populationWithinCap: finalState.population <= 30_000,
  activeTravelersMatchActors:
    finalWorld.stats.active_travelers === finalState.travelers.length + queuedResidents,
  restoreSaveEqual,
  restoreSnapshotEqual,
  restoreMovementEqual,
};

// Measure the resumed continuation separately from the 300k-tick workload.
const continuationStartedAt = performance.now();
engine.step(batchTicks);
restored.step(batchTicks);
const continuationMs = performance.now() - continuationStartedAt;
const continuationEqual =
  engine.save() === restored.save() && engine.snapshot() === restored.snapshot();
invariants.continuationEqualAfterOneBatch = continuationEqual;
const wasmPeakBytes = wasm.memory.buffer.byteLength;
peakRssBytes = Math.max(peakRssBytes, process.memoryUsage().rss);

const failedInvariants = Object.entries(invariants)
  .filter(([, passed]) => !passed)
  .map(([name]) => name);
const output = {
  profile: 'mature-wasm-collection-caps',
  context: {
    command: 'node scripts/benchmark/profile-mature-wasm.mjs',
    testEquivalentWorkload: 'seed 1204, development rate 60, 150 batches × 2000 synchronous ticks',
    concurrentFullTestRun: false,
    runtime: {
      node: process.version,
      platform: process.platform,
      arch: process.arch,
      cpu: cpus()[0]?.model ?? 'unknown',
      logicalCpus: cpus().length,
      totalMemoryBytes: totalmem(),
    },
    sourceWasmSha256: createHash('sha256').update(wasmBytes).digest('hex'),
    timingNote:
      'simulation timings in this file are from the profiling run; compare only to unprofiledBaselineReference because Inspector sampling adds overhead.',
  },
  unprofiledBaselineReference: {
    workload:
      'same seed 1204, development rate 60, 150 × 2000 synchronous ticks; standalone run without Node Inspector active',
    elapsedMs: 56_746.903458,
    batchDurationMs: {
      samples: 150,
      p50Ms: 468.716583,
      p90Ms: 563.756083,
      p95Ms: 611.569167,
      p99Ms: 772.681583,
      maxMs: 935.756333,
      meanMs: 376.355471,
    },
    steadyMaturityBatches61to150Ms: {
      samples: 90,
      p50Ms: 518.07175,
      p90Ms: 586.044167,
      p95Ms: 712.505417,
      p99Ms: 935.756333,
      maxMs: 935.756333,
      meanMs: 522.611196,
    },
    sourceWasmSha256: '3b37a8ea30fdf52f54bd4b577ba98fc540ec58c8cc34c41d431971c56a2bb7b6',
  },
  configuration: {
    seed,
    developmentRate,
    batchCount,
    batchTicks,
    totalTicks: batchCount * batchTicks,
    snapshotSampleEveryBatches: sampleEveryBatches,
  },
  simulation: {
    elapsedMs: simulationElapsedMs,
    batchDurationMs: distribution(batchDurations),
    phases: [
      batchPhase(
        'founding-and-early-growth',
        batchDurations.slice(0, 30).map((elapsedMs, index) => ({ batch: index + 1, elapsedMs })),
      ),
      batchPhase(
        'mature-buildout',
        batchDurations.slice(30, 60).map((elapsedMs, index) => ({ batch: index + 31, elapsedMs })),
      ),
      batchPhase(
        'steady-maturity',
        batchDurations.slice(60).map((elapsedMs, index) => ({ batch: index + 61, elapsedMs })),
      ),
    ],
    batches: batchDurations.map((elapsedMs, index) => ({
      batch: index + 1,
      endTick: (index + 1) * batchTicks,
      elapsedMs,
    })),
    sparseSnapshots: snapshots,
  },
  functionProfile: {
    tool: 'Node Inspector CPU sampling profiler',
    scope: {
      firstBatch: profileFirstBatch,
      lastBatch: profileLastBatch,
      firstTickExclusive: (profileFirstBatch - 1) * batchTicks,
      lastTickInclusive: profileLastBatch * batchTicks,
      stageAtStart:
        snapshots.find((snapshot) => snapshot.batch === profileFirstBatch - 1)?.counts.stage ??
        'Maturity',
      developmentRate,
      intervalUs: profilerIntervalUs,
      wallDurationMs:
        profilerStartedAt == null || profilerStoppedAt == null
          ? null
          : profilerStoppedAt - profilerStartedAt,
    },
    ...summarizeCpuProfile(cpuProfile),
  },
  serialization: {
    finalSnapshotMs,
    saveMs,
    restoreMs,
    continuationMs,
    restoreSaveEqual,
    restoreSnapshotEqual,
    restoreMovementEqual,
    continuationEqual,
  },
  memory: {
    wasmStartBytes,
    wasmPeakBytes,
    processRssSampledPeakBytes: peakRssBytes,
    rssSamplingIntervalBatches: 1,
  },
  finalCounts: {
    ...counts,
    savedPopulation: finalState.population,
    queuedResidents,
    seed: finalState.seed,
  },
  invariants,
  allInvariantsPass: failedInvariants.length === 0,
  failedInvariants,
  sourceInspection: {
    likelyCostDrivers: [
      'At development rate 60, the accumulator adds 60 per simulation tick against DEVELOPMENT_TICKS_PER_DAY=800, so this 300000-tick workload executes about 22500 development_day passes.',
      'development_day scans buildings for activity/vacancy and housing arrivals and advances projects; sync_residents returns immediately once the resident vector reaches MAX_RESIDENTS=800, so it does not keep scanning residents during this profile’s mature phase.',
      'step_one evaluates growth every 40 ticks, and mature growth may run the bounded progressive frontage scan every 160 ticks (up to 24 roads and 48 sites per evaluation). It also updates resident/delivery schedules every 200 ticks, advances travelers/trains/buses every tick, and advances the district measurement cursor every tick. These are source-grounded candidates, not isolated costs.',
    ],
    validationCost:
      'restore_json invokes engine.validate twice while rebuilding/canonicalizing a save; step_one does not call engine.validate. The measured ~231ms restore time is small relative to the ~56.7s stepping run, so validation is not the observed stepping hotspot.',
    evidenceFiles: [
      'engine/src/lib.rs:step',
      'engine/src/lib.rs:step_one',
      'engine/src/lib.rs:development_day',
      'engine/src/lib.rs:evaluate_growth',
      'engine/src/lib.rs:try_progressive_frontage_infill',
      'engine/src/lib.rs:advance_travelers',
      'engine/src/lib.rs:advance_district_measurement',
      'engine/src/lib.rs:sync_residents',
      'engine/src/lib.rs:restore_json',
      'engine/src/lib.rs:validate',
    ],
    samplingProfilerCheck:
      'Node Inspector sampled only batches 61–150; the resulting self/inclusive rankings and symbol-resolved frame names are included in functionProfile.',
    caveat:
      'Sampling is statistical and adds runtime overhead. It identifies sampled CPU work in this steady-maturity window, not a guaranteed optimization or browser frame-time improvement.',
  },
  limitations: [
    'This is a synchronous public-WASM engine workload without rendering, worker scheduling, or browser frame pacing.',
    'Sparse snapshots are outside each timed step batch; their cost is reported separately and is not added to batch durations.',
    'The sampled process RSS and WASM memory are runtime observations, not a device-memory guarantee.',
  ],
};

const outArg = process.argv.find((argument) => argument.startsWith('--out='));
const outPath = resolve(
  root,
  outArg ? outArg.slice('--out='.length) : 'validation/mature-wasm-profile.json',
);
const profileOutArg = process.argv.find((argument) => argument.startsWith('--profile-out='));
const profileOutPath = resolve(
  root,
  profileOutArg
    ? profileOutArg.slice('--profile-out='.length)
    : 'validation/mature-wasm-steady-profile.cpuprofile',
);
const profileText = JSON.stringify(cpuProfile);
writeFileSync(profileOutPath, `${profileText}\n`);
output.functionProfile.artifact = profileOutPath;
output.functionProfile.artifactBytes = Buffer.byteLength(profileText);
writeFileSync(outPath, `${JSON.stringify(output, null, 2)}\n`);
process.stdout.write(
  `${JSON.stringify({ output: outPath, profile: profileOutPath, elapsedMs: simulationElapsedMs, batchDurationMs: output.simulation.batchDurationMs, profileScope: output.functionProfile.scope, sampleCount: output.functionProfile.sampleCount, selfTimeTop: output.functionProfile.selfTimeTop.slice(0, 12), inclusiveTimeTop: output.functionProfile.inclusiveTimeTop.slice(0, 12), finalCounts: output.finalCounts, serialization: output.serialization, allInvariantsPass: output.allInvariantsPass, failedInvariants }, null, 2)}\n`,
);
engine.free();
restored.free();
if (!output.allInvariantsPass) process.exitCode = 1;
