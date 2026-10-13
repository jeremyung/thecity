// CPU-only parity/timing probe. Preserve the pre-change CityRenderer bundle at
// .tools/tree-clearance/before.mjs before changing the implementation.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { cpus } from 'node:os';
import { build } from 'esbuild';
import * as THREE from 'three/webgpu';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
await build({ entryPoints: ['web/src/render/city.ts'], bundle: true, platform: 'node', format: 'esm', packages: 'external', outfile: '.tools/tree-clearance/after.mjs' });
const { CityRenderer: Before } = await import('../.tools/tree-clearance/before.mjs');
const { CityRenderer: After } = await import('../.tools/tree-clearance/after.mjs');
initSync({ module: readFileSync('public/wasm/city_engine_bg.wasm') });
const engine = CityEngine.restore(readFileSync('.tools/movement-support/presentation-cost-save.json', 'utf8'));
const worlds = [JSON.parse(engine.snapshot())];
let token = worlds[0].presentation_revision, last = 0;
for (let i = 1; i <= 150; i++) {
  engine.step(8);
  const next = engine.presentation_revision();
  if (next !== token || i-last >= 10) { worlds.push(JSON.parse(engine.snapshot())); token=next; last=i; }
}
engine.free();
function harness(Renderer) {
  const r = Object.create(Renderer.prototype);
  // Fixed synthetic tree sites isolate clearance; city snapshots are real growth.
  r.treePoints = Array.from({length:1024}, (_,i) => ({x:-387+(i%32)*25,z:-387+Math.floor(i/32)*25,s:1.5+(i%7)*.4}));
  const geometry = new THREE.BoxGeometry(1,1,1), material = new THREE.MeshBasicMaterial();
  r.treeBatches=[{indices:r.treePoints.map((_,i)=>i), trees:new THREE.InstancedMesh(geometry,material,1024), trunks:new THREE.InstancedMesh(geometry,material,1024)}];
  r.treeClearanceKey='';
  return r;
}
const before = harness(Before), after=harness(After);
const costs={before:[],after:[]}; let skipped=0;
// Warm the JS paths, then reset the cache so first real input must still compute.
for(let i=0;i<3;i++){before.updateTrees(worlds[0]);after.treeClearanceKey='';after.updateTrees(worlds[0]);}
after.treeClearanceKey='';
for(let i=0;i<worlds.length;i++){
  const version=after.treeBatches[0].trees.instanceMatrix.version;
  for(const [name,r] of i%2 ? [['after',after],['before',before]] : [['before',before],['after',after]]){
    const start=performance.now();r.updateTrees(worlds[i]);costs[name].push(performance.now()-start);
  }
  if(after.treeBatches[0].trees.instanceMatrix.version===version)skipped++;
  assert.deepEqual(after.treeBatches[0].trees.instanceMatrix.array,before.treeBatches[0].trees.instanceMatrix.array);
  assert.deepEqual(after.treeBatches[0].trunks.instanceMatrix.array,before.treeBatches[0].trunks.instanceMatrix.array);
  assert.deepEqual(after.treeBatches[0].trees.boundingSphere,before.treeBatches[0].trees.boundingSphere);
}
function stats(a){const s=[...a].sort((a,b)=>a-b);return {count:a.length,totalMs:a.reduce((a,b)=>a+b,0),p50Ms:s[Math.floor(s.length*.5)],p95Ms:s[Math.min(s.length-1,Math.floor(s.length*.95))],maxMs:s.at(-1)};}
const report={scope:'Node CPU only: actual updateTrees, synthetic 1024 tree sites, seeded growth snapshots; no browser/GPU or tree-population claim',host:{cpu:cpus()[0].model,node:process.version},seed:worlds[0].seed,ticks:[worlds[0].tick,worlds.at(-1).tick],snapshots:worlds.length,skipped,parity:'Exact trunk/foliage instance arrays and tree bounding spheres at every snapshot',costs:{before:stats(costs.before),after:stats(costs.after)}};
writeFileSync('validation/tree-clearance-cache-cost.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));
for(const r of [before,after]){r.treeBatches[0].trees.geometry.dispose();r.treeBatches[0].trees.material.dispose();}
