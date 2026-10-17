import {expect,it} from 'vitest';
import {corporateGlassNormal,corporateOutline} from '../web/src/render/architecture/corporate-skyscraper';
it('keeps curved glass normals unit length, outward and continuous across LOD chords',()=>{
 for(const segments of [2,4,6]){
  const points=corporateOutline(30,24,4,segments);
  for(const p of points){
   const n=corporateGlassNormal(p.x,p.z,30,24,4);
   expect(n.length()).toBeCloseTo(1,6);
   expect(n.x*p.x+n.z*p.z).toBeGreaterThan(0);
  }
  expect(corporateGlassNormal(15,8,30,24,4).toArray()).toEqual([1,0,0]);
  expect(corporateGlassNormal(11,12,30,24,4).toArray()).toEqual([0,0,1]);
 }
});
