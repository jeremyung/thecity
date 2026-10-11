import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {CityEngine,initSync} from '../public/wasm/city_engine.js';
const sha=value=>createHash('sha256').update(value).digest('hex');
const input=readFileSync('validation/early-reservation-wasm-87231-save.json','utf8');
const wasm=readFileSync('public/wasm/city_engine_bg.wasm');
initSync({module:wasm});
const engine=CityEngine.restore(input);
const original=JSON.parse(engine.snapshot());
const originalIds=new Set(original.buildings.map(b=>b.id));
let resumed;const checkpoints=[];
try {
 engine.set_development_rate(16);
 for(let batch=1;batch<=150;batch++) {
  engine.step(200);
  if(resumed) {resumed.step(200);assert.equal(resumed.save(),engine.save(),`save mismatch at batch${batch}`);}
  if(batch===75) {resumed=CityEngine.restore(engine.save());assert.equal(resumed.save(),engine.save());}
  if(batch%25===0) {const s=JSON.parse(engine.snapshot());checkpoints.push({tick:s.tick,buildings:s.buildings.length,roads:s.roads.length});}
 }
 const final=JSON.parse(engine.snapshot());const state=JSON.parse(engine.save());
 const added=final.buildings.filter(b=>!originalIds.has(b.id));
 const narrow=added.filter(b=>b.width===8&&b.depth===12);
 assert.ok(final.buildings.length<=2000&&final.parcels.length<=4000&&final.roads.length<=5000);
 for(const b of narrow) {const parcel=final.parcels.find(p=>p.id===b.parcel_id);assert.ok(parcel);assert.ok(state.parcel_frontages.some(f=>f.parcel_id===parcel.id));}
 const report={scope:'Node WASM mature continuation; normal scheduling and admissions, no cursor override.30,000ticks at rate16. Save/resume exact every200ticks during second15,000ticks. Not browser performance or visual acceptance.',inputSha256:sha(input),wasmSha256:sha(wasm),checkpoints,initialBuildings:original.buildings.length,finalBuildings:final.buildings.length,addedBuildings:added.map(b=>({id:b.id,parcelId:b.parcel_id,width:b.width,depth:b.depth,stage:b.stage,occupancy:b.occupancy,x:b.x,z:b.z})),narrow8x12Count:narrow.length,narrow8x12OpenCount:narrow.filter(b=>b.stage==='open').length,finalSaveSha256:sha(engine.save()),resumeExact:true};
 writeFileSync('validation/synthetic-morphology/narrow-frontage-wasm.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({initial:report.initialBuildings,final:report.finalBuildings,narrow:narrow.length,open:report.narrow8x12OpenCount,resumeExact:true}));
} finally {resumed?.free();engine.free();}
