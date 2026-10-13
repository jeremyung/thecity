// Read-only geometry diagnostic for the naturally seeded office 1814 road scene.
// Computes projected XZ intersection areas for the renderer's ground-road boxes.
import assert from 'node:assert/strict';
import { build } from 'esbuild';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

const out = resolve('validation/road-surface-overlap.json');
const seed = 0, developmentRate = 16, targetTick = 9200, sampleTicks = 200;
const target = { id: 1814, x: 129.54147, z: 4.5897775, radius: 52 };
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
await build({ stdin: { contents: "export {junctionFootprint} from './web/src/render/road-junction'; export {roadWidth} from './web/src/render/road-elevation';", resolveDir: process.cwd() }, bundle: true, platform: 'node', format: 'esm', outfile: '.tools/road-junction-diagnostic.mjs' });
const { junctionFootprint, roadWidth } = await import('../.tools/road-junction-diagnostic.mjs');
initSync({ module: await readFile('public/wasm/city_engine_bg.wasm') });

const engine = new CityEngine(seed);
const worlds = [];
try {
  engine.set_development_rate(developmentRate);
  while (JSON.parse(engine.snapshot()).tick < targetTick) {
    const tick = JSON.parse(engine.snapshot()).tick;
    engine.step(Math.min(200, targetTick - tick));
  }
  for (let i = 0; i <= sampleTicks; i++) {
    worlds.push(JSON.parse(engine.snapshot()));
    if (i < sampleTicks) engine.step(1);
  }
} finally { engine.free(); }
assert.equal(worlds[0].tick, targetTick);

const cross = (a,b,c) => (b.x-a.x)*(c.z-a.z)-(b.z-a.z)*(c.x-a.x);
function area(poly) { let s=0; for(let i=0;i<poly.length;i++){const a=poly[i],b=poly[(i+1)%poly.length];s+=a.x*b.z-b.x*a.z;}return Math.abs(s)/2; }
function clip(subject, clipper) {
  let p=subject;
  const sign=Math.sign(clipper.reduce((s,a,i)=>s+a.x*clipper[(i+1)%clipper.length].z-clipper[(i+1)%clipper.length].x*a.z,0))||1;
  for(let i=0;i<clipper.length&&p.length;i++){
    const a=clipper[i],b=clipper[(i+1)%clipper.length],q=[];
    for(let j=0;j<p.length;j++){const u=p[j],v=p[(j+1)%p.length],cu=sign*cross(a,b,u),cv=sign*cross(a,b,v),iu=cu>=-1e-9,iv=cv>=-1e-9;
      if(iu)q.push(u); if(iu!==iv){const t=cu/(cu-cv);q.push({x:u.x+t*(v.x-u.x),z:u.z+t*(v.z-u.z)});}}
    p=q;
  }
  return p;
}
function rectangle(a,b,width) {
  const dx=b.x-a.x,dz=b.z-a.z,len=Math.hypot(dx,dz),nx=-dz/len*width/2,nz=dx/len*width/2;
  return [{x:a.x+nx,z:a.z+nz},{x:b.x+nx,z:b.z+nz},{x:b.x-nx,z:b.z-nz},{x:a.x-nx,z:a.z-nz}];
}
function centerlineRelation(a,b,c,d) {
  const orient=(p,q,r)=>cross(p,q,r), eps=1e-9;
  const o1=orient(a,b,c),o2=orient(a,b,d),o3=orient(c,d,a),o4=orient(c,d,b);
  const intersects=o1*o2<-eps&&o3*o4<-eps;
  const pointSegment=(p,u,v)=>{const dx=v.x-u.x,dz=v.z-u.z,l2=dx*dx+dz*dz,t=l2?Math.max(0,Math.min(1,((p.x-u.x)*dx+(p.z-u.z)*dz)/l2)):0;return Math.hypot(p.x-u.x-t*dx,p.z-u.z-t*dz);};
  return {segmentsIntersect:intersects,minimumSegmentDistanceM:intersects?0:Math.min(pointSegment(a,c,d),pointSegment(b,c,d),pointSegment(c,a,b),pointSegment(d,a,b))};
}
const ground = r => ['arterial','local','lane'].includes(r.kind);
function geometry(world) {
  const nodes=new Map(world.nodes.map(n=>[n.id,n])), roads=[];
  for(const r of world.roads){
    if(!ground(r))continue; const a=nodes.get(r.from),fullB=nodes.get(r.to);if(!a||!fullB)continue;
    let b=fullB, stage=r.stage, progress=r.progress??world.projects.find(p=>p.kind==='road'&&p.id===r.id)?.progress;
    if(stage!=='open'){const fraction=Math.max(0,Math.min(1,((progress??0)-.12)/.88));if(fraction<.015)continue;b={x:a.x+(fullB.x-a.x)*fraction,z:a.z+(fullB.z-a.z)*fraction};}
    if(Math.hypot(b.x-a.x,b.z-a.z)<1e-9)continue;
    roads.push({id:r.id,kind:r.kind,stage,from:r.from,to:r.to,progress:progress??null,poly:rectangle(a,b,roadWidth(r.kind)),a,b,fullB,rendererSource:'road'});
  }
  const duplicates=[];
  for(const p of world.projects.filter(p=>p.kind==='road'&&p.progress<1)){
    const a=nodes.get(p.from),b=nodes.get(p.to);if(!a||!b)continue;
    const skip=world.roads.some(r=>r.from===a.id&&r.to===b.id&&r.stage!=='open');
    if(!skip){const match=world.roads.find(r=>(r.from===a.id&&r.to===b.id)||(r.from===b.id&&r.to===a.id));
      if(match&&match.stage!=='open')duplicates.push({projectId:p.id,roadId:match.id,projectFrom:p.from,projectTo:p.to,roadFrom:match.from,roadTo:match.to,skipPredicateMissesReversed:match.from===b.id&&match.to===a.id});
      const kind=match?.kind??(Math.abs(a.x-b.x)>100?'bridge':'local'),f=Math.max(0,Math.min(1,(p.progress-.12)/.88));
      const end={x:a.x+(b.x-a.x)*f,z:a.z+(b.z-a.z)*f};
      if(ground({kind})&&f>=.015&&Math.hypot(end.x-a.x,end.z-a.z)>=1e-9)roads.push({id:p.id,kind,stage:p.stage,from:p.from,to:p.to,progress:p.progress,poly:rectangle(a,end,roadWidth(kind)),a,b:end,fullB:b,rendererSource:'project'});
    }
  }
  return {roads,duplicates};
}
function junctions(world) {
  const nodes=new Map(world.nodes.map(n=>[n.id,n])),arms=new Map();
  for(const r of world.roads){if(r.stage!=='open'||!['arterial','local','lane'].includes(r.kind))continue;const a=nodes.get(r.from),b=nodes.get(r.to);if(!a||!b)continue;
    for(const [n,end] of [[a,b],[b,a]]){const v=arms.get(n.id)||[];v.push({end,width:roadWidth(r.kind)});arms.set(n.id,v);}}
  return [...arms].flatMap(([id,as])=>{const node=nodes.get(id),poly=junctionFootprint(node,as);return poly.length?[{id,poly}]:[];});
}
function analyze(world) {
  const {roads,duplicates}=geometry(world), pads=junctions(world), near=roads.filter(r=>Math.hypot((r.a.x+r.b.x)/2-target.x,(r.a.z+r.b.z)/2-target.z)<target.radius), overlaps=[];
  for(let i=0;i<near.length;i++)for(let j=i+1;j<near.length;j++){
    const a=near[i],b=near[j],poly=clip(a.poly,b.poly),m2=area(poly);if(m2<1e-8)continue;
    const shared=[a.from,a.to].find(id=>id===b.from||id===b.to);
    let padArea=0,padId=null; if(shared!==undefined){const pad=pads.find(p=>p.id===shared);if(pad){padId=shared;padArea=area(clip(poly,pad.poly));}}
    overlaps.push({a:{id:a.id,source:a.rendererSource,stage:a.stage,kind:a.kind,from:a.from,to:a.to,progress:a.progress},b:{id:b.id,source:b.rendererSource,stage:b.stage,kind:b.kind,from:b.from,to:b.to,progress:b.progress},sharedNode:shared??null,overlapAreaM2:m2,sharedOpenJunctionPadId:padId,overlapInsidePadM2:padArea,overlapOutsidePadM2:Math.max(0,m2-padArea),intersection:poly});
  }
  return {tick:world.tick,projectRoads:world.projects.filter(p=>p.kind==='road'&&p.progress<1).map(p=>({id:p.id,from:p.from,to:p.to,stage:p.stage,progress:p.progress})),nearRoadCount:near.length,overlaps,duplicates};
}
const rawSamples=worlds.map(analyze);
const samples=rawSamples.map(s=>({tick:s.tick,projectRoads:s.projectRoads,overlaps:s.overlaps.filter(o=>o.overlapAreaM2>=.01).map(({a,b,sharedNode,overlapAreaM2,sharedOpenJunctionPadId,overlapInsidePadM2,overlapOutsidePadM2})=>({a,b,sharedNode,overlapAreaM2,sharedOpenJunctionPadId,overlapInsidePadM2,overlapOutsidePadM2})),duplicates:s.duplicates}));
const pair=(s,a,b)=>s.overlaps.find(o=>o.a.id===a&&o.b.id===b||o.a.id===b&&o.b.id===a);
const edgeOverlapSeries=rawSamples.map(s=>{const o=pair(s,1787,1807);return o?{tick:s.tick,road1807Stage:o.a.id===1807?o.a.stage:o.b.stage,road1807Progress:o.a.id===1807?o.a.progress:o.b.progress,overlapAreaM2:o.overlapAreaM2}:null;}).filter(Boolean);
const edgeOverlapChanges=edgeOverlapSeries.filter((s,i)=>i===0||s.overlapAreaM2!==edgeOverlapSeries[i-1].overlapAreaM2||s.road1807Stage!==edgeOverlapSeries[i-1].road1807Stage||s.road1807Progress!==edgeOverlapSeries[i-1].road1807Progress);
const edgeOverlapAt=[9200,9250,9300,9350,9400].map(tick=>{const s=rawSamples[tick-targetTick],o=pair(s,1787,1807);return{tick,polygon:o?.intersection??null,overlapAreaM2:o?.overlapAreaM2??0};});
const firstGeom=geometry(worlds[0]).roads,local1787=firstGeom.find(r=>r.id===1787),arterial1807=firstGeom.find(r=>r.id===1807);
const centerline= centerlineRelation(local1787.a,local1787.fullB,arterial1807.a,arterial1807.fullB);
const report={schemaVersion:1,source:'fresh seed 0 CityEngine via public WASM API; development rate 16; start at tick 0; per-tick snapshots 9200..9400',wasmSha256:sha(await readFile('public/wasm/city_engine_bg.wasm')),target,method:{renderer:'city.ts roadKit ground road surface is an axis-aligned-in-road-direction box of kind width, endpoint-truncated by clamp((progress-.12)/.88), omitted when fraction < .015; project loop skip predicate checks only same endpoint orientation; shared completed junction pads use open ordinary roads through junctionFootprint. Bridges are excluded because their tops slope.',measure:'convex XZ polygon intersections of the exact road-box plan footprints; junction coverage is exact junctionFootprint polygon clipping; centerline segment relation is computed independently from original road endpoints; excludes vertical facade/materials and does not model rasterization.'},headline:{constructionEdgeOverlap:{localRoad:1787,arterialRoad:1807,projectId:1808,sharedGraphNode:null,centerline,centerlineInterpretation:'No centerline intersection; this is edge-only overlap between road-width envelopes. At tick 9200 to 9300, the advancing arterial’s surface footprint grows across the near edge of the local road footprint. The overlap remains after arterial opens.',areaAndStageChangePoints:edgeOverlapChanges,selectedPolygons:edgeOverlapAt},projectDuplicateDraws:rawSamples.reduce((n,s)=>n+s.duplicates.length,0),otherExistingPairs:[1273,1347].map((id)=>{const s=rawSamples[0];return s.overlaps.filter(o=>o.a.id===id||o.b.id===id).map(o=>({roads:[o.a.id,o.b.id],sharedNode:o.sharedNode,areaM2:o.overlapAreaM2,outsidePadM2:o.overlapOutsidePadM2}));}).flat()},samples};
await writeFile(out,JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({out,headline:report.headline,selected:samples.filter(s=>[9200,9250,9300,9350,9400].includes(s.tick))},null,2));
