// Same-target live/reloaded replay; optionally verify native construction saves.
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
initSync({ module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)) });
const kinds = ['building', 'road', 'rail', 'expressway'];
for (const seed of [0, 87231]) {
  const live = new CityEngine(seed);
  try {
    live.set_development_rate(16);
    const covered = new Set();
    while (JSON.parse(live.save()).tick < 160000 && covered.size < kinds.length) {
      const saved = live.save();
      const state = JSON.parse(saved);
      const active = [
        ...new Set(
          state.projects
            .filter(
              (p) =>
                p.progress > 0 && p.progress < 1 && kinds.includes(p.kind) && !covered.has(p.kind),
            )
            .map((p) => p.kind),
        ),
      ];
      if (!active.length) {
        live.step(80);
        continue;
      }
      const restored = CityEngine.restore(saved);
      try {
        assert.equal(restored.save(), saved);
        for (let i = 0; i < 20; i++) {
          live.step(80);
          restored.step(80);
          assert.equal(live.save(), restored.save(), `seed ${seed}, tick ${state.tick}, ${active}`);
        }
      } finally {
        restored.free();
      }
      active.forEach((kind) => covered.add(kind));
    }
    assert.deepEqual([...covered].sort(), [...kinds].sort());
    for (const field of ['node_ids', 'road_ids', 'lot_parcels']) {
      for (const replacement of [0, 4294967295]) {
        const broken = JSON.parse(live.save());
        const index = broken.masterplan[field].findIndex((id) => id !== 0);
        broken.masterplan[field][index] = replacement;
        assert.throws(() => {
          const unexpected = CityEngine.restore(JSON.stringify(broken));
          unexpected.free();
        }, `${field} accepted invalid mapping ${replacement}`);
      }
    }
    console.log(`seed ${seed}: four construction lifecycles replayed; invalid mappings rejected`);
  } finally {
    live.free();
  }
}
const fixtureDirectory = process.argv[2];
if (fixtureDirectory) {
  const files = readdirSync(fixtureDirectory).filter((name) =>
    /^seed-\d+-(building|road|rail|expressway)\.json$/.test(name),
  );
  assert.equal(files.length, 8, 'expected two native seeds × four construction kinds');
  for (const file of files) {
    const source = readFileSync(join(fixtureDirectory, file), 'utf8');
    const restored = CityEngine.restore(source);
    try {
      assert.deepEqual(JSON.parse(restored.save()), JSON.parse(source));
      restored.step(80);
    } finally {
      restored.free();
    }
  }
  console.log(`${files.length} native construction saves validated and stepped in WASM`);
}
