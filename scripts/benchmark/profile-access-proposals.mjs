// Requires npm run wasm:legacy; production WASM deliberately excludes these APIs.
import { readFileSync, writeFileSync } from 'node:fs';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import { AccessProposalWorld, initSync } from '../../.tools/wasm-legacy/city_engine.js';
initSync({
  module: readFileSync(new URL('../../.tools/wasm-legacy/city_engine_bg.wasm', import.meta.url)),
});
const read = (name, directory = 'tests/fixtures/validation') =>
  JSON.parse(readFileSync(new URL(`../../${directory}/${name}`, import.meta.url), 'utf8'));
const reference = read('access-descriptor-reference.json');
const validationBits = [
  ...reference.ordered_descriptor_bits,
  read('accepted-access-candidate.json').bits,
];
const native = read('incremental-proposal-validation.json', 'validation')[0].proposals;
const u = new Uint32Array(1),
  f = new Float32Array(u.buffer);
const decode = (bits) => {
  u[0] = bits;
  return f[0];
};
const begin = performance.now();
const world = new AccessProposalWorld(
  readFileSync(
    new URL(
      '../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json',
      import.meta.url,
    ),
    'utf8',
  ),
);
const captureMs = performance.now() - begin;
const runs = [];
let baseline;
try {
  for (const quota of [48, 4096]) {
    let work = 0,
      restores = 0,
      maxCallMs = 0;
    const proposals = [];
    for (const [index, row] of validationBits.entries()) {
      let query = world.query(
        JSON.stringify({
          host_id: row[0],
          host_t: decode(row[1]),
          end_x: decode(row[2]),
          end_z: decode(row[3]),
          tie_index: index % 48,
        }),
      );
      let sinceSave = 0;
      try {
        assert.equal(JSON.parse(query.advance(0)).work_used, 0);
        for (;;) {
          const start = performance.now();
          const step = JSON.parse(query.advance(quota));
          maxCallMs = Math.max(maxCallMs, performance.now() - start);
          assert.ok(step.work_used <= quota);
          work += step.work_used;
          sinceSave += step.work_used;
          assert.ok(work < 20_000_000);
          if (step.done || sinceSave >= 4093) {
            const checkpoint = query.save();
            const restored = world.restore_query(checkpoint);
            query.free();
            query = restored;
            sinceSave = 0;
            restores++;
          }
          if (step.done) break;
          assert.ok(step.work_used > 0);
        }
        const result = JSON.parse(query.result());
        assert.equal(result.done, true);
        if (result.proposal) proposals.push(result.proposal);
      } finally {
        query.free();
      }
    }
    if (baseline) assert.deepEqual(proposals, baseline);
    else baseline = proposals;
    runs.push({ quota, work, restores, max_call_ms: maxCallMs, proposals });
  }
} finally {
  world.free();
}
assert.equal(runs[0].work, runs[1].work);
const report = {
  wasm_sha256: createHash('sha256')
    .update(readFileSync(new URL('../../.tools/wasm-legacy/city_engine_bg.wasm', import.meta.url)))
    .digest('hex'),
  scope: 'Node WASM proposal validation; not browser render timing or live scheduler',
  capture_including_city_restore_ms: captureMs,
  native_proposal_equal: JSON.stringify(native) === JSON.stringify(baseline),
  runs,
};
// JSON key order is not meaningful; use structural comparison for parity.
try {
  assert.deepEqual(baseline, native);
  report.native_proposal_equal = true;
} catch {
  report.native_proposal_equal = false;
}
writeFileSync(
  new URL('../../validation/access-proposal-wasm-profile.json', import.meta.url),
  JSON.stringify(report, null, 2) + '\n',
);
console.log(JSON.stringify(report, null, 2));
if (!report.native_proposal_equal) process.exitCode = 1;
