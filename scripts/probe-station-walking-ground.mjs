// Diagnostic support union, including boundary points; not a mesh/height clearance test.
import {readFileSync,writeFileSync} from 'node:fs';
import {build} from 'esbuild';
await build({stdin:{contents:"export {stationApproachOutline} from './web/src/render/station-approach'; export {junctionFootprint} from './web/src/render/road-junction'; export {roadWidth} from './web/src/render/road-elevation'; export {groundRoadFootprint} from './web/src/render/ground-road-footprint';",resolveDir:process.cwd(),loader:'ts'},bundle:true,platform:'node',format:'esm',packages:'external',outfile:'.tools/station-walking-audit.mjs'});
const {stationApproachOutline,junctionFootprint,roadWidth,groundRoadFootprint}=await import('../.tools/station-walking-audit.mjs');
const w=JSON.parse(readFileSync(process.argv[2] ?? '.tools/walking-offset-87231/station-exit-world.json'));
const trace=JSON.parse(readFileSync(process.argv[3] ?? 'validation/station-walking-offset-exit-trace.json'));
const inside=(p,poly)=>{let yes=false;for(let i=0,j=poly.length-1;i<poly.length;j=i++){const a=poly[i],b=poly[j],dx=b[0]-a[0],dz=b[1]-a[1],t=Math.max(0,Math.min(1,((p[0]-a[0])*dx+(p[1]-a[1])*dz)/(dx*dx+dz*dz)));if(Math.hypot(p[0]-a[0]-t*dx,p[1]-a[1]-t*dz)<1e-5)return true;if((a[1]>p[1])!==(b[1]>p[1])&&p[0]<(b[0]-a[0])*(p[1]-a[1])/(b[1]-a[1])+a[0])yes=!yes;}return yes;};
const surfaces=w.ground.map(g=>g.points);
for(const p of w.rail_platforms)surfaces.push(stationApproachOutline(p).map(v=>[p.x+p.uz*v.x+p.ux*v.z,p.z-p.ux*v.x+p.uz*v.z]));
for(const r of w.roads.filter(r=>r.stage==='open'&&['local','lane','arterial'].includes(r.kind))){
 const a=w.nodes.find(n=>n.id===r.from),b=w.nodes.find(n=>n.id===r.to);
 surfaces.push(groundRoadFootprint(a,b,roadWidth(r.kind)).map(p=>[p.x,p.z]));
}
const junctions=new Map();
for(const r of w.roads.filter(r=>r.stage==='open'&&!['rail','bridge','expressway'].includes(r.kind))){
 const a=w.nodes.find(n=>n.id===r.from),b=w.nodes.find(n=>n.id===r.to);
 for(const [node,end] of [[a,b],[b,a]]){const arms=junctions.get(node.id)??[];arms.push({end,width:roadWidth(r.kind)});junctions.set(node.id,arms);}
}
for(const [id,arms] of junctions){const poly=junctionFootprint(w.nodes.find(n=>n.id===id),arms);if(poly.length)surfaces.push(poly.map(p=>[p.x,p.z]));}
let count=0;const uncovered=[];const bodyUncovered=[];const path=trace.actor.path.points;
for(let i=4;i<path.length;i++){const a=path[i-1],b=path[i],n=Math.ceil(Math.hypot(b.x-a.x,b.z-a.z)/.1);for(let j=0;j<=n;j++){const p=[a.x+(b.x-a.x)*j/n,a.z+(b.z-a.z)*j/n];count++;if(!surfaces.some(poly=>inside(p,poly)))uncovered.push({leg:i,point:p});
 const length=Math.hypot(b.x-a.x,b.z-a.z),nx=-(b.z-a.z)/length,nz=(b.x-a.x)/length;
 for(const side of [-1,1]){const edge=[p[0]+nx*.33*side,p[1]+nz*.33*side];if(!surfaces.some(poly=>inside(edge,poly)))bodyUncovered.push({leg:i,point:edge});}}}
const report={scope:'Plan-only centerline and lateral +/-0.33m probe against production approach outline, production ground carriageway footprints and emitted ground polygons; includes production junction footprints; excludes arterial sidewalks and height.',count,uncovered,bodyUncovered};writeFileSync(process.argv[4] ?? 'validation/station-walking-offset-ground-probe.json',JSON.stringify(report,null,2)+'\n');console.log({count,uncovered:uncovered.length,bodyUncovered:bodyUncovered.length,firstBody:bodyUncovered[0],lastBody:bodyUncovered.at(-1),first:uncovered[0],last:uncovered.at(-1)});
