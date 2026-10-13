import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import type { RailPlatform } from '../web/src/types';
import { buildStationPlatform } from '../web/src/render/station-kit';
import { disposeGroup } from '../web/src/render/kit';
import { pedestrianSurface } from '../web/src/movement';
import { PLATFORM_EDGE_X } from '../web/src/render/rail-profile';

describe('rail platform physical support', () => {
  it('supports every queue slot in rotated and reversed terminal frames', () => {
    const materials = new Map<string, THREE.MeshBasicNodeMaterial>();
    const art = { get(color: string) { let m=materials.get(color); if(!m){m=new THREE.MeshBasicNodeMaterial();materials.set(color,m);}return m; } };
    try {
      for (const [ux,uz] of [[1,0],[-1,0],[.6,.8],[-.6,-.8]]) {
        const layout: RailPlatform = {node_id:1,x:24,z:-31,ux,uz,surface_y:8,stair_x:4.05,stair_bottom_z:-20,stair_top_z:-1,ground_y:.27,stair_steps:44};
        const group=buildStationPlatform(layout,art);group.updateMatrixWorld(true);
        const surfaceAt=(x:number,z:number)=>{
          const world=new THREE.Vector3(x,11,z).applyMatrix4(group.matrixWorld);
          const ray=new THREE.Raycaster(world,new THREE.Vector3(0,-1,0));
          return ray.intersectObject(group,true).filter(hit=>(hit.object as THREE.Mesh).material===materials.get('#b9bbb1'))[0]?.point.y;
        };
        expect(surfaceAt(PLATFORM_EDGE_X + .02, 23.5)).toBeCloseTo(8, 4);
        expect(surfaceAt(PLATFORM_EDGE_X - .02, 23.5)).toBeUndefined();
        for(let slot=0;slot<96;slot++){
          // Independent acceptance footprint: two clear rows of people, away from furniture.
          const x=slot%2===0?3.6:4.5,z=2+Math.floor(slot/2)*.9;
          expect(surfaceAt(x,z)).toBeCloseTo(8,4);
          expect(x+.33).toBeLessThan(5.125); // nearest bench edge
          expect(x-.33).toBeGreaterThan(PLATFORM_EDGE_X + .525); // tactile strip outer edge
        }
        let previous=layout.ground_y;
        for(let step=0;step<layout.stair_steps;step++){
          const t=(step+.5)/layout.stair_steps;
          const y=surfaceAt(layout.stair_x,layout.stair_bottom_z+(layout.stair_top_z-layout.stair_bottom_z)*t)!;
          expect(y).toBeGreaterThan(previous);
          expect(y-previous).toBeLessThan(.177);
          previous=y;
        }
        expect(previous).toBeCloseTo(layout.surface_y,4);
        disposeGroup(group);
      }
    }finally{for(const m of materials.values())m.dispose();}
  });

  it('uses explicit walking surfaces without lifting nearby ground pedestrians',()=>{
    expect(pedestrianSurface(8,1.7)).toBe(8);
    expect(pedestrianSurface(-1)).toBe(.27);
    expect(pedestrianSurface(-1,1.7)).toBeCloseTo(1.97);
    expect(pedestrianSurface(0)).toBe(0);
  });
});
