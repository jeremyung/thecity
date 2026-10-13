import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

const ROOT = new URL('../', import.meta.url);
const wasm = readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url));
initSync({ module: wasm });

const SEEDS = [0, 1, 87231];
const HORIZON = 12_000;
const SAMPLE_EVERY = 200;
const DEVELOPMENT_RATE = 16;
const FIXTURE_PATH = '.tools/fidelity-review/road-staging-live-growth.json';
const OUTPUT_PATH = 'validation/building-access-gate-corpus.json';
const sha256 = value => createHash('sha256').update(value).digest('hex');
const witnesses = [];
let checks = 0;
let buildingObservations = 0;
let endpointChecks = 0;
let failures = 0;
const addFailure = witness => {
  failures++;
  if (witnesses.length < 8) witnesses.push(witness);
};

function inspect(world, context) {
  const buildings = world.buildings ?? [];
  const access = world.building_access ?? [];
  const nodes = new Set((world.nodes ?? []).map(node => node.id));
  const roadsById = new Map((world.roads ?? []).map(road => [road.id, road]));
  const accessByBuilding = new Map();
  for (const entry of access) {
    const entries = accessByBuilding.get(entry.building_id) ?? [];
    entries.push(entry);
    accessByBuilding.set(entry.building_id, entries);
  }
  for (const building of buildings) {
    checks++;
    buildingObservations++;
    const entries = accessByBuilding.get(building.id) ?? [];
    if (entries.length === 0) {
      addFailure({ ...context, reason: 'missing_building_access', building_id: building.id });
      continue;
    }
    if (entries.length !== 1) addFailure({ ...context, reason: 'ambiguous_building_access', building_id: building.id, entries: entries.length });
    for (const entry of entries) {
      const road = roadsById.get(entry.road_id);
      if (!road) {
        addFailure({ ...context, reason: 'missing_access_road', building_id: building.id, road_id: entry.road_id });
        continue;
      }
      if (road.stage !== 'open' || road.connected !== true) {
        addFailure({ ...context, reason: 'access_road_not_open_connected', building_id: building.id, road_id: road.id, road_stage: road.stage, connected: road.connected });
      }
      endpointChecks += 2;
      if (!nodes.has(road.from) || !nodes.has(road.to)) {
        addFailure({ ...context, reason: 'missing_road_endpoint', building_id: building.id, road_id: road.id,
          from: road.from, from_exists: nodes.has(road.from), to: road.to, to_exists: nodes.has(road.to) });
      }
    }
  }
  return { tick: world.tick, buildings: buildings.length, accessEntries: access.length };
}

const runs = [];
for (const seed of SEEDS) {
  const engine = new CityEngine(seed);
  try {
    engine.set_development_rate(DEVELOPMENT_RATE);
    let state = JSON.parse(engine.snapshot());
    const startTick = state.tick;
    const samples = [inspect(state, { source: 'seeded', seed, tick: state.tick })];
    for (let advanced = 0; advanced < HORIZON; advanced += SAMPLE_EVERY) {
      engine.step(SAMPLE_EVERY);
      state = JSON.parse(engine.snapshot());
      samples.push(inspect(state, { source: 'seeded', seed, tick: state.tick }));
    }
    assert.equal(state.tick - startTick, HORIZON, `seed ${seed}: wrong simulation horizon`);
    runs.push({ seed, startTick, finalTick: state.tick, developmentRate: DEVELOPMENT_RATE,
      sampledEveryTicks: SAMPLE_EVERY, sampleCount: samples.length, finalBuildings: state.buildings.length,
      finalAccessEntries: state.building_access?.length ?? 0 });
  } finally { engine.free(); }
}

const fixtureBytes = readFileSync(FIXTURE_PATH);
const fixtureEngine = CityEngine.restore(fixtureBytes.toString());
let fixtureSummary;
try {
  const fixtureWorld = JSON.parse(fixtureEngine.snapshot());
  fixtureSummary = { path: FIXTURE_PATH, sha256: sha256(fixtureBytes), tick: fixtureWorld.tick,
    seed: fixtureWorld.seed, stage: fixtureWorld.stage, buildings: fixtureWorld.buildings.length,
    accessEntries: fixtureWorld.building_access?.length ?? 0 };
  inspect(fixtureWorld, { source: 'restored_fixture', seed: fixtureWorld.seed, tick: fixtureWorld.tick });
} finally { fixtureEngine.free(); }

const report = {
  schemaVersion: 1,
  gate: 'building_access points to exactly one open connected road; referenced road endpoints exist',
  settings: { seeds: SEEDS, horizonTicks: HORIZON, sampleEveryTicks: SAMPLE_EVERY, developmentRate: DEVELOPMENT_RATE,
    startingState: 'new CityEngine(seed), development rate set before advancing', fixture: FIXTURE_PATH },
  inputs: { wasmSha256: sha256(wasm), fixture: fixtureSummary },
  runs,
  counts: { checks, buildingObservations, endpointChecks, failures },
  missingWitnesses: witnesses,
  passed: failures === 0,
};
mkdirSync('validation', { recursive: true });
writeFileSync(OUTPUT_PATH, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ output: OUTPUT_PATH, passed: report.passed, counts: report.counts,
  runs: runs.map(({ seed, finalTick, finalBuildings }) => ({ seed, finalTick, finalBuildings })), witnesses }));
if (!report.passed) process.exitCode = 1;
