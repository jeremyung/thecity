// Offline scheduling/geometry parity diagnostic, not browser frame acceptance.
// Usage: node scripts/audit-sidewalk-build.mjs snapshot.json
import {build} from 'esbuild';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import os from 'node:os';
import {mergeGeometries} from 'three/addons/utils/BufferGeometryUtils.js';
const fixture=process.argv[2];
if(!fixture)throw Error('Pass a world snapshot');
await build({stdin:{contents:"export {SidewalkBuild} from './web/src/render/sidewalk-build';export {arterialSidewalkPolygons} from './web/src/render/arterial-sidewalks';export {sidewalkGeometry} from './web/src/render/sidewalk-kit';",resolveDir:process.cwd()},bundle:true,platform:'node',format:'esm',packages:'external',outfile:'.tools/sidewalk-build-audit.mjs'});
const {SidewalkBuild,arterialSidewalkPolygons,sidewalkGeometry}=await import('../.tools/sidewalk-build-audit.mjs');
const source=readFileSync(fixture),world=JSON.parse(source);
const parts=arterialSidewalkPolygons(world).map(sidewalkGeometry),expected=parts.length?mergeGeometries(parts,false):null;
for(const p of parts)p.dispose();
function hash(g){
 if(!g)return null;
 const h=createHash('sha256');
 for(const key of Object.keys(g.attributes).sort()){
  const a=g.getAttribute(key);h.update(key);h.update(Buffer.from(a.array.buffer,a.array.byteOffset,a.array.byteLength));
 }
 // Compare numeric indices: valid merger implementations may choose different index widths.
 if(g.index)h.update(JSON.stringify(Array.from(g.index.array)));
 return h.digest('hex');
}
const expectedHash=hash(expected),runs=[];
for(let run=0;run<6;run++){
 const begin=performance.now(),job=new SidewalkBuild(world),constructMs=performance.now()-begin,samples=[];
 let done=false;
 while(!done){const t=performance.now();done=job.step();samples.push(performance.now()-t);if(samples.length>100000)throw Error('Job did not complete');}
 const result=job.takeGeometry();
 if(hash(result)!==expectedHash)throw Error('Geometry differs from synchronous merge');
 job.dispose();result?.dispose();
 const sorted=[...samples].sort((a,b)=>a-b);
 runs.push({run,warmup:run<2,constructMs,updates:samples.length,totalStepMs:samples.reduce((a,b)=>a+b,0),maxStepMs:sorted.at(-1),p95StepMs:sorted[Math.floor(sorted.length*.95)]});
}
expected?.dispose();
console.log(JSON.stringify({scope:'Node CPU only; no GPU upload, frame scheduling or lifecycle swap acceptance',hardware:os.cpus()[0].model,fixture,fixtureSha256:createHash('sha256').update(source).digest('hex'),seed:world.seed,tick:world.tick,geometrySha256:expectedHash,exactGeometryParity:true,runs},null,2));
