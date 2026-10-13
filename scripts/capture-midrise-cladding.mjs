import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

// Capture an ordinary, seeded public-WASM construction lifecycle. This script
// deliberately uses only CityEngine's public create/restore/step/save/snapshot
// API; it does not edit private save state or inject a building.
const targetKind = process.argv[3] ?? 'office';
const minimumFloors = Number(process.argv[4] ?? 4);
assert.ok(Number.isInteger(minimumFloors) && minimumFloors >= 4 && minimumFloors <= 11, 'Expected minimum floors between 4 and 11');
assert.ok(['office', 'apartment'].includes(targetKind), 'Expected office or apartment');
const outputDir = resolve(process.argv[2] ?? '.tools/midrise-cladding');
const developmentRate = 16;
const searchLimit = 40_000;
const searchBatch = 200;
const openingLimit = 2_000;
const seed = 0;
const sha256 = value => createHash('sha256').update(value).digest('hex');

initSync({ module: await readFile(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)) });
await mkdir(outputDir, { recursive: true });

const live = new CityEngine(seed);
let restored;
try {
  live.set_development_rate(developmentRate);
  let beforeWorld;
  let target;
  for (let advanced = 0; advanced < searchLimit; advanced += searchBatch) {
    live.step(searchBatch);
    const world = JSON.parse(live.snapshot());
    target = world.buildings.find(building => building.kind === targetKind
      && building.floors >= minimumFloors && building.floors <= 11
      && !building.anchor && (targetKind === 'apartment' || building.appearance_seed % 3 === 0)
      && building.progress >= 0.85 && building.progress <= 0.9
      && (building.stage === 'surface' || building.stage === 'finishing'));
    if (target) { beforeWorld = world; break; }
  }
  assert.ok(target, `No qualifying ${targetKind} found within ${searchLimit} ticks`);
  assert.equal(beforeWorld.seed, seed);
  const targetId = target.id;
  const targetProject = JSON.parse(live.save()).projects.find(project =>
    project.kind === 'building' && project.parcel_id === target.parcel_id && project.progress < 1);
  assert.ok(targetProject, `No active building project for target ${targetId}`);

  const beforeSave = live.save();
  const beforeSnapshot = live.snapshot();
  // Checkpoint and restore the exact original live state while construction is
  // in progress, then continue both copies through the opening boundary.
  restored = CityEngine.restore(beforeSave);
  assert.equal(restored.save(), beforeSave, 'pre-opening save must restore canonically');
  assert.equal(restored.snapshot(), beforeSnapshot, 'restored pre-opening snapshot must match');

  let stepsToOpen = 0;
  let afterWorld;
  while (stepsToOpen < openingLimit) {
    live.step(1);
    restored.step(1);
    stepsToOpen++;
    assert.equal(restored.save(), live.save(), `restored continuation diverged at tick ${beforeWorld.tick + stepsToOpen}`);
    const candidateWorld = JSON.parse(live.snapshot());
    const candidate = candidateWorld.buildings.find(building => building.id === targetId);
    if (candidate?.stage === 'open') {
      afterWorld = candidateWorld;
      break;
    }
  }
  assert.ok(afterWorld, `Building ${targetId} did not open within ${openingLimit} ticks`);
  const afterTarget = afterWorld.buildings.find(building => building.id === targetId);
  assert.equal(afterTarget.stage, 'open');
  assert.equal(afterTarget.progress, 1);
  assert.equal(restored.snapshot(), live.snapshot(), 'live and restored continuations must agree at opening');

  const parcel = beforeWorld.parcels.find(candidate => candidate.id === target.parcel_id);
  const metadata = {
    schemaVersion: 1,
    source: 'public CityEngine WASM API; normal seeded development and project stepping',
    wasmSha256: sha256(await readFile(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)),),
    seed,
    developmentRate,
    target: {
      id: targetId,
      parcelId: target.parcel_id,
      kind: target.kind,
      floors: target.floors,
      appearanceSeed: target.appearance_seed,
      anchor: target.anchor,
      stage: target.stage,
      progress: target.progress,
      geometry: {
        x: target.x, z: target.z, width: target.width, depth: target.depth,
        rotation: target.rotation,
        parcel: parcel ?? null,
      },
    },
    before: {
      tick: beforeWorld.tick,
      worldStage: beforeWorld.stage,
      buildingCount: beforeWorld.buildings.length,
      project: targetProject,
      snapshotSha256: sha256(beforeSnapshot),
      saveSha256: sha256(beforeSave),
    },
    after: {
      tick: afterWorld.tick,
      worldStage: afterWorld.stage,
      buildingCount: afterWorld.buildings.length,
      targetStage: afterTarget.stage,
      targetProgress: afterTarget.progress,
      stepsToOpen,
      snapshotSha256: sha256(live.snapshot()),
      saveSha256: sha256(live.save()),
    },
    continuation: {
      originalLiveSavedBeforeRestore: true,
      canonicalPreOpeningRestore: true,
      originalVsRestoredEqualAtEveryStep: true,
      comparedTicks: stepsToOpen,
      equalAtOpening: true,
    },
    scope: 'Compact developing-city browser-review fixture. The snapshots show the same naturally admitted non-anchor midrise immediately before opening and at its first observed open state.',
  };

  await writeFile(resolve(outputDir, 'before-opening.json'), `${JSON.stringify(beforeWorld, null, 2)}\n`);
  await writeFile(resolve(outputDir, 'after-opening.json'), `${JSON.stringify(afterWorld, null, 2)}\n`);
  await writeFile(resolve(outputDir, 'before-opening.save.json'), `${beforeSave}\n`);
  await writeFile(resolve(outputDir, 'after-opening.save.json'), `${live.save()}\n`);
  await writeFile(resolve(outputDir, 'metadata.json'), `${JSON.stringify(metadata, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify({ outputDir, ...metadata }, null, 2)}\n`);
} finally {
  restored?.free();
  live.free();
}
