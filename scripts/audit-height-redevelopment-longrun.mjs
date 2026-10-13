// Run unedited fresh growth through repeated height-renewal cycles.
// Captures state every 200 ticks so even the shortest current project spans many observations.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [output = 'validation/height-redevelopment-longrun.json', tickText = '720000', ...seedTexts] = process.argv.slice(2);
const tickLimit = Number(tickText);
const seeds = (seedTexts.length ? seedTexts : ['87231', '0', '5', '77']).map(Number);
const SAMPLE_TICKS = 200;
if (!Number.isSafeInteger(tickLimit) || tickLimit < SAMPLE_TICKS || tickLimit % SAMPLE_TICKS !== 0
  || seeds.some(seed => !Number.isSafeInteger(seed) || seed < 0 || seed > 0xffff_ffff)) {
  throw new Error(`Expected tick limit as a positive multiple of ${SAMPLE_TICKS} followed by uint32 seeds`);
}

const { CityEngine, initSync } = await import(pathToFileURL(resolve('public/wasm/city_engine.js')));
initSync({ module: readFileSync('public/wasm/city_engine_bg.wasm') });
const towerCounts = world => world.buildings.filter(building => building.floors >= 16).length;
const activeCount = world => world.buildings.filter(building => building.stage === 'open'
  && (['home', 'apartment'].includes(building.kind) ? building.occupancy > 0
    : ['shop', 'cafe', 'office'].includes(building.kind) ? building.business_active : false)).length;

function summarizeSeed(seed) {
  const engine = new CityEngine(seed);
  engine.set_development_rate(16);
  const started = performance.now();
  const observations = [];
  const admissions = [];
  const completions = [];
  const seenProjects = new Set();
  const siteAdmissions = new Map();
  let minPopulation = Infinity, maxPopulation = 0, maxBuildings = 0;
  let priorProject = null;
  let finalSave, finalSnapshot;
  try {
    for (let tick = SAMPLE_TICKS; tick <= tickLimit; tick += SAMPLE_TICKS) {
      engine.step(SAMPLE_TICKS);
      const world = JSON.parse(engine.snapshot());
      const currentTowers = towerCounts(world);
      minPopulation = Math.min(minPopulation, world.stats.population);
      maxPopulation = Math.max(maxPopulation, world.stats.population);
      maxBuildings = Math.max(maxBuildings, world.buildings.length);
      observations.push({ tick, towers: currentTowers, population: world.stats.population,
        buildings: world.buildings.length, activeBuildings: activeCount(world) });

      const activeProject = world.projects.find(project => project.kind === 'redevelopment' && project.progress < 1);
      if (activeProject && !seenProjects.has(activeProject.id)) {
        const state = JSON.parse(engine.save());
        const pending = state.height_redevelopment;
        if (!pending) throw new Error(`Project ${activeProject.id} has no pending height-redevelopment record`);
        seenProjects.add(pending.project_id);
        const building = state.buildings.find(item => item.id === pending.building_id);
        const parcel = state.parcels.find(item => item.id === pending.parcel_id);
        const count = (siteAdmissions.get(pending.parcel_id) ?? 0) + 1;
        siteAdmissions.set(pending.parcel_id, count);
        const admission = { projectId: pending.project_id, tick, buildingId: building.id,
          parcelId: pending.parcel_id, siteAdmissionNumber: count, name: building.name,
          floorsBefore: pending.original_floors, floorsTarget: pending.target_floors,
          capacityBefore: pending.original_capacity, capacityTarget: pending.target_capacity,
          appearanceSeed: building.appearance_seed, anchor: building.anchor,
          characterRetentionEligible: building.appearance_seed % 4 !== 0 && !building.anchor,
          buildingStage: building.stage, buildingHistory: [...building.history],
          totalBuildings: state.buildings.length, towersAtAdmission: currentTowers,
          occupiedPopulation: world.stats.population, activeBuildings: activeCount(world),
          parcelArea: parcel.width * parcel.depth,
          realizedLots: state.masterplan.lot_parcels.filter(Boolean).length,
          plannedLots: state.masterplan.lot_parcels.length };
        admissions.push(admission);
        priorProject = admission;
      }
      if (priorProject && !activeProject) {
        const state = JSON.parse(engine.save());
        const completedBuilding = state.buildings.find(item => item.id === priorProject.buildingId);
        completions.push({ projectId: priorProject.projectId, tick,
          buildingId: completedBuilding.id, parcelId: completedBuilding.parcel_id,
          floorsAfter: completedBuilding.floors, buildingStage: completedBuilding.stage,
          buildingHistory: [...completedBuilding.history], totalBuildings: state.buildings.length,
          towersAtCompletion: currentTowers });
        priorProject = null;
      }
      if (tick % 2_000 === 0 || tick === SAMPLE_TICKS) {
        const last = observations.at(-1);
        console.log(`seed ${seed} tick ${tick}: towers=${last.towers} buildings=${last.buildings} redevelopments=${admissions.length}/${completions.length}`);
      }
    }

    finalSave = engine.save();
    finalSnapshot = engine.snapshot();
    const restored = CityEngine.restore(finalSave);
    const immediateRestoreEquivalent = restored.save() === finalSave
      && restored.snapshot() === finalSnapshot;
    const inProgressAtEnd = Boolean(JSON.parse(finalSave).height_redevelopment);
    engine.step(SAMPLE_TICKS);
    restored.step(SAMPLE_TICKS);
    const replayEquivalent200 = engine.save() === restored.save()
      && engine.snapshot() === restored.snapshot();
    const result = { seed, requestedTicks: tickLimit, sampleEveryTicks: SAMPLE_TICKS,
      developmentRate: 16, admissions, completions,
      inProgressAtEnd,
      final: observations.at(-1), maxPopulation, minPopulation,
      maxBuildings, buildingCap: 3_000, towerCountSeries: observations.map(({ tick, towers }) => ({ tick, towers })),
      siteAdmissionCounts: [...siteAdmissions].map(([parcelId, count]) => ({ parcelId, count })),
      repeatedSiteCount: [...siteAdmissions.values()].filter(count => count > 1).length,
      immediateRestoreEquivalent, replayEquivalent200, replayTicks: SAMPLE_TICKS,
      elapsedMs: performance.now() - started };
    restored.free();
    return result;
  } finally {
    engine.free();
  }
}

const started = performance.now();
const seedsResult = seeds.map(summarizeSeed);
const report = { generatedAt: new Date().toISOString(), tickLimit, sampleEveryTicks: SAMPLE_TICKS,
  developmentRate: 16, seeds: seedsResult,
  limitations: ['Sampling observes admissions/completions within at most 200 simulation ticks; the shortest redevelopment project spans more than 2000 ticks at development rate 16.',
    'Population/building min/max and tower counts are sampled, not instrumented at every internal mutation.',
    'Node WASM normal growth only; no browser rendering or visual acceptance claim.'],
  elapsedMs: performance.now() - started };
mkdirSync(dirname(resolve(output)), { recursive: true });
writeFileSync(output, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ output, seeds: seedsResult.map(({ seed, admissions, completions, final, maxBuildings, replayEquivalent200 }) =>
  ({ seed, admissions: admissions.length, completions: completions.length, towers: final.towers,
    buildings: final.buildings, maxBuildings, replayEquivalent200 })), elapsedMs: report.elapsedMs }));
if (seedsResult.some(seed => seed.maxBuildings > seed.buildingCap || !seed.immediateRestoreEquivalent || !seed.replayEquivalent200)) {
  process.exitCode = 1;
}
