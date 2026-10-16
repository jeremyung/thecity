import { readFileSync, writeFileSync } from 'node:fs';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { performance } from 'node:perf_hooks';
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
const referencePath =
  process.env.ACCESS_REFERENCE_PATH ||
  '../../tests/fixtures/validation/access-descriptor-reference.json';
const referenceText = readFileSync(new URL(referencePath, import.meta.url), 'utf8');
const reference = JSON.parse(referenceText);
const runs = [];
let priorWork;
let wasmBaseline;
function compareBits(actual, expected) {
  let mismatchCount = 0,
    maxCoordinateDelta = 0;
  const examples = [];
  const buf = new ArrayBuffer(4),
    ints = new Uint32Array(buf),
    floats = new Float32Array(buf);
  const decode = (bits) => {
    ints[0] = bits;
    return floats[0];
  };
  for (let i = 0; i < Math.max(actual.length, expected.length); i++) {
    if (JSON.stringify(actual[i]) === JSON.stringify(expected[i])) continue;
    mismatchCount++;
    if (examples.length < 4) examples.push({ index: i, wasm: actual[i], native: expected[i] });
    if (actual[i] && expected[i])
      for (const j of [2, 3])
        maxCoordinateDelta = Math.max(
          maxCoordinateDelta,
          Math.abs(decode(actual[i][j]) - decode(expected[i][j])),
        );
  }
  return {
    exact: mismatchCount === 0,
    mismatching_descriptors: mismatchCount,
    max_coordinate_delta: maxCoordinateDelta,
    examples,
  };
}
for (const { quota, checkpointing } of [
  { quota: 4096, checkpointing: false },
  ...[48, 4096, 16384].map((quota) => ({ quota, checkpointing: true })),
]) {
  const start = performance.now();
  let job = new AccessDiscovery(save);
  const constructMs = performance.now() - start;
  let work = 0,
    sinceSave = 0,
    restores = 0,
    maxBytes = 0;
  const durations = [];
  try {
    assert.equal(JSON.parse(job.advance(0)).work_used, 0);
    for (;;) {
      const stepStart = performance.now();
      const step = JSON.parse(job.advance(quota));
      durations.push(performance.now() - stepStart);
      assert.ok(step.work_used <= quota);
      assert.equal(step.error, null);
      work += step.work_used;
      sinceSave += step.work_used;
      assert.ok(work < 25_000_000, 'bounded discovery completion');
      if (checkpointing && (step.done || sinceSave >= 262144)) {
        const checkpoint = job.save();
        maxBytes = Math.max(maxBytes, Buffer.byteLength(checkpoint));
        const restored = AccessDiscovery.restore(checkpoint);
        job.free();
        job = restored;
        restores++;
        sinceSave = 0;
      }
      if (step.done) break;
      assert.ok(step.work_used > 0);
    }
    const bits = JSON.parse(job.descriptor_bits());
    const nativeComparison = compareBits(bits, reference.ordered_descriptor_bits);
    if (wasmBaseline)
      assert.deepEqual(bits, wasmBaseline, `WASM quota/save-resume parity at ${quota}`);
    else wasmBaseline = bits;
    if (priorWork !== undefined) assert.equal(work, priorWork);
    priorWork = work;
    durations.sort((a, b) => a - b);
    runs.push({
      quota,
      checkpointing,
      native_comparison: nativeComparison,
      wasm_quota_parity: true,
      work,
      restores,
      max_checkpoint_bytes: maxBytes,
      descriptor_count: bits.length,
      construct_including_city_restore_ms: constructMs,
      calls: durations.length,
      advance_median_ms: durations[Math.floor(durations.length / 2)],
      advance_p95_ms: durations[Math.floor(durations.length * 0.95)],
      advance_max_ms: durations.at(-1),
      elapsed_including_persistence_ms: performance.now() - start,
    });
  } finally {
    job.free();
  }
}
const report = {
  reference_path: referencePath,
  reference_sha256: createHash('sha256').update(referenceText).digest('hex'),
  wasm_sha256: createHash('sha256')
    .update(readFileSync(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url)))
    .digest('hex'),
  scope:
    'Node WASM, not browser rendering or worker performance; advance includes WASM boundary and status JSON; construction includes city restore',
  runs,
};
writeFileSync(
  new URL('../../validation/access-discovery-wasm-profile.json', import.meta.url),
  JSON.stringify(report, null, 2) + '\n',
);
console.log(JSON.stringify(report, null, 2));

if (runs.some((run) => !run.native_comparison.exact)) process.exitCode = 1;
