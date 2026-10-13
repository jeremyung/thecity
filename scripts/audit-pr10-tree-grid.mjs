// Actual updateTrees parity/cost, with only the broad phase disabled in baseline.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {cpus} from 'node:os';
import {build} from 'esbuild';
import * as THREE from 'three/webgpu';
import {CityEngine,initSync} from '../public/wasm/city_engine.js';
mkdirSync('.tools/pr10-grid',{recursive:true});
for(const mode of ['before','after'])await build({entryPoints:['web/src/render/city.ts'],bundle:true,platform:'node',format:'esm',packages:'external',outfile:`.tools/pr10-grid/${mode}.mjs`,plugins:mode==='before'?[{name:'full-scan-baseline',setup(b){b.onLoad({filter:/\/render\/city\.ts$/},args=>{
 let s=readFileSync(args.path,'utf8');
 const start=s.indexOf('    const maxCanopy=',s.indexOf('  private updateTrees('));
 const end=s.indexOf('    for(const batch',start);
 assert(start>=0&&end>start);
 s=s.slice(0,start)+`    const candidatesAt=()=>({buildings:world.buildings,roads:world.roads.filter(r=>r.kind!=='expressway'&&nodes.has(r.from)&&nodes.has(r.to)),rearExtensions:(world.rear_extensions??[]).slice(0,2).filter(e=>e.stage!=='survey')});\n`+s.slice(end);
 return{contents:s,loader:'ts'};
});}}]:[]});
const {CityRenderer:Before}=await import('../.tools/pr10-grid/before.mjs');
const {CityRenderer:After}=await import('../.tools/pr10-grid/after.mjs');
initSync({module:readFileSync('public/wasm/city_engine_bg.wasm')});
const engine=CityEngine.restore(readFileSync('.tools/movement-support/browser/growth-723.json','utf8'));
const worlds=[JSON.parse(engine.snapshot())];
for(let i=0;i<3;i++){engine.step(400);worlds.push(JSON.parse(engine.snapshot()));}engine.free();
function harness(C){
 const r=Object.create(C.prototype),geo=new THREE.BoxGeometry(1,1,1),mat=new THREE.MeshBasicMaterial();
 r.treePoints=Array.from({length:1024},(_,i)=>({x:-387+(i%32)*25,z:-387+Math.floor(i/32)*25,s:1.5+(i%7)*.4}));
 r.treeBatches=[{indices:r.treePoints.map((_,i)=>i),trees:new THREE.InstancedMesh(geo,mat,1024),trunks:new THREE.InstancedMesh(geo,mat,1024)}];r.treeClearanceKey='';return r;
}
const before=harness(Before),after=harness(After),costs={before:[],after:[]};
for(const w of worlds)for(let round=0;round<6;round++){
 for(const [name,r]of round%2?[['after',after],['before',before]]:[['before',before],['after',after]]){
  r.treeClearanceKey='';const start=performance.now();r.updateTrees(w);if(round)costs[name].push(performance.now()-start);
 }
 for(const key of ['trees','trunks']){
  assert.deepEqual(after.treeBatches[0][key].instanceMatrix.array,before.treeBatches[0][key].instanceMatrix.array);
  assert.deepEqual(after.treeBatches[0][key].boundingSphere,before.treeBatches[0][key].boundingSphere);
 }
}
const stats=a=>{a.sort((a,b)=>a-b);return{count:a.length,p50Ms:a[Math.floor(a.length*.5)],p95Ms:a[Math.floor(a.length*.95)]}};
const result={scope:'Node CPU actual updateTrees; cache forced invalid per run to measure structural rebuild, synthetic 1024 tree sites; no browser performance claim',cpu:cpus()[0].model,node:process.version,seed:worlds[0].seed,ticks:worlds.map(w=>w.tick),buildings:worlds.map(w=>w.buildings.length),parity:'exact trunk/tree matrices and bounding spheres for all four real growth snapshots',before:stats(costs.before),after:stats(costs.after)};
writeFileSync('validation/pr10-tree-grid-cost.json',JSON.stringify(result,null,2)+'\n');console.log(result);
for(const r of [before,after]){const b=r.treeBatches[0];b.trees.geometry.dispose();b.trees.material.dispose();b.trees.dispose();b.trunks.dispose();}
