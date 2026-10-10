// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import type { World } from '../web/src/types';

it('continues useful connected street growth beyond 160 roads and develops its frontage', () => {
  initSync({module:readFileSync(new URL('../public/wasm/city_engine_bg.wasm',import.meta.url))});
  const engine=CityEngine.restore(readFileSync(new URL('../validation/mature-world.bounded-selection.json',import.meta.url),'utf8'));
  let resumed:CityEngine|undefined;
  try {
    const before:World=JSON.parse(engine.snapshot());
    expect(before.roads.filter(road=>road.stage==='open').length).toBeGreaterThan(160);
    const existingRoads=new Set(before.roads.map(road=>road.id));
    for(let batch=0;batch<32;batch++)engine.step(2000);
    const after:World=JSON.parse(engine.snapshot());
    expect(after.tick-before.tick).toBe(64000);
    const newRoads=new Set(after.roads.filter(road=>!existingRoads.has(road.id)&&road.kind!=='rail'&&road.kind!=='bridge'&&road.stage==='open'&&road.connected).map(road=>road.id));
    expect(newRoads.size).toBeGreaterThan(0);
    const save=engine.save();
    const stored=JSON.parse(save) as {parcel_frontages:Array<{parcel_id:number;road_id:number}>};
    const newRoadParcels=new Set(stored.parcel_frontages.filter(frontage=>newRoads.has(frontage.road_id)).map(frontage=>frontage.parcel_id));
    expect(after.buildings.some(building=>newRoadParcels.has(building.parcel_id)&&building.stage==='open'&&(building.occupancy>0||building.business_active))).toBe(true);
    for(const parcel of after.parcels.filter(parcel=>newRoadParcels.has(parcel.id))) {
      expect(after.roads.some(road=>road.connected&&road.stage==='open'&&(road.from===parcel.access_node||road.to===parcel.access_node))).toBe(true);
    }
    resumed=CityEngine.restore(save);expect(resumed.save()).toBe(save);
    engine.step(2000);resumed.step(2000);expect(resumed.save()).toBe(engine.save());
  } finally {resumed?.free();engine.free();}
},60000);
