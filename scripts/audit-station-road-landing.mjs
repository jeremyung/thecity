import {readFileSync,writeFileSync} from 'node:fs';
import {build} from 'esbuild';
await build({stdin:{contents:"export {groundRoadFootprint} from './web/src/render/ground-road-footprint'; export {roadWidth} from './web/src/render/road-elevation';",resolveDir:process.cwd(),loader:'ts'},bundle:true,platform:'node',format:'esm',packages:'external',outfile:'.tools/station-landing-audit.mjs'});
const {groundRoadFootprint,roadWidth}=await import('../.tools/station-landing-audit.mjs');
const cross=(a,b,c)=>(b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0]);
const inside=(p,poly)=>{let yes=false;for(let i=0,j=poly.length-1;i<poly.length;j=i++){const a=poly[i],b=poly[j];if((a[1]>p[1])!==(b[1]>p[1])&&p[0]<(b[0]-a[0])*(p[1]-a[1])/(b[1]-a[1])+a[0])yes=!yes;}return yes;};
const overlap=(a,b)=>a.some(p=>inside(p,b))||b.some(p=>inside(p,a))||a.some((p,i)=>b.some((q,j)=>cross(p,a[(i+1)%a.length],q)*cross(p,a[(i+1)%a.length],b[(j+1)%b.length])<0&&cross(q,b[(j+1)%b.length],p)*cross(q,b[(j+1)%b.length],a[(i+1)%a.length])<0));
const rectangle=b=>{const c=Math.cos(b.rotation),s=Math.sin(b.rotation);return[[-1,-1],[1,-1],[1,1],[-1,1]].map(([x,z])=>[b.x+x*b.width/2*c+z*b.depth/2*s,b.z-x*b.width/2*s+z*b.depth/2*c]);};

const cases=[];
for(const seed of [87231,0,5]){
 const w=JSON.parse(readFileSync(`.tools/walking-offset-${seed}/station-exit-world.json`));
 for(const road of w.roads.filter(r=>r.stage==='open'&&['local','lane','arterial'].includes(r.kind))){
  const a=w.nodes.find(n=>n.id===road.from),b=w.nodes.find(n=>n.id===road.to);
  if(a.kind!=='station'&&b.kind!=='station')continue;
  const full=groundRoadFootprint(a,b,roadWidth(road.kind));
  const legacy=groundRoadFootprint({...a,kind:undefined},{...b,kind:undefined},roadWidth(road.kind));
  const indices=a.kind==='station'?[0,3]:[1,2];
  const cap=[full[indices[0]],full[indices[1]],legacy[indices[1]],legacy[indices[0]]].map(p=>[p.x,p.z]);
  const hits=items=>items.filter(item=>overlap(cap,rectangle(item))).map(item=>item.id);
  const piers=w.rail_platforms.flatMap(p=>[2,23.5,45].map(z=>({id:`${p.node_id}:${z}`,x:p.x+p.uz*5.2+p.ux*z,z:p.z-p.ux*5.2+p.uz*z,width:2.4,depth:2.4,rotation:Math.atan2(p.ux,p.uz)})));
  cases.push({seed,road:road.id,cap,outsideWorld:cap.some(([x,z])=>x<w.bounds.min_x||x>w.bounds.max_x||z<w.bounds.min_z||z>w.bounds.max_z),buildingOverlaps:hits(w.buildings),parcelOverlaps:hits(w.parcels),pierOverlaps:hits(piers),protectedOverlap:w.terrain.protected.some(p=>overlap(cap,p.map(v=>[v.x,v.z]))),riverOverlap:w.terrain.river.slice(1).some((b,i)=>{const a=w.terrain.river[i],h=w.terrain.river_half_width;return overlap(cap,[[a.x-h,a.z],[b.x-h,b.z],[b.x+h,b.z],[a.x+h,a.z]]);})});
 }
}
writeFileSync('validation/station-road-landing-clearance.json',JSON.stringify({scope:'Exact additional station end apron only, checked against 2D building/parcel/station-pier/protected/river envelopes and world bounds.',cases},null,2)+'\n');
console.log(cases);
if(cases.some(c=>c.outsideWorld||c.buildingOverlaps.length||c.parcelOverlaps.length||c.pierOverlaps.length||c.protectedOverlap||c.riverOverlap))process.exitCode=1;
