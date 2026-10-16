// Matched public-WASM comparison. Keep raw numeric serialization so u64 RNG
// values are never rounded through JavaScript's JSON number representation.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import * as baseline from '../../validation/district-runtime-baseline/city_engine.js';
import * as candidate from '../../public/wasm/city_engine.js';
const hash = (x) => createHash('sha256').update(x).digest('hex');
const oldBytes = await readFile(
  new URL('../../validation/district-runtime-baseline/city_engine_bg.wasm', import.meta.url),
);
const newBytes = await readFile(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url));
if (hash(oldBytes) === hash(newBytes))
  throw Error('Candidate WASM must be rebuilt before this comparison');
baseline.initSync({ module: oldBytes });
candidate.initSync({ module: newBytes });
function worldWithoutMeasurement(save) {
  const parsed = JSON.parse(save);
  if (!Object.hasOwn(parsed, 'district_controller')) return save;
  if (Object.keys(parsed).at(-1) !== 'district_controller')
    throw Error('Update comparison for changed save field ordering');
  const offset = save.lastIndexOf(',"district_controller":');
  if (offset < 0) throw Error('Missing canonical controller field');
  return save.slice(0, offset) + '}';
}
const report = {
  baselineWasm: hash(oldBytes),
  candidateWasm: hash(newBytes),
  cases: [],
  note: 'Sequential single-run timings are observations, not a device performance benchmark.',
};
for (const seed of ['87231', 'seed0']) {
  const path = new URL(
    `../../validation/interior-access-verified-${seed}.json.final.json`,
    import.meta.url,
  );
  const source = await readFile(path, 'utf8');
  const old = baseline.CityEngine.restore(source),
    next = candidate.CityEngine.restore(source);
  const startTick = JSON.parse(old.save()).tick;
  let oldMs = 0,
    newMs = 0;
  try {
    for (let batch = 0; batch < 5; batch++) {
      let t = performance.now();
      old.step(2000);
      oldMs += performance.now() - t;
      t = performance.now();
      next.step(2000);
      newMs += performance.now() - t;
      if (worldWithoutMeasurement(old.save()) !== worldWithoutMeasurement(next.save()))
        throw Error(`${seed}: authoritative state changed at batch ${batch}`);
    }
    const save = next.save(),
      state = JSON.parse(save);
    if (!state.district_controller?.completed) throw Error('Runtime measurement did not complete');
    if (state.tick !== startTick + 10000) throw Error('Wrong comparison horizon');
    const resumed = candidate.CityEngine.restore(save);
    try {
      if (resumed.save() !== save) throw Error('Noncanonical runtime checkpoint');
      next.step(2000);
      resumed.step(2000);
      if (next.save() !== resumed.save()) throw Error('Runtime checkpoint replay diverged');
    } finally {
      resumed.free();
    }
    report.cases.push({
      seed: state.seed,
      startTick,
      endTick: state.tick,
      comparedTicks: 10000,
      rawWorldEqual: true,
      baselineStepMs: oldMs,
      candidateStepMs: newMs,
      saveBytes: Buffer.byteLength(save),
      controllerPresent: !!state.district_controller,
      completedCaptureTick: state.district_controller?.completed_capture_tick,
      saveSha256: hash(save),
    });
  } finally {
    old.free();
    next.free();
  }
}
await writeFile(
  new URL('../../validation/district-runtime-comparison.json', import.meta.url),
  JSON.stringify(report, null, 2) + '\n',
);
console.log(JSON.stringify(report, null, 2));
