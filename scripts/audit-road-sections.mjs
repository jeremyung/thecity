// Instrument only an ignored diagnostic bundle; production source is untouched.
import {build} from 'esbuild';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import os from 'node:os';
import * as THREE from 'three/webgpu';
import {initSync,CityEngine} from '../public/wasm/city_engine.js';
mkdirSync('.tools/road-batch-review',{recursive:true});
const legacyBox=process.argv.includes('--legacy-box');
const source=readFileSync('web/src/render/city.ts','utf8');
const start=source.indexOf('  private rebuildRoads('),end=source.indexOf('  private roadKit(',start);
let body=source.slice(start,end);
const injections=[
 ['    const rebuildCompleted=',"    const timings:Record<string,number>={};let stamp=performance.now();const mark=(name:string)=>{const now=performance.now();timings[name]=now-stamp;stamp=now;};\n    const rebuildCompleted="],
 ['    const deckHeights=',"    mark('retire-and-bank');\n    const deckHeights="],
 ['    for(const road of world.roads){const a=nodes.get(road.from),b=nodes.get(road.to);if(a&&b&&',"    mark('indexes-and-curves');\n    for(const road of world.roads){const a=nodes.get(road.from),b=nodes.get(road.to);if(a&&b&&"],
 ['    if(rebuildCompleted){\n      const junctions=',"    mark('road-kits');\n    if(rebuildCompleted){\n      const junctions="],
 ['    this.constructionRoads.add(constructionKit.finish());',"    mark('junctions-and-projects');\n    this.constructionRoads.add(constructionKit.finish());\n    mark('construction-merge');"],
 ['      this.roads.add(kit.finish(),buildStreetLighting(world,this.art));',"      const merged=kit.finish();mark('completed-merge');this.roads.add(merged,buildStreetLighting(world,this.art));mark('lighting');"],
 ['      this.completedRoadSignature=completedSignature;',"      this.completedRoadSignature=completedSignature;mark('station');\n      (this as any).sectionTimings=timings;"],
];
for(const[a,b]of injections){if(!body.includes(a))throw Error('Instrumentation anchor missing: '+a);body=body.replace(a,b);}
const instrumented=source.slice(0,start)+body+source.slice(end);
await build({stdin:{contents:"export {CityRenderer} from './web/src/render/city';export {ArtMaterials} from './web/src/render/materials';export {disposeGroup} from './web/src/render/kit';",resolveDir:process.cwd()},bundle:true,format:'esm',platform:'node',packages:'external',outfile:'.tools/road-batch-review/sections.mjs',plugins:[{name:'sections',setup(b){b.onLoad({filter:/\/render\/city\.ts$/},()=>({contents:instrumented,loader:'ts'}));if(legacyBox)b.onLoad({filter:/\/render\/kit\.ts$/},args=>{const source=readFileSync(args.path,'utf8');const from='this.add(boxGeometry(x, y, z, w, h, d, rotation), mat);';if(!source.includes(from))throw Error('Legacy box anchor missing');return{contents:source.replace(from,'const geo=new THREE.BoxGeometry(w,h,d);geo.rotateY(rotation);geo.translate(x,y,z);this.add(geo,mat);'),loader:'ts'};});}}]});
globalThis.document={createElement(){return{width:0,height:0,getContext:()=>({fillRect(){},strokeRect(){},fillText(){},beginPath(){},ellipse(){},fill(){},moveTo(){},quadraticCurveTo(){},stroke(){}})};}};
initSync({module:readFileSync('public/wasm/city_engine_bg.wasm')});
const savePath=process.argv[2]??'.tools/fidelity-review/sidewalk-live-growth.json';
const e=CityEngine.restore(readFileSync(savePath,'utf8'));let world;try{world=JSON.parse(e.snapshot());}finally{e.free();}
const api=await import('../.tools/road-batch-review/sections.mjs');
const r=Object.create(api.CityRenderer.prototype);Object.assign(r,{art:new api.ArtMaterials(),scene:new THREE.Scene(),roads:new THREE.Group(),constructionRoads:new THREE.Group(),station:new THREE.Group(),pendingCompiles:0,retiredResources:[],riverHalfWidth:world.terrain.river_half_width??32,worldState:world});
const samples=[];
try{for(let i=0;i<13;i++){r.completedRoadSignature='';r.rebuildRoads(world);samples.push(r.sectionTimings);}}finally{for(const g of[r.roads,r.constructionRoads,r.station])api.disposeGroup(g);r.art.dispose();}
const summary={};for(const k of Object.keys(samples[0])){const v=samples.slice(1).map(s=>s[k]).sort((a,b)=>a-b);summary[k]={medianMs:v[6],maxMs:v.at(-1)};}
const result={legacyBox,cpu:os.cpus()[0]?.model,savePath,seed:world.seed,tick:world.tick,roads:world.roads.length,summary,samples};writeFileSync(legacyBox?'validation/road-section-cost.json':'validation/road-section-cost-optimized.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(summary,null,2));
