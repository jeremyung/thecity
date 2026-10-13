// Fabric acceptance audit for master-plan cities (specs/02-world-growth.md#fabric-targets).
//
// For each snapshot: picks the densest 250 m compact crop on a 25 m lattice,
// a 250 m crop centered on the station, and the whole map. Rendered coverage
// comes from the production building kit via audit-rendered-coverage.mjs; lot,
// floor and road-share statistics come from the snapshot itself.
//
// Usage: node scripts/audit-masterplan-fabric.mjs --output report.json snapshot.json [...]
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const CROP = 250;
// Mirrors partyWallBodyWidth/partyWallBodyDepth in web/src/render/kit.ts.
function bodyFootprint(b) {
  const lowrise = b.floors < 4 && ['home', 'shop', 'cafe'].includes(b.kind);
  const width = b.anchor || b.floors >= 14 ? Math.max(6, b.width * (b.anchor ? .66 : .84)) : Math.max(6, b.width - (lowrise ? .9 : 1.3));
  return Math.min(width, b.width) * Math.min(Math.max(6, b.depth * .84), b.depth);
}

const targets = {
  compact: { coverage: [40, 50], footprintMedian: [55, 140], footprintP90: [0, 350], floorsMedian: [2, 4], roadShare: [18, 28] },
  station: { coverage: [38, 48], footprintMedian: [100, 300], footprintP90: [0, 900], floorsMedian: [4, 7], roadShare: [20, 30] },
};

function quantile(values, p) {
  if (!values.length) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const index = (sorted.length - 1) * p;
  const lo = Math.floor(index), hi = Math.ceil(index);
  return sorted[lo] + (sorted[hi] - sorted[lo]) * (index - lo);
}

function inside(b, crop) {
  return b.x >= crop[0] && b.x < crop[2] && b.z >= crop[1] && b.z < crop[3];
}

function densestCrop(world) {
  let best = null;
  for (let x0 = -400; x0 <= 400 - CROP; x0 += 25) {
    for (let z0 = -400; z0 <= 400 - CROP; z0 += 25) {
      const crop = [x0, z0, x0 + CROP, z0 + CROP];
      const area = world.buildings.filter(b => inside(b, crop)).reduce((sum, b) => sum + b.width * b.depth, 0);
      if (!best || area > best.area) best = { area, crop };
    }
  }
  return best.crop;
}

function stationCrop(world) {
  const station = world.nodes.find(n => n.kind === 'station');
  if (!station) return null;
  const x0 = Math.max(-400, Math.min(400 - CROP, station.x - CROP / 2));
  const z0 = Math.max(-400, Math.min(400 - CROP, station.z - CROP / 2));
  return [x0, z0, x0 + CROP, z0 + CROP];
}

function renderedCoverage(snapshotPath, crop) {
  const dir = mkdtempSync(join(tmpdir(), 'fabric-'));
  try {
    const output = join(dir, 'coverage.json');
    execFileSync(process.execPath, [join(root, 'scripts/audit-rendered-coverage.mjs'), '--snapshot', snapshotPath, '--crop', crop.join(','), '--output', output], { stdio: ['ignore', 'ignore', 'inherit'] });
    return JSON.parse(readFileSync(output, 'utf8'));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

function within(value, [lo, hi]) {
  return value !== null && value >= lo && value <= hi;
}

function auditCrop(snapshotPath, world, name, crop) {
  const buildings = world.buildings.filter(b => b.stage === 'open' && inside(b, crop));
  const coverage = renderedCoverage(snapshotPath, crop);
  const domain = coverage.domain;
  const lots = buildings.map(b => b.width * b.depth);
  const bodies = buildings.map(bodyFootprint);
  const result = {
    name,
    crop,
    buildings: buildings.length,
    renderedCoveragePercent: +domain.projectedKitGeometryPercent.toFixed(2),
    lotEnvelopeCoveragePercent: +domain.nominalParcelEnvelopeUnionPercent.toFixed(2),
    openStreetAdjustedDiagnostic: coverage.openStreetAdjustedDiagnostic,
    lotAreaMedianM2: +quantile(lots, 0.5)?.toFixed(1),
    lotAreaP90M2: +quantile(lots, 0.9)?.toFixed(1),
    footprintMedianM2: +quantile(bodies, 0.5)?.toFixed(1),
    footprintP90M2: +quantile(bodies, 0.9)?.toFixed(1),
    floorsMedian: quantile(buildings.map(b => b.floors), 0.5),
    roadSharePercent: +(100 * coverage.openStreetAdjustedDiagnostic.openStreetAreaM2 / domain.areaM2).toFixed(2),
  };
  const target = targets[name];
  if (target) {
    result.withinTargets = {
      coverage: within(result.renderedCoveragePercent, target.coverage),
      footprintMedian: within(result.footprintMedianM2, target.footprintMedian),
      footprintP90: within(result.footprintP90M2, target.footprintP90),
      floorsMedian: within(result.floorsMedian, target.floorsMedian),
      roadShare: within(result.roadSharePercent, target.roadShare),
    };
  }
  return result;
}

const args = process.argv.slice(2);
let outputPath = null;
const inputs = [];
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--output') outputPath = args[++i];
  else inputs.push(resolve(args[i]));
}
if (!inputs.length) {
  console.error('Usage: node scripts/audit-masterplan-fabric.mjs [--output report.json] snapshot.json [...]');
  process.exit(1);
}

const report = {
  method: 'Rendered coverage: production kit plan-view union (audit-rendered-coverage.mjs). Footprint is the main body box from the kit party-wall sizing (rear service volumes and roofs excluded); lot area is the parcel envelope. Road share rasterizes open street right-of-way (carriageway plus sidewalks). Compact crop: densest 250 m window on a 25 m lattice by lot area.',
  limitations: [
    'Projected coverage includes all snapshot building stages clipped to crop; count/floor/size statistics use open buildings centered in crop.',
    'Compact crop selected by centered parcel-envelope area, not maximum clipped rendered union.',
    'Footprint quantiles are initial party-wall rectangle proxies before lowrise massing; not exact final footprints.',
    'Street subtraction is not net developable land: river, protected land, rail and expressway remain in denominator.',
  ],
  targets,
  worlds: [],
};
for (const input of inputs) {
  const world = JSON.parse(readFileSync(input, 'utf8'));
  const crops = [['compact', densestCrop(world)], ['station', stationCrop(world)], ['whole', [-400, -400, 400, 400]]];
  const entry = { snapshot: input.replace(root + '/', ''), seed: world.seed, tick: world.tick, buildings: world.buildings.length, crops: [] };
  for (const [name, crop] of crops) {
    if (!crop) continue;
    const result = auditCrop(input, world, name, crop);
    entry.crops.push(result);
    console.error(`seed ${world.seed} ${name.padEnd(8)} rendered ${result.renderedCoveragePercent}%  footprint p50 ${result.footprintMedianM2} p90 ${result.footprintP90M2}  floors ${result.floorsMedian}  roads ${result.roadSharePercent}%`);
  }
  report.worlds.push(entry);
}
const json = `${JSON.stringify(report, null, 2)}\n`;
if (outputPath) writeFileSync(outputPath, json);
else process.stdout.write(json);
