import{expect,it,vi}from'vitest';
import*as THREE from'three/webgpu';
import{styleWarmup}from'../web/src/render/style-warmup';
it('warms material layouts without copying the city instance arrays or owning source geometry',()=>{
 const scene=new THREE.Scene(),geometry=new THREE.BoxGeometry(),material=new THREE.MeshStandardNodeMaterial();
 const source=new THREE.InstancedMesh(geometry,material,20000);source.setColorAt(0,new THREE.Color('#408050'));source.receiveShadow=true;scene.add(source);
 const duplicate=new THREE.InstancedMesh(geometry,material,5000);duplicate.setColorAt(0,new THREE.Color('#408050'));duplicate.receiveShadow=true;scene.add(duplicate);
 const plain=new THREE.Mesh(geometry,material);scene.add(plain,new THREE.HemisphereLight());
 const warm=styleWarmup(scene,m=>m),meshes=warm.scene.children.filter(o=>o instanceof THREE.Mesh);
 expect(meshes).toHaveLength(2);
 const instance=meshes.find(o=>o instanceof THREE.InstancedMesh) as THREE.InstancedMesh;
 expect(instance.instanceMatrix.count).toBe(1);expect(instance.instanceColor?.count).toBe(1);expect(instance.geometry).toBe(geometry);
 expect(instance.instanceMatrix.array).not.toBe(source.instanceMatrix.array);
 const disposeGeometry=vi.spyOn(geometry,'dispose'),disposeMaterial=vi.spyOn(material,'dispose');warm.dispose();
 expect(disposeGeometry).not.toHaveBeenCalled();expect(disposeMaterial).not.toHaveBeenCalled();expect(source.instanceMatrix.count).toBe(20000);
 source.dispose();duplicate.dispose();geometry.dispose();material.dispose();
});

import{compileStyleSamples}from'../web/src/render/style-warmup';
it('submits one shader layout per task while retaining lights and restoring sample ownership',async()=>{
 const scene=new THREE.Scene(),light=new THREE.HemisphereLight();scene.add(light);
 const geometry=new THREE.BoxGeometry(),material=new THREE.MeshStandardNodeMaterial();
 const samples=[new THREE.Mesh(geometry,material),new THREE.Mesh(geometry,material)];scene.add(...samples);
 const events:string[]=[];
 const timing={submitMs:0,waitMs:0,maxSubmitMs:0,compiledSamples:0};
 await compileStyleSamples(scene,async current=>{
   expect(current.children).toContain(light);
   const meshes=current.children.filter(o=>o instanceof THREE.Mesh);
   expect(meshes).toHaveLength(1);events.push(`compile${samples.findIndex(sample=>sample===meshes[0])}`);
 },()=>true,timing,async()=>{events.push('yield');});
 expect(events).toEqual(['compile0','yield','compile1','yield']);
 expect(timing.compiledSamples).toBe(2);expect(scene.children).toEqual([light,...samples]);
 geometry.dispose();material.dispose();
});
it('stops superseded warmup before the next layout and restores samples after errors',async()=>{
 const scene=new THREE.Scene(),geometry=new THREE.BoxGeometry(),material=new THREE.MeshStandardNodeMaterial();
 const samples=[new THREE.Mesh(geometry,material),new THREE.Mesh(geometry,material)];scene.add(...samples);
 const timing={submitMs:0,waitMs:0,maxSubmitMs:0,compiledSamples:0};let current=true;
 await compileStyleSamples(scene,async()=>{},()=>current,timing,async()=>{current=false;});
 expect(timing.compiledSamples).toBe(1);expect(scene.children).toEqual(samples);
 await expect(compileStyleSamples(scene,async()=>{throw new Error('compile failed');},()=>true,timing,async()=>{})).rejects.toThrow('compile failed');
 expect(scene.children).toEqual(samples);
 geometry.dispose();material.dispose();
});
