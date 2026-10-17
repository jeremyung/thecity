import {expect,it} from 'vitest';
import * as THREE from 'three/webgpu';
import {ArtMaterials} from '../web/src/render/materials';
import {buildingKit,disposeGroup} from '../web/src/render/kit';
import type {Building} from '../web/src/types';
const make=(width=12,depth=14,floors=7,seed=32):Building=>({id:1,parcel_id:1,x:0,z:0,rotation:0,width,depth,floors,kind:'apartment',stage:'open',progress:1,capacity:40,occupancy:20,built_day:0,name:'pilot',appearance_seed:seed,history:[],anchor:false,vacant_days:0});
it('fits variable parcels without stretching components and reconstructs deterministically',()=>{
 const art=new ArtMaterials();
 try{for(const width of [5,7.5,12,24])for(const depth of [8,14,24])for(const floors of [4,7,11]){
  const b=make(width,depth,floors),before=JSON.stringify(b);
  const g=buildingKit(b,art,{apartmentComponents:true});
  const bounds=new THREE.Box3().setFromObject(g);
  expect(bounds.min.x).toBeGreaterThanOrEqual(-width/2);expect(bounds.max.x).toBeLessThanOrEqual(width/2);
  expect(bounds.min.z).toBeGreaterThanOrEqual(-depth/2);expect(bounds.max.z).toBeLessThanOrEqual(depth/2);
  expect(bounds.max.y).toBeGreaterThan(floors*3.25);expect(g.userData.apartmentPilot.installedFloors).toBe(floors);
  expect(JSON.stringify(b)).toBe(before);
  const again=buildingKit({...b},art,{apartmentComponents:true});
  expect(again.children.length).toBe(g.children.length);
  for(let i=0;i<g.children.length;i++)expect(Array.from((again.children[i] as THREE.Mesh).geometry.getAttribute('position').array)).toEqual(Array.from((g.children[i] as THREE.Mesh).geometry.getAttribute('position').array));
  disposeGroup(g);disposeGroup(again);
 }}finally{art.dispose();}
},20000);
it('keeps incomplete floors and roof out of cladding stages, supports renovation, and preserves legacy fallback',()=>{
 const art=new ArtMaterials();
 try{
  for(const stage of ['surface','finishing'] as const){const b={...make(),stage,progress:.8};const g=buildingKit(b,art,{apartmentComponents:true});expect(g.userData.apartmentPilot.installedFloors).toBeLessThan(b.floors);expect(g.userData.apartmentPilot.roofInstalled).toBe(false);expect(new THREE.Box3().setFromObject(g).max.y).toBeLessThanOrEqual(.59+b.floors*3.25);disposeGroup(g);}
  const g=buildingKit({...make(),stage:'renovation',progress:.5},art,{apartmentComponents:true});expect(g.userData.apartmentPilot.installedFloors).toBe(7);disposeGroup(g);
  const legacy=buildingKit(make(),art);expect(legacy.userData.apartmentPilot).toBeUndefined();disposeGroup(legacy);
  const structure=buildingKit({...make(),stage:'structure',progress:.5},art,{apartmentComponents:true});expect(structure.userData.apartmentPilot.installedFloors).toBe(0);expect(structure.userData.apartmentPilot.width).toBe(10.7);disposeGroup(structure);
 }finally{art.dispose();}
});
it('places a continuous balcony in front of genuinely recessed unit openings',()=>{
 const art=new ArtMaterials();
 try{
  const b=make(),g=buildingKit(b,art,{apartmentComponents:true});g.updateMatrixWorld(true);
  const meta=g.userData.apartmentPilot,w=meta.width,core=meta.coreWidth;
  const left=-w/2+.14,right=w/2-core-.14,x=left+(right-left)/meta.bays*.5;
  const front=meta.circulation.front,y=.58+3*3.25;
  const at=(height:number)=>new THREE.Raycaster(new THREE.Vector3(x,height,front+6),new THREE.Vector3(0,0,-1)).intersectObject(g,true)[0];
  const guard=at(y+.55),opening=at(y+1.5);
  expect(guard).toBeDefined();expect(opening).toBeDefined();
  expect(guard.point.z-opening.point.z).toBeGreaterThan(1.25);
  expect(opening.point.z).toBeLessThan(front);
  disposeGroup(g);
 }finally{art.dispose();}
});
