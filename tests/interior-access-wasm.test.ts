// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import type { World } from '../web/src/types';

type Lot={x:number;z:number;width:number;depth:number;rotation:number;along:number;side:number};
type Plan={host_t:number;split_node_id:number;branch_endpoint_id:number;branch_road_id:number;project_id:number;selected_lots:Lot[]};
type Saved={tick:number;pending_interior_access:Plan|null;roads:World['roads'];projects:World['projects'];
  parcels:Array<Lot&{id:number;access_node:number}>;parcel_frontages:Array<{parcel_id:number;road_id:number;along:number;side:number}>};
const saved=(engine:CityEngine):Saved=>JSON.parse(engine.save());

it.each([
  ['canonical organic branch', 'interior-access-verified-87231.json.project-3736.pending.json'],
  ['geometry-derived noncanonical branch', 'generalized-access-pending-save.json'],
])('WASM resumes %s and releases lots only after construction',(_label,fixture)=>{
  initSync({module:readFileSync(new URL('../public/wasm/city_engine_bg.wasm',import.meta.url))});
  const engine=CityEngine.restore(readFileSync(new URL(`./fixtures/access-joins/${fixture}`,import.meta.url),'utf8'));
  let resumed:CityEngine|undefined;
  try{
    let state=saved(engine);
    // Capture a real admitted project to isolate its lifecycle from changes
    // in infill priority. Native growth tests cover admission itself.
    expect(state.pending_interior_access).not.toBeNull();
    const plan=state.pending_interior_access!;
    const branch=state.roads.find(road=>road.id===plan.branch_road_id)!;
    expect(branch.stage).not.toBe('open');expect(branch.connected).toBe(false);
    const snapshot:World=JSON.parse(engine.snapshot());
    expect(snapshot.roads.find(road=>road.id===branch.id)).toMatchObject({stage:branch.stage,progress:branch.progress,connected:false});
    const oldParcelIds=new Set(state.parcels.map(parcel=>parcel.id));
    const canonical=engine.save();resumed=CityEngine.restore(canonical);
    expect(resumed.save()).toBe(canonical);
    // The fraction must describe the actual host junction, not merely fall in [0,1].
    const corrupt=JSON.parse(canonical);
    corrupt.pending_interior_access.host_t=plan.host_t===.5?.25:.5;
    let accepted:CityEngine|undefined;
    try{expect(()=>{accepted=CityEngine.restore(JSON.stringify(corrupt));}).toThrow();}
    finally{accepted?.free();}
    let sawProgress=false;
    for(let batch=0;batch<100&&state.pending_interior_access;batch++){
      engine.step(8);resumed.step(8);
      expect(resumed.save()).toBe(engine.save());
      state=saved(engine);
      if(state.pending_interior_access){
        const road=state.roads.find(road=>road.id===branch.id)!;
        const project=state.projects.find(project=>project.id===plan.project_id)!;
        expect(road.connected).toBe(false);expect(road.stage).not.toBe('open');
        expect(road.progress).toBe(project.progress);expect(road.stage).toBe(project.stage);
        sawProgress ||= road.progress>branch.progress;
        for(const lot of plan.selected_lots)expect(state.parcels.some(parcel=>Math.hypot(parcel.x-lot.x,parcel.z-lot.z)<.001)).toBe(false);
      }
    }
    expect(sawProgress).toBe(true);
    expect(state.pending_interior_access).toBeNull();
    expect(state.roads.find(road=>road.id===branch.id)).toMatchObject({stage:'open',connected:true});
    for(const lot of plan.selected_lots){
      const parcel=state.parcels.find(parcel=>!oldParcelIds.has(parcel.id)&&Math.hypot(parcel.x-lot.x,parcel.z-lot.z)<.001)!;
      expect(parcel).toBeDefined();
      for(const key of ['width','depth','rotation'] as const)expect(parcel[key]).toBeCloseTo(lot[key],3);
      const frontage=state.parcel_frontages.find(frontage=>frontage.parcel_id===parcel.id)!;
      expect(frontage.road_id).toBe(branch.id);expect(frontage.side).toBe(lot.side);
      expect(frontage.along).toBeCloseTo(lot.along,5);
    }
    engine.step(2000);resumed.step(2000);
    expect(resumed.save()).toBe(engine.save());
  }finally{resumed?.free();engine.free();}
},60000);
