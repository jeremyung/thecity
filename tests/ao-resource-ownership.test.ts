import {expect,it,vi} from 'vitest';
import {PerspectiveCamera,Texture} from 'three/webgpu';
import {texture} from 'three/tsl';
import {ownedAO} from '../web/src/render/postprocessing/owned-ao';

it('releases private AO noise and targets once while preserving borrowed attachments',()=>{
  const depth=new Texture(),normal=new Texture();
  const borrowed=[vi.spyOn(depth,'dispose'),vi.spyOn(normal,'dispose')];
  const effect=ownedAO(texture(depth),texture(normal),new PerspectiveCamera()) as any;
  const noise=vi.spyOn(effect._noiseNode.value,'dispose');
  const target=vi.spyOn(effect._aoRenderTarget,'dispose');
  const material=vi.spyOn(effect._material,'dispose');
  effect.dispose();effect.dispose();
  for(const spy of [noise,target,material])expect(spy).toHaveBeenCalledTimes(1);
  for(const spy of borrowed)expect(spy).not.toHaveBeenCalled();
});

it('tolerates an absent private noise field while still releasing AO targets',()=>{
  const effect=ownedAO(texture(new Texture()),texture(new Texture()),new PerspectiveCamera()) as any;
  effect._noiseNode.value.dispose();
  delete effect._noiseNode;
  const target=vi.spyOn(effect._aoRenderTarget,'dispose');
  expect(()=>{effect.dispose();effect.dispose();}).not.toThrow();
  expect(target).toHaveBeenCalledTimes(1);
});
