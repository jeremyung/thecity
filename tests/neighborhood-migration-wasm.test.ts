// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

it('retires a completed legacy block without losing its street classification or changing its geometry', () => {
  initSync({module:readFileSync(new URL('../public/wasm/city_engine_bg.wasm',import.meta.url))});
  const raw=readFileSync(new URL('../validation/mature-world.activity-baseline.json',import.meta.url),'utf8');
  const old=JSON.parse(raw);
  expect(old.neighborhood_road_ids).toBeUndefined();
  expect(old.neighborhood_plan.next_segment).toBe(4);
  const ids=old.neighborhood_plan.segments.map(([a,b]:[number,number])=>
    old.roads.find((r:{from:number;to:number})=>(r.from===a&&r.to===b)||(r.from===b&&r.to===a)).id);
  const engine=CityEngine.restore(raw);
  let replay:CityEngine|undefined;
  try {
    const migrated=JSON.parse(engine.save());
    expect(migrated.neighborhood_plan).toBeNull();
    expect(migrated.neighborhood_road_ids).toEqual([...ids].sort((a,b)=>a-b));
    for(const field of ['buildings','parcels','roads','nodes','residents','events','rng_growth','tick']) {
      expect(old[field],field).toBeDefined();
      expect(migrated[field],field).toEqual(old[field]);
    }
    replay=CityEngine.restore(engine.save());
    expect(replay.save()).toBe(engine.save());
    engine.step(2000);replay.step(2000);
    expect(replay.save()).toBe(engine.save());
    const continued=JSON.parse(engine.save());
    for(const id of ids) expect(continued.neighborhood_road_ids).toContain(id);
  } finally { replay?.free();engine.free(); }
},20000);
