import * as THREE from 'three/webgpu';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { TrainFleet } from '../web/src/render/trains';
import { ArtMaterials } from '../web/src/render/materials';
import type { World } from '../web/src/types';
const world = (route=[1,2], nodes=[{id:1,x:0,z:0},{id:2,x:0,z:200}]) => ({trains:[{id:42,route}],nodes,roads:[]}) as unknown as World;
function setup() {
  vi.stubGlobal('document',{createElement:()=>({width:0,height:0,getContext:()=>({fillRect(){},fillText(){},strokeRect(){}})})});
  const art=new ArtMaterials(); return {art,train:new TrainFleet(art)};
}
function render(train:TrainFleet,yaw:number) {train.begin();train.add(42,12,80,yaw,6.935,0,1/60);train.finish(1);}
const matrices=(train:TrainFleet)=>train.meshes.map(m=>Array.from(m.instanceMatrix.array));
afterEach(()=>vi.unstubAllGlobals());
describe('commuter train terminal reversal',()=>{
  it('keeps the physical consist fixed through interpolated reversal and swaps lamp direction',()=>{
    const {art,train}=setup();try{
      train.setWorld(world());render(train,0);const before=matrices(train);
      const cab=train.meshes.find(m=>m.count===2)!;
      expect(cab.geometry.getAttribute('trainFacing').getX(0)).toBe(1);
      expect(cab.geometry.getAttribute('trainFacing').getX(1)).toBe(-1);
      for(const yaw of [Math.PI/4,Math.PI/2,3*Math.PI/4,Math.PI]){render(train,yaw);expect(matrices(train)).toEqual(before);}
      expect(cab.geometry.getAttribute('trainFacing').getX(0)).toBe(-1);
      expect(cab.geometry.getAttribute('trainFacing').getX(1)).toBe(1);
    }finally{train.dispose();art.dispose();}
  });
  it('clears cached route headings when the world is replaced',()=>{
    const {art,train}=setup();try{
      train.setWorld(world());train.setWorld(world([7,8],[{id:7,x:0,z:0}]));render(train,Math.PI/2);
      const m=new THREE.Matrix4();train.meshes[0].getMatrixAt(0,m);
      const forward=new THREE.Vector3(0,0,1).transformDirection(m);
      expect(forward.x).toBeCloseTo(1,5);expect(forward.z).toBeCloseTo(0,5);
    }finally{train.dispose();art.dispose();}
  });
});
