// Offline attribution from explicit saved road/frontage records. This does not
// infer block membership from proximity or claim rendered/net land coverage.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { relative, resolve } from 'node:path';

const [input, output] = process.argv.slice(2);
if (!input || !output)
  throw new Error('Usage: node scripts/audit-frontage-allocation.mjs SAVE OUTPUT');
const bytes = await readFile(input);
const state = JSON.parse(bytes);
const roads = new Map(state.roads.map((road) => [road.id, road]));
const tags = new Set(state.neighborhood_road_ids);
const frontages = new Map();
for (const frontage of state.parcel_frontages) {
  if (frontages.has(frontage.parcel_id))
    throw new Error(`Duplicate frontage for ${frontage.parcel_id}`);
  frontages.set(frontage.parcel_id, frontage);
}
const groups = {};
for (const building of state.buildings) {
  const frontage = frontages.get(building.parcel_id);
  const road = frontage && roads.get(frontage.road_id);
  const category = !frontage
    ? 'no_frontage_record'
    : !road
      ? 'missing_road'
      : tags.has(road.id)
        ? 'tagged_neighborhood_road'
        : 'other_road';
  const group = (groups[category] ??= {
    buildings: 0,
    nominalEnvelopeAreaM2: 0,
    nominalFloorAreaM2: 0,
    capacitySlots: 0,
    byKind: {},
    depthHistogramM: {},
  });
  const area = building.width * building.depth;
  if (!Number.isFinite(area) || area <= 0) throw new Error(`Invalid building ${building.id}`);
  group.buildings++;
  group.nominalEnvelopeAreaM2 += area;
  group.nominalFloorAreaM2 += area * building.floors;
  group.capacitySlots += building.capacity;
  group.byKind[building.kind] = (group.byKind[building.kind] ?? 0) + 1;
  const depthBin = String(Math.round(building.depth));
  group.depthHistogramM[depthBin] = (group.depthHistogramM[depthBin] ?? 0) + 1;
}
const report = {
  source: {
    save: relative(process.cwd(), resolve(input)),
    sha256: createHash('sha256').update(bytes).digest('hex'),
    seed: state.seed,
    tick: state.tick,
  },
  method:
    'Attribute each building through its saved parcel_frontage.road_id to neighborhood_road_ids membership.',
  limitations: [
    'A tagged road is not proof that a building came from a canonical coordinated block; exterior and later infill frontage may share that road.',
    'Envelope/floor areas are sums of saved width × depth (× floors), not geometry unions or developable-land coverage.',
    'Depth histogram rounds saved metres; capacity slots are the engine use/floor formula, not area-calibrated population.',
  ],
  roadTags: { total: tags.size, live: [...tags].filter((id) => roads.has(id)).length },
  groups,
};
await writeFile(output, `${JSON.stringify(report, null, 2)}\n`);
console.log(
  JSON.stringify({
    source: report.source,
    roadTags: report.roadTags,
    groups: Object.fromEntries(
      Object.entries(groups).map(([key, value]) => [
        key,
        { buildings: value.buildings, nominalEnvelopeAreaM2: value.nominalEnvelopeAreaM2 },
      ]),
    ),
  }),
);
