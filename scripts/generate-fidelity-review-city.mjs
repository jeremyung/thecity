// Fresh, current-format review fixture: never repair an obsolete saved city.
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

const output = process.argv[2] ?? '.tools/fidelity-review';
const report = process.argv[3] ?? 'validation/fidelity-review-generation.json';
const seed = Number(process.argv[4] ?? 87231), developmentRate = 16, targetTick = 80000;
if (!Number.isInteger(seed) || seed < 0 || seed > 0xffffffff) throw new Error('seed must be a uint32');
const wasm = readFileSync('public/wasm/city_engine_bg.wasm');
initSync({ module: wasm });
mkdirSync(output, { recursive: true });
const engine = new CityEngine(seed);
engine.set_development_rate(developmentRate);
const checkpoints = [];
try {
  for (let tick = 2000; tick <= targetTick; tick += 2000) {
    engine.step(2000);
    if (tick % 10000 !== 0) continue;
    const saved = engine.save(), restored = CityEngine.restore(saved);
    try {
      assert.equal(restored.save(), saved, `Save equivalence at ${tick}`);
      assert.deepEqual(restored.movement_packet(), engine.movement_packet());
    } finally { restored.free(); }
    const world = JSON.parse(engine.snapshot()), state = JSON.parse(saved);
    const sample = { tick: world.tick, buildings: world.buildings.length,
      realizedLots: state.masterplan.lot_parcels.filter(Boolean).length,
      plannedLots: state.masterplan.lot_parcels.length,
      realizedRoads: state.masterplan.road_ids.filter(Boolean).length,
      plannedRoads: state.masterplan.road_ids.length };
    checkpoints.push(sample);
    writeFileSync(`${output}/tick-${tick}.json`, saved);
    console.log(JSON.stringify(sample));
  }
  const saved = engine.save(), restored = CityEngine.restore(saved);
  try {
    engine.step(80); restored.step(80);
    assert.equal(restored.save(), engine.save(), 'Live/restored continuation');
  } finally { restored.free(); }
  writeFileSync(report, JSON.stringify({
    seed, developmentRate, targetTick,
    wasmSha256: createHash('sha256').update(wasm).digest('hex'),
    scope: 'Fresh ordinary fixed-step growth; save equivalence every 10000 ticks and 80-tick live/restored continuation at the final checkpoint. Not browser scheduling or visual acceptance.',
    checkpoints,
  }, null, 2) + '\n');
} finally { engine.free(); }
