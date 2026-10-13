import fs from 'node:fs';
import {build} from 'esbuild';
import {pathToFileURL} from 'node:url';
const source=fs.readFileSync('web/src/render/arterial-sidewalks.ts','utf8');
const start=source.indexOf('    // Join the outer sidewalk edges');
const end=source.indexOf('    for(const {poly:strip,corner}',start);
const before=source.slice(0,start)+source.slice(end);
await build({stdin:{contents:before,resolveDir:process.cwd()+'/web/src/render',loader:'ts'},bundle:true,platform:'node',format:'esm',outfile:'.tools/fidelity-review/sidewalk-before.mjs'});
await build({entryPoints:['web/src/render/arterial-sidewalks.ts'],bundle:true,platform:'node',format:'esm',outfile:'.tools/fidelity-review/sidewalk-candidate.mjs'});
const w=JSON.parse(fs.readFileSync('.tools/fidelity-review/junction-city77.json'));
const area=p=>Math.abs(p.reduce((s,a,i)=>{const b=p[(i+1)%p.length];return s+a.x*b.z-b.x*a.z;},0)/2);
const cross=(a,b,p)=>(b.x-a.x)*(p.z-a.z)-(b.z-a.z)*(p.x-a.x);
function overlap(poly,rect){let out=poly;for(let i=0;i<rect.length&&out.length;i++){const a=rect[i],b=rect[(i+1)%rect.length],next=[];for(let j=0;j<out.length;j++){const s=out[j],e=out[(j+1)%out.length],cs=cross(a,b,s),ce=cross(a,b,e);if(cs>=0)next.push(s);if((cs>=0)!==(ce>=0)){const t=cs/(cs-ce);next.push({x:s.x+(e.x-s.x)*t,z:s.z+(e.z-s.z)*t});}}out=next;}return area(out);}
const rectangles=w.buildings.map(b=>({id:b.id,p:[[-1,-1],[1,-1],[1,1],[-1,1]].map(([x,z])=>{x*=b.width/2;z*=b.depth/2;return{x:b.x+x*Math.cos(b.rotation)+z*Math.sin(b.rotation),z:b.z-x*Math.sin(b.rotation)+z*Math.cos(b.rotation)};})}));
const result={seed:w.seed,tick:w.tick,buildings:w.buildings.length};
for(const variant of ['before','candidate']){const {arterialSidewalkPolygons}=await import(pathToFileURL(process.cwd()+`/.tools/fidelity-review/sidewalk-${variant}.mjs`).href);const t=performance.now(),polys=arterialSidewalkPolygons(w),elapsed=performance.now()-t;const conflicts=[];for(const r of rectangles){const m2=polys.reduce((sum,p)=>sum+overlap(p,r.p),0);if(m2>1e-5)conflicts.push({building:r.id,m2});}result[variant]={polygons:polys.length,vertices:polys.reduce((s,p)=>s+p.length,0),area:polys.reduce((s,p)=>s+area(p),0),elapsedDiagnosticMs:elapsed,buildingEnvelopeConflicts:conflicts};}
fs.writeFileSync('validation/sidewalk-corner-city77.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result));
