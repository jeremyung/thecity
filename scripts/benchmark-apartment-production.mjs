import {build} from 'esbuild';
import {mkdir,writeFile,readFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
const folder=new URL('../.tools/apartment-benchmark/',import.meta.url);await mkdir(folder,{recursive:true});
await build({stdin:{contents:`export {ApartmentResources} from './web/src/render/architecture/resources';export {buildApartmentBuilding,buildResidentialTower,RESIDENTIAL_TOWERS,APARTMENT_RECIPE_IDS} from './web/src/render/architecture';export {disposeGroup} from './web/src/render/kit';`,resolveDir:process.cwd(),sourcefile:'benchmark.ts'},outfile:new URL('kit.mjs',folder).pathname,bundle:true,platform:'node',format:'esm',packages:'external'});
const {ApartmentResources,buildApartmentBuilding,buildResidentialTower,RESIDENTIAL_TOWERS,APARTMENT_RECIPE_IDS,disposeGroup}=await import(pathToFileURL(new URL('kit.mjs',folder).pathname));
const r=new ApartmentResources(),report=[];
try{for(const family of ['midrise','tower'])for(const detail of ['close','district','far']){
 const timings=[],triangles=[];for(let i=0;i<60;i++){
  const b={id:i+1,parcel_id:i+1,x:0,z:0,width:26,depth:24,rotation:0,floors:family==='tower'?32:7,appearance_seed:i,kind:'apartment',stage:'open',progress:1,occupancy:40,capacity:60,built_day:0,name:'Benchmark',history:[],anchor:false,vacant_days:0};
  const ids=family==='tower'?Object.keys(RESIDENTIAL_TOWERS):APARTMENT_RECIPE_IDS,start=performance.now();
  const g=family==='tower'?buildResidentialTower(b,r.art,{recipe:ids[i%ids.length],detail}):buildApartmentBuilding(b,r.art,{recipe:ids[i%ids.length],detail});
  timings.push(performance.now()-start);let count=0;g.traverse(o=>{if(o.isMesh)count+=(o.geometry.index?.count??o.geometry.getAttribute('position').count)/3;});triangles.push(count);disposeGroup(g);
 }
 timings.sort((a,b)=>a-b);report.push({family,detail,samples:60,p50BuildMs:timings[30],p95BuildMs:timings[57],maxBuildMs:timings[59],meanTriangles:triangles.reduce((a,b)=>a+b,0)/60,maxTriangles:Math.max(...triangles)});
}}finally{r.dispose();}
const result={method:'Node CPU geometry creation only; 60 buildings per family/LOD, materials shared, warm template cache. Not a GPU frame-time benchmark.',report};const out=process.argv[2]??'.tools/apartment-benchmark/results.json';await writeFile(out,JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result,null,2));
