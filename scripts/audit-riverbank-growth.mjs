// Restore ordinary checkpoints, then compare retained bank-tree crowns with transport.
import {readFileSync,writeFileSync,readdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {build} from 'esbuild';
import {CityEngine,initSync} from '../public/wasm/city_engine.js';
const wasm=readFileSync('public/wasm/city_engine_bg.wasm');initSync({module:wasm});
await build({stdin:{contents:"export {riverbankTreePoints} from './web/src/render/riverbank-planting'; export {createPavedGroundTest} from './web/src/render/paved-ground'; export {createTreeClearanceGrid} from './web/src/render/tree-clearance-grid'; export {overlapsStation} from './web/src/render/station-clearance'; export {roadWidth} from './web/src/render/road-elevation'; export {distanceToSegment} from './web/src/render/tree-occlusion'; export {createFoliageGeometry} from './web/src/render/foliage';",resolveDir:process.cwd(),loader:'ts'},bundle:true,platform:'node',format:'esm',packages:'external',outfile:'.tools/bank-growth-audit.mjs'});
const {riverbankTreePoints,createPavedGroundTest,createTreeClearanceGrid,overlapsStation,roadWidth,distanceToSegment,createFoliageGeometry}=await import('../.tools/bank-growth-audit.mjs');
const source=readFileSync('web/src/render/city.ts','utf8');
const block=source.match(/      let hidden=onPaving\(p\)([\s\S]*?)      dummy.position.set/);
if(!block)throw new Error('Tree predicate changed: review extraction');
const hidden=new Function('world','p','candidates','onPaving','nodes','roadWidth','distanceToSegment','overlapsStation','let hidden=onPaving(p)'+block[1]+'return hidden;');
let crownRadius=0,crownBottom=Infinity,crownTop=-Infinity;
for(const lod of ['near','far']){const g=createFoliageGeometry(lod),p=g.attributes.position;for(let i=0;i<p.count;i++){crownRadius=Math.max(crownRadius,Math.hypot(p.getX(i),p.getZ(i)));crownBottom=Math.min(crownBottom,p.getY(i));crownTop=Math.max(crownTop,p.getY(i));}g.dispose();}
const cases=[];
for(const seed of [87231,0,5])for(const checkpoint of ['phase-0','phase-1','phase-2','phase-3','phase-4','tick-10000','tick-40000','tick-80000',...readdirSync(`.tools/walking-offset-${seed}`).filter(f=>f.startsWith('bridge-stage-')&&f.endsWith('.json')).map(f=>f.slice(0,-5))]){
 const path=checkpoint.startsWith('phase')?`.tools/station-construction/${seed}-${checkpoint}.json`:`.tools/walking-offset-${seed}/${checkpoint}.json`;
 const saved=readFileSync(path,'utf8'),engine=CityEngine.restore(saved);let w;try{w=JSON.parse(engine.snapshot());}finally{engine.free();}
 const trees=riverbankTreePoints(w.terrain.river,w.terrain.river_half_width??32),nodes=new Map(w.nodes.map(n=>[n.id,n]));
 const paving=createPavedGroundTest(w.ground??[]),at=createTreeClearanceGrid(w.buildings,w.roads,(w.rear_extensions??[]).slice(0,2),w.nodes,5.06,kind=>roadWidth(kind)/2+(kind==='bridge'?2.5+10:kind==='lane'?1.5:3.5));
 const retained=trees.filter(p=>!hidden(w,p,at(p),paving,nodes,roadWidth,distanceToSegment,overlapsStation));
 const conflicts=[];let bridgeMargin=null,railMargin=null;
 for(const p of retained)for(const r of w.roads){if(!['bridge','rail'].includes(r.kind))continue;const a=nodes.get(r.from),b=nodes.get(r.to);if(!a||!b)continue;
  // Bridge includes the 2.5m sidewalks; rail uses its 7m deck envelope.
  const margin=distanceToSegment(p,a,b)-roadWidth(r.kind)/2-(r.kind==='bridge'?2.5:0)-crownRadius;
  if(r.kind==='bridge')bridgeMargin=bridgeMargin===null?margin:Math.min(bridgeMargin,margin);else railMargin=railMargin===null?margin:Math.min(railMargin,margin);
  if(margin<0)conflicts.push({tree:p,road:r.id,kind:r.kind,stage:r.stage,margin});
 }
 cases.push({seed,checkpoint,tick:w.tick,buildings:w.buildings.length,projects:w.projects.length,generated:trees.length,retained:retained.length,bridgeMargin,railMargin,conflicts,inputSha256:createHash('sha256').update(saved).digest('hex')});
}
writeFileSync('validation/riverbank-growth.json',JSON.stringify({scope:'Ordinary saved checkpoints including actual bridge construction stages. Exact renderer visibility predicate. Conservative horizontal crown envelope versus bridge including sidewalks and rail deck, independent of height; does not cover every object, continuous growth or curved expressway piers.',wasmSha256:createHash('sha256').update(wasm).digest('hex'),rendererSha256:createHash('sha256').update(source).digest('hex'),crownRadius,crownBottom,crownTop,cases},null,2)+'\n');
console.log(JSON.stringify(cases.map(({inputSha256,conflicts,...c})=>({...c,conflicts:conflicts.length})),null,2));
if(cases.some(c=>c.conflicts.length))process.exitCode=1;
