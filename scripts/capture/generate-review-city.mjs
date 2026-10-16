import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { CityEngine, initSync } from '../../public/wasm/city_engine.js';

// Keep candidate captures separate from the browser's established review fixture.
// Generate through the same compiled API used by the worker, not private state edits.
const output = resolve(process.argv[2] ?? 'validation/mature-world.candidate.json');
const seed = Number(process.argv[3] ?? 87231);
const ticks = Number(process.argv[4] ?? 242000);
const developmentRate = Number(process.argv[5] ?? 16);
if (
  !Number.isInteger(seed) ||
  seed < 0 ||
  seed > 0xffffffff ||
  !Number.isSafeInteger(ticks) ||
  ticks < 0 ||
  ticks > 2000000 ||
  !Number.isInteger(developmentRate) ||
  developmentRate < 1 ||
  developmentRate > 16
) {
  throw new Error(
    'Usage: node scripts/capture/generate-review-city.mjs [output.json] [uint32 seed] [ticks 0..2000000] [development rate 1..16]',
  );
}
const wasm = await readFile(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url));
initSync({ module: wasm });
const engine = new CityEngine(seed);
let resumed;
try {
  engine.set_development_rate(developmentRate);
  for (let remaining = ticks; remaining > 0; remaining -= 2000)
    engine.step(Math.min(2000, remaining));
  const save = engine.save();
  const state = JSON.parse(save);
  const snapshot = JSON.parse(engine.snapshot());
  if (snapshot.tick !== ticks) throw new Error(`Expected tick ${ticks}, received ${snapshot.tick}`);
  resumed = CityEngine.restore(save);
  if (resumed.save() !== save) throw new Error('Candidate save did not restore canonically');
  engine.step(2000);
  resumed.step(2000);
  if (engine.save() !== resumed.save())
    throw new Error('Candidate continuation differs after restoration');
  const orientedIds = new Set(
    snapshot.parcels
      .filter((parcel) => Number.isFinite(parcel.rotation))
      .map((parcel) => parcel.id),
  );
  const profile = {
    source: 'public CityEngine WASM API',
    seed,
    tick: snapshot.tick,
    developmentRate,
    engineVersion: snapshot.engine_version,
    wasmSha256: createHash('sha256').update(wasm).digest('hex'),
    saveSha256: createHash('sha256').update(save).digest('hex'),
    canonicalRestore: true,
    continuationTicksVerified: 2000,
    stage: snapshot.stage,
    buildings: snapshot.buildings.length,
    roads: snapshot.roads.length,
    explicitFootprintParcels: orientedIds.size,
    explicitFootprintBuildings: snapshot.buildings.filter((building) =>
      orientedIds.has(building.parcel_id),
    ).length,
    housing: snapshot.buildings.filter((b) => ['home', 'apartment'].includes(b.kind)).length,
    commercial: snapshot.buildings.filter((b) => ['shop', 'cafe', 'office'].includes(b.kind))
      .length,
    towers: snapshot.buildings
      .filter((b) => b.floors >= 16 && b.stage === 'open')
      .map((b) => ({ id: b.id, kind: b.kind, floors: b.floors, x: b.x, z: b.z })),
    neighborhoodPlan: state.neighborhood_plan ?? null,
    historicNeighborhoodRoads: state.neighborhood_road_ids?.length ?? 0,
    historicNeighborhoodFrontages: (state.parcel_frontages ?? []).filter((frontage) =>
      (state.neighborhood_road_ids ?? []).includes(frontage.road_id),
    ).length,
    acceptance: 'Candidate only: requires density, connectivity, and browser visual review.',
  };
  await writeFile(output, `${save}\n`);
  await writeFile(`${output}.provenance.json`, `${JSON.stringify(profile, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify({ output, ...profile }, null, 2)}\n`);
} finally {
  resumed?.free();
  engine.free();
}
