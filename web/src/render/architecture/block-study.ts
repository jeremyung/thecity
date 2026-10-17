import {applyBuildingWeathering} from './character';
import * as THREE from 'three/webgpu';
import type {Building} from '../../types';
import type {ArtMaterials} from '../materials';
import {buildingKit} from '../kit';
import {buildApartmentBuilding,type ApartmentRecipeId,type ApartmentBuildOptions} from './index';
import {Kit} from './geometry-kit';

/** Synthetic, dimensioned fixture; not a generated CityEngine neighbourhood. */
export function blockStudyBuildings(seed=32):Array<Building&{recipe?:ApartmentRecipeId}>{
 const row=[
  {width:8,depth:12,floors:5,kind:'apartment',recipe:'walkup'},
  {width:12,depth:14,floors:7,kind:'apartment',recipe:'gallery'},
  {width:9,depth:12,floors:2,kind:'shop'},
  {width:13,depth:15,floors:6,kind:'apartment',recipe:'terrace'},
  {width:8,depth:11,floors:2,kind:'home'},
  {width:7,depth:10,floors:8,kind:'apartment',recipe:'compact'},
  {width:11,depth:13,floors:5,kind:'apartment',recipe:'brick'},
  {width:14,depth:16,floors:9,kind:'apartment',recipe:'modern'},
  {width:10,depth:13,floors:6,kind:'apartment',recipe:'urban'},
 ] as const;
 return Array.from({length:24},(_,i)=>{
  const side=i<12?0:1,slot=i%12,spec=row[(slot+side*3)%row.length];
  const ordered=Array.from({length:12},(_,j)=>row[(j+side*3)%row.length]);
  const length=ordered.reduce((sum,b)=>sum+b.width+1.1,0);
  const start=-length/2+ordered.slice(0,slot).reduce((sum,b)=>sum+b.width+1.1,0);
  return {...spec,id:i+1,parcel_id:i+1,x:start+spec.width/2,z:(side?1:-1)*(8+spec.depth/2),rotation:side?Math.PI:0,stage:'open',progress:1,capacity:40,occupancy:24,built_day:0,name:`Block specimen ${i+1}`,appearance_seed:(seed+i*19)>>>0,history:[],anchor:false,vacant_days:0};
 });
}
export function buildBlockStudy(art:ArtMaterials,seed=32,detail:ApartmentBuildOptions['detail']='district'){
 const group=new THREE.Group(),buildings=blockStudyBuildings(seed);
 for(const b of buildings){
  const model=buildApartmentBuilding(b,art,{recipe:b.recipe,detail})??buildingKit(b,art);
  if(b.kind!=='apartment')applyBuildingWeathering(model,{profile:'maintained',strength:.12,phase:b.appearance_seed%997});
  model.position.set(b.x,0,b.z);model.rotation.y=b.rotation;group.add(model);
 }
 const street=new Kit(),road=art.get('#67706e'),curb=art.get('#bdbaac'),paint=art.get('#d7c8a0');
 street.box(0,-.04,0,155,.08,12,road);
 for(const z of [-7,7]){street.box(0,.025,z,155,.05,2,curb);street.box(0,.13,z+(z>0?-1:1),155,.18,.14,curb);}
 for(let x=-75;x<75;x+=5)street.box(x,.012,0,2.2,.015,.08,paint);
 group.add(street.finish());
 group.userData={blockStudy:true,buildings:buildings.length,detail,recipeIds:['gallery','walkup','terrace','compact','brick','modern','urban'],source:'synthetic fixture using Building records'};
 return group;
}
