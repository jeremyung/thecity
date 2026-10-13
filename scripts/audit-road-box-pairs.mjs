// Paired full road rebuilds isolate Kit.box construction. No browser/GPU timing.
import {build} from 'esbuild';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import os from 'node:os';
import * as THREE from 'three/webgpu';
import {initSync,CityEngine} from '../public/wasm/city_engine.js';
mkdirSync('.tools/road-batch-review',{recursive:true});
for(const variant of ['legacy','candidate']){
  await build({stdin:{contents:"export {CityRenderer} from './web/src/render/city';export {ArtMaterials} from './web/src/render/materials';export {disposeGroup} from './web/src/render/kit';",resolveDir:process.cwd()},bundle:true,format:'esm',platform:'node',packages:'external',outfile:`.tools/road-batch-review/box-${variant}.mjs`,plugins:variant==='candidate'?[]:[{name:'legacy-box',setup(b){b.onLoad({filter:/\/render\/kit\.ts$/},args=>{const source=readFileSync(args.path,'utf8'),anchor='this.add(boxGeometry(x, y, z, w, h, d, rotation), mat);';if(!source.includes(anchor))throw Error('Legacy anchor missing');return{contents:source.replace(anchor,'const geo=new THREE.BoxGeometry(w,h,d);geo.rotateY(rotation);geo.translate(x,y,z);this.add(geo,mat);'),loader:'ts'};});}}]});
}
globalThis.document={createElement(){return{width:0,height:0,getContext:()=>({fillRect(){},strokeRect(){},fillText(){},beginPath(){},ellipse(){},fill(){},moveTo(){},quadraticCurveTo(){},stroke(){}})};}};
initSync({module:readFileSync('public/wasm/city_engine_bg.wasm')});
const savePath=process.argv[2]??'.tools/fidelity-review/sidewalk-live-growth.json',e=CityEngine.restore(readFileSync(savePath,'utf8'));let world;try{world=JSON.parse(e.snapshot());}finally{e.free();}
const owners={};for(const variant of ['legacy','candidate']){const api=await import(`../.tools/road-batch-review/box-${variant}.mjs`),r=Object.create(api.CityRenderer.prototype);Object.assign(r,{art:new api.ArtMaterials(),scene:new THREE.Scene(),roads:new THREE.Group(),constructionRoads:new THREE.Group(),station:new THREE.Group(),pendingCompiles:0,retiredResources:[],riverHalfWidth:world.terrain.river_half_width??32,worldState:world});owners[variant]={api,r};}
const pairs=[];try{for(let i=0;i<18;i++){const pair={};for(const variant of i%2?['candidate','legacy']:['legacy','candidate']){const r=owners[variant].r;r.completedRoadSignature='';const start=performance.now();r.rebuildRoads(world);pair[variant]=performance.now()-start;}pairs.push(pair);}}finally{for(const{api,r}of Object.values(owners)){for(const g of[r.roads,r.constructionRoads,r.station])api.disposeGroup(g);r.art.dispose();}}
const measured=pairs.slice(2),summary={};for(const variant of ['legacy','candidate']){const times=measured.map(p=>p[variant]).sort((a,b)=>a-b);summary[variant]={medianMs:(times[7]+times[8])/2,minMs:times[0],maxMs:times.at(-1),totalMs:times.reduce((a,b)=>a+b,0)};}
const report={cpu:os.cpus()[0]?.model,savePath,seed:world.seed,tick:world.tick,excludedWarmupPairs:2,candidateFasterPairs:measured.filter(p=>p.candidate<p.legacy).length,summary,pairs};writeFileSync('validation/road-box-pairs.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report.summary),report.candidateFasterPairs);
