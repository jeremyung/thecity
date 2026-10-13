// Ray-test actual production station and road mesh constructors on ordinary routes.
import {readFileSync,writeFileSync} from 'node:fs';
import {build} from 'esbuild';
import * as THREE from 'three/webgpu';
await build({stdin:{contents:"export {buildStationPlatform} from './web/src/render/station-kit'; export {groundRoadFootprint,groundRoadSurfaceGeometry} from './web/src/render/ground-road-footprint'; export {roadWidth} from './web/src/render/road-elevation';",resolveDir:process.cwd(),loader:'ts'},bundle:true,platform:'node',format:'esm',packages:'external',outfile:'.tools/station-height-audit.mjs'});
const {buildStationPlatform,groundRoadFootprint,groundRoadSurfaceGeometry,roadWidth}=await import('../.tools/station-height-audit.mjs');
const cases=[];
for(const seed of [87231,0,5]){
  const world=JSON.parse(readFileSync(`.tools/walking-offset-${seed}/station-exit-world.json`));
  const suffix=seed===87231?'':`-${seed}`;
  const trace=JSON.parse(readFileSync(`validation/station-walking-offset${suffix}-exit-trace.json`));
  const material=new THREE.MeshBasicNodeMaterial({side:THREE.DoubleSide});
  const meshes=world.rail_platforms.map(p=>buildStationPlatform(p,{get:()=>material}));
  for(const r of world.roads.filter(r=>r.stage==='open'&&['local','lane','arterial'].includes(r.kind))){
    const a=world.nodes.find(n=>n.id===r.from),b=world.nodes.find(n=>n.id===r.to);
    meshes.push(new THREE.Mesh(groundRoadSurfaceGeometry(groundRoadFootprint(a,b,roadWidth(r.kind))),material));
  }
  for(const mesh of meshes)mesh.updateMatrixWorld(true);
  let samples=0,minHeight=Infinity,maxHeight=-Infinity;const missing=[],unexpectedHeight=[];
  const path=trace.actor.path.points;
  for(let leg=4;leg<path.length;leg++){
    const a=path[leg-1],b=path[leg],steps=Math.ceil(Math.hypot(b.x-a.x,b.z-a.z)/.1);
    for(let i=0;i<=steps;i++)for(let sample=0;sample<17;sample++){
      const angle=sample*Math.PI/8,radius=sample===16?0:.33;
      const x=a.x+(b.x-a.x)*i/steps+radius*Math.cos(angle);
      const z=a.z+(b.z-a.z)*i/steps+radius*Math.sin(angle);
      const hit=new THREE.Raycaster(new THREE.Vector3(x,1,z),new THREE.Vector3(0,-1,0),0,1).intersectObjects(meshes,true)[0];
      samples++;
      if(!hit){missing.push({leg,x,z});continue;}
      minHeight=Math.min(minHeight,hit.point.y);maxHeight=Math.max(maxHeight,hit.point.y);
      // Near the stair foot, the turning body's disk can touch the first tread.
      const nearFoot=Math.hypot(x-path[3].x,z-path[3].z)<.34;
      if(hit.point.y<.269||hit.point.y>(nearFoot?.447:.301))unexpectedHeight.push({leg,x,z,y:hit.point.y});
    }
  }
  cases.push({seed,samples,minHeight,maxHeight,missing,unexpectedHeight});
  for(const group of meshes)group.traverse(o=>o.geometry?.dispose());material.dispose();
}
writeFileSync('validation/station-road-landing-height.json',JSON.stringify({scope:'Ground legs of actual saved exits, centers and 16 perimeter points on a 0.33m body disk every <=0.1m. Production station/road constructors, no generic ground patches; flat roads only, not a whole-city collision audit.',cases},null,2)+'\n');
console.log(cases.map(({missing,unexpectedHeight,...c})=>({...c,missing:missing.length,unexpectedHeight:unexpectedHeight.length})));
if(cases.some(c=>c.missing.length||c.unexpectedHeight.length))process.exitCode=1;
