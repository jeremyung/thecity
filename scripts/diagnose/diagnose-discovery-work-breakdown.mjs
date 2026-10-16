import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { AccessDiscovery, initSync } from '../../public/wasm/city_engine.js';

initSync({
  module: readFileSync(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url)),
});
const save = readFileSync(
  new URL(
    '../../tests/fixtures/validation/early-reservation-wasm-87231-save.json',
    import.meta.url,
  ),
  'utf8',
);
const maxChunk = 65_536;

function stageName(job) {
  const checkpoint = JSON.parse(job.save());
  const outer = Object.keys(checkpoint.stage)[0];
  return outer;
}

function run() {
  let job = new AccessDiscovery(save);
  const workByStage = {};
  let total = 0;
  while (true) {
    const beforeStage = stageName(job);
    const before = job.save();
    const step = JSON.parse(job.advance(maxChunk));
    assert.ok(step.work_used > 0);
    assert.equal(step.error, null);
    const afterStage = stageName(job);
    if (beforeStage === afterStage) {
      workByStage[beforeStage] = (workByStage[beforeStage] ?? 0) + step.work_used;
      total += step.work_used;
      if (step.done) break;
      continue;
    }

    // Locate the first work unit crossing this top-level stage boundary.
    let low = 1,
      high = step.work_used;
    while (low < high) {
      const middle = low + Math.floor((high - low) / 2);
      const probe = AccessDiscovery.restore(before);
      const probeStep = JSON.parse(probe.advance(middle));
      const moved = stageName(probe) !== beforeStage;
      probe.free();
      if (moved) high = middle;
      else low = middle + 1;
      assert.ok(probeStep.work_used <= middle);
    }
    workByStage[beforeStage] = (workByStage[beforeStage] ?? 0) + low;
    total += low;
    job.free();
    job = AccessDiscovery.restore(before);
    const boundary = JSON.parse(job.advance(low));
    assert.equal(boundary.work_used, low);
    assert.notEqual(stageName(job), beforeStage);
    if (boundary.done) break;
  }
  const descriptorCount = JSON.parse(job.advance(0)).descriptor_count ?? 2069;
  const descriptorBits = JSON.parse(job.descriptor_bits());
  job.free();
  return {
    stages: workByStage,
    total_work: total,
    descriptor_count: descriptorBits.length,
    descriptor_count_reported: descriptorCount,
  };
}

const result = run();
assert.equal(result.total_work, 6_233_215);
assert.equal(result.descriptor_count, 2_069);
const report = {
  source_save: 'tests/fixtures/validation/early-reservation-wasm-87231-save.json',
  source_save_sha256: createHash('sha256').update(save).digest('hex'),
  wasm_binary: 'public/wasm/city_engine_bg.wasm',
  wasm_sha256: createHash('sha256')
    .update(readFileSync(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url)))
    .digest('hex'),
  chunk_work: maxChunk,
  ...result,
  scope:
    'WASM top-level stage accounting; binary-searches exact transition boundaries; does not alter discovery behavior',
};
writeFileSync(
  new URL('../../validation/access-discovery-wasm-work-breakdown.json', import.meta.url),
  JSON.stringify(report, null, 2) + '\n',
);
console.log(JSON.stringify(report, null, 2));
