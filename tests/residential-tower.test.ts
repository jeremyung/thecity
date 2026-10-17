import {expect,it,vi} from 'vitest';
import * as THREE from 'three/webgpu';
import {buildResidentialTower,RESIDENTIAL_TOWERS} from '../web/src/render/architecture/residential-tower';
import {ApartmentResources} from '../web/src/render/architecture/resources';
import {disposeGroup} from '../web/src/render/kit';
import type {Building} from '../web/src/types';
const specimen=(width=26,depth=24,floors=32):Building=>({id:1,parcel_id:1,x:0,z:0,rotation:0,width,depth,floors,kind:'apartment',stage:'open',progress:1,capacity:100,occupancy:70,built_day:0,name:'Tower prototype',appearance_seed:32,anchor:false,history:[],vacant_days:0});
const triangles=(g:THREE.Group)=>{let n=0;g.traverse(o=>{if(o instanceof THREE.Mesh)n+=(o.geometry.index?.count??o.geometry.getAttribute('position').count)/3;});return n;};
it('builds all five tower options inside variable parcels without mutating authoritative inputs',()=>{
 const error=vi.spyOn(console,'error');
 const resources=new ApartmentResources();try{for(const recipe of Object.keys(RESIDENTIAL_TOWERS) as (keyof typeof RESIDENTIAL_TOWERS)[])for(const width of [14,26,40])for(const depth of [14,24,40])for(const floors of [12,32,60]){
  const b=specimen(width,depth,floors),before=JSON.stringify(b),g=buildResidentialTower(b,resources.art,{recipe})!;
  const bounds=new THREE.Box3().setFromObject(g);expect(bounds.min.x).toBeGreaterThanOrEqual(-width/2-.001);expect(bounds.max.x).toBeLessThanOrEqual(width/2+.001);expect(bounds.min.z).toBeGreaterThanOrEqual(-depth/2-.001);expect(bounds.max.z).toBeLessThanOrEqual(depth/2+.001);
  expect(bounds.max.y).toBeGreaterThan(floors*3.25);expect(g.userData.residentialTower.recipe).toBe(recipe);expect(JSON.stringify(b)).toBe(before);
  g.traverse(o=>{if(o instanceof THREE.Mesh){expect(Array.from(o.geometry.getAttribute('position').array).every(Number.isFinite)).toBe(true);if((o.material as THREE.Material).userData.apartmentSurface&&(o.material as THREE.Material).userData.apartmentRole!=='glass')expect(o.geometry.getAttribute('architectureWear')).toBeDefined();}});disposeGroup(g);
 }expect(error).not.toHaveBeenCalled();}finally{resources.dispose();error.mockRestore();}
},20000);
it('distinguishes recipes, is deterministic, and reduces detail without losing its roof silhouette',()=>{
 const resources=new ApartmentResources();try{
  const b=specimen(),counts:number[]=[];
  for(const recipe of Object.keys(RESIDENTIAL_TOWERS) as (keyof typeof RESIDENTIAL_TOWERS)[]){
   const close=buildResidentialTower(b,resources.art,{recipe,detail:'close'})!,repeat=buildResidentialTower(b,resources.art,{recipe,detail:'close'})!,far=buildResidentialTower(b,resources.art,{recipe,detail:'far'})!;
   counts.push(triangles(close));expect(triangles(close)).toBe(triangles(repeat));expect(triangles(far)).toBeLessThan(triangles(close));
   const bounds=new THREE.Box3().setFromObject(close),other=new THREE.Box3().setFromObject(far);expect(bounds.max.distanceTo(other.max)).toBeLessThan(.001);
   for(let i=0;i<close.children.length;i++)expect(Array.from((close.children[i] as THREE.Mesh).geometry.getAttribute('position').array)).toEqual(Array.from((repeat.children[i] as THREE.Mesh).geometry.getAttribute('position').array));
   disposeGroup(close);disposeGroup(repeat);disposeGroup(far);
  }
  expect(new Set(counts).size).toBe(5);expect(Object.keys(RESIDENTIAL_TOWERS)).toHaveLength(5);
 }finally{resources.dispose();}
});
it('supports foundation, frame and partial cladding with no completed crown, and rejects unsuitable inputs',()=>{
 const resources=new ApartmentResources();try{
  for(const stage of ['foundation','structure','surface','finishing']){
   const b={...specimen(),stage,progress:.76},g=buildResidentialTower(b,resources.art)!;
   expect(g.userData.residentialTower.roofInstalled).toBe(false);expect(g.userData.residentialTower.installedFloors).toBeLessThan(b.floors);expect(new THREE.Box3().setFromObject(g).max.y).toBeLessThanOrEqual(.6+b.floors*3.25);disposeGroup(g);
  }
  for(const b of [{...specimen(),floors:11},{...specimen(),width:8},{...specimen(),kind:'office'},{...specimen(),width:NaN}])expect(buildResidentialTower(b,resources.art)).toBeNull();
 }finally{resources.dispose();}
});
it('uses seeded shaft proportions as well as unit-level glazing variation',()=>{
 const resources=new ApartmentResources();try{const proportions=new Set<string>();for(let appearance_seed=0;appearance_seed<12;appearance_seed++){
  const g=buildResidentialTower({...specimen(),appearance_seed},resources.art,{recipe:'ribbon'})!;const meta=g.userData.residentialTower;proportions.add(`${meta.shaftWidth}/${meta.shaftDepth}`);disposeGroup(g);
 }expect(proportions.size).toBe(3);}finally{resources.dispose();}
});
