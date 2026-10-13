// CPU-only geometry timing/parity; does not measure browser frames or GPU time.
// Default baseline forces completed-batch regeneration in the current renderer.
// Optional third argument "before" or "before-box" uses the locally captured pre-change bundle.
import {build} from 'esbuild';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import os from 'node:os';
import * as THREE from 'three/webgpu';
import {CityEngine,initSync} from '../public/wasm/city_engine.js';
mkdirSync('.tools/road-batch-review',{recursive:true});
await build({stdin:{contents:"export {CityRenderer} from './web/src/render/city'; export {ArtMaterials} from './web/src/render/materials'; export {disposeGroup} from './web/src/render/kit';",resolveDir:process.cwd()},bundle:true,platform:'node',format:'esm',packages:'external',outfile:'.tools/road-batch-review/after.mjs'});
globalThis.document={createElement(){const context={fillRect(){},strokeRect(){},fillText(){},beginPath(){},ellipse(){},fill(){},moveTo(){},quadraticCurveTo(){},stroke(){}};return {width:0,height:0,getContext:()=>context};}};
initSync({module:readFileSync('public/wasm/city_engine_bg.wasm')});
const savePath=process.argv[2]??'.tools/fidelity-review/sidewalk-live-growth.json';
const save=readFileSync(savePath,'utf8');const engine=CityEngine.restore(save),worlds=[];
try{for(let i=0;i<32;i++){worlds.push(JSON.parse(engine.snapshot()));engine.step(80);}}finally{engine.free();}
function harness(api){const r=Object.create(api.CityRenderer.prototype);Object.assign(r,{art:new api.ArtMaterials(),scene:new THREE.Scene(),roads:new THREE.Group(),constructionRoads:new THREE.Group(),station:new THREE.Group(),completedRoadSignature:'',pendingCompiles:0,retiredResources:[],riverHalfWidth:worlds[0].terrain.river_half_width??32});return r;}
function digest(groups){const hashes=[];let meshes=0,triangles=0;for(const group of groups)group?.traverse(o=>{if(!o.isMesh)return;meshes++;const g=o.geometry,p=g.attributes.position,idx=g.index;const m=o.material;const material=[m.type,m.color?.getHex(),m.roughness,m.metalness,Boolean(m.map)].join(':');for(let i=0;i<(idx?.count??p.count);i+=3){const values=[material];for(let j=0;j<3;j++){const n=idx?idx.getX(i+j):i+j;for(const [name,a]of Object.entries(g.attributes).sort()){values.push(name);for(let k=0;k<a.itemSize;k++)values.push(a.array[n*a.itemSize+k]);}}hashes.push(createHash('sha256').update(values.join(',')).digest('hex'));triangles++;}if(o.isInstancedMesh)hashes.push(createHash('sha256').update(new Uint8Array(o.instanceMatrix.array.buffer)).digest('hex'));});return {meshes,triangles,hash:createHash('sha256').update(hashes.sort().join('')).digest('hex')};}
const baseline=['before','before-box'].includes(process.argv[3])?process.argv[3]:'uncached';
const results={};for(const name of [baseline,'after']){const api=await import(`../.tools/road-batch-review/${name==='uncached'?'after':name}.mjs`);const r=harness(api),times=[],retained=[],samples=[];try{for(let i=0;i<worlds.length;i++){const w=worlds[i],old=r.roads;r.worldState=w;if(name==='uncached')r.completedRoadSignature='';const t=performance.now();r.rebuildRoads(w);times.push(performance.now()-t);retained.push(old===r.roads);if(i===0||i===worlds.length-1)samples.push({tick:w.tick,...digest([r.roads,r.constructionRoads,r.station])});}}finally{for(const g of [r.roads,r.constructionRoads,r.station])api.disposeGroup(g);r.art.dispose();}const sorted=times.slice(1).sort((a,b)=>a-b);results[name]={times,retained:retained.filter(Boolean).length,medianMs:sorted[Math.floor(sorted.length/2)],p95Ms:sorted[Math.floor(sorted.length*.95)],samples};}
const report={hardware:{cpu:os.cpus()[0]?.model,platform:os.platform(),arch:os.arch()},savePath,saveSha256:createHash('sha256').update(save).digest('hex'),settings:{updates:32,ticksPerUpdate:80,growthRate:worlds[0].development_rate},results,baseline,parity:results[baseline].samples.every((s,i)=>s.hash===results.after.samples[i].hash)};
writeFileSync(baseline==='before'?'validation/road-batch-cost.json':baseline==='before-box'?'validation/road-box-cost.json':'validation/road-batch-retention-cost.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));
