import { afterEach, describe, expect, it, vi } from 'vitest';
import { TrainFleet, TRAIN_HALF_LENGTH } from '../web/src/render/trains';
import { ArtMaterials } from '../web/src/render/materials';
import type { World } from '../web/src/types';
afterEach(()=>vi.unstubAllGlobals());
describe('fleet door packet integration',()=>{
  it('updates all car slots on movement, preserves interpolation on snapshot, and closes on departure',()=>{
    vi.stubGlobal('document',{createElement:()=>({width:0,height:0,getContext:()=>({fillRect(){},fillText(){},strokeRect(){}})})});
    const art=new ArtMaterials(),train=new TrainFleet(art);
    const world={trains:[{id:42,route:[1,2],door_open_fraction:1,door_terminal:1}],nodes:[{id:1,x:0,z:0},{id:2,x:0,z:200}],roads:[],rail_platforms:[{node_id:1,x:0,z:0,ux:0,uz:1}]} as unknown as World;
    const render=(z:number,alpha:number)=>{train.begin();train.add(42,0,z,0,6.935,0,1/60,alpha);train.finish(1);};
    const openings=()=>train.doorMeshes.flatMap(m=>Array.from({length:m.count},(_,i)=>Math.abs(m.geometry.getAttribute('doorOpening').getX(i))));
    try{
      train.setWorld(world);train.setMovement(new Float32Array([42,1,1]));render(TRAIN_HALF_LENGTH,1);expect(openings()).toEqual([1,1,1,1]);
      train.setMovement(new Float32Array([42,1,0]));train.setWorld(world);
      render(TRAIN_HALF_LENGTH,.5);expect(openings()).toEqual([.5,.5,.5,.5]);
      render(TRAIN_HALF_LENGTH,1);expect(openings()).toEqual([0,0,0,0]);
      train.setMovement(new Float32Array([42,1,1]));render(TRAIN_HALF_LENGTH+1,1);expect(openings()).toEqual([0,0,0,0]);
      train.reset();expect(train.doorMeshes.every(m=>m.count===0)).toBe(true);
      render(TRAIN_HALF_LENGTH,1);expect(openings()).toEqual([0,0,0,0]);
    }finally{train.dispose();art.dispose();}
  });
  // Geometry passage checks are retained in train-door-apertures.test.ts;
  // GPU displacement still requires browser verification (CPU raycasts do not run TSL).
});
