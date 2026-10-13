// Compare the candidate with an explicit preserved pre-trial kit source.
// node scripts/audit-office-side-facades.mjs BASELINE_KIT OUTPUT
import {readFile,writeFile,mkdtemp,rm} from 'node:fs/promises';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {build} from 'esbuild';
import * as THREE from 'three/webgpu';
const [baselinePath,output]=process.argv.slice(2);
if(!baselinePath||!output)throw Error('Expected BASELINE_KIT OUTPUT');
const baseline=await readFile(baselinePath,'utf8'),dir=await mkdtemp(resolve('.tools/office-facade-'));
const hash=s=>createHash('sha256').update(s).digest('hex');
try{
 const runtimes=[];
 for(const version of ['baseline','candidate']){
  const outfile=resolve(dir,version+'.mjs');
  await build({stdin:{contents:"export {buildingKit,disposeGroup} from './web/src/render/kit';export {ArtMaterials} from './web/src/render/materials';",resolveDir:process.cwd()},bundle:true,platform:'node',format:'esm',packages:'external',outfile,plugins:version==='baseline'?[{name:'preserved-kit',setup(b){b.onLoad({filter:/web\/src\/render\/kit\.ts$/},()=>({contents:baseline,loader:'ts',resolveDir:resolve('web/src/render')}));}}]:[]});
  runtimes.push(await import(pathToFileURL(outfile)));
 }
 const rows=[];
 for(const seed of [87231,0,5]){
  const bytes=await readFile(`.tools/walking-offset-${seed}/station-exit-world.json`),world=JSON.parse(bytes);
  const buildings=world.buildings.filter(b=>b.kind==='office'&&b.stage==='open'&&b.floors<12);
  const art=runtimes.map(r=>{const a=new r.ArtMaterials();a.tenantSign=()=>a.get('#ffffff');return a;});let before=0,after=0;const conflicts=[],worsened=[];
  for(const b of buildings){
   const samples=runtimes.map((r,i)=>{const group=r.buildingKit(b,art[i]);let triangles=0;group.traverse(o=>{if(o.isMesh)triangles+=(o.geometry.index?.count??o.geometry.attributes.position.count)/3;});const box=new THREE.Box3().setFromObject(group);r.disposeGroup(group);return{triangles,min:box.min.toArray(),max:box.max.toArray()};});
   before+=samples[0].triangles;after+=samples[1].triangles;
   const c=samples[1];if(c.min[0]<-b.width/2-1e-5||c.max[0]>b.width/2+1e-5||c.min[2]<-b.depth/2-1e-5||c.max[2]>b.depth/2+1e-5){const previous=samples[0];const conflict={id:b.id,width:b.width,depth:b.depth,baseline:previous,...c};conflicts.push(conflict);if(c.min[0]<Math.min(-b.width/2,previous.min[0])-1e-5||c.max[0]>Math.max(b.width/2,previous.max[0])+1e-5||c.min[2]<Math.min(-b.depth/2,previous.min[2])-1e-5||c.max[2]>Math.max(b.depth/2,previous.max[2])+1e-5)worsened.push(conflict);}
  }
  art.forEach(a=>a.dispose());rows.push({seed,tick:world.tick,inputSha256:hash(bytes),offices:buildings.length,beforeTriangles:before,afterTriangles:after,deltaTriangles:after-before,parcelEnvelopeConflicts:conflicts,newOrWorsenedConflicts:worsened});
 }
 await writeFile(output,JSON.stringify({baselineKitSha256:hash(baseline),candidateKitSha256:hash(await readFile('web/src/render/kit.ts')),scope:'Actual complete open office meshes below 12 floors, local parcel bounding boxes, three ordinary checkpoints. Tenant-sign canvas material replaced by a plain material for Node execution; sign geometry retained. Excludes tower kit, construction stages, neighbor occlusion, projected appearance and measured GPU cost.',rows},null,2)+'\n');console.log(rows.map(({parcelEnvelopeConflicts,newOrWorsenedConflicts,...r})=>({...r,parcelEnvelopeConflicts:parcelEnvelopeConflicts.length,newOrWorsenedConflicts:newOrWorsenedConflicts.length})));
 if(rows.some(r=>r.newOrWorsenedConflicts.length))process.exitCode=1;
}finally{await rm(dir,{recursive:true,force:true});}
