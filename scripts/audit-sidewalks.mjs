// Node CPU/geometry only, not a browser frame benchmark.
// Usage: node scripts/audit-sidewalks.mjs snapshot.json
import {build} from 'esbuild';
import {readFileSync} from 'node:fs';
import os from 'node:os';
if(!process.argv[2])throw Error('Pass a world snapshot (not an engine save)');
await build({stdin:{contents:"export {arterialSidewalkPolygons,arterialSidewalkSteps} from './web/src/render/arterial-sidewalks'; export {addArterialSidewalks,sidewalkGeometry} from './web/src/render/sidewalk-kit';export {Kit,disposeGroup} from './web/src/render/kit';export {ArtMaterials} from './web/src/render/materials';",resolveDir:process.cwd()},bundle:true,platform:'node',format:'esm',packages:'external',outfile:'.tools/sidewalk-cost.mjs'});
const {arterialSidewalkPolygons,addArterialSidewalks,sidewalkGeometry,Kit,disposeGroup,ArtMaterials}=await import('../.tools/sidewalk-cost.mjs');
const world=JSON.parse(readFileSync(process.argv[2],'utf8'));
const art=new ArtMaterials(),samples=[],stages=[];let triangles=0,batches=0;
for(let i=0;i<8;i++){
 const start=performance.now(),kit=new Kit();
 const polygons=arterialSidewalkPolygons(world),polygonEnd=performance.now();
 const material=art.get('#b4b8a6');
 for(const polygon of polygons)kit.add(sidewalkGeometry(polygon),material);
 const geometryEnd=performance.now(),group=kit.finish(),mergeEnd=performance.now();
 if(i>=3){samples.push(mergeEnd-start);stages.push({polygonsMs:polygonEnd-start,geometryMs:geometryEnd-polygonEnd,mergeMs:mergeEnd-geometryEnd});}
 batches=group.children.length;triangles=group.children.reduce((n,m)=>n+m.geometry.index.count/3,0);disposeGroup(group);
}
art.dispose();console.log(JSON.stringify({scope:'Node CPU only, not browser frame cost',hardware:os.cpus()[0].model,fixture:process.argv[2],seed:world.seed,tick:world.tick,roads:world.roads.length,arterials:world.roads.filter(r=>r.kind==='arterial'&&r.stage==='open').length,polygons:arterialSidewalkPolygons(world).length,triangles,batches,samplesMs:samples,stages},null,2));
