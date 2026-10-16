// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

type Lot = { center: { x: number; z: number }; width: number; depth: number; rotation: number };
type Saved = {
  neighborhood_plan: null | { segments: number[][]; selected_lots?: Lot[] };
  parcels: Array<{
    id: number;
    x: number;
    z: number;
    width: number;
    depth: number;
    rotation?: number | null;
    access_node: number;
  }>;
  parcel_frontages: Array<{ parcel_id: number; road_id: number; along: number; side: number }>;
};
const saved = (engine: CityEngine): Saved => JSON.parse(engine.save());

it('naturally reserves partial shared-edge lots and completes the same layout after restore', () => {
  initSync({
    module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)),
  });
  const engine = CityEngine.newLegacy(87231);
  let resumed: CityEngine | undefined;
  try {
    engine.set_development_rate(16);
    let state = saved(engine);
    for (let batch = 0; batch < 36 && !state.neighborhood_plan?.selected_lots; batch++) {
      engine.step(2000);
      state = saved(engine);
    }
    const plan = state.neighborhood_plan;
    expect(plan?.selected_lots).toBeDefined();
    const lots = plan!.selected_lots!;
    expect(lots.length).toBeGreaterThanOrEqual(4);
    expect(lots.length).toBeLessThan(10);
    const original = new Map(state.parcels.map((parcel) => [parcel.id, parcel]));
    const frontages = state.parcel_frontages;
    const canonical = engine.save();
    resumed = CityEngine.restore(canonical);
    expect(resumed.save()).toBe(canonical);
    for (let batch = 0; batch < 100 && saved(engine).neighborhood_plan; batch++) {
      engine.step(80);
      resumed.step(80);
      expect(resumed.save()).toBe(engine.save());
    }
    const after = saved(engine);
    expect(after.neighborhood_plan).toBeNull();
    for (const parcel of state.parcels) {
      const retained = after.parcels.find((item) => item.id === parcel.id)!;
      expect(retained).toBeDefined();
      for (const key of ['x', 'z', 'width', 'depth', 'rotation', 'access_node'] as const)
        expect(retained[key]).toEqual(parcel[key]);
    }
    for (const frontage of frontages) expect(after.parcel_frontages).toContainEqual(frontage);
    const added = after.parcels.filter((parcel) => !original.has(parcel.id));
    for (const lot of lots)
      expect(
        added.some(
          (parcel) =>
            Math.abs(parcel.x - lot.center.x) < 0.001 &&
            Math.abs(parcel.z - lot.center.z) < 0.001 &&
            Math.abs(parcel.width - lot.width) < 0.001 &&
            Math.abs(parcel.depth - lot.depth) < 0.001,
        ),
      ).toBe(true);
  } finally {
    resumed?.free();
    engine.free();
  }
}, 60000);
