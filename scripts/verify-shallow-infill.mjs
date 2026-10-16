import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
const wasm = readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url));
initSync({ module: wasm });
const input = 'validation/spine-ranking-seed-87231-by-sites-false.json';
const engine = CityEngine.restore(readFileSync(input, 'utf8'));
let restored;
try {
  const before = JSON.parse(engine.snapshot());
  engine.set_development_rate(16);
  // Public step calls are capped; advance in bounded batches.
  for (let i = 0; i < 100; i++) engine.step(200);
  const after = JSON.parse(engine.snapshot());
  assert.equal(after.tick - before.tick, 20000);
  const oldIds = new Set(before.parcels.map((p) => p.id));
  const added = after.parcels.filter((p) => !oldIds.has(p.id) && p.depth === 8);
  assert.ok(added.length > 0, 'new shallow plots must be admitted');
  const ids = new Set(added.map((p) => p.id));
  const buildings = after.buildings.filter((b) => ids.has(b.parcel_id) && b.stage === 'open');
  assert.equal(buildings.length, added.length, 'shallow construction completes');
  for (const parcel of added)
    assert.ok(
      after.roads.some(
        (r) =>
          r.connected &&
          r.stage === 'open' &&
          (r.from === parcel.access_node || r.to === parcel.access_node),
      ),
      'connected access',
    );
  const saved = engine.save();
  restored = CityEngine.restore(saved);
  assert.equal(restored.save(), saved);
  engine.step(400);
  restored.step(400);
  assert.equal(engine.save(), restored.save());
  const report = {
    wasmSha256: createHash('sha256').update(wasm).digest('hex'),
    input,
    startTick: before.tick,
    endTick: after.tick,
    newShallowParcels: added.length,
    openShallowBuildings: buildings.length,
    roadDelta: after.roads.length - before.roads.length,
    connectedAccess: true,
    canonicalRestore: true,
    replayTicks: 400,
  };
  writeFileSync(
    'validation/initial-repo-shallow-wasm.json',
    JSON.stringify(report, null, 2) + '\n',
  );
  console.log(JSON.stringify(report));
} finally {
  restored?.free();
  engine.free();
}
