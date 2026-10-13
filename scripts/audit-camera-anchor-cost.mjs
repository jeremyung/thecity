// Bounded Node CPU A/B of CameraDirector's pure snapshot work. This does not
// instantiate controls or run the browser renderer.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import os from 'node:os';
import { build } from 'esbuild';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import { performance } from 'node:perf_hooks';

const savePath = '.tools/movement-support/browser/growth-575.json';
const beforePath = '.tools/camera-cost/before.mjs';
const afterPath = '.tools/camera-cost/after.mjs';
const outputPath = 'validation/camera-anchor-cost.json';
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const quantile = (values, q) => [...values].sort((a, b) => a - b)[Math.min(values.length - 1, Math.floor(values.length * q))];
const summarize = values => ({
  n: values.length,
  p50Ms: quantile(values, .5),
  p95Ms: quantile(values, .95),
  maxMs: Math.max(...values),
});

const [saveBytes, wasmBytes, beforeBundle] = await Promise.all([
  readFile(savePath), readFile('public/wasm/city_engine_bg.wasm'), readFile(beforePath),
]);
await mkdir('.tools/camera-cost', { recursive: true });
await build({ entryPoints: ['web/src/camera/director.ts'], bundle: true,
  platform: 'node', format: 'esm', packages: 'external', outfile: afterPath });
const [{ CameraDirector: Before }, { CameraDirector: After }] = await Promise.all([
  import('../.tools/camera-cost/before.mjs'), import('../.tools/camera-cost/after.mjs'),
]);
initSync({ module: wasmBytes });

function snapshotSeries() {
  const engine = CityEngine.restore(saveBytes.toString());
  try {
    engine.set_development_rate(16);
    const worlds = [JSON.parse(engine.snapshot())];
    // Four evenly spaced observations from the real engine after restore. A
    // bounded 8,000-tick window is enough to include ordinary project growth.
    for (let i = 0; i < 4; i++) {
      engine.step(2_000);
      worlds.push(JSON.parse(engine.snapshot()));
    }
    return worlds;
  } finally { engine.free(); }
}

const worlds = snapshotSeries();
function makeHarness(Director) {
  return Object.create(Director.prototype);
}
function normalize(value) {
  if (value === null || typeof value !== 'object') return value;
  if (value.isVector3) return { x: value.x, y: value.y, z: value.z };
  if (Array.isArray(value)) return value.map(normalize);
  return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, normalize(item)]));
}
function measure(Director, world, rounds = 3) {
  const director = makeHarness(Director);
  const timings = { obstacleEnvelopes: [], buildAnchors: [], developedExtent: [] };
  let result;
  for (let round = 0; round < rounds + 1; round++) {
    for (const [name, method] of [
      ['obstacleEnvelopes', 'cameraObstacleEnvelopes'],
      ['buildAnchors', 'buildAnchors'],
      ['developedExtent', 'developedExtent'],
    ]) {
      const start = performance.now();
      const output = director[method](world);
      const elapsed = performance.now() - start;
      if (round > 0) timings[name].push(elapsed);
      if (name === 'buildAnchors') result = normalize(output);
    }
  }
  return { timings, result };
}

const perWorld = [];
const all = { before: { obstacleEnvelopes: [], buildAnchors: [], developedExtent: [] },
  after: { obstacleEnvelopes: [], buildAnchors: [], developedExtent: [] } };
for (const world of worlds) {
  const before = measure(Before, world), after = measure(After, world);
  assert.deepEqual(after.result, before.result,
    `Anchor output changed at tick ${world.tick}; A/B must preserve exact anchor fields and order`);
  for (const version of ['before', 'after']) {
    const sample = version === 'before' ? before : after;
    for (const method of Object.keys(all[version])) all[version][method].push(...sample.timings[method]);
  }
  perWorld.push({ tick: world.tick, buildings: world.buildings.length, nodes: world.nodes.length,
    roads: world.roads.length, anchors: before.result.length, exactAnchorParity: true });
}

const report = {
  schemaVersion: 1,
  scope: 'Bounded Node CPU comparison of production CameraDirector snapshot methods on restored WASM growth worlds; no browser, renderer, GPU, frame-time, or camera-movement claim.',
  host: { cpu: os.cpus()[0]?.model ?? 'unknown', platform: os.platform(), arch: os.arch(), node: process.version },
  inputs: { savePath, saveSha256: sha256(saveBytes), wasmSha256: sha256(wasmBytes),
    baselineBundle: beforePath, baselineBundleSha256: sha256(beforeBundle), candidateBundle: afterPath,
    advancement: 'Restore seed-87231 tick-22026 snapshot, set development rate 16, take four more snapshots at 2,000-tick intervals.' },
  measurements: { roundsPerWorldPerVersion: 3, methods: Object.fromEntries(
    Object.keys(all.before).map(method => [method, { before: summarize(all.before[method]), after: summarize(all.after[method]) }])) },
  worlds: perWorld,
  parity: { exactNormalizedAnchorOutputPerWorld: true, worldCount: perWorld.length },
  limitations: [
    'Uses object-created CameraDirector prototypes and invokes only the three selected methods; it does not instantiate OrbitControls or call world().',
    'Node timings are CPU-only and are not representative of browser main-thread contention, garbage collection over long sessions, or GPU work.',
    'The baseline is the preserved .tools/camera-cost/before.mjs bundle; only current source is bundled during this run.',
  ],
};
await writeFile(outputPath, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ outputPath, worlds: perWorld, measurements: report.measurements }, null, 2));
