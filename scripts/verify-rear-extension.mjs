import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

const [input, output, completedSnapshot] = process.argv.slice(2);
if (!input || !output || input === output)
  throw new Error(
    'Usage: node scripts/verify-rear-extension.mjs pending-save.json distinct-report.json',
  );
const source = readFileSync(input);
const wasm = readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url));
const hash = (data) => createHash('sha256').update(data).digest('hex');
initSync({ module: wasm });
const engine = CityEngine.restore(source.toString());
let twin;
try {
  twin = CityEngine.restore(engine.save());
  const before = JSON.parse(engine.snapshot());
  const ids = before.rear_extensions.map((view) => view.building_id);
  assert(ids.length > 0 && ids.length <= 2, 'Input must have active rear extensions');
  const hosts = before.buildings.filter((building) => ids.includes(building.id));
  assert.equal(hosts.length, ids.length);
  let steps = 0;
  let after = before;
  while (steps < 4000) {
    engine.step(20);
    twin.step(20);
    steps += 20;
    assert.equal(engine.save(), twin.save(), 'WASM save/replay diverged');
    after = JSON.parse(engine.snapshot());
    for (const original of hosts) {
      const current = after.buildings.find((building) => building.id === original.id);
      assert.equal(current.stage, 'open');
      const stillPending = after.rear_extensions.some((view) => view.building_id === original.id);
      if (stillPending) {
        for (const field of ['x', 'z', 'width', 'depth', 'rotation', 'capacity'])
          assert.equal(current[field], original[field]);
      }
    }
    if (ids.every((id) => !after.rear_extensions.some((view) => view.building_id === id))) break;
  }
  const completed = hosts.map((original) => {
    const current = after.buildings.find((building) => building.id === original.id);
    assert(current.capacity > original.capacity, 'Extension did not complete within horizon');
    const retained = after.building_original_footprints.find(
      (form) => form.building_id === original.id,
    );
    const expected =
      before.building_original_footprints?.find((form) => form.building_id === original.id) ??
      original;
    assert(retained, 'Completed extension lost its original built form');
    for (const field of ['x', 'z', 'width', 'depth', 'rotation'])
      assert.equal(retained[field], expected[field]);
    for (const [coordinate, trig] of [
      ['x', Math.sin],
      ['z', Math.cos],
    ]) {
      assert(
        Math.abs(
          original[coordinate] +
            (trig(original.rotation) * original.depth) / 2 -
            current[coordinate] -
            (trig(current.rotation) * current.depth) / 2,
        ) < 0.001,
      );
    }
    return {
      id: original.id,
      beforeCapacity: original.capacity,
      afterCapacity: current.capacity,
      beforeDepth: original.depth,
      afterDepth: current.depth,
    };
  });
  const report = {
    sourceSha256: hash(source),
    wasmSha256: hash(wasm),
    sourceTick: before.tick,
    finalTick: after.tick,
    steps,
    hosts: completed,
    sameTargetReplay: true,
    noEarlyCapacity: true,
    nominalFrontPreserved: true,
    originalFormsPersisted: true,
    visualAcceptance: false,
  };
  const restored = CityEngine.restore(engine.save());
  try {
    assert.deepEqual(
      JSON.parse(restored.snapshot()).building_original_footprints,
      after.building_original_footprints,
    );
  } finally {
    restored.free();
  }
  writeFileSync(output, JSON.stringify(report, null, 2) + '\n');
  if (completedSnapshot) writeFileSync(completedSnapshot, JSON.stringify(after) + '\n');
  console.log(JSON.stringify(report, null, 2));
} finally {
  twin?.free();
  engine.free();
}
