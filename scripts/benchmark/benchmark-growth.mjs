import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { cpus } from 'node:os';
import { performance } from 'node:perf_hooks';
import { CityEngine, initSync } from '../../public/wasm/city_engine.js';

const output = process.argv[2];
if (!output) throw new Error('Usage: node scripts/benchmark/benchmark-growth.mjs output.json');
const wasm = await readFile(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url));
initSync({ module: wasm });
const engine = new CityEngine(87231);
const windows = [];
const windowTicks = 12000;
try {
  engine.set_development_rate(16);
  for (let startTick = 0; startTick < 72000; startTick += windowTicks) {
    const samples = [];
    let maxMs = -1,
      maxTick = 0;
    for (let offset = 1; offset <= windowTicks; offset++) {
      const start = performance.now();
      engine.step(1);
      const elapsed = performance.now() - start;
      samples.push(elapsed);
      if (elapsed > maxMs) {
        maxMs = elapsed;
        maxTick = startTick + offset;
      }
    }
    const world = JSON.parse(engine.snapshot());
    samples.sort((a, b) => a - b);
    const percentile = (p) => samples[Math.ceil(samples.length * p) - 1];
    windows.push({
      startTick,
      endTick: startTick + windowTicks,
      p50Ms: percentile(0.5),
      p95Ms: percentile(0.95),
      p99Ms: percentile(0.99),
      maxMs,
      maxTick,
      totalStepMs: samples.reduce((a, b) => a + b, 0),
      buildings: world.buildings.length,
      roads: world.roads.length,
      explicitFootprintParcels: world.parcels.filter((parcel) => Number.isFinite(parcel.rotation))
        .length,
    });
  }
  const report = {
    capturedAt: new Date().toISOString(),
    runtime: process.version,
    cpu: cpus()[0]?.model,
    logicalCpus: cpus().length,
    wasmSha256: createHash('sha256').update(wasm).digest('hex'),
    finalSaveSha256: createHash('sha256').update(engine.save()).digest('hex'),
    seed: 87231,
    developmentRate: 16,
    measuredTicks: 72000,
    windows,
    note: 'Node-hosted public WASM, sequential single-tick calls, no rendering or pacing. Includes startup/JIT effects. One run is not a browser/device acceptance benchmark; compare repeated runs and state hashes.',
  };
  await writeFile(output, `${JSON.stringify(report, null, 2)}\n`);
  console.log(
    JSON.stringify({
      output,
      finalSaveSha256: report.finalSaveSha256,
      totalStepMs: windows.reduce((sum, w) => sum + w.totalStepMs, 0),
    }),
  );
} finally {
  engine.free();
}
