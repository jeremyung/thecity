import { createHash } from 'node:crypto';
import { build } from 'esbuild';
import { cpus } from 'node:os';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

const root=resolve(import.meta.dirname,'..'), output=resolve(process.argv[2]??'validation/growth-building-incremental-profile.json');
const wasm=await readFile(resolve(root,'public/wasm/city_engine_bg.wasm'));
initSync({module:wasm});
const engine=new CityEngine(0); engine.set_development_rate(16);
for(let n=9250;n>0;n-=2000)engine.step(Math.min(n,2000));
const world=JSON.parse(engine.snapshot()); engine.free();
if(world.tick!==9250)throw Error(`tick ${world.tick}`);
const cache=resolve(root,'node_modules/.cache',`thecity-growth-buildings-${process.pid}`);
const files=['web/src/render/extended-building.ts','web/src/render/kit.ts','web/src/render/building-chunks.ts','web/src/render/incremental-building-contours.ts','web/src/render/incremental-geometry-merge.ts','web/src/render/incremental-geometry-transform.ts','web/src/render/materials.ts','web/src/render/construction-batches.ts'];
globalThis.document={createElement(name){if(name!=='canvas')throw Error(name);return{width:0,height:0,getContext:()=>({fillRect(){},strokeRect(){},fillText(){},beginPath(){},ellipse(){},fill(){}})}}};
await rm(cache,{recursive:true,force:true});await mkdir(cache,{recursive:true});
try{
 await build({entryPoints:files.map(f=>resolve(root,f)),outdir:cache,bundle:true,format:'esm',platform:'node',packages:'external',logLevel:'silent'});
 const THREE=await import('three/webgpu');
 const [{disposeGroup},{ArtMaterials},{ConstructionBatches},{buildExtendedBuilding},{BuildingChunks},{IncrementalBuildingContours}]=await Promise.all([
  import(pathToFileURL(resolve(cache,'kit.js')).href),import(pathToFileURL(resolve(cache,'materials.js')).href),
  import(pathToFileURL(resolve(cache,'construction-batches.js')).href),import(pathToFileURL(resolve(cache,'extended-building.js')).href),import(pathToFileURL(resolve(cache,'building-chunks.js')).href),import(pathToFileURL(resolve(cache,'incremental-building-contours.js')).href)]);
 const art=new ArtMaterials(), batches=new ConstructionBatches(m=>art.buildingBatch(m),fn=>fn(),art.contours);
 const originals=new Map((world.building_original_footprints??[]).map(v=>[v.building_id,v]));
 const samples=[];
 for(const b of world.buildings){
  const history=[...(b.history??[])], original=originals.get(b.id), start=performance.now();
  const group=buildExtendedBuilding({...b,history},original?{...original}:undefined,art);
  group.position.set(b.x,0,b.z);group.rotation.y=b.rotation;
  const factoryMs=performance.now()-start;
  const setStart=performance.now();batches.set(b.id,group,{createGroup(){const regenerated=buildExtendedBuilding({...b,history},original?{...original}:undefined,art);regenerated.position.set(b.x,0,b.z);regenerated.rotation.y=b.rotation;return regenerated;},disposeGroup},b.stage!=='open');
  samples.push({id:b.id,stage:b.stage,kind:b.kind,floors:b.floors,chunk:`${Math.floor((b.x+400)/200)}:${Math.floor((b.z+400)/200)}`,factoryMs,setMs:performance.now()-setStart});
 }
 const updates=[];
 for(let i=0;batches.pendingCount&&i<50000;i++){const start=performance.now();batches.update();updates.push(performance.now()-start)}
 const q=a=>{const s=[...a].sort((x,y)=>x-y),at=p=>s[Math.min(s.length-1,Math.ceil(s.length*p)-1)]??0;return{count:a.length,p50:at(.5),p95:at(.95),max:at(1)}};
 const isolated=[];
 for(const id of [1602,1611]){
  const b=world.buildings.find(v=>v.id===id);if(!b)throw Error(`missing source ${id}`);
  const history=[...(b.history??[])],original=originals.get(id);
  const make=()=>{const g=buildExtendedBuilding({...b,history},original?{...original}:undefined,art);g.position.set(b.x,0,b.z);g.rotation.y=b.rotation;return g};
  const group=make(),oracleValues=[];
  group.updateMatrixWorld(true);group.traverse(o=>{if(!(o instanceof THREE.Mesh))return;const edges=new THREE.EdgesGeometry(o.geometry,42);edges.applyMatrix4(o.matrixWorld);const p=edges.getAttribute('position');for(let i=0;i<p.count;i+=2){const dx=p.getX(i+1)-p.getX(i),dy=p.getY(i+1)-p.getY(i),dz=p.getZ(i+1)-p.getZ(i);if(dx*dx+dy*dy+dz*dz<12.25)continue;for(let j=0;j<2;j++)oracleValues.push(p.getX(i+j),p.getY(i+j),p.getZ(i+j));}edges.dispose();});
  const oracle=new Float32Array(oracleValues),oracleHash=createHash('sha256').update(Buffer.from(oracle.buffer)).digest('hex');
  const helper=new IncrementalBuildingContours(group,12.25,42),stepMs=[],stepWork=[];
  while(true){const started=performance.now(),done=helper.step(2000);stepMs.push(performance.now()-started);stepWork.push(helper.processedWork);if(done)break;}
  const actual=helper.result;if(!actual)throw Error(`no contour result ${id}`);const actualHash=createHash('sha256').update(Buffer.from(actual.buffer)).digest('hex'),exact=actual.length===oracle.length&&actual.every((v,i)=>v===oracle[i]);
  if(!exact)throw Error(`incremental contour output mismatch for ${id}: ${actual.length} vs ${oracle.length}`);
  helper.dispose();
  const registrations=[],boundedRuns=[];
  for(let repeat=0;repeat<5;repeat++){
   const chunks=new BuildingChunks(m=>art.buildingBatch(m),fn=>fn(),art.contours),source=make(),started=performance.now();
   chunks.set(id,source,{createGroup:make,disposeGroup});const registrationMs=performance.now()-started;registrations.push(registrationMs);
   const updates=[];let count=0;
   while(chunks.pendingCount&&count<50000){const t=performance.now();chunks.update();updates.push(performance.now()-t);count++;}
   if(chunks.pendingCount)throw Error(`incremental chunk did not drain ${id}`);
   const committed=[];chunks.root.traverse(o=>{if(o instanceof THREE.LineSegments){const p=o.geometry.getAttribute('position');for(let i=0;i<p.array.length;i++)committed.push(p.array[i]);}});
   if(committed.length!==oracle.length||committed.some((v,i)=>v!==oracle[i]))throw Error(`committed outline mismatch ${id}`);
   boundedRuns.push({repeat,registrationMs,updateMs:q(updates),updateCount:count,committedExactMatch:true});chunks.clear();
  }
  disposeGroup(group);
  isolated.push({id,kind:b.kind,floors:b.floors,oracle:{floatCount:oracle.length,sha256:oracleHash},incremental:{floatCount:actual.length,sha256:actualHash,exactMatch:exact,stepQuota:2000,stepWork:q(stepWork),stepMs:q(stepMs)},registrationMs:q(registrations),boundedRuns});
 }
 const top=[...samples].sort((a,b)=>b.factoryMs-a.factoryMs).slice(0,20);
 const report={scope:'Node CPU only; public CityEngine seed 0 development-rate 16 snapshot tick 9250; production buildExtendedBuilding and ConstructionBatches, no browser/GPU',host:{cpu:cpus()[0].model,node:process.version},source:{seed:world.seed,tick:world.tick,buildings:world.buildings.length,wasmSha256:createHash('sha256').update(wasm).digest('hex'),snapshotSha256:createHash('sha256').update(JSON.stringify(world)).digest('hex')},ordinaryBuildings:samples.filter(x=>x.stage==='open').length,constructionBuildings:samples.filter(x=>x.stage!=='open').length,constructionSources:samples.filter(x=>x.stage!=='open'),perBuildingFactory:q(samples.map(x=>x.factoryMs)),setBookkeeping:q(samples.map(x=>x.setMs)),schedulerUpdate:q(updates),schedulerUpdateCount:updates.length,topFactorySources:top,isolatedRegistration:isolated,method:'Node CPU diagnostic only. Tick-9250 ordinary/construction scene profile remains as a broad baseline; isolated buildings 1602/1611 compare incremental contours at quota 2000 work units per update against independent THREE.EdgesGeometry(42), applyMatrix4, world-length >=12.25 oracle, and time actual BuildingChunks descriptor registration plus bounded update drain. Descriptor regeneration factory restores building root position and rotation. Browser/GPU frame behavior and CityRenderer admission are not modeled.'};
 await writeFile(output,JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));batches.clear();art.dispose();
}finally{await rm(cache,{recursive:true,force:true});}
