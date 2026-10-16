import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { CityEngine, initSync } from '../../public/wasm/city_engine.js';

const wasm = readFileSync(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url));
const source = readFileSync(
  new URL('../../validation/early-reservation-wasm-87231-save.json', import.meta.url),
  'utf8',
);
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
initSync({ module: wasm });
const uninterrupted = CityEngine.restore(source);
let resumed = CityEngine.restore(source);
let sawJob = false;
try {
  for (let i = 0; i < 200; i++) {
    uninterrupted.step(1);
    resumed.step(1);
    if (i === 0) {
      sawJob = !!JSON.parse(uninterrupted.save()).live_access_checkpoint;
      assert.ok(sawJob, 'fixture must exercise live capture on its first step');
    }
    if (i % 17 === 0) {
      const saved = resumed.save();
      resumed.free();
      resumed = CityEngine.restore(saved);
      assert.equal(resumed.save(), saved);
    }
  }
  const final = uninterrupted.save();
  assert.equal(resumed.save(), final);
  const report = {
    scope: 'Node WASM live paired-capture startup and 200-tick save/resume; not browser timing',
    wasmSha256: hash(wasm),
    sourceSha256: hash(source),
    jobStartedOnFirstStep: sawJob,
    resumeEveryTicks: 17,
    steps: 200,
    finalSaveEqual: true,
    finalSaveSha256: hash(final),
  };
  writeFileSync(
    new URL('../../validation/shared-capture-wasm.json', import.meta.url),
    JSON.stringify(report, null, 2) + '\n',
  );
  console.log(JSON.stringify(report, null, 2));
} finally {
  uninterrupted.free();
  resumed.free();
}
