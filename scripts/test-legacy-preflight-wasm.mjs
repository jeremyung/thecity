import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

if (!process.argv[2]) {
  throw new Error('Usage: node scripts/test-legacy-preflight-wasm.mjs <exported-legacy-save.json>');
}
const wasm = readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url));
initSync({ module: wasm });
const source = readFileSync(process.argv[2], 'utf8');
const path = ['live_access_checkpoint', 'planning', 'discovery', 'stage', 'Trace', 'trace', 'stage', 'Preflight'];
const preflightOf = (save) => path.reduce((value, key) => value[key], save);
const expectedWorld = JSON.parse(source);
assert.equal(preflightOf(expectedWorld).schema, 1, 'fixture must contain a legacy preflight');

const first = CityEngine.restore(source);
const second = CityEngine.restore(source);
let frozenNodes, frozenEdges;
try {
  const migrated = first.save();
  const actual = JSON.parse(migrated);
  const preflight = preflightOf(actual);
  assert.equal(preflight.schema, 2);
  assert.equal(preflight.stage, 'Nodes');
  assert.equal(preflight.node_cursor, 0);
  for (const key of ['nodes', 'edges', 'proposed_edge']) {
    assert.deepEqual(preflight[key], preflightOf(expectedWorld)[key]);
  }
  frozenNodes = preflight.nodes.length;
  frozenEdges = preflight.edges.length;
  delete expectedWorld.live_access_checkpoint;
  const actualWorld = structuredClone(actual);
  delete actualWorld.live_access_checkpoint;
  assert.deepEqual(actualWorld, expectedWorld);

  const roundtrip = CityEngine.restore(migrated);
  try { assert.equal(roundtrip.save(), migrated); } finally { roundtrip.free(); }
  for (let i = 0; i < 24; i++) {
    first.step(1);
    second.step(1);
  }
  assert.equal(first.save(), second.save());
  for (const change of ['future', 'corrupt']) {
    const bad = structuredClone(actual);
    const checkpoint = preflightOf(bad);
    if (change === 'future') checkpoint.schema = 3;
    else checkpoint.node_cursor = 4294967295;
    assert.throws(() => CityEngine.restore(JSON.stringify(bad)));
  }
} finally {
  first.free();
  second.free();
}
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const report = {
  scope: 'Node WASM legacy preflight migration, not browser UI',
  sourceSha256: sha256(source), wasmSha256: sha256(wasm),
  worldFieldsPreserved: true, migratedRoundtripExact: true,
  deterministicContinuationTicks: 24, futureAndCorruptCurrentRejected: true,
  frozenNodes, frozenEdges,
};
writeFileSync(new URL('../validation/legacy-preflight-wasm.json', import.meta.url), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify(report, null, 2));
