import { readFileSync } from 'node:fs';
import { cpus, totalmem } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const wasmBytes = readFileSync(resolve(root, 'public/wasm/city_engine_bg.wasm'));
const wasm = initSync({ module: wasmBytes });
const seed = 87_231;
const developmentRate = 16;
const warmupTicks = 240_000;
const measuredTicks = 10_000;
const warmupBatch = 2_000;
const durations = new Float64Array(measuredTicks);
const engine = new CityEngine(seed);
const earlyGrowth = [];

function percentile(sorted, p) {
  return sorted[Math.max(0, Math.ceil(sorted.length * p) - 1)];
}

function quantileReport(samples) {
  const sorted = Array.from(samples).sort((a, b) => a - b);
  const total = sorted.reduce((sum, value) => sum + value, 0);
  return {
    p50Ms: percentile(sorted, 0.5),
    p95Ms: percentile(sorted, 0.95),
    p99Ms: percentile(sorted, 0.99),
    maxMs: sorted.at(-1),
    meanMs: total / sorted.length,
    ticksPerSecond: 1_000 / (total / sorted.length),
    budget50ms: {
      p50: percentile(sorted, 0.5) / 50,
      p95: percentile(sorted, 0.95) / 50,
      p99: percentile(sorted, 0.99) / 50,
      max: sorted.at(-1) / 50,
    },
  };
}

engine.set_development_rate(developmentRate);
const warmupStart = performance.now();
let ticks = 0;
for (; ticks < 24_000; ticks += 1_000) {
  engine.step(1_000);
  const world = JSON.parse(engine.snapshot());
  earlyGrowth.push({
    tick: world.tick,
    stage: world.stage,
    buildings: world.buildings.length,
    roads: world.roads.length,
    parcels: world.parcels.length,
    residents: world.residents?.length ?? 0,
    trains: world.trains.length,
    onboardResidents:
      world.residents?.filter(
        (resident) => resident.train_id !== null && resident.train_id !== undefined,
      ).length ?? 0,
    trainPassengers: world.trains.reduce((sum, train) => sum + (train.passengers ?? 0), 0),
    trainCapacity: world.trains.reduce((sum, train) => sum + (train.capacity ?? 0), 0),
    stationWaiting: world.trains.reduce((sum, train) => sum + (train.station_waiting ?? 0), 0),
    gatewayWaiting: world.trains.reduce((sum, train) => sum + (train.gateway_waiting ?? 0), 0),
    transfers: (world.residents ?? []).reduce((counts, resident) => {
      const state = resident.transfer ?? 'untracked';
      counts[state] = (counts[state] ?? 0) + 1;
      return counts;
    }, {}),
    population: world.stats.population,
    activeProjects: world.stats.active_projects,
  });
}
for (; ticks < warmupTicks; ticks += warmupBatch) engine.step(warmupBatch);
const warmupMs = performance.now() - warmupStart;
const warmedSnapshot = JSON.parse(engine.snapshot());
if (warmedSnapshot.stage !== 'Maturity') {
  engine.free();
  throw new Error(`Benchmark warmup expected Maturity, received ${warmedSnapshot.stage}`);
}

let peakRssBytes = process.memoryUsage().rss;
const wasmStartBytes = wasm.memory.buffer.byteLength;
const runStart = performance.now();
for (let tick = 0; tick < measuredTicks; tick++) {
  const start = performance.now();
  engine.step(1);
  durations[tick] = performance.now() - start;
  if ((tick + 1) % 100 === 0) peakRssBytes = Math.max(peakRssBytes, process.memoryUsage().rss);
}
const runtimeMs = performance.now() - runStart;
peakRssBytes = Math.max(peakRssBytes, process.memoryUsage().rss);
const finalWorld = JSON.parse(engine.snapshot());
const finalState = JSON.parse(engine.save());
const wasmPeakBytes = wasm.memory.buffer.byteLength;
const onboardResidents =
  finalWorld.residents?.filter(
    (resident) => resident.train_id !== null && resident.train_id !== undefined,
  ) ?? [];
const transferStates = (finalWorld.residents ?? []).reduce((counts, resident) => {
  const state = resident.transfer ?? 'untracked';
  counts[state] = (counts[state] ?? 0) + 1;
  return counts;
}, {});
const output = {
  benchmark: 'mature-city-wasm-single-tick',
  runtime: {
    node: process.version,
    platform: process.platform,
    arch: process.arch,
    cpu: cpus()[0]?.model ?? 'unknown',
    logicalCpuCount: cpus().length,
    totalMemoryBytes: totalmem(),
  },
  configuration: {
    seed,
    developmentRate,
    matureStage: warmedSnapshot.stage,
    warmupTicks,
    warmupBatch,
    warmupMs,
    measuredTicks,
    targetTickRateHz: 20,
    targetTickBudgetMs: 50,
    earlyGrowthEvery1000Ticks: earlyGrowth,
  },
  run: {
    runtimeMs,
    durationMs: quantileReport(durations),
    memoryPeak: {
      wasmStartBytes,
      wasmPeakBytes,
      processRssSampledPeakBytes: peakRssBytes,
      rssSamplingIntervalTicks: 100,
    },
    counts: {
      buildings: finalWorld.buildings.length,
      roads: finalWorld.roads.length,
      parcels: finalWorld.parcels.length,
      residents: finalWorld.residents?.length ?? 0,
      trains: finalWorld.trains.length,
      onboardResidents: onboardResidents.length,
      trainPassengers: finalWorld.trains.reduce((sum, train) => sum + (train.passengers ?? 0), 0),
      trainCapacity: finalWorld.trains.reduce((sum, train) => sum + (train.capacity ?? 0), 0),
      stationWaiting: finalWorld.trains.reduce(
        (sum, train) => sum + (train.station_waiting ?? 0),
        0,
      ),
      gatewayWaiting: finalWorld.trains.reduce(
        (sum, train) => sum + (train.gateway_waiting ?? 0),
        0,
      ),
      transferStates,
      travelers: finalState.travelers?.length ?? 0,
      projects: finalState.projects.length,
      pendingProjects: finalState.projects.filter((project) => project.progress < 1).length,
      events: finalWorld.events.length,
      population: finalWorld.stats.population,
      tick: finalWorld.tick,
      stage: finalWorld.stage,
    },
  },
  note: 'Single calls are measured without pacing or rendering. Percentiles describe this runtime and hardware; no absolute performance threshold is asserted.',
};
engine.free();
process.stdout.write(`${JSON.stringify(output, null, 2)}\n`);
