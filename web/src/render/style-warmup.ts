import * as THREE from 'three/webgpu';

/** One representative per material/geometry shader layout, with one instance.
 * Geometry is borrowed; neither the live instance arrays nor the world tree are cloned. */
export function styleWarmup(scene:THREE.Scene,variant:(material:THREE.Material)=>THREE.Material){
  const prepared=scene.clone(false),keys=new Set<string>(),instances:THREE.InstancedMesh[]=[];
  scene.traverse(object=>{
    if(!(object instanceof THREE.Mesh))return;
    const original=Array.isArray(object.material)?object.material:[object.material];
    const materials=original.map(variant);
    // Shared water/sky nodes already render in both styles. Compiling them again
    // runs viewport-copy hooks outside a render pass, with no valid framebuffer.
    if(materials.every((material,index)=>material===original[index]))return;
    const layout=Object.entries(object.geometry.attributes as Record<string,THREE.BufferAttribute|THREE.InterleavedBufferAttribute>).map(([name,attribute])=>`${name}:${attribute.itemSize}:${attribute.normalized}`).sort().join(',');
    const instanced=object instanceof THREE.InstancedMesh;
    const key=`${materials.map(m=>m.uuid).join('/')}:${layout}:${instanced}:${instanced&&!!object.instanceColor}:${object.receiveShadow}`;
    if(keys.has(key))return;keys.add(key);
    const material=Array.isArray(object.material)?materials:materials[0];
    let mesh:THREE.Mesh;
    if(instanced){
      const representative=new THREE.InstancedMesh(object.geometry,material,1);
      const matrix=new THREE.Matrix4();object.getMatrixAt(0,matrix);representative.setMatrixAt(0,matrix);
      if(object.instanceColor){const color=new THREE.Color();object.getColorAt(0,color);representative.setColorAt(0,color);}
      instances.push(representative);mesh=representative;
    }else mesh=new THREE.Mesh(object.geometry,material);
    mesh.matrixAutoUpdate=false;mesh.matrix.copy(object.matrixWorld);mesh.castShadow=object.castShadow;mesh.receiveShadow=object.receiveShadow;mesh.frustumCulled=false;
    prepared.add(mesh);
  });
  // Compile against the live target scene so lighting identity matches actual
  // rendering. The renderer borrows its lights without reparenting or cloning
  // them; only representative instance arrays belong to this temporary scene.
  let disposed=false;
  return{scene:prepared,targetScene:scene,dispose(){if(disposed)return;disposed=true;for(const instance of instances)instance.dispose();prepared.clear();}};
}

export interface WarmupTiming { submitMs:number; waitMs:number; maxSubmitMs:number; compiledSamples:number }

/** Submit one layout at a time: compileAsync performs synchronous shader work
 * before returning its promise. Yield a browser task between layouts. */
export async function compileStyleSamples(
  scene:THREE.Scene,
  compile:(scene:THREE.Scene)=>Promise<unknown>,
  isCurrent:()=>boolean,
  timing:WarmupTiming,
  yieldTask:()=>Promise<void>=()=>new Promise(resolve=>setTimeout(resolve,0)),
):Promise<void>{
  const samples=scene.children.filter(object=>object instanceof THREE.Mesh);
  for(const sample of samples)scene.remove(sample);
  try{
    for(const sample of samples){
      if(!isCurrent())return;
      scene.add(sample);
      try{
        const start=performance.now();
        const pending=compile(scene);
        const submitted=performance.now()-start;
        timing.submitMs+=submitted;
        timing.maxSubmitMs=Math.max(timing.maxSubmitMs,submitted);
        const waiting=performance.now();
        try{await pending;}finally{timing.waitMs+=performance.now()-waiting;}
        timing.compiledSamples++;
      }finally{scene.remove(sample);}
      // An already-resolved compiler promise alone only yields a microtask.
      await yieldTask();
    }
  }finally{
    // Restore ownership for the caller's single disposal path, including cancel/error.
    for(const sample of samples)scene.add(sample);
  }
}

/** Match the live scene attachments during synchronous compiler submission.
 * compileAsync runs node update hooks (including viewport copies) before yielding.
 * Restore immediately so animation never borrows the warmup target while awaiting GPU work.
 * Caller must initialize the renderer before invoking this helper.
 */
export function compileForScenePass(
  renderer: THREE.WebGPURenderer,
  pass: {renderTarget: THREE.RenderTarget; getMRT(): ReturnType<THREE.WebGPURenderer['getMRT']> | null},
  scene: THREE.Scene, camera: THREE.Camera, targetScene: THREE.Scene,
): Promise<void> {
  const target=renderer.getRenderTarget(),mrt=renderer.getMRT();
  const face=renderer.getActiveCubeFace(),mip=renderer.getActiveMipmapLevel();
  try {
    renderer.setRenderTarget(pass.renderTarget);
    renderer.setMRT(pass.getMRT());
    return renderer.compileAsync(scene,camera,targetScene);
  } finally {
    renderer.setRenderTarget(target,face,mip);
    renderer.setMRT(mrt);
  }
}
