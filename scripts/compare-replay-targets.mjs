import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { initSync, CityEngine } from '../public/wasm/city_engine.js';

const [seedArg, rateArg, tickArg, intervalArg] = process.argv.slice(2);
const seed = Number(seedArg ?? 87231);
const rate = Number(rateArg ?? 16);
const target = Number(tickArg ?? 242000);
const interval = Number(intervalArg ?? 1000);
if (![seed, rate, target, interval].every(Number.isSafeInteger) || interval < 1 || target < 0) {
  throw new Error('Usage: node scripts/compare-replay-targets.mjs [seed rate targetTicks intervalTicks]');
}

const env = { ...process.env };
if (existsSync('.tools/cargo/bin')) {
  env.CARGO_HOME = resolve('.tools/cargo');
  env.RUSTUP_HOME = resolve('.tools/rustup');
  env.PATH = `${resolve('.tools/cargo/bin')}:${env.PATH}`;
}
const command = spawnSync('cargo', [
  'run', '--quiet', '--locked', '--release', '--manifest-path', 'engine/Cargo.toml',
  '--example', 'replay-trace', '--', String(seed), String(rate), String(target), String(interval),
], { encoding: 'utf8', env, maxBuffer: 128 * 1024 * 1024 });
if (command.error) throw command.error;
if (command.status !== 0) throw new Error(`native trace failed (${command.status}):\n${command.stderr}`);
const native = command.stdout.trim().split('\n').map((line) => JSON.parse(line));

initSync({ module: readFileSync(resolve('public/wasm/city_engine_bg.wasm')) });
const wasm = new CityEngine(seed);
wasm.set_development_rate(rate);
function compact() {
  const snapshot = JSON.parse(wasm.snapshot());
  const saved = wasm.save();
  const state = JSON.parse(saved);
  const exactU64 = (key) => saved.match(new RegExp(`"${key}":(\\d+)`))?.[1] ?? null;
  const slim = (entries, fields) => entries.map((entry) => Object.fromEntries(fields.map((field) => [field, entry[field]])));
  return {
    structures: {
      tick: snapshot.tick,
      stage: snapshot.stage,
      population: snapshot.stats.population,
      nodes: slim(snapshot.nodes, ['id', 'x', 'z', 'kind']),
      roads: slim(snapshot.roads, ['id', 'from', 'to', 'kind', 'stage', 'connected', 'length']),
      parcels: slim(snapshot.parcels, ['id', 'x', 'z', 'width', 'depth', 'access_node', 'protected', 'reservation']),
      buildings: slim(snapshot.buildings, ['id', 'parcel_id', 'x', 'z', 'width', 'depth', 'floors', 'kind', 'stage', 'progress', 'capacity', 'occupancy', 'built_day', 'anchor']),
      projects: slim(snapshot.projects, ['id', 'kind', 'from', 'to', 'parcel_id', 'stage', 'progress', 'duration', 'reason', 'started_tick']),
      development_rate: snapshot.development_rate,
    },
    internal: {
      rng_growth: exactU64('rng_growth'),
      rng_schedule: exactU64('rng_schedule'),
      rng_weather: exactU64('rng_weather'),
      development_accum: state.development_accum,
      growth_cursor: state.growth_cursor,
      road_origin_cursor: state.road_origin_cursor,
      road_infill_due: state.road_infill_due,
      frontage_road_cursor: state.frontage_road_cursor,
      frontage_slot_cursor: state.frontage_slot_cursor,
      neighborhood_plan: state.neighborhood_plan,
      demand: state.demand,
    },
  };
}
const wasmTrace = [compact()];
let elapsed = 0;
while (elapsed < target) {
  const delta = Math.min(interval, target - elapsed);
  wasm.step(delta);
  elapsed += delta;
  wasmTrace.push(compact());
}
wasm.free();

function isNumeric(value) { return typeof value === 'number' && Number.isFinite(value); }
function collectDiffs(a, b, path = '$', out = [], tol = 1e-4, limit = 12) {
  if (out.length >= limit) return out;
  if (isNumeric(a) && isNumeric(b)) {
    if (Math.abs(a - b) > tol) out.push({ path, native: a, wasm: b, delta: a - b });
    return out;
  }
  if (Object.is(a, b)) return out;
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) out.push({ path: `${path}.length`, native: a.length, wasm: b.length });
    for (let i = 0; i < Math.min(a.length, b.length) && out.length < limit; i++) collectDiffs(a[i], b[i], `${path}[${i}]`, out, tol, limit);
    return out;
  }
  if (a && b && typeof a === 'object' && typeof b === 'object') {
    const keys = [...new Set([...Object.keys(a), ...Object.keys(b)])].sort();
    for (const key of keys) {
      if (!(key in a) || !(key in b)) out.push({ path: `${path}.${key}`, native: a[key], wasm: b[key] });
      else collectDiffs(a[key], b[key], `${path}.${key}`, out, tol, limit);
      if (out.length >= limit) break;
    }
    return out;
  }
  out.push({ path, native: a, wasm: b });
  return out;
}

let first = null;
let structuralEqualWithinFloatToleranceThrough = 0;
for (let i = 0; i < Math.min(native.length, wasmTrace.length); i++) {
  const diffs = collectDiffs(native[i].structures, wasmTrace[i].structures);
  if (diffs.length) {
    const entityMatch = diffs[0].path.match(/^\$\.(nodes|roads|parcels|buildings|projects)\[(\d+)\]/);
    let entityDetails = null;
    if (entityMatch) {
      const [, collection, rawIndex] = entityMatch;
      const index = Number(rawIndex);
      const before = native[i].structures[collection]?.[index];
      const after = wasmTrace[i].structures[collection]?.[index];
      const accessNodes = collection === 'parcels' ? [before?.access_node, after?.access_node] : [];
      entityDetails = {
        collection,
        index,
        native: before,
        wasm: after,
        accessNodeGeometry: accessNodes.map((id) => ({
          id,
          native: native[i].structures.nodes.find((n) => n.id === id),
          wasm: wasmTrace[i].structures.nodes.find((n) => n.id === id),
        })),
      };
    }
    first = { index: i, tick: native[i].structures.tick, structuralDiffs: diffs,
      internalDiffs: collectDiffs(native[i].internal, wasmTrace[i].internal), entityDetails,
      previousCheckpoint: i > 0 ? { tick: native[i - 1].structures.tick,
        structuralDiffs: collectDiffs(native[i - 1].structures, wasmTrace[i - 1].structures),
        internalDiffs: collectDiffs(native[i - 1].internal, wasmTrace[i - 1].internal) } : null };
    break;
  }
  structuralEqualWithinFloatToleranceThrough = native[i].structures.tick;
}

const lastNative = native.at(-1);
const lastWasm = wasmTrace.at(-1);
const candidatePath = resolve('validation/mature-world.candidate.json');
let candidateComparison = null;
try {
  const candidate = JSON.parse(readFileSync(candidatePath, 'utf8'));
  candidateComparison = {
    tick: candidate.tick,
    buildingCount: candidate.buildings?.length,
    roadCount: candidate.roads?.length,
    sameSeed: candidate.seed === seed,
    sameTick: candidate.tick === target,
    freshWasmCounts: { buildings: lastWasm.structures.buildings.length, roads: lastWasm.structures.roads.length },
  };
} catch (error) {
  candidateComparison = { unavailable: String(error) };
}

const result = {
  generatedAt: new Date().toISOString(),
  parameters: { seed, developmentRate: rate, targetTick: target, checkpointInterval: interval, floatTolerance: 1e-4 },
  artifactHashes: {
    engineLibSha256: createHash('sha256').update(readFileSync(resolve('engine/src/lib.rs'))).digest('hex'),
    publicWasmSha256: createHash('sha256').update(readFileSync(resolve('public/wasm/city_engine_bg.wasm'))).digest('hex'),
  },
  nativeProfile: {
    platform: `${process.platform}-${process.arch}`,
    rustc: spawnSync('rustc', ['-Vv'], { encoding: 'utf8', env }).stdout.trim(),
    build: 'cargo run --locked --release --example replay-trace',
  },
  nativeCheckpoints: native.length,
  wasmCheckpoints: wasmTrace.length,
  structuralEqualWithinFloatToleranceThrough,
  firstDivergence: first,
  finalCounts: {
    native: { buildings: lastNative.structures.buildings.length, roads: lastNative.structures.roads.length, projects: lastNative.structures.projects.length, population: lastNative.structures.population },
    wasm: { buildings: lastWasm.structures.buildings.length, roads: lastWasm.structures.roads.length, projects: lastWasm.structures.projects.length, population: lastWasm.structures.population },
  },
  finalInternalDiffs: collectDiffs(lastNative.internal, lastWasm.internal),
  existingCandidate: candidateComparison,
  interpretation: 'Numeric geometry/progress fields are treated as equal within 1e-4; IDs, categories, and array order are compared exactly. The first reported mismatch is bounded by the checkpoint interval and does not prove where a latent branch began. When RNG and cursors still match, inspect candidate eligibility and ordering before attributing a later site difference to a stale build or to floating-point ties.',
};
const output = resolve('validation/replay-target-diagnostic.json');
writeFileSync(output, `${JSON.stringify(result, null, 2)}\n`);
console.log(`Wrote ${output}`);
console.log(JSON.stringify({ firstDivergence: result.firstDivergence, finalCounts: result.finalCounts, finalInternalDiffs: result.finalInternalDiffs, existingCandidate: candidateComparison }, null, 2));
