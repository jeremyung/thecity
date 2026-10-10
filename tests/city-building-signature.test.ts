import { describe, expect, it } from 'vitest';
import { buildingRenderSignature } from '../web/src/render/city';
import type { Building } from '../web/src/types';

const building=(overrides:Partial<Building>={}):Building=>({
  id:1,parcel_id:1,x:0,z:0,rotation:0,width:8,depth:8,floors:2,kind:'home',stage:'open',progress:1,
  capacity:12,occupancy:0,business_active:false,built_day:1,name:'Lane House',appearance_seed:41,history:[],anchor:false,vacant_days:0,
  ...overrides,
});

describe('building render signatures',()=>{
  it('rebuilds home windows only when occupancy crosses the vacant boundary',()=>{
    const vacant=building({occupancy:0});
    const occupied=building({occupancy:1});
    const fuller=building({occupancy:2});
    expect(buildingRenderSignature(vacant)).not.toBe(buildingRenderSignature(occupied));
    expect(buildingRenderSignature(occupied)).toBe(buildingRenderSignature(fuller));
    expect(buildingRenderSignature(fuller)).not.toBe(buildingRenderSignature(building({occupancy:0})));
  });

  it('does not rebuild unrelated building kinds for occupancy changes',()=>{
    for(const kind of ['shop','cafe','apartment','office']){
      const empty=building({kind,occupancy:0});
      const occupied=building({kind,occupancy:11});
      expect(buildingRenderSignature(empty),kind).toBe(buildingRenderSignature(occupied));
    }
  });
});
