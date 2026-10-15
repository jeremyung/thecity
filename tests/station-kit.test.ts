import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import type { RailPlatform } from '../web/src/types';
import { buildStationPlatform } from '../web/src/render/station-kit';
import { disposeGroup } from '../web/src/render/kit';
import { pedestrianSurface } from '../web/src/movement';
import { PLATFORM_EDGE_X, TRAIN_HALF_LENGTH, RAIL_PLATFORM_END, RAIL_GATEWAY_STAIR_BOTTOM } from '../web/src/render/rail-profile';

describe('rail platform physical support', () => {
  it('supports every queue slot in rotated and reversed terminal frames', () => {
    const materials = new Map<string, THREE.MeshBasicNodeMaterial>();
    const art = { get(color: string) { let m=materials.get(color); if(!m){m=new THREE.MeshBasicNodeMaterial();materials.set(color,m);}return m; } };
    try {
      for (const [ux,uz] of [[1,0],[-1,0],[.6,.8],[-.6,-.8]]) for (const gateway of [false, true]) {
        const layout: RailPlatform = {node_id:1,x:24,z:-31,ux,uz,surface_y:8,stair_x:4.05,stair_bottom_z:gateway?RAIL_GATEWAY_STAIR_BOTTOM:-20,stair_top_z:gateway?RAIL_PLATFORM_END:-1,ground_y:.27,stair_steps:44, ground_path: [{x:0,z:0},{x:7.5,z:-1.5},{x:7.5,z:gateway?RAIL_GATEWAY_STAIR_BOTTOM+1.5:-21.5},{x:4.05,z:gateway?RAIL_GATEWAY_STAIR_BOTTOM+1.5:-21.5},{x:4.05,z:gateway?RAIL_GATEWAY_STAIR_BOTTOM:-20}], ground_width: 1.6, construction_phase: 4};
        const group=buildStationPlatform(layout,art);group.updateMatrixWorld(true);
        const surfaceAt=(x:number,z:number)=>{
          const world=new THREE.Vector3(x,11,z).applyMatrix4(group.matrixWorld);
          const ray=new THREE.Raycaster(world,new THREE.Vector3(0,-1,0));
          return ray.intersectObject(group,true).filter(hit=>(hit.object as THREE.Mesh).material===materials.get('#b9bbb1'))[0]?.point.y;
        };
        expect(surfaceAt(PLATFORM_EDGE_X + .02, TRAIN_HALF_LENGTH)).toBeCloseTo(8, 4);
        expect(surfaceAt(PLATFORM_EDGE_X - .02, TRAIN_HALF_LENGTH)).toBeUndefined();
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
        // No unsupported gap at either slab/stair junction, including reverse run.
        for (const dz of [-.005, .005]) {
          expect(surfaceAt(layout.stair_x, layout.stair_top_z + dz)).toBeCloseTo(8,4);
        }
        // Ground walkers need body clearance, not just a supported stair centerline.
        const apronZ=gateway?RAIL_GATEWAY_STAIR_BOTTOM+1.5:-21.5;
        const approach=[[0,0],[7.5,-1.5],[7.5,apronZ],[4.05,apronZ],[4.05,layout.stair_bottom_z+.02*(gateway?1:-1)]];
        for(let i=1;i<approach.length;i++){
          const [ax,az]=approach[i-1], [bx,bz]=approach[i];
          const length=Math.hypot(bx-ax,bz-az), dx=(bx-ax)/length, dz=(bz-az)/length;
          for(const offset of [-.33,0,.33]) for(const height of [.4,1.2,2.07]){
            const origin=new THREE.Vector3(ax-dz*offset,height,az+dx*offset).applyMatrix4(group.matrixWorld);
            const direction=new THREE.Vector3(dx,0,dz).transformDirection(group.matrixWorld);
            const ray=new THREE.Raycaster(origin,direction,0,length);
            expect(ray.intersectObject(group,true),`ground leg ${i}, body ${offset}/${height}, gateway=${gateway}`).toHaveLength(0);
          }
        }
        disposeGroup(group);
      }
    }finally{for(const m of materials.values())m.dispose();}
  });

  it('builds supports, deck/access, canopy and fittings in physical order', () => {
    const material = new THREE.MeshBasicNodeMaterial();
    const art = { get: () => material };
    try {
      for (const gateway of [false, true]) {
        for (const phase of [0, 1, 2, 3, 4]) {
          const layout: RailPlatform = { node_id: 1, x: 24, z: -31, ux: .6, uz: .8,
            surface_y: 8, stair_x: 4.05, stair_bottom_z: gateway ? RAIL_GATEWAY_STAIR_BOTTOM : -20,
            stair_top_z: gateway ? RAIL_PLATFORM_END : -1, ground_y: .27, stair_steps: 44, ground_path: [{x:0,z:0},{x:7.5,z:-1.5},{x:7.5,z:gateway?RAIL_GATEWAY_STAIR_BOTTOM+1.5:-21.5},{x:4.05,z:gateway?RAIL_GATEWAY_STAIR_BOTTOM+1.5:-21.5},{x:4.05,z:gateway?RAIL_GATEWAY_STAIR_BOTTOM:-20}], ground_width: 1.6,
            construction_phase: phase };
          const group = buildStationPlatform(layout, art);
          group.updateMatrixWorld(true);
          const highestAt = (x: number, z: number) => {
            const origin = new THREE.Vector3(x, 20, z).applyMatrix4(group.matrixWorld);
            return new THREE.Raycaster(origin, new THREE.Vector3(0,-1,0))
              .intersectObject(group, true)[0]?.point.y;
          };
          // At a pier center: ground pad, support, slab, then overhead canopy.
          expect(highestAt(5.2, TRAIN_HALF_LENGTH)).toBeCloseTo([.2, 6, 8, 12.11, 12.11][phase], 4);
          const stair = highestAt(4.05, (layout.stair_bottom_z + layout.stair_top_z) / 2);
          if (phase < 2) expect(stair).toBeUndefined();
          else expect(stair).toBeGreaterThan(4);
          // A horizontal ray below the canopy isolates the vending machine from the roof.
          const origin = new THREE.Vector3(5.6, 8.85, 40).applyMatrix4(group.matrixWorld);
          const direction = new THREE.Vector3(0,0,1).transformDirection(group.matrixWorld);
          const fittings = new THREE.Raycaster(origin, direction, 0, 3)
            .intersectObject(group, true);
          expect(fittings.length > 0).toBe(phase === 4);
          disposeGroup(group);
        }
      }
    } finally { material.dispose(); }
  });

  it('supports the full approach width and turning landings only after the deck phase', () => {
    const paving=new THREE.MeshBasicNodeMaterial(), other=new THREE.MeshBasicNodeMaterial();
    const art={get:(color:string)=>color==='#b3ac98'?paving:other};
    try {
      for(const gateway of [false,true]) for(const phase of [1,2,4]) {
        const apron=gateway?RAIL_GATEWAY_STAIR_BOTTOM+1.5:-21.5,bottom=gateway?RAIL_GATEWAY_STAIR_BOTTOM:-20;
        const ground_path=[{x:0,z:0},{x:7.5,z:-1.5},{x:7.5,z:apron},{x:4.05,z:apron},{x:4.05,z:bottom}];
        const layout:RailPlatform={node_id:1,x:12,z:31,ux:.6,uz:.8,surface_y:8,
          stair_x:4.05,stair_bottom_z:bottom,stair_top_z:gateway?RAIL_PLATFORM_END:-1,ground_y:.27,
          stair_steps:44,construction_phase:phase,ground_path,ground_width:1.6};
        const group=buildStationPlatform(layout,art);group.updateMatrixWorld(true);
        for(let leg=1;leg<ground_path.length;leg++) {
          const a=ground_path[leg-1],b=ground_path[leg],len=Math.hypot(b.x-a.x,b.z-a.z);
          for(let t=.05;t<1;t+=.05) for(const offset of [-.33,0,.33]) {
            const x=a.x+(b.x-a.x)*t-(b.z-a.z)/len*offset,z=a.z+(b.z-a.z)*t+(b.x-a.x)/len*offset;
            const origin=new THREE.Vector3(x,1,z).applyMatrix4(group.matrixWorld);
            const hits=new THREE.Raycaster(origin,new THREE.Vector3(0,-1,0)).intersectObject(group,true)
              .filter(hit=>(hit.object as THREE.Mesh).material===paving);
            if(phase<2)expect(hits).toHaveLength(0);else expect(hits[0]?.point.y).toBeCloseTo(.27,4);
          }
        }
        // Full paving width, not only the pedestrian body, clears gateway piers.
        expect(7.5-layout.ground_width/2).toBeGreaterThan(5.2+1.2);
        disposeGroup(group);
      }
    }finally{paving.dispose();other.dispose();}
  });

  it('uses explicit walking surfaces without lifting nearby ground pedestrians',()=>{
    expect(pedestrianSurface(8,1.7)).toBe(8);
    expect(pedestrianSurface(-1)).toBe(.27);
    expect(pedestrianSurface(-1,1.7)).toBeCloseTo(1.97);
    expect(pedestrianSurface(0)).toBe(0);
  });
});
