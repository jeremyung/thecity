import { readFile } from 'node:fs/promises';

// Counts explicit saved frontage assignments, not traffic capacity or all
// buildings reachable through the road graph. Compare equal-seed horizons.
const paths = process.argv.slice(2);
if (!paths.length)
  throw new Error('Usage: node scripts/audit-road-frontage.mjs save.json [save.json ...]');
const reports = [];
for (const path of paths) {
  const state = JSON.parse(await readFile(path, 'utf8'));
  const historic = new Set(state.neighborhood_road_ids ?? []);
  const developed = new Set(state.buildings.map((building) => building.parcel_id));
  const frontage = new Map();
  for (const assignment of state.parcel_frontages ?? []) {
    if (!developed.has(assignment.parcel_id)) continue;
    if (!frontage.has(assignment.road_id)) frontage.set(assignment.road_id, new Set());
    frontage.get(assignment.road_id).add(assignment.parcel_id);
  }
  const groups = {};
  for (const coordinated of [true, false]) {
    const roads = state.roads.filter(
      (road) => !['rail', 'bridge'].includes(road.kind) && historic.has(road.id) === coordinated,
    );
    const lengthM = roads.reduce((sum, road) => sum + road.length, 0);
    const frontageBuildings = roads.reduce(
      (sum, road) => sum + (frontage.get(road.id)?.size ?? 0),
      0,
    );
    groups[coordinated ? 'coordinated' : 'ordinary_other'] = {
      roads: roads.length,
      lengthM,
      frontageBuildings,
      frontageBuildingsPer100m: lengthM > 0 ? (frontageBuildings / lengthM) * 100 : null,
      roadsWithAtMostTwoFrontages: roads.filter((road) => (frontage.get(road.id)?.size ?? 0) <= 2)
        .length,
    };
  }
  reports.push({ path, seed: state.seed, tick: state.tick, groups });
}
console.log(
  JSON.stringify(
    {
      method:
        'Explicit developed parcel frontage per ordinary road length; excludes rail/bridges. Coordinated classification uses persisted neighborhood_road_ids. Does not measure network connection benefits, traffic, occupancy, or causal attribution.',
      reports,
    },
    null,
    2,
  ),
);
