// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import type { Building, World } from '../web/src/types';

it('migrates a real legacy city without changing its history or treating commerce as residents', () => {
  initSync({module:readFileSync(new URL('../public/wasm/city_engine_bg.wasm',import.meta.url))});
  const raw=readFileSync(new URL('../validation/mature-world.json',import.meta.url),'utf8');
  const legacy=JSON.parse(raw);
  expect(legacy.activity_schema_version).toBeUndefined();
  const engine=CityEngine.restore(raw);
  let replay:CityEngine|undefined;
  try {
    const migrated=JSON.parse(engine.save());
    expect(migrated.activity_schema_version).toBe(2);
    for(const field of ['tick','rng_growth','rng_schedule','rng_weather','next_id','events','residents','roads','parcels']) {
      expect(legacy[field],field).toBeDefined();
      expect(migrated[field],field).toEqual(legacy[field]);
    }
    const world:World=JSON.parse(engine.snapshot());
    const commerce=world.buildings.filter(b=>['shop','cafe','office'].includes(b.kind));
    expect(commerce.length).toBeGreaterThan(100);
    for(const b of commerce) {
      expect(b.business_active).toBe(b.stage==='open');
      expect(b.occupancy).toBe(0);
      expect(b.vacant_days).toBe(0);
    }
    for(const b of world.buildings.filter(b=>['home','apartment'].includes(b.kind))) {
      const old=legacy.buildings.find((previous:Building)=>previous.id===b.id);
      expect(b.occupancy).toBe(old.occupancy);
      expect(b.business_active).toBe(false);
    }
    replay=CityEngine.restore(engine.save());
    expect(replay.save()).toBe(engine.save());
    engine.step(2000);replay.step(2000);
    expect(replay.save()).toBe(engine.save());
    expect(replay.movements()).toEqual(engine.movements());
  } finally { replay?.free();engine.free(); }
},20000);
