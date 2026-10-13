import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { CityTrains } from '../web/src/render/trains';
import type { World } from '../web/src/types';
import { RAIL_HEAD_Y } from '../web/src/render/rail-profile';

const art = {get:(color:string)=>new THREE.MeshStandardMaterial({color})};
function world(fraction:number, terminal=1):World {
  return {trains:[{id:42,route:[1,2],progress:23.5,door_open_fraction:fraction,door_terminal:terminal}],
    nodes:[{id:1,x:0,z:0,kind:'station'},{id:2,x:0,z:200,kind:'rail'}],
    rail_platforms:[{node_id:1,x:0,z:0,ux:0,uz:1},{node_id:2,x:0,z:200,ux:0,uz:-1}]} as World;
}
function render(t:CityTrains,z=23.5,now=1){t.begin();t.set(0,42,0,z,0,now);t.finish(1);t.root.updateMatrixWorld(true);}
function doorway(t:CityTrains,side:number,z=23.5) {
  const ray=new THREE.Raycaster(new THREE.Vector3(side*2,RAIL_HEAD_Y+2.4,z+.12),new THREE.Vector3(-side,0,0));
  return ray.intersectObject(t.root,true)[0]?.point.x;
}
describe('train sliding doors',()=>{
  it('opens a real passage only on the platform side at either terminal',()=>{
    for(const terminal of [1,2]){
      const t=new CityTrains(art),z=terminal===1?23.5:176.5,side=terminal===1?1:-1;
      try{
        t.setWorld(world(0,terminal));render(t,z);
        expect(doorway(t,side,z)!*side).toBeGreaterThan(1.4);
        t.setMovement(new Float32Array([42,terminal,1]));render(t,z,1);
        expect(doorway(t,side,z)!*side).toBeLessThan(-1.4);
        expect(doorway(t,-side,z)!*-side).toBeGreaterThan(1.4);
        expect(t.doors.ids).toHaveLength(36);
        expect(new Set(t.doors.ids)).toEqual(new Set([42]));
      }finally{t.dispose();}
    }
  });
  it('uses movement interpolation for closing and ignores later coarse world snapshots',()=>{
    const t=new CityTrains(art), matrix=new THREE.Matrix4();
    const leafX=()=>{t.doors.meshes[0].getMatrixAt(18,matrix);return matrix.elements[14];};
    try{
      t.setWorld(world(1));t.setMovement(new Float32Array([42,1,1]));
      render(t);const open=leafX();
      t.setMovement(new Float32Array([42,1,0]));
      render(t,23.5,0);expect(leafX()).toBeCloseTo(open);
      render(t,23.5,.5);const midway=leafX();
      t.setWorld(world(1)); // World cadence must not reopen a closing door.
      render(t,23.5,1);const closed=leafX();
      expect(midway).toBeCloseTo((open+closed)/2);
      expect(Math.abs(open-closed)).toBeCloseTo(.56);
      render(t,23.5,1);expect(leafX()).toBeCloseTo(closed); // paused frame
      t.setMovement(new Float32Array([42,-1,0]));
      render(t,24.4,1);expect(doorway(t,1,24.4)).toBeGreaterThan(1.4);
    }finally{t.dispose();}
  });
  it('closes stale open doors on departure and clears all instances on reset',()=>{
    const t=new CityTrains(art);
    try{
      t.setWorld(world(1));render(t);expect(doorway(t,1)).toBeLessThan(-1.4);
      render(t,24.4);expect(doorway(t,1,24.4)).toBeGreaterThan(1.4);
      t.reset();for(const mesh of t.doors.meshes)expect(mesh.count).toBe(0);
      expect(t.doors.ids).toEqual([]);
      render(t);expect(doorway(t,1)).toBeGreaterThan(1.4);
    }finally{t.dispose();}
  });
});
