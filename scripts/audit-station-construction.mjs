// Ordinary seeded growth: observe every station phase, save and replay each boundary.
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
const wasm = readFileSync('public/wasm/city_engine_bg.wasm');
initSync({ module: wasm });
const directory = '.tools/station-construction';
mkdirSync(directory, { recursive: true });
const seeds = [];
for (const seed of [87231, 0, 5]) {
  const live = new CityEngine(seed), phases = [];
  live.set_development_rate(16);
  try {
    for (let step = 0; step < 2000; step++) {
      const world = JSON.parse(live.snapshot());
      const phase = world.rail_platforms[0]?.construction_phase;
      assert.notEqual(phase, undefined);
      assert.ok(world.rail_platforms.every(p => p.construction_phase === phase));
      if (!phases.some(p => p.phase === phase)) {
        const saved = live.save(), state = JSON.parse(saved);
        const project = world.projects.find(p => p.kind === 'station');
        const restored = CityEngine.restore(saved);
        writeFileSync(`${directory}/${seed}-phase-${phase}.json`, saved);
        phases.push({ phase, tick: world.tick, progress: project?.progress ?? 1,
          stage: project?.stage ?? 'complete', ready: state.station_ready,
          online: state.station_online, presentationRevision: world.presentation_revision });
        if (phase < 4) assert.equal(state.station_ready, false);
        try {
          assert.deepEqual(JSON.parse(restored.snapshot()).rail_platforms, world.rail_platforms);
          live.step(1); restored.step(1);
          assert.equal(live.save(), restored.save());
          assert.deepEqual(live.movement_packet(), restored.movement_packet());
        } finally { restored.free(); }
      } else live.step(1);
      if (phase === 4) break;
    }
    assert.deepEqual(phases.map(p => p.phase), [0,1,2,3,4]);
    seeds.push({ seed, phases });
  } finally { live.free(); }
}
writeFileSync('validation/station-construction-current.json', JSON.stringify({
  wasmSha256: createHash('sha256').update(wasm).digest('hex'),
  method: 'Unmodified ordinary growth; current-save and one-step replay at each phase. No browser or performance acceptance.',
  seeds,
}, null, 2) + '\n');
console.log(JSON.stringify(seeds));
