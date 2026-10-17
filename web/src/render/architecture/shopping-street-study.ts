import type {Building,World} from '../../types';
import {architectureCityStudy} from './city-study';
import {architecturalChoice as pick} from './variation';
import type {CommercialDistrict} from './commercial-district';
/** Packed opposing retail rows, generated from parcel dimensions with real road access. */
export function shoppingStreetStudy(seed=32,district:CommercialDistrict='shibuya'):World{
 const source=architectureCityStudy(seed),buildings:Building[]=[];
 for(let row=0;row<2;row++){
  let cursor=-79;
  for(let col=0;col<12;col++){
   const id=row*12+col+1,s=pick(seed,550,id),width=9.5+(s%5)*.6,depth=18+s%3,market=district==='ameyoko';
   const b:Building={id,parcel_id:id,x:cursor+width/2,z:(row?1:-1)*(7+depth/2),rotation:row?Math.PI:0,width,depth,floors:market?2+s%3:4+s%6,kind:col%5===0?'cafe':'shop',stage:'open',progress:1,capacity:60,occupancy:45,business_active:true,built_day:0,name:`${district} tenant ${id}`,appearance_seed:s,anchor:false,vacant_days:0,history:[]};buildings.push(b);cursor+=width+.65;
  }
 }
 return {...source,bounds:{min_x:-130,max_x:130,min_z:-85,max_z:85},buildings,nodes:[{id:1,x:-108,z:0,kind:'station'},{id:2,x:108,z:0,kind:'junction'}],roads:[{id:1,from:1,to:2,kind:'arterial',stage:'open',progress:1,connected:true,length:216,lanes:1}],parcels:buildings.map(b=>({id:b.id,x:b.x,z:b.z,width:b.width,depth:b.depth,rotation:b.rotation,access_node:1,suitability:1,protected:false,reservation:null})),building_access:buildings.map(b=>({building_id:b.id,road_id:1,points:[{x:b.x,z:b.z>0?7:-7},{x:b.x,z:0}]}))};
}
