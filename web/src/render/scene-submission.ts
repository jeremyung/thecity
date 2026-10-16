import {ColorManagement,NoToneMapping,NodeFrame} from 'three/webgpu';
import type {PassNode,RenderPipeline,WebGPURenderer} from 'three/webgpu';

/** Submit the shared scene at a stable nesting depth before optional effects.
 * Three r183 keys scene/shadow programs by render-call depth. Letting DOF
 * request the scene from inside its first pass recompiles the entire city.
 */
export function submitSceneAndEffects(renderer:WebGPURenderer,scene:PassNode,pipeline:RenderPipeline,frame:NodeFrame){
  const toneMapping=renderer.toneMapping,outputColorSpace=renderer.outputColorSpace;
  frame.renderer=renderer;
  try{
    // Match RenderPipeline's linear HDR scene input; final output conversion
    // still belongs to the selected pipeline, with the user's tone mapping.
    renderer.toneMapping=NoToneMapping;
    renderer.outputColorSpace=ColorManagement.workingColorSpace;
    scene.updateBefore(frame);
  }finally{
    renderer.toneMapping=toneMapping;
    renderer.outputColorSpace=outputColorSpace;
  }
  pipeline.render();
}
