import { describe, expect, it } from 'vitest';
import { lowriseBuildingWidth, lowriseMassing, type LowriseMassing } from '../web/src/render/lowrise-massing';

function building(overrides:Partial<{kind:string;floors:number;width:number;depth:number;appearance_seed:number;anchor:boolean}>={}){
  return {kind:'home',floors:2,width:10,depth:9,appearance_seed:51,anchor:false,...overrides};
}

function expectInside(layout:LowriseMassing,width:number,depth:number){
  for(const mass of layout.masses){
    expect(mass.x-mass.width/2).toBeGreaterThanOrEqual(-width/2-1e-8);
    expect(mass.x+mass.width/2).toBeLessThanOrEqual(width/2+1e-8);
    expect(mass.z-mass.depth/2).toBeGreaterThanOrEqual(-depth/2-1e-8);
    expect(mass.z+mass.depth/2).toBeLessThanOrEqual(depth/2+1e-8);
    expect(mass.baseFloor).toBe(0);
    expect(Number.isInteger(mass.floors)).toBe(true);
    expect(mass.floors).toBeGreaterThan(0);
  }
}

describe('seeded compact low-rise massing',()=>{
  it('keeps a full-height frontage and supported one-floor rear service wing inside small parcels',()=>{
    for(const kind of ['home','shop','cafe'])for(const floors of [1,2,3])for(const [width,depth] of [[6,6],[8,10],[13,9]] as const){
      const b=building({kind,floors,width,depth,appearance_seed:Math.imul(width*depth+floors,2654435761)>>>0});
      const d=Math.max(6,depth*.84),w=lowriseBuildingWidth(b,Math.max(6,width*.84),d);
      const layout=lowriseMassing(b,w,d)!;
      const front=layout.masses.find(mass=>mass.role==='frontage')!;
      const service=layout.masses.find(mass=>mass.role==='service')!;
      expectInside(layout,width,depth);
      expect(front.z+front.depth/2).toBeCloseTo(d/2,8);
      expect(front.width/width).toBeGreaterThanOrEqual(.92);
      expect(front.floors).toBe(floors);
      expect(service.floors).toBe(1);
      expect(service.z+service.depth/2).toBeGreaterThanOrEqual(front.z-front.depth/2-.001);
      expect(service.z-service.depth/2).toBeLessThan(front.z-front.depth/2);
      expect(layout.quietSides).toHaveLength(1);
      expect(['gable','shed','parapet']).toContain(front.roof);
      expect(['gable','shed','parapet']).toContain(service.roof);
    }
  });

  it('is deterministic while varying roof profiles and quiet sides by seed',()=>{
    const b=building({width:14,depth:7});
    const first=lowriseMassing(b,11.76,6)!;
    expect(lowriseMassing(b,11.76,6)).toEqual(first);
    const variants=Array.from({length:16},(_,seed)=>lowriseMassing({...b,appearance_seed:seed},11.76,6)!);
    expect(new Set(variants.map(layout=>layout.masses[0].roof)).size).toBeGreaterThan(1);
    expect(new Set(variants.map(layout=>layout.quietSides[0])).size).toBe(2);
    variants.forEach(layout=>expectInside(layout,b.width,b.depth));
  });

  it('leaves anchors, other uses, unsupported floor counts and undersized envelopes to their existing kit paths',()=>{
    const base=building();
    expect(lowriseBuildingWidth({...base,anchor:true},8,8)).toBe(8);
    expect(lowriseBuildingWidth({...base,kind:'apartment'},8,8)).toBe(8);
    expect(lowriseBuildingWidth({...base,floors:4},8,8)).toBe(8);
    expect(lowriseMassing({...base,anchor:true},8,8)).toBeNull();
    expect(lowriseMassing({...base,kind:'apartment'},8,8)).toBeNull();
    expect(lowriseMassing({...base,floors:4},8,8)).toBeNull();
    expect(lowriseMassing({...base,floors:0},8,8)).toBeNull();
    expect(lowriseMassing({...base,width:4.99},4.99,8)).toBeNull();
    expect(lowriseMassing(base,Number.NaN,8)).toBeNull();
  });
});
