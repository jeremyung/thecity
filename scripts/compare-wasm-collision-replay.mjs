import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import assert from 'node:assert/strict';

// Usage: node scripts/compare-wasm-collision-replay.mjs BASELINE_DIRECTORY
// Alternate execution order to reduce timing bias. Timings are Node/WASM
// measurements, not browser frame rates, and include no Inspector profiling.
const baselineDirectory = process.argv[2];
if (!baselineDirectory) throw new Error('A preserved baseline JS/WASM directory is required');
const hash = (value) => createHash('sha256').update(value).digest('hex');
async function load(directory, label) {
  const bytes = readFileSync(resolve(directory, 'city_engine_bg.wasm'));
  const source = readFileSync(resolve(directory, 'city_engine.js'));
  const api = await import(`data:text/javascript;base64,${source.toString('base64')}#${label}`);
  api.initSync({ module: bytes });
  const engine = new api.CityEngine(1204);
  engine.set_development_rate(60);
  return { api, engine, wasmSha256: hash(bytes), samples: [] };
}
const before = await load(baselineDirectory, 'before');
const after = await load('public/wasm', 'after');
assert.notEqual(before.wasmSha256, after.wasmSha256, 'Comparison requires different WASM builds');
const checkpoints = [];
try {
  for (let batch = 1; batch <= 150; batch++) {
    for (const run of batch % 2 ? [before, after] : [after, before]) {
      const started = performance.now();
      run.engine.step(2000);
      run.samples.push(performance.now() - started);
    }
    if (batch === 1 || batch % 10 === 0) {
      const saved = before.engine.save();
      const snapshot = before.engine.snapshot();
      assert.equal(after.engine.save(), saved, `Save diverged at batch ${batch}`);
      assert.equal(after.engine.snapshot(), snapshot, `Snapshot diverged at batch ${batch}`);
      assert.deepEqual(after.engine.movements(), before.engine.movements(), `Movement diverged at batch ${batch}`);
      checkpoints.push({ tick: batch * 2000, saveSha256: hash(saved), snapshotSha256: hash(snapshot) });
      console.log(`Replay equal at tick ${batch * 2000}`);
    }
  }
  for (const run of [before, after]) {
    const restored = run.api.CityEngine.restore(run.engine.save());
    try {
      assert.equal(restored.save(), run.engine.save());
      restored.step(2000);
      run.engine.step(2000);
      assert.equal(restored.save(), run.engine.save());
      assert.equal(restored.snapshot(), run.engine.snapshot());
    } finally { restored.free(); }
  }
  assert.equal(before.engine.save(), after.engine.save());
  const summary = (run) => {
    const sorted = [...run.samples].sort((a, b) => a - b);
    return { wasmSha256: run.wasmSha256, totalStepMs: run.samples.reduce((a, b) => a + b, 0),
      p50BatchMs: sorted[74], p95BatchMs: sorted[142], maxBatchMs: sorted[149], batchMs: run.samples };
  };
  const report = { workload: { seed: 1204, developmentRate: 60, batches: 150, ticksPerBatch: 2000 },
    runtime: { node: process.version, platform: process.platform, arch: process.arch },
    note: 'Alternating sequential Node/WASM runs on a shared host; not browser performance. All checkpoints compare complete save, snapshot, and movement output.',
    checkpoints, restoredContinuationEqual: true, before: summary(before), after: summary(after) };
  writeFileSync('validation/collision-wasm-replay-comparison.json', `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify({ beforeStepMs: report.before.totalStepMs, afterStepMs: report.after.totalStepMs, checkpoints: checkpoints.length }));
} finally {
  before.engine.free();
  after.engine.free();
}
