import {CORPORATE_SKYSCRAPERS} from './corporate-tower-recipes';
import type {Building} from '../../types';
import type {ApartmentEra} from './era-preference';
import {architecturalChoice} from './variation';
export type BusinessFamily='office'|'commercial'|'mixed-use';
export type DisplayMode='off'|'static'|'animated';
export function parseDisplayMode(value:unknown):DisplayMode{return value==='off'||value==='static'?value:'animated';}
export interface BusinessRecipe {
 mediaLandmark?:'wrap'|'stack'|'frame'|'curved';
 district?:import('./commercial-district').DenseDistrict;
 towerForm?:'split'|'rounded'|'chamfered'|'setback'|'crown';
 id:string;family:BusinessFamily;era:'older'|'contemporary';label:string;
 form:'punched'|'ribbon'|'frame'|'curtain'|'stepped'|'shop-house'|'tenant'|'corner'|'podium'|'arcade'|'fins'|'screen'|'media'|'department'|'slab'|'loft';
 wall:string;roughness:number;bay:number;upper:'office'|'home'|'shop';retailFloors:number;awning:boolean;
}
/** Different floorplates and frontage grammars, rather than colour variants of one box. */
export const BUSINESS_RECIPES:readonly BusinessRecipe[]=[
 {id:'tiled-office',family:'office',era:'older',label:'Tiled tenant office',form:'punched',wall:'#ece9df',roughness:.85,bay:2.5,upper:'office',retailFloors:0,awning:false},
 {id:'alc-ribbon',family:'office',era:'older',label:'ALC ribbon office',form:'ribbon',wall:'#d5d8ce',roughness:.85,bay:3.6,upper:'office',retailFloors:0,awning:false},
 {id:'deep-frame',family:'office',era:'contemporary',label:'Deep framed office',form:'frame',wall:'#e7e8e3',roughness:.85,bay:3.8,upper:'office',retailFloors:0,awning:false},
 {id:'curtain-tower',family:'office',era:'contemporary',label:'Curtain wall office',form:'curtain',wall:'#626762',roughness:.85,bay:2.1,upper:'office',retailFloors:0,awning:false},
 {id:'terrace-office',family:'office',era:'contemporary',label:'Stepped terrace office',form:'stepped',wall:'#d8cfbb',roughness:.85,bay:3.4,upper:'office',retailFloors:0,awning:false},
 {id:'neighborhood-shops',family:'commercial',era:'older',label:'Neighborhood shopfronts',form:'shop-house',wall:'#986b56',roughness:.85,bay:3.2,upper:'shop',retailFloors:1,awning:true},
 {id:'corner-cafe',family:'commercial',era:'older',label:'Corner cafe',form:'corner',wall:'#dad8cf',roughness:.85,bay:3.5,upper:'shop',retailFloors:1,awning:true},
 {id:'market-arcade',family:'commercial',era:'older',label:'Covered market arcade',form:'arcade',wall:'#c9bdad',roughness:.85,bay:3,upper:'shop',retailFloors:1,awning:true},
 {id:'vertical-retail',family:'commercial',era:'contemporary',label:'Vertical retail tenants',form:'tenant',wall:'#747771',roughness:.85,bay:4,upper:'shop',retailFloors:2,awning:false},
 {id:'glass-showroom',family:'commercial',era:'contemporary',label:'Glass showroom',form:'curtain',wall:'#ece9df',roughness:.85,bay:4.5,upper:'shop',retailFloors:2,awning:false},
 {id:'shop-residences',family:'mixed-use',era:'older',label:'Shop with homes above',form:'shop-house',wall:'#d4d6ce',roughness:.85,bay:3,upper:'home',retailFloors:1,awning:true},
 {id:'vertical-tenants',family:'mixed-use',era:'older',label:'Vertical tenant stack',form:'tenant',wall:'#bfb9b0',roughness:.85,bay:2.7,upper:'office',retailFloors:1,awning:false},
 {id:'corner-worklive',family:'mixed-use',era:'contemporary',label:'Corner work/live',form:'corner',wall:'#897c69',roughness:.55,bay:3.5,upper:'home',retailFloors:1,awning:true},
 {id:'retail-office-podium',family:'mixed-use',era:'contemporary',label:'Retail and office podium',form:'podium',wall:'#d9dfdf',roughness:.85,bay:3.6,upper:'office',retailFloors:2,awning:false},
 {id:'retail-residential-tower',family:'mixed-use',era:'contemporary',label:'Retail and residential tower',form:'podium',wall:'#e1e2da',roughness:.85,bay:3.8,upper:'home',retailFloors:2,awning:false},
 {id:'concrete-office',family:'office',era:'older',label:'Concrete structural office',form:'frame',wall:'#c8cfc6',roughness:.85,bay:2.9,upper:'office',retailFloors:0,awning:false},
 {id:'narrow-office',family:'office',era:'older',label:'Narrow vertical office',form:'tenant',wall:'#bfb9b0',roughness:.85,bay:2.4,upper:'office',retailFloors:0,awning:false},
 {id:'corporate-slab',family:'office',era:'older',label:'Older corporate slab',form:'slab',wall:'#c9bdad',roughness:.85,bay:3.2,upper:'office',retailFloors:0,awning:false},
 {id:'vertical-fin-office',family:'office',era:'contemporary',label:'Vertical-fin office',form:'fins',wall:'#a3947e',roughness:.55,bay:2.8,upper:'office',retailFloors:0,awning:false},
 {id:'corporate-retail-tower',family:'office',era:'contemporary',label:'Office tower and glazed podium',form:'podium',wall:'#626762',roughness:.85,bay:2.2,upper:'office',retailFloors:2,awning:false},
 {id:'market-stalls',family:'commercial',era:'older',label:'Open-front market stalls',form:'loft',wall:'#b7b7a9',roughness:.85,bay:2.6,upper:'shop',retailFloors:1,awning:true},
 {id:'old-department-store',family:'commercial',era:'older',label:'Older department store',form:'department',wall:'#d9d0be',roughness:.85,bay:3.1,upper:'shop',retailFloors:2,awning:false},
 {id:'screened-flagship',family:'commercial',era:'contemporary',label:'Screened flagship showroom',form:'screen',wall:'#ece9df',roughness:.85,bay:4.5,upper:'shop',retailFloors:2,awning:false},
 {id:'terraced-retail',family:'commercial',era:'contemporary',label:'Terraced contemporary retail',form:'stepped',wall:'#d8cfbb',roughness:.85,bay:4.2,upper:'shop',retailFloors:2,awning:false},
 {id:'media-corner',family:'commercial',era:'contemporary',label:'Media corner commercial',form:'media',wall:'#626762',roughness:.85,bay:4.1,upper:'shop',retailFloors:2,awning:false},
 {id:'market-workshops',family:'mixed-use',era:'older',label:'Market and workshops above',form:'loft',wall:'#986b56',roughness:.85,bay:2.8,upper:'office',retailFloors:1,awning:true},
 {id:'concrete-shop-offices',family:'mixed-use',era:'older',label:'Concrete shop and office building',form:'frame',wall:'#c8cfc6',roughness:.85,bay:3,upper:'office',retailFloors:1,awning:true},
 {id:'postwar-shop-flats',family:'mixed-use',era:'older',label:'Postwar shops and flats',form:'ribbon',wall:'#c9bdad',roughness:.85,bay:2.6,upper:'home',retailFloors:1,awning:true},
 {id:'fin-worklive',family:'mixed-use',era:'contemporary',label:'Screened work/live building',form:'fins',wall:'#897c69',roughness:.55,bay:3,upper:'home',retailFloors:1,awning:false},
 {id:'terraced-workplace',family:'mixed-use',era:'contemporary',label:'Retail and terraced workplace',form:'stepped',wall:'#d8cfbb',roughness:.85,bay:3.6,upper:'office',retailFloors:2,awning:false},
];
export const BUSINESS_RECIPE_VERSION=8;
export function supportsBusiness(b:Building,retained=false){return !retained&&!b.anchor&&['office','shop','cafe','apartment'].includes(b.kind)&&Number.isFinite(b.width)&&b.width>=5&&b.width<=90&&Number.isFinite(b.depth)&&b.depth>=6&&b.depth<=80&&Number.isInteger(b.floors)&&b.floors>=1&&b.floors<=60&&Number.isFinite(b.progress)&&['foundation','structure','surface','finishing','open','renovation','demolition'].includes(b.stage);}
export function businessPool(b:Building,mode:ApartmentEra='mixed'):readonly BusinessRecipe[]{
 if(mode==='legacy'||!supportsBusiness(b))return [];
 const seed=b.appearance_seed;
 if(b.kind==='office'&&b.floors>=24&&b.width>=24&&b.depth>=20&&mode!=='older'&&(mode==='contemporary'||architecturalChoice(seed,386)%5!==0))return CORPORATE_SKYSCRAPERS;
 const family:BusinessFamily=b.kind==='apartment'?'mixed-use':b.kind==='office'?(b.floors>=3&&architecturalChoice(seed,301)%4===0?'mixed-use':'office'):b.floors>=3&&architecturalChoice(seed,302)%2===0?'mixed-use':'commercial';
 // Apartment ground-floor commerce is a deliberate minority; saves keep their existing capacity.
 if(b.kind==='apartment'&&(b.floors<4||architecturalChoice(seed,303)%5!==0))return [];
 return BUSINESS_RECIPES.filter(r=>r.family===family&&(mode==='mixed'||r.era===mode)&&
  (b.kind!=='apartment'||r.upper==='home')&&(b.kind!=='office'||r.upper==='office')&&
  (b.kind!=='cafe'||r.id!=='glass-showroom')&&
  (b.floors>=12?['curtain','podium','frame','ribbon','fins','slab'].includes(r.form):r.id!=='retail-residential-tower')&&
  (r.retailFloors<b.floors||r.family==='commercial')&&
  (r.form!=='podium'||b.width>=12&&b.depth>=12));
}
export function selectBusinessRecipe(b:Building,mode:ApartmentEra='mixed',retained=false):BusinessRecipe|undefined{
 if(retained)return;const pool=businessPool(b,mode);return pool[architecturalChoice(b.appearance_seed,304)%pool.length];
}
/** Stable local anti-repeat selection. Adding later IDs does not reshuffle older neighbours. */
export function resolveBusinessRecipes(buildings:readonly Building[],mode:ApartmentEra,retained:ReadonlySet<number>=new Set()){
 const result=new Map<number,BusinessRecipe>(),cells=new Map<string,Building[]>(),cell=48;
 for(const b of [...buildings].sort((a,c)=>a.id-c.id)){
  const first=selectBusinessRecipe(b,mode,retained.has(b.id));if(!first)continue;
  const pool=businessPool(b,mode),cx=Math.floor(b.x/cell),cz=Math.floor(b.z/cell),near:Building[]=[];
  for(let x=-1;x<=1;x++)for(let z=-1;z<=1;z++)near.push(...cells.get(`${cx+x}:${cz+z}`)??[]);
  const closest=near.filter(o=>Math.hypot(o.x-b.x,o.z-b.z)<cell).sort((a,c)=>Math.hypot(a.x-b.x,a.z-b.z)-Math.hypot(c.x-b.x,c.z-b.z)).slice(0,12);
  let best=Infinity,choice=first;
  for(let n=0;n<pool.length;n++){const candidate=pool[(pool.indexOf(first)+n)%pool.length],score=closest.reduce((sum,o)=>sum+(result.get(o.id)?.id===candidate.id?1/(1+Math.hypot(o.x-b.x,o.z-b.z)):0),0);if(score<best){best=score;choice=candidate;if(!score)break;}}
  result.set(b.id,choice);const key=`${cx}:${cz}`,list=cells.get(key)??[];list.push(b);cells.set(key,list);
 }return result;
}

/** Corporate/showroom facades keep restrained branding; media and tenant buildings host ads. */
export function businessCanHostAdvertising(recipe:BusinessRecipe){return recipe.district?recipe.district!=='ameyoko':!recipe.towerForm&&(recipe.era==='older'||['media','tenant','podium','department'].includes(recipe.form));}
