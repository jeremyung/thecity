// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import type { World } from '../web/src/types';

it('develops explicitly oriented street-facing lots through the public engine and resumes them', () => {
  initSync({
    module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)),
  });
  const engine = CityEngine.newLegacy(87231);
  let replay: CityEngine | undefined;
  try {
    engine.set_development_rate(16);
    let world: World = JSON.parse(engine.snapshot());
    let occupied = false;
    for (let tick = 0; tick < 242000; tick += 2000) {
      engine.step(2000);
      world = JSON.parse(engine.snapshot());
      const lots = new Set(
        world.parcels.filter((parcel) => parcel.rotation !== undefined).map((parcel) => parcel.id),
      );
      occupied = world.buildings.some(
        (building) =>
          lots.has(building.parcel_id) && building.stage === 'open' && building.occupancy > 0,
      );
      if (occupied && lots.size >= 10) break;
    }
    expect(occupied, 'natural growth must open occupied homes in explicitly oriented parcels').toBe(
      true,
    );
    const lots = world.parcels.filter((parcel) => parcel.rotation !== undefined);
    expect(lots.length).toBeGreaterThanOrEqual(10);
    for (const lot of lots) {
      expect(Number.isFinite(lot.rotation)).toBe(true);
      expect(
        world.roads.some(
          (road) =>
            road.connected &&
            road.stage === 'open' &&
            (road.from === lot.access_node || road.to === lot.access_node),
        ),
      ).toBe(true);
      const building = world.buildings.find((building) => building.parcel_id === lot.id);
      if (building) {
        expect(Math.sin(building.rotation)).toBeCloseTo(Math.sin(lot.rotation!), 5);
        expect(Math.cos(building.rotation)).toBeCloseTo(Math.cos(lot.rotation!), 5);
      }
    }
    const save = engine.save();
    replay = CityEngine.restore(save);
    expect(replay.save()).toBe(save);
    engine.step(2000);
    replay.step(2000);
    expect(replay.save()).toBe(engine.save());
  } finally {
    replay?.free();
    engine.free();
  }
}, 60000);
