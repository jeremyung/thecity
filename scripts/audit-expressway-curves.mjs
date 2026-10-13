// Inspect authoritative cubic geometry in a native-exported snapshot after WASM restore.
// Usage: node scripts/audit-expressway-curves.mjs snapshot.json
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {CityEngine,initSync} from '../public/wasm/city_engine.js';
initSync({module:readFileSync(new URL('../public/wasm/city_engine_bg.wasm',import.meta.url))});
const file=process.argv[2];
if(!file)throw Error('Pass a native master-plan snapshot with its .save.json companion');
const native=JSON.parse(readFileSync(file,'utf8'));
const engine=CityEngine.restore(readFileSync(`${file}.save.json`,'utf8'));
try{
 const world=JSON.parse(engine.snapshot());
 assert.deepEqual(world.road_decks,native.road_decks,'native/WASM curve definitions differ');
 const nodes=new Map(world.nodes.map(n=>[n.id,n]));const roads=new Map(world.roads.map(r=>[r.id,r]));
 let minRadius=Infinity;
 for(const deck of world.road_decks){
  const road=roads.get(deck.road_id),a=nodes.get(road.from),b=nodes.get(road.to);
  const [c,d]=deck.controls.map(([x,z])=>({x,z}));
  for(let i=0;i<=128;i++){
   const t=i/128,s=1-t;
   const vx=3*s*s*(c.x-a.x)+6*s*t*(d.x-c.x)+3*t*t*(b.x-d.x);
   const vz=3*s*s*(c.z-a.z)+6*s*t*(d.z-c.z)+3*t*t*(b.z-d.z);
   const ax=6*s*(d.x-2*c.x+a.x)+6*t*(b.x-2*d.x+c.x);
   const az=6*s*(d.z-2*c.z+a.z)+6*t*(b.z-2*d.z+c.z);
   const speed=Math.hypot(vx,vz),cross=Math.abs(vx*az-vz*ax);
   assert(speed>1e-4,'degenerate road tangent');
   if(cross>1e-8)minRadius=Math.min(minRadius,speed**3/cross);
  }
 }
 assert(minRadius>10.8,`inner deck folds: minimum curve radius ${minRadius}`);
 const saved=engine.save(),restored=CityEngine.restore(saved);
 try{for(let i=0;i<10;i++){engine.step(80);restored.step(80);assert.equal(engine.save(),restored.save());}}
 finally{restored.free();}
 console.log(JSON.stringify({seed:world.seed,tick:world.tick,roads:world.road_decks.length,minSampledRadius:minRadius,nativeWasmControls:'equal',replayTicks:800},null,2));
}finally{engine.free();}
