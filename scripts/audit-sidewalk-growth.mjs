// Offline worker/render scheduler harness for sidewalk rebuild starvation.
// Usage: node scripts/audit-sidewalk-growth.mjs [growth16-save.json] [output.json]
import { build } from 'esbuild';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import os from 'node:os';
import { performance } from 'node:perf_hooks';
import * as THREE from 'three/webgpu';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

const fixture = process.argv[2] ?? '.tools/fidelity-review/isolated-ground-growth.json';
const output = process.argv[3] ?? 'validation/sidewalk-growth-scheduling.json';
const fixtureBytes = await readFile(fixture);
const save = JSON.parse(fixtureBytes.toString());
const wasmBytes = await readFile(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url));
initSync({ module: wasmBytes });
await build({ stdin: { contents: "export {CitySidewalks} from './web/src/render/sidewalks';", resolveDir: process.cwd() }, bundle: true, platform: 'node', format: 'esm', packages: 'external', outfile: '.tools/city-sidewalk-growth-harness.mjs' });
const { CitySidewalks } = await import('../.tools/city-sidewalk-growth-harness.mjs');

// Restore the exact persisted JSON bytes; the WASM validator also preserves
// the source encoding of its saved master-plan progress checkpoint.
const engine = CityEngine.restore(fixtureBytes.toString());
const material = new THREE.MeshBasicMaterial();
const sidewalks = new CitySidewalks(() => material, release => release());
const growthRate = 16, simulationSpeed = 8, renderFps = 30, wallSeconds = 120;
const frameCount = renderFps * wallSeconds;
const workerPublishMs = 500;
const framesPerPublish = Math.round(workerPublishMs / (1000 / renderFps));
const tickRate = 20;
let advancedTicks = 0;
let previousSignature = '';
let committedSignature = '';
let worldSnapshots = 0, structuralSnapshots = 0, replacedJobs = 0, commits = 0;
let pendingFrames = 0, pendingStartFrame = null, longestPendingFrames = 0, longestPendingEndFrame = null;
let wasPending = false;
const signatureFrames = new Map();
const commitLags = [];
let observedCommits = '';
let queuedUpdates = 0;
const commitFrames = [];
const start = JSON.parse(engine.snapshot());
const startRoadCount = start.roads.length;
engine.set_development_rate(growthRate);

function sidewalkSignature(world) {
  const roads = world.roads.filter(r => r.kind !== 'rail' && r.kind !== 'expressway')
    .map(r => ({ id: r.id, from: r.from, to: r.to, kind: r.kind, stage: r.stage === 'open' ? 'open' : 'closed' })).sort((a,b) => a.id-b.id);
  const ids = new Set(roads.flatMap(r => [r.from, r.to]));
  const nodes = world.nodes.filter(n => ids.has(n.id)).map(n => ({id:n.id,x:n.x,z:n.z,kind:n.kind})).sort((a,b)=>a.id-b.id);
  return JSON.stringify({nodes,roads});
}
function publishIfStructural(frame) {
  const world = JSON.parse(engine.snapshot());
  worldSnapshots++;
  const signature = sidewalkSignature(world);
  if (signature === previousSignature) return;
  structuralSnapshots++;
  const previousJob = sidewalks.job;
  previousSignature = signature;
  signatureFrames.set(signature, frame);
  sidewalks.setWorld(world);
  if (previousJob && sidewalks.job !== previousJob) replacedJobs++;
  else if (previousJob) queuedUpdates++;
}

const harnessStarted = performance.now();
publishIfStructural(0);
try {
for (let frame = 1; frame <= frameCount; frame++) {
  // Cumulative target preserves the fractional tick across 30 Hz frames.
  const targetTicks = Math.floor(frame * tickRate * simulationSpeed / renderFps);
  const ticks = targetTicks - advancedTicks;
  if (ticks > 0) { engine.step(ticks); advancedTicks = targetTicks; }

  // Worker runFrame publishes worlds no more often than every 500 wall ms;
  // at this simulated 30 fps cadence that is exactly every 15 render frames.
  if (frame % framesPerPublish === 0) publishIfStructural(frame);

  // CityRenderer's render owner advances the builder once per rendered frame.
  sidewalks.update();
  const pending = sidewalks.pendingCount === 1;
  if (pending) {
    if (!wasPending) pendingStartFrame = frame;
    pendingFrames++;
    wasPending = true;
  } else if (wasPending) {
    const duration = frame - pendingStartFrame;
    if (duration > longestPendingFrames) { longestPendingFrames = duration; longestPendingEndFrame = frame; }
    pendingStartFrame = null;
    wasPending = false;
  }
  // Read the committed input, not the latest published input: coalesced jobs lag.
  const committed = sidewalks.committedSignature;
  if (committed && committed !== observedCommits) {
    commits++; commitFrames.push(frame);
    committedSignature = committed; observedCommits = committed;
    commitLags.push(frame - signatureFrames.get(committed));
  }
}
if (wasPending) {
  const duration = frameCount - pendingStartFrame + 1;
  if (duration > longestPendingFrames) { longestPendingFrames = duration; longestPendingEndFrame = frameCount; }
}
const harnessMs = performance.now() - harnessStarted;
const end = JSON.parse(engine.snapshot());
const finalSignature = sidewalkSignature(end);
const signatureHash = value => createHash('sha256').update(value).digest('hex');
const report = {
  schemaVersion: 1,
  scope: 'Offline Node CPU harness; real public WASM engine and production CitySidewalks/SidewalkBuild. No browser/GPU/frame timing.',
  fixtures: {
    save: fixture,
    saveSha256: createHash('sha256').update(fixtureBytes).digest('hex'),
    wasmSha256: createHash('sha256').update(wasmBytes).digest('hex'),
    seed: save.seed,
    initialTick: start.tick,
    initialRoadCount: startRoadCount,
  },
  settings: { growthRate, simulationSpeed, renderFps, wallSeconds, frameCount, idealTicksPerWallSecond: tickRate * simulationSpeed, workerWorldPublishCadenceMs: workerPublishMs, workerCadenceBasis: 'simulation.worker.ts WORLD_PUBLISH_MS=500; at 30 render frames/s, one publish opportunity each 15 frames; structural snapshots are revision/signature gated' },
  schedulerAssumptions: [
    'Each render frame advances to cumulative floor(frame*20*8/30) engine ticks, then calls CitySidewalks.update exactly once.',
    'Worker snapshots are checked every 500 ms of simulated wall time; snapshot publication occurs only when sidewalk-relevant road/node signature changes.',
    'Replacement counts compare active job identity before/after setWorld; queued updates preserve that job. Commits read the owner committedSignature, including empty results.',
  ],
  results: {
    simulatedTicks: end.tick - start.tick,
    worldSnapshotChecks: worldSnapshots,
    structuralSnapshots,
    replacedJobs,
    queuedUpdates,
    commits,
    commitFrames,
    longestCommitGapFrames: Math.max(0,...commitFrames.map((f,i)=>f-(commitFrames[i-1]??0))),
    maxCommittedInputAgeFrames: Math.max(0,...commitLags),
    finalPending: sidewalks.pendingCount === 1,
    pendingFrames,
    longestContinuousPending: { frames: longestPendingFrames, wallSeconds: longestPendingFrames / renderFps, endedAtFrame: longestPendingEndFrame },
    finalRoadCount: end.roads.length,
    roadCountDelta: end.roads.length - startRoadCount,
    finalTick: end.tick,
    finalCommittedSignatureSha256: committedSignature ? signatureHash(committedSignature) : null,
    finalWorldSignatureSha256: signatureHash(finalSignature),
    committedMatchesFinalWorld: committedSignature === finalSignature,
  },
  harness: { cpu: os.cpus()[0]?.model ?? null, runtime: process.version, cpuHarnessMs: Number(harnessMs.toFixed(2)) },
};
await writeFile(output, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify({ output, ...report.results, cpuHarnessMs: report.harness.cpuHarnessMs }));
} finally {
  sidewalks.reset();
  material.dispose();
engine.free();
}
