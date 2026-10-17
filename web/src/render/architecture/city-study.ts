import { architecturalChoice } from './variation';
import { RESIDENTIAL_TOWER_IDS } from './tower-recipes';
import type { World } from '../../types';
import { blockStudyBuildings } from './block-study';
/** Isolated snapshot exercises the actual renderer without reading/writing a saved city. */
export function architectureCityStudy(seed = 32, towers = false): World {
  const buildings = towers
    ? RESIDENTIAL_TOWER_IDS.map((recipe, i) => {
        let appearance_seed = (seed + i * 19) >>> 0;
        while (
          RESIDENTIAL_TOWER_IDS[
            architecturalChoice(appearance_seed, 192) % RESIDENTIAL_TOWER_IDS.length
          ] !== recipe
        )
          appearance_seed = (appearance_seed + 1) >>> 0;
        return {
          id: i + 1,
          parcel_id: i + 1,
          x: (i - 2) * 36,
          z: -24,
          rotation: 0,
          width: 26,
          depth: 24,
          floors: [24, 32, 40, 28, 36][i],
          kind: 'apartment' as const,
          stage: 'open' as const,
          progress: 1,
          capacity: 160,
          occupancy: 90,
          built_day: 0,
          name: `Tower ${recipe}`,
          appearance_seed,
          anchor: false,
          history: [],
          vacant_days: 0,
          business_active: false,
        };
      })
    : blockStudyBuildings(seed);
  return {
    schema_version: 1,
    engine_version: 'architecture-study',
    seed,
    tick: 0,
    day: 1,
    minute_of_day: 900,
    development_rate: 1,
    revision: 0,
    stage: 'established' as const,
    bounds: { min_x: -200, max_x: 200, min_z: -100, max_z: 100 },
    terrain: {
      river_half_width: 32,
      river: [
        { x: 300, z: -600 },
        { x: 300, z: 600 },
      ],
      protected: [],
      gateways: [],
    },
    weather: { kind: 'clear' as const, intensity: 0 },
    nodes: [
      { id: 1, x: -90, z: 0, kind: 'junction' as const },
      { id: 2, x: 90, z: 0, kind: 'junction' as const },
    ],
    roads: [
      {
        id: 1,
        from: 1,
        to: 2,
        kind: 'arterial' as const,
        stage: 'open' as const,
        progress: 1,
        connected: true,
        length: 180,
        lanes: 2,
      },
    ],
    buildings,
    parcels: buildings.map((b) => ({
      id: b.parcel_id,
      x: b.x,
      z: b.z,
      width: b.width,
      depth: b.depth,
      rotation: b.rotation,
      access_node: 1,
      suitability: 1,
      protected: false,
      reservation: null,
    })),
    projects: [],
    trains: [],
    events: [],
    building_access: buildings.map((b) => ({
      building_id: b.id,
      road_id: 1,
      points: [
        { x: b.x, z: b.z + ((b.z < 0 ? 1 : -1) * b.depth) / 2 },
        { x: b.x, z: b.z < 0 ? -7 : 7 },
      ],
    })),
    stats: { population: 0, active_travelers: 0, buildings: buildings.length, active_projects: 0 },
  };
}
