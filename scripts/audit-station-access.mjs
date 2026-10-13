// Check current engine platform envelopes through fresh saved development stages.
// This is plan-view evidence, not detailed mesh clearance or visual acceptance.
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {CityEngine,initSync} from '../public/wasm/city_engine.js';
const wasm=readFileSync('public/wasm/city_engine_bg.wasm');initSync({module:wasm});
const [output,...inputs]=process.argv.slice(2);if(!output||!inputs.length)throw Error('Expected OUTPUT SAVE...');
const overlap=(a,b)=>[a,b].every(poly=>poly.every((u,i)=>{
 const v=poly[(i+1)%poly.length],axis=[u[1]-v[1],v[0]-u[0]];
 const aa=a.map(p=>p[0]*axis[0]+p[1]*axis[1]),bb=b.map(p=>p[0]*axis[0]+p[1]*axis[1]);
 return Math.max(...aa)>Math.min(...bb)&&Math.max(...bb)>Math.min(...aa);
}));
const results=[];
for(const input of inputs){
 const bytes=readFileSync(input),state=JSON.parse(bytes),e=CityEngine.restore(bytes.toString());
 try {
 const w=JSON.parse(e.snapshot());
 const buildings=w.buildings.map(b=>{const c=Math.cos(b.rotation),s=Math.sin(b.rotation);return {id:b.id,poly:[[-1,-1],[1,-1],[1,1],[-1,1]].map(([x,z])=>{x*=b.width/2;z*=b.depth/2;return[b.x+x*c+z*s,b.z-x*s+z*c];})};});
 results.push({input,sha256:createHash('sha256').update(bytes).digest('hex'),seed:w.seed,tick:w.tick,stage:w.stage,stationReady:state.station_ready,stationOnline:state.station_online,stationProject:w.projects.find(p=>p.kind==='station')??null,buildings:w.buildings.length,platforms:(w.rail_platforms??[]).map(p=>{
 const map=([x,z])=>[p.x+p.uz*x+p.ux*z,p.z-p.ux*x+p.uz*z];
 const stair=[[p.stair_x-1.2,p.stair_bottom_z],[p.stair_x+1.2,p.stair_bottom_z],[p.stair_x+1.2,p.stair_top_z],[p.stair_x-1.2,p.stair_top_z]].map(map);
 return {node:p.node_id,kind:w.nodes.find(n=>n.id===p.node_id)?.kind,stairCorners:stair,outsideWorld:stair.some(([x,z])=>x<w.bounds.min_x||x>w.bounds.max_x||z<w.bounds.min_z||z>w.bounds.max_z),buildingEnvelopeOverlaps:buildings.filter(b=>overlap(stair,b.poly)).map(b=>b.id)};
 })});
 }finally{e.free();}
}
writeFileSync(output,JSON.stringify({method:'Restored current WASM snapshots. 2D SAT between complete 2.4m stair rectangles and rotated saved building envelopes. No terrain/height/all-infrastructure collision claim.',wasmSha256:createHash('sha256').update(wasm).digest('hex'),results},null,2)+'\n');
console.log(JSON.stringify({checkpoints:results.length,stairs:results.flatMap(r=>r.platforms).length,boundaryFailures:results.flatMap(r=>r.platforms).filter(p=>p.outsideWorld).length,overlaps:results.flatMap(r=>r.platforms).filter(p=>p.buildingEnvelopeOverlaps.length).length,unfinished:results.filter(r=>r.platforms.length&&!r.stationReady).map(r=>({seed:r.seed,tick:r.tick,project:r.stationProject}))}));
