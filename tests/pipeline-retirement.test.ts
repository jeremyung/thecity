import {expect,it,vi} from 'vitest';
import {CityRenderer} from '../web/src/render/city';

it('retains borrowed pipeline resources through pending compilation without retiring their replacements',()=>{
  const renderer=Object.create(CityRenderer.prototype) as CityRenderer;
  const slots=['occlusion','reflections','shafts','pipeline','depthOfField','shine','scenePass'] as const;
  const old=slots.map(()=>({dispose:vi.fn()}));
  const next=slots.map(()=>({dispose:vi.fn()}));
  Object.assign(renderer,{pendingCompiles:1,retiredResources:[],...Object.fromEntries(slots.map((slot,i)=>[slot,old[i]]))});
  renderer['disposePipeline']();
  for(const slot of slots)expect(renderer[slot]).toBeNull();
  for(const resource of old)expect(resource.dispose).not.toHaveBeenCalled();
  Object.assign(renderer,Object.fromEntries(slots.map((slot,i)=>[slot,next[i]])));
  Object.assign(renderer,{pendingCompiles:0});
  for(const release of renderer['retiredResources'].splice(0))release();
  for(const resource of old)expect(resource.dispose).toHaveBeenCalledTimes(1);
  for(const resource of next)expect(resource.dispose).not.toHaveBeenCalled();
  renderer['disposePipeline']();
  for(const resource of next)expect(resource.dispose).toHaveBeenCalledTimes(1);
  expect(renderer['retiredResources']).toHaveLength(0);
});
