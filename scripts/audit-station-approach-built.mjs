// Uses production outline generation and current WASM snapshots, including corners.
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {build} from 'esbuild';
import {CityEngine,initSync} from '../public/wasm/city_engine.js';
await build({entryPoints:['web/src/render/station-approach.ts'],bundle:true,platform:'node',format:'esm',packages:'external',outfile:'.tools/station-approach-audit.mjs'});
const {stationApproachOutline}=await import('../.tools/station-approach-audit.mjs');
const wasm=readFileSync('public/wasm/city_engine_bg.wasm');initSync({module:wasm});
const cross=(a,b,c)=>(b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0]);
const inside=(p,poly)=>{let yes=false;for(let i=0,j=poly.length-1;i<poly.length;j=i++){const a=poly[i],b=poly[j];if((a[1]>p[1])!==(b[1]>p[1])&&p[0]<(b[0]-a[0])*(p[1]-a[1])/(b[1]-a[1])+a[0])yes=!yes;}return yes;};
const overlap=(a,b)=>a.some(p=>inside(p,b))||b.some(p=>inside(p,a))||a.some((p,i)=>b.some((q,j)=>cross(p,a[(i+1)%a.length],q)*cross(p,a[(i+1)%a.length],b[(j+1)%b.length])<0&&cross(q,b[(j+1)%b.length],p)*cross(q,b[(j+1)%b.length],a[(i+1)%a.length])<0));
const rectangle=b=>{const c=Math.cos(b.rotation),s=Math.sin(b.rotation);return[[-1,-1],[1,-1],[1,1],[-1,1]].map(([x,z])=>[b.x+x*b.width/2*c+z*b.depth/2*s,b.z-x*b.width/2*s+z*b.depth/2*c]);};
const cases=[];
for(const seed of [87231,0,5])for(const tick of [10000,80000]){
 const input=`.tools/approach-${seed}/tick-${tick}.json`,bytes=readFileSync(input),e=CityEngine.restore(bytes.toString());
 try{
  const w=JSON.parse(e.snapshot()),buildings=w.buildings.map(b=>({id:b.id,poly:rectangle(b)})),parcels=w.parcels.map(b=>({id:b.id,poly:rectangle(b)}));
  const river=w.terrain.river.slice(1).map((b,i)=>{const a=w.terrain.river[i],h=w.terrain.river_half_width;return[[a.x-h,a.z],[b.x-h,b.z],[b.x+h,b.z],[a.x+h,a.z]];});
  for(const p of w.rail_platforms){
   const local=stationApproachOutline(p).map(v=>[v.x,v.z]);
   const world=local.map(([x,z])=>[p.x+p.uz*x+p.ux*z,p.z-p.ux*x+p.uz*z]);
   const hits=items=>items.filter(b=>overlap(world,b.poly)).map(b=>b.id);
   const pier=[2,23.5,45].map(z=>rectangle({x:5.2,z,width:2.4,depth:2.4,rotation:0}));
   cases.push({seed,tick,node:p.node_id,phase:p.construction_phase,outline:world,
    outsideWorld:world.some(([x,z])=>x<w.bounds.min_x||x>w.bounds.max_x||z<w.bounds.min_z||z>w.bounds.max_z),
    buildingOverlaps:hits(buildings),parcelOverlaps:hits(parcels),pierOverlap:pier.some(poly=>overlap(local,poly)),
    protectedOverlap:w.terrain.protected.some(poly=>overlap(world,poly.map(v=>[v.x,v.z]))),riverOverlap:river.some(poly=>overlap(world,poly)),
    inputSha256:createHash('sha256').update(bytes).digest('hex')});
  }
 }finally{e.free();}
}
writeFileSync('validation/station-approach-built.json',JSON.stringify({method:'Production mitered paving outline; concave polygon containment/edge tests against envelopes. No road/terrain-height/all-object proof.',wasmSha256:createHash('sha256').update(wasm).digest('hex'),cases},null,2)+'\n');
console.log(JSON.stringify(cases.map(({outline,inputSha256,...rest})=>rest)));
if(cases.some(c=>c.outsideWorld||c.buildingOverlaps.length||c.parcelOverlaps.length||c.pierOverlap||c.protectedOverlap||c.riverOverlap))process.exitCode=1;
