import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { cpus } from 'node:os';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { pathToFileURL } from 'node:url';
import { build } from 'esbuild';

const root = resolve(new URL('..', import.meta.url).pathname);
const supportDir = resolve(root, '.tools/movement-support');
const wasmJsPath = resolve(root, 'public/wasm/city_engine.js');
const wasmPath = resolve(root, 'public/wasm/city_engine_bg.wasm');
const bundlePath = resolve(supportDir, 'presentation-modules.mjs');
const savePath = resolve(supportDir, 'presentation-cost-save.json');
const outputPath = resolve(root, 'validation/movement-presentation-cost.json');
const wasmBytes = await readFile(wasmPath);
const wasmSha256 = createHash('sha256').update(wasmBytes).digest('hex');

await mkdir(supportDir, { recursive: true });
await build({
  stdin: {
    contents: "export { decodeMovementPacket } from './web/src/movement-support.ts';\nexport { VisibleInfrastructure } from './web/src/render/visible-infrastructure.ts';\nexport { MovementPresentation } from './web/src/render/movement-presentation.ts';\n",
    resolveDir: root,
    sourcefile: 'presentation-cost-entry.ts',
  },
  bundle: true,
  platform: 'node',
  format: 'esm',
  outfile: bundlePath,
});

const bound = await import(pathToFileURL(wasmJsPath).href);
await bound.default({ module_or_path: new Uint8Array(wasmBytes) });
const { decodeMovementPacket, VisibleInfrastructure, MovementPresentation } = await import(pathToFileURL(bundlePath).href);
const engine = new bound.CityEngine(87231);
engine.set_development_rate(16);
for (let ticks = 0; ticks < 12_000; ticks += 2_000) engine.step(2_000);

const phaseTimes = { getter: [], packetGeneration: [], decode: [], receive: [], sample: [], snapshotParse: [], infrastructureUpdate: [] };
const time = (fn) => {
  const start = process.hrtime.bigint();
  const result = fn();
  return { result, milliseconds: Number(process.hrtime.bigint() - start) / 1e6 };
};
const record = (phase, milliseconds) => phaseTimes[phase].push(milliseconds);
function readSnapshot() {
  const timed = time(() => JSON.parse(engine.snapshot()));
  record('snapshotParse', timed.milliseconds);
  return timed.result;
}
function assertFiniteMovement(movement) {
  for (const value of movement) assert.ok(Number.isFinite(value), `nonfinite decoded pose value: ${value}`);
}

const infrastructure = new VisibleInfrastructure();
const presentation = new MovementPresentation();
let world = readSnapshot();
let tokenRead = time(() => engine.presentation_revision());
record('getter', tokenRead.milliseconds);
assert.equal(world.presentation_revision, tokenRead.result, 'initial snapshot/getter token mismatch');
infrastructure.request(world);
infrastructure.commit(infrastructure.capture(world));
let publishedToken = tokenRead.result;
const publicationReasons = { presentationRevision: 0, periodicSnapshot: 0, both: 0, none: 0 };
let maxPacketBytes = 0;
let maxActors = 0;
let actorTotal = 0;
let finalActorCount = 0;
const publicationCount = 300;
const periodicInterval = 10;
let lastWorldPublication = 0;

for (let publication = 1; publication <= publicationCount; publication++) {
  engine.step(8);

  tokenRead = time(() => engine.presentation_revision());
  record('getter', tokenRead.milliseconds);

  const packetResult = time(() => engine.movement_packet());
  record('packetGeneration', packetResult.milliseconds);
  const packet = packetResult.result;
  assert.ok(packet instanceof Uint32Array, 'WASM movement_packet() did not return Uint32Array');
  maxPacketBytes = Math.max(maxPacketBytes, packet.byteLength);

  const decodedResult = time(() => decodeMovementPacket(packet));
  record('decode', decodedResult.milliseconds);
  assertFiniteMovement(decodedResult.result.movement);
  const actors = decodedResult.result.support.size;
  finalActorCount = actors;
  maxActors = Math.max(maxActors, actors);
  actorTotal += actors;

  const receiveResult = time(() => presentation.receive(decodedResult.result.movement, decodedResult.result.support, 50));
  record('receive', receiveResult.milliseconds);

  const revisionChanged = tokenRead.result !== publishedToken;
  const periodic = publication - lastWorldPublication >= periodicInterval;
  const reason = revisionChanged && periodic ? 'both' : revisionChanged ? 'presentationRevision' : periodic ? 'periodicSnapshot' : 'none';
  publicationReasons[reason]++;
  if (revisionChanged || periodic) {
    world = readSnapshot();
    assert.equal(world.presentation_revision, tokenRead.result, `snapshot/getter token mismatch at publication ${publication}`);
    const infrastructureResult = time(() => {
      infrastructure.request(world);
      infrastructure.commit(infrastructure.capture(world));
    });
    record('infrastructureUpdate', infrastructureResult.milliseconds);
    lastWorldPublication = publication;
    publishedToken = tokenRead.result;
  }

  for (let frame = 0; frame < 3; frame++) {
    const sampleResult = time(() => presentation.sample(publication * 50 + frame * (1000 / 60), infrastructure));
    record('sample', sampleResult.milliseconds);
    for (const pose of sampleResult.result.values()) {
      assert.ok([pose.x, pose.z, pose.h, pose.k, pose.y].every(Number.isFinite), 'nonfinite presented pose');
    }
  }
}

const saved = engine.save();
await writeFile(savePath, saved);

function summarize(samples) {
  const sorted = [...samples].sort((a, b) => a - b);
  const pick = (quantile) => sorted[Math.min(sorted.length - 1, Math.ceil(quantile * sorted.length) - 1)] ?? 0;
  return {
    samples: sorted.length,
    p50Ms: Number(pick(.5).toFixed(5)),
    p95Ms: Number(pick(.95).toFixed(5)),
    maxMs: Number((sorted.at(-1) ?? 0).toFixed(5)),
  };
}
const report = {
  generatedAt: new Date().toISOString(),
  scope: 'CPU-only diagnostic of actual WASM packet generation and bundled TypeScript presentation logic.',
  limits: [
    'No worker scheduling or message-transfer cost measured.',
    'No GPU, renderer, style compilation, camera, or infrastructure commit delay measured; requested worlds commit immediately in this harness.',
    'No heap or retained-memory claim measured.',
  ],
  host: {
    platform: process.platform,
    architecture: process.arch,
    node: process.version,
    cpu: cpus()[0]?.model ?? 'unknown',
    logicalCpuCount: cpus().length,
  },
  wasm: { path: 'public/wasm/city_engine_bg.wasm', sha256: wasmSha256 },
  scenario: {
    seed: 87231,
    developmentRate: 16,
    warmupTicks: 12000,
    movementPublications: publicationCount,
    ticksPerPublication: 8,
    renderSamplesPerPublication: 3,
    ordinarySnapshotCadencePublications: periodicInterval,
    publicationReasons,
    actors: { finalDecodedRows: finalActorCount, maximumObserved: maxActors, meanObserved: Number((actorTotal / publicationCount).toFixed(2)) },
    maximumPacketBytes: maxPacketBytes,
    finalTick: JSON.parse(saved).tick,
    finalSnapshotTick: world.tick,
    finalBuildings: world.buildings.length,
    finalRoads: world.roads.length,
    finalPresentationRevision: publishedToken,
  },
  phaseCosts: Object.fromEntries(Object.entries(phaseTimes).map(([phase, samples]) => [phase, summarize(samples)])),
  assertions: {
    snapshotGetterTokensMatched: true,
    decodedAndPresentedPosesFinite: true,
  },
  checkpoint: '.tools/movement-support/presentation-cost-save.json',
};
await mkdir(dirname(outputPath), { recursive: true });
await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify({
  tick: report.scenario.finalTick,
  actors: report.scenario.actors,
  maxPacketBytes,
  publicationReasons,
  phaseCosts: report.phaseCosts,
  wasmSha256,
  outputPath,
  savePath,
}, null, 2));
engine.free();
