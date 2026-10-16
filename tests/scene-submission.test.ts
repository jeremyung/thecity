import {expect,it,vi} from 'vitest';
import {ACESFilmicToneMapping,ColorManagement,NoToneMapping,NodeFrame,SRGBColorSpace} from 'three/webgpu';
import {submitSceneAndEffects} from '../web/src/render/scene-submission';

it('submits one linear scene before either output, restoring output conversion',()=>{
  const renderer={toneMapping:ACESFilmicToneMapping,outputColorSpace:SRGBColorSpace} as any;
  const frame=new NodeFrame(),order:string[]=[];
  const scene={updateBefore:vi.fn((f:NodeFrame)=>{expect(f.renderer).toBe(renderer);expect(renderer.toneMapping).toBe(NoToneMapping);expect(renderer.outputColorSpace).toBe(ColorManagement.workingColorSpace);order.push('scene');})} as any;
  const output=()=>({render:vi.fn(()=>{expect(renderer.toneMapping).toBe(ACESFilmicToneMapping);expect(renderer.outputColorSpace).toBe(SRGBColorSpace);order.push('output');})}) as any;
  const off=output(),on=output();
  submitSceneAndEffects(renderer,scene,off,frame);submitSceneAndEffects(renderer,scene,on,frame);
  expect(order).toEqual(['scene','output','scene','output']);
  expect(off.render).toHaveBeenCalledTimes(1);expect(on.render).toHaveBeenCalledTimes(1);
});

it('restores renderer state and skips effects after a failed scene submission',()=>{
 const renderer={toneMapping:ACESFilmicToneMapping,outputColorSpace:SRGBColorSpace} as any;
 const scene={updateBefore:()=>{throw Error('scene failed');}} as any;
 const pipeline={render:vi.fn()} as any;
 expect(()=>submitSceneAndEffects(renderer,scene,pipeline,new NodeFrame())).toThrow('scene failed');
 expect(renderer.toneMapping).toBe(ACESFilmicToneMapping);expect(renderer.outputColorSpace).toBe(SRGBColorSpace);
 expect(pipeline.render).not.toHaveBeenCalled();
});
