import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

// Match the 300,000-tick endurance test; batch observations stay outside timing.
const output = process.argv[2];
if (!output) throw new Error('Usage: node scripts/profile-endurance-wasm.mjs OUTPUT.json');
const wasm = readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url));
initSync({ module: wasm });
const engine = new CityEngine(1204);
let restored;
const batches = [];
const measure = (fn) => {
  const start = performance.now();
  const value = fn();
  return { value, ms: performance.now() - start };
};
try {
  engine.set_development_rate(60);
  for (let batch = 0; batch < 150; batch++) {
    const { ms } = measure(() => engine.step(2000));
    batches.push({ tick: (batch + 1) * 2000, ms });
    if ((batch + 1) % 15 === 0) console.log(JSON.stringify(batches.at(-1)));
  }
  const snapshot = measure(() => engine.snapshot());
  const save = measure(() => engine.save());
  const restore = measure(() => CityEngine.restore(save.value));
  restored = restore.value;
  const resave = measure(() => restored.save());
  assert.equal(resave.value, save.value);
  const world = JSON.parse(snapshot.value);
  const state = JSON.parse(save.value);
  const counts = {
    buildings: world.buildings.length,
    roads: world.roads.length,
    parcels: world.parcels.length,
    projects: state.projects.length,
    pendingProjects: state.projects.filter((p) => p.progress < 1).length,
    travelers: state.travelers.length,
    events: state.events.length,
    population: state.population,
  };
  for (const [key, cap] of Object.entries({
    buildings: 2000,
    roads: 5000,
    parcels: 4000,
    projects: 76,
    pendingProjects: 12,
    travelers: 2000,
    events: 10000,
    population: 30000,
  }))
    assert.ok(counts[key] <= cap, key);
  const queued = state.residents.filter((r) =>
    ['waiting_station', 'waiting_gateway', 'waiting_bus'].includes(r.transfer),
  ).length;
  assert.equal(world.stats.active_travelers, counts.travelers + queued);
  const hash = (value) => createHash('sha256').update(value).digest('hex');
  const report = {
    scope: 'Node WASM endurance timings; no rendering or browser performance claim',
    node: process.version,
    seed: 1204,
    developmentRate: 60,
    ticks: 300000,
    nodeExecArgv: process.execArgv,
    wasmSha256: hash(wasm),
    finalSaveSha256: hash(save.value),
    saveBytes: Buffer.byteLength(save.value),
    counts,
    roundTripExact: true,
    simulationMs: batches.reduce((sum, batch) => sum + batch.ms, 0),
    snapshotMs: snapshot.ms,
    saveMs: save.ms,
    restoreMs: restore.ms,
    resaveMs: resave.ms,
    batches,
  };
  writeFileSync(output, JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify({ ...report, batches: undefined }, null, 2));
} finally {
  restored?.free();
  engine.free();
}
