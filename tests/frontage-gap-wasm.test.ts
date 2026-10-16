// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import type { World } from '../web/src/types';

it('develops compact gaps on existing streets through normal construction and resumes them', () => {
  initSync({
    module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)),
  });
  const engine = CityEngine.restore(
    readFileSync(
      new URL('./fixtures/access-joins/infill-oriented-continued.json', import.meta.url),
      'utf8',
    ),
  );
  let resumed: CityEngine | undefined;
  try {
    const before: World = JSON.parse(engine.snapshot());
    const oldParcels = new Set(before.parcels.map((parcel) => parcel.id));
    const oldRoads = new Set(before.roads.map((road) => road.id));
    for (let batch = 0; batch < 32; batch++) engine.step(2000);
    const after: World = JSON.parse(engine.snapshot());
    expect(after.tick - before.tick).toBe(64000);
    const compact = after.parcels.filter(
      (parcel) => !oldParcels.has(parcel.id) && parcel.width >= 8 && parcel.width < 14,
    );
    expect(compact.length).toBeGreaterThan(0);
    const save = engine.save();
    const stored = JSON.parse(save) as {
      parcel_frontages: { parcel_id: number; road_id: number }[];
    };
    for (const parcel of compact) {
      expect(Number.isFinite(parcel.rotation)).toBe(true);
      const frontage = stored.parcel_frontages.find((item) => item.parcel_id === parcel.id);
      expect(frontage).toBeDefined();
      expect(
        after.roads.some(
          (road) =>
            road.stage === 'open' &&
            road.connected &&
            (road.from === parcel.access_node || road.to === parcel.access_node),
        ),
      ).toBe(true);
    }
    // Roads may now continue growing after the former 160-road stop. Require
    // compact infill on the original network without forbidding it on new roads.
    const existingStreetCompact = compact.filter((parcel) =>
      stored.parcel_frontages.some(
        (frontage) => frontage.parcel_id === parcel.id && oldRoads.has(frontage.road_id),
      ),
    );
    expect(existingStreetCompact.length).toBeGreaterThan(0);
    const compactIds = new Set(existingStreetCompact.map((parcel) => parcel.id));
    expect(
      after.buildings.some(
        (building) =>
          compactIds.has(building.parcel_id) &&
          building.stage === 'open' &&
          (building.occupancy > 0 || building.business_active),
      ),
    ).toBe(true);
    resumed = CityEngine.restore(save);
    expect(resumed.save()).toBe(save);
    engine.step(2000);
    resumed.step(2000);
    expect(resumed.save()).toBe(engine.save());
  } finally {
    resumed?.free();
    engine.free();
  }
}, 60000);
