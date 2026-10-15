// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import type { World } from '../web/src/types';

it('constructs shallow infill on connected existing streets and replays the resulting city', () => {
  initSync({ module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)) });
  const engine = CityEngine.restore(readFileSync(new URL('./fixtures/access-joins/spine-ranking-seed-87231-by-sites-false.json', import.meta.url), 'utf8'));
  let resumed: CityEngine | undefined;
  try {
    const before: World = JSON.parse(engine.snapshot());
    const previousIds = new Set(before.parcels.map(p => p.id));
    engine.set_development_rate(16);
    for (let batch = 0; batch < 100; batch++) engine.step(200);
    const after: World = JSON.parse(engine.snapshot());
    expect(after.tick - before.tick).toBe(20000);
    const added = after.parcels.filter(p => !previousIds.has(p.id) && p.depth === 8);
    expect(added.length).toBeGreaterThan(0);
    // Unrelated network growth may continue during this interval. Verify the
    // actual admitted infill uses streets that were already open, rather than
    // requiring the entire city's road count to remain frozen.
    const existingRoads = new Set(before.roads.filter(r => r.connected && r.stage === 'open').map(r => r.id));
    let completedInfillOnExistingStreets = 0;
    for (const parcel of added) {
      expect(after.buildings.some(b => b.parcel_id === parcel.id && b.stage === 'open')).toBe(true);
      const building = after.buildings.find(b => b.parcel_id === parcel.id)!;
      const access = after.building_access?.find(a => a.building_id === building.id);
      expect(access, `shallow parcel ${parcel.id} must have resolved access`).toBeDefined();
      if (existingRoads.has(access!.road_id)) completedInfillOnExistingStreets++;
      expect(after.roads.some(r => r.connected && r.stage === 'open' &&
        (r.from === parcel.access_node || r.to === parcel.access_node))).toBe(true);
    }
    // Some shallow parcels can also be built on newly opened streets. Those
    // must not be mistaken for evidence of infill on the original network.
    expect(completedInfillOnExistingStreets).toBeGreaterThan(0);
    const checkpoint = engine.save();
    resumed = CityEngine.restore(checkpoint);
    expect(resumed.save()).toBe(checkpoint);
    engine.step(400); resumed.step(400);
    expect(resumed.save()).toBe(engine.save());
  } finally { resumed?.free(); engine.free(); }
}, 60000);
