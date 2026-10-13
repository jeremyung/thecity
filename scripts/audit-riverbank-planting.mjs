// Read-only audit of the current renderer's actual placement and clearance rules.
// Extract the small source blocks rather than duplicate rules that could drift.
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {build} from 'esbuild';
const source=readFileSync('web/src/render/city.ts','utf8');
const placement=source.match(/const rng=random\(world.seed\+937\);([\s\S]*?)this.treePoints=positions;/)?.[1];
const clearance=source.match(/      let hidden=onPaving\(p\)([\s\S]*?)      dummy.position.set/);
if(!placement||!clearance)throw new Error('Renderer source changed: inspect extraction before rerunning');
mkdirSync('.tools',{recursive:true});
await build({stdin:{contents:"export {riverbankTreePoints} from './web/src/render/riverbank-planting'; export {createPavedGroundTest} from './web/src/render/paved-ground'; export {createTreeClearanceGrid} from './web/src/render/tree-clearance-grid'; export {overlapsStation} from './web/src/render/station-clearance'; export {roadWidth} from './web/src/render/road-elevation'; export {distanceToSegment} from './web/src/render/tree-occlusion';",resolveDir:process.cwd(),loader:'ts'},bundle:true,platform:'node',format:'esm',packages:'external',outfile:'.tools/bank-planting-audit.mjs'});
const helpers=await import('../.tools/bank-planting-audit.mjs');
const {riverbankTreePoints,createPavedGroundTest,createTreeClearanceGrid,roadWidth,overlapsStation,distanceToSegment}=helpers;
function random(seed){return()=>{seed^=seed<<13;seed^=seed>>>17;seed^=seed<<5;return(seed>>>0)/4294967296;};}
function riverX(points,z){for(let i=0;i<points.length-1;i++)if(z<=points[i+1].z){const t=(z-points[i].z)/(points[i+1].z-points[i].z);return points[i].x+(points[i+1].x-points[i].x)*t;}return points.at(-1).x;}
const generate=new Function('world','river','halfWidth','rng','riverbankTreePoints',placement.replace('const positions:Array<{x:number,z:number,s:number}>=[]','const positions=[]')+'return positions;');
const isHidden=new Function('world','p','candidates','onPaving','nodes','roadWidth','distanceToSegment','overlapsStation','let hidden=onPaving(p)'+clearance[1]+'return hidden;');
const cases=[];
for(const seed of [87231,0,5]){
 const path=`.tools/walking-offset-${seed}/station-exit-world.json`,raw=readFileSync(path),world=JSON.parse(raw);
 const river=world.terrain.river,halfWidth=world.terrain.river_half_width??32;
 const points=generate.call({riverX},world,river,halfWidth,random(world.seed+937),riverbankTreePoints);
 const nodes=new Map(world.nodes.map(n=>[n.id,n])),onPaving=createPavedGroundTest(world.ground??[]);
 const candidatesAt=createTreeClearanceGrid(world.buildings,world.roads,(world.rear_extensions??[]).slice(0,2),world.nodes,Math.max(0,...points.map(p=>p.s*1.7+2)),kind=>roadWidth(kind)/2+(kind==='bridge'?2.5+10:kind==='lane'?1.5:3.5));
 const bands={bank0to17:{generated:0,visible:0},bank17to50:{generated:0,visible:0},inland:{generated:0,visible:0}};
 const pointsReport=points.map(p=>{
  const distance=Math.abs(p.x-riverX(river,p.z))-halfWidth;
  const band=distance<17-1e-6?'bank0to17':distance<50?'bank17to50':'inland';
  const hidden=isHidden(world,p,candidatesAt(p),onPaving,nodes,roadWidth,distanceToSegment,overlapsStation);
  bands[band].generated++;if(!hidden)bands[band].visible++;
  return {...p,bankDistance:distance,hidden,onPaving:onPaving(p),band};
 });
 // Candidate feasibility only: small trees in the unpaved inner strip.
 // Conservatively exclude every transport corridor, including elevated decks.
 const innerStrip=[];
 for(const side of [-1,1])for(let z=-360;z<=360;z+=18){
  const p={x:riverX(river,z)+side*(halfWidth+3.5),z,s:1.8};
  const transport=world.roads.some(r=>{const a=nodes.get(r.from),b=nodes.get(r.to);return a&&b&&distanceToSegment(p,a,b)<roadWidth(r.kind)/2+5.1;});
  const hidden=isHidden(world,p,candidatesAt(p),onPaving,nodes,roadWidth,distanceToSegment,overlapsStation);
  innerStrip.push({...p,side,hidden,transport,eligible:!hidden&&!transport});
 }
 cases.push({innerStripFeasibility:{proposed:innerStrip.length,eligible:innerStrip.filter(p=>p.eligible).length,points:innerStrip},seed,tick:world.tick,input:path,inputSha256:createHash('sha256').update(raw).digest('hex'),halfWidth,bands,total:points.length,visible:pointsReport.filter(p=>!p.hidden).length,bankPavingRejected:pointsReport.filter(p=>p.band!=='inland'&&p.onPaving).length,points:pointsReport});
}
const report={scope:'Exact placement and visibility predicate extracted from CityRenderer source; no runtime mutation. Distance is X-offset from the river edge, matching renderer convention. Not screen-space visibility or a path/collision acceptance test.',rendererSha256:createHash('sha256').update(source).digest('hex'),cases};
writeFileSync(process.argv[2]??'validation/riverbank-planting-current.json',JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(cases.map(({points,innerStripFeasibility,...c})=>({...c,innerStripFeasibility:{proposed:innerStripFeasibility.proposed,eligible:innerStripFeasibility.eligible}})),null,2));
