import type {ApartmentEra} from './era-preference';
import type {Building,World} from '../../types';
import {architectureCityStudy} from './city-study';
import {BUSINESS_RECIPES,selectBusinessRecipe,type BusinessFamily} from './business-recipes';
/** Fixed complete-street coverage of every recipe through the production selector. */
export function businessCityStudy(seed=32,family:BusinessFamily|'business'='business',era:ApartmentEra='mixed'):World{
 const mode=era==='legacy'?'mixed':era;
 const recipes=BUSINESS_RECIPES.filter(r=>(family==='business'||r.family===family)&&(mode==='mixed'||r.era===mode));
 const buildings:Building[]=recipes.map((r,i)=>{
  const row=Math.floor(i/5),col=i%5,floors=r.id==='curtain-tower'||r.id==='vertical-fin-office'?22:r.id==='retail-residential-tower'?24:r.id==='retail-office-podium'?14:r.family==='commercial'?(['vertical-retail','media-corner','old-department-store'].includes(r.id)?6:['screened-flagship','terraced-retail'].includes(r.id)?4:2):7;
  const b:Building={id:i+1,parcel_id:i+1,x:(col-2)*28,z:row*45-65,rotation:0,width:r.form==='podium'?26:r.form==='arcade'?24:20,depth:22,floors,kind:r.family==='office'?'office':r.family==='mixed-use'?(r.upper==='home'?'apartment':'office'):r.id==='corner-cafe'?'cafe':'shop',stage:'open',progress:1,capacity:80,occupancy:50,business_active:true,built_day:0,name:r.label,appearance_seed:seed>>>0,anchor:false,vacant_days:0,history:[]};
  let tries=0;while(selectBusinessRecipe(b,mode)?.id!==r.id&&tries++<100000)b.appearance_seed=(b.appearance_seed+1)>>>0;
  if(selectBusinessRecipe(b,mode)?.id!==r.id)throw Error(`Unreachable business recipe ${r.id}`);return b;
 });
 const source=architectureCityStudy(seed),rows=Math.ceil(buildings.length/5),nodes=Array.from({length:rows},(_,i)=>[{id:i*2+1,x:-100,z:i*45-46,kind:i===0?'station':'junction'},{id:i*2+2,x:100,z:i*45-46,kind:'junction'}]).flat();
 return {...source,bounds:{min_x:-160,max_x:160,min_z:-120,max_z:Math.max(120,rows*45)},nodes,roads:Array.from({length:rows},(_,i)=>({id:i+1,from:i*2+1,to:i*2+2,kind:'arterial',stage:'open',progress:1,connected:true,length:200,lanes:2})),buildings,parcels:buildings.map(b=>({id:b.parcel_id,x:b.x,z:b.z,width:b.width,depth:b.depth,rotation:0,access_node:Math.floor((b.id-1)/5)*2+1,suitability:1,protected:false,reservation:null})),building_access:buildings.map(b=>({building_id:b.id,road_id:Math.floor((b.id-1)/5)+1,points:[{x:b.x,z:b.z+b.depth/2},{x:b.x,z:b.z+12}]}))};
}
