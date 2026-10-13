// Design audit only: candidate paving along station access, not implemented geometry.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
const wasm=readFileSync('public/wasm/city_engine_bg.wasm');initSync({module:wasm});
const sat=(a,b)=>[a,b].every(poly=>poly.every((u,i)=>{const v=poly[(i+1)%poly.length],axis=[u[1]-v[1],v[0]-u[0]],aa=a.map(p=>p[0]*axis[0]+p[1]*axis[1]),bb=b.map(p=>p[0]*axis[0]+p[1]*axis[1]);return Math.max(...aa)>Math.min(...bb)+1e-5&&Math.max(...bb)>Math.min(...aa)+1e-5;}));
const rectangle=b=>{const c=Math.cos(b.rotation),s=Math.sin(b.rotation);return[[-1,-1],[1,-1],[1,1],[-1,1]].map(([x,z])=>[b.x+x*b.width/2*c+z*b.depth/2*s,b.z-x*b.width/2*s+z*b.depth/2*c]);};
const band=(a,b,w)=>{const len=Math.hypot(b[0]-a[0],b[1]-a[1]),nx=-(b[1]-a[1])/len*w/2,nz=(b[0]-a[0])/len*w/2;return [[a[0]+nx,a[1]+nz],[b[0]+nx,b[1]+nz],[b[0]-nx,b[1]-nz],[a[0]-nx,a[1]-nz]];};
const cases=[];
for(const [seed,dir] of [[87231,'.tools/stair-review'],[0,'.tools/stair-seed0'],[5,'.tools/stair-seed5']]){
 const input=`${dir}/tick-80000.json`,bytes=readFileSync(input),e=CityEngine.restore(bytes.toString());
 try{
 const w=JSON.parse(e.snapshot()),river=w.terrain.river.slice(1).map((b,i)=>{const a=w.terrain.river[i],h=w.terrain.river_half_width;return [[a.x-h,a.z],[b.x-h,b.z],[b.x+h,b.z],[a.x+h,a.z]];}),buildings=w.buildings.map(b=>({id:b.id,poly:rectangle(b)})),parcels=w.parcels.map(b=>({id:b.id,poly:rectangle(b)}));
 for(const p of w.rail_platforms)for(const side of [6.9,7.5]){
 const inward=p.stair_bottom_z>p.stair_top_z,apron=p.stair_bottom_z+(inward?1.5:-1.5);
 const local=[[0,0],[side,-1.5],[side,apron],[p.stair_x,apron],[p.stair_x,p.stair_bottom_z]],map=([x,z])=>[p.x+p.uz*x+p.ux*z,p.z-p.ux*x+p.uz*z],width=1.6;
 const bands=local.slice(1).map((b,i)=>band(local[i],b,width)),polys=bands.map(poly=>poly.map(map));
 const intersects=items=>items.filter(b=>polys.some(poly=>sat(poly,b.poly))).map(b=>b.id);
 const supports=[2,23.5,45].map(z=>rectangle({x:5.2,z,width:2.4,depth:2.4,rotation:0}));
 cases.push({seed,node:p.node_id,terminal:inward?'gateway':'station',side,width,points:local.map(map),outsideWorld:polys.some(poly=>poly.some(([x,z])=>x<w.bounds.min_x||x>w.bounds.max_x||z<w.bounds.min_z||z>w.bounds.max_z)),buildingOverlaps:intersects(buildings),parcelOverlaps:intersects(parcels),riverOverlaps:river.filter(r=>polys.some(q=>sat(q,r))).length,protectedOverlaps:w.terrain.protected.filter(poly=>polys.some(q=>sat(q,poly.map(p=>[p.x,p.z])))).length,pierOverlaps:bands.some(poly=>supports.some(s=>sat(poly,s))),input,inputSha256:createHash('sha256').update(bytes).digest('hex')});
 }
 }finally{e.free();}
}
writeFileSync('validation/station-approach-candidate.json',JSON.stringify({method:'Proposed 1.6m route strip, butt joins; 2D footprint audit of mature/developing checkpoints. No paving implementation, terrain-height/road overlap or full collision proof; river check uses supplied piecewise-linear horizontal half-width.',wasmSha256:createHash('sha256').update(wasm).digest('hex'),cases},null,2)+'\n');
console.log(JSON.stringify(cases.map(({points,input,inputSha256,...rest})=>rest)));
