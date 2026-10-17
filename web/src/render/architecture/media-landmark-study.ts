import { businessCityStudy } from './business-study';
import { selectBusinessRecipe } from './business-recipes';
import { architecturalChoice } from './variation';
import { MEDIA_LANDMARK_FORMS } from './media-landmark-selection';
/** Three spaced production-selected landmark hosts, with space to inspect their frontages. */
export function mediaLandmarkStudy(seed = 32, curvedOnly = false) {
  const source = businessCityStudy(seed, 'commercial', 'contemporary');
  const forms = curvedOnly ? (['curved'] as const) : MEDIA_LANDMARK_FORMS.slice(0, 3);
  const buildings = forms.map((form, i) => {
    const b = {
      ...source.buildings[0],
      id: i + 1,
      parcel_id: i + 1,
      x: curvedOnly ? 0 : (i - 1) * 106,
      z: 0,
      width: curvedOnly ? 38 : 34 + i * 4,
      depth: curvedOnly ? 30 : 26,
      floors: curvedOnly ? 12 : 10 + i * 2,
      rotation: 0,
      appearance_seed: seed >>> 0,
      name: `Media landmark · ${form}`,
    };
    let tries = 0;
    while (
      (selectBusinessRecipe(b, 'contemporary')?.era !== 'contemporary' ||
        selectBusinessRecipe(b, 'mixed')?.era !== 'contemporary' ||
        architecturalChoice(b.appearance_seed, 760) % MEDIA_LANDMARK_FORMS.length !==
          MEDIA_LANDMARK_FORMS.indexOf(form)) &&
      tries++ < 100000
    )
      b.appearance_seed++;
    return b;
  });
  // Production allocation prioritizes selected media landmarks within its three-screen budget.
  const nodes = [
    { id: 1, x: -106, z: 32, kind: 'station' as const },
    { id: 2, x: 106, z: 32, kind: 'station' as const },
  ];
  return {
    ...source,
    buildings,
    nodes,
    bounds: { min_x: -180, max_x: 180, min_z: -80, max_z: 100 },
    roads: [
      {
        id: 1,
        from: 1,
        to: 2,
        kind: 'arterial' as const,
        stage: 'open' as const,
        progress: 1,
        connected: true,
        length: 212,
        lanes: 3,
      },
    ],
    parcels: buildings.map((b) => ({
      id: b.id,
      x: b.x,
      z: b.z,
      width: b.width,
      depth: b.depth,
      rotation: 0,
      access_node: 1,
      suitability: 1,
      protected: false,
      reservation: null,
    })),
    building_access: buildings.map((b) => ({
      building_id: b.id,
      road_id: 1,
      points: [
        { x: b.x, z: b.depth / 2 },
        { x: b.x, z: 32 },
      ],
    })),
  };
}
