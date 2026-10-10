import { afterEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { CityBuses } from '../web/src/render/buses';
import { ArtMaterials } from '../web/src/render/materials';
import type { World } from '../web/src/types';

afterEach(()=>vi.unstubAllGlobals());
const world={nodes:[{id:4,x:12,z:30,kind:'station'}],buses:[{id:91,line_id:2,progress:0,direction:1,dwelling:true,passengers:7,capacity:24,next_stop:4,waiting:2}],bus_lines:[{id:2,stops:[4],route:[4],active:true,stop_positions:[{node_id:4,x:12,z:36.2,heading:0,direction:1}]}]} as World;
describe('bus presentation',()=>{
  it('uses authoritative curb markers, bounded vehicle instances, and visible dwell state',()=>{
    vi.stubGlobal('document',{createElement:()=>({width:0,height:0,getContext:()=>({fillRect(){},fillText(){},strokeRect(){}})})});
    const art=new ArtMaterials(),retired:Array<()=>void>=[],buses=new CityBuses(art,release=>retired.push(release));
    buses.setWorld(world);const stopGroup=buses.root.children.at(-1)!;
    stopGroup.updateMatrixWorld(true);const bounds=new THREE.Box3().setFromObject(stopGroup);
    expect(bounds.getCenter(new THREE.Vector3()).x).toBeCloseTo(12);
    expect(bounds.getCenter(new THREE.Vector3()).z).toBeCloseTo(36.2);
    const children=buses.root.children.length;buses.setWorld(world);expect(buses.root.children.length).toBe(children);
    buses.begin();buses.set(0,91,30,40,Math.PI/2,world);buses.finish(1);
    expect(buses.ids).toEqual([91]);
    for(const mesh of buses.meshes){expect(mesh.count).toBe(1);const m=new THREE.Matrix4();mesh.getMatrixAt(0,m);expect(new THREE.Vector3().setFromMatrixPosition(m).toArray()).toEqual([30,0,40]);expect(mesh.geometry.getAttribute('position').count).toBeGreaterThan(0);}
    const entry=buses.root.children.find(o=>o instanceof THREE.InstancedMesh&&!buses.meshes.includes(o)) as THREE.InstancedMesh;
    expect(entry.count).toBe(1);
    // A bus facing +X opens on its left curb (+Z), matching engine queues.
    const doorMatrix=new THREE.Matrix4();entry.getMatrixAt(0,doorMatrix);
    expect(new THREE.Vector3().setFromMatrixPosition(doorMatrix).z).toBeGreaterThan(40);
    buses.begin();buses.set(0,91,31,40,Math.PI/2,{...world,buses:[{...world.buses![0],dwelling:false}]});buses.finish(1);expect(entry.count).toBe(0);
    buses.set(2,999,100,100,0,world);expect(buses.ids).toEqual([91]);
    buses.reset();expect(buses.ids).toEqual([]);expect(buses.meshes.every(mesh=>mesh.count===0)).toBe(true);
    buses.dispose();retired.forEach(release=>release());art.dispose();
  });
});
