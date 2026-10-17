import type {Building} from '../../types';
import type {ArtMaterials} from '../materials';
import {supportsApartment,type ApartmentRecipeId,type ApartmentBuildOptions} from './recipes';
import {buildApartmentBuilding} from './index';
import {buildResidentialTower,supportsResidentialTower,type ResidentialTowerRecipe} from './residential-tower';
import {RESIDENTIAL_TOWER_IDS} from './tower-recipes';
import {architecturalChoice} from './variation';
import {type ApartmentEra} from './era-preference';
export {APARTMENT_ERAS,parseApartmentEra,type ApartmentEra} from './era-preference';
export type ApartmentArchitecture={kind:'legacy'}|{kind:'midrise';era:'older'|'contemporary';recipe:ApartmentRecipeId}|{kind:'tower';era:'contemporary';recipe:ResidentialTowerRecipe};
const older=['gallery','walkup','terrace','compact','brick'] as const,newer=['modern','urban','ceramic','limestone','courtyard'] as const;
/** Stable across occupancy, construction, LOD, style and world reloads. No simulated age mutation. */
export function selectApartmentArchitecture(b:Building,mode:ApartmentEra='mixed',retainedFootprint=false):ApartmentArchitecture{
 if(mode==='legacy'||retainedFootprint||b.kind!=='apartment')return {kind:'legacy'};
 const contemporary=mode==='contemporary'||mode==='mixed'&&architecturalChoice(b.appearance_seed,191)%2===0;
 if(supportsResidentialTower(b))return contemporary?{kind:'tower',era:'contemporary',recipe:RESIDENTIAL_TOWER_IDS[architecturalChoice(b.appearance_seed,192)%RESIDENTIAL_TOWER_IDS.length]}:{kind:'legacy'};
 if(!supportsApartment(b))return {kind:'legacy'};
 const pool=contemporary?newer:older;
 return {kind:'midrise',era:contemporary?'contemporary':'older',recipe:pool[architecturalChoice(b.appearance_seed,193)%pool.length]};
}
export function apartmentArchitectureKey(choice:ApartmentArchitecture){return choice.kind==='legacy'?'legacy':`${choice.kind}:${choice.recipe}`;}
export function buildSelectedApartment(b:Building,art:ArtMaterials,choice:ApartmentArchitecture,detail:ApartmentBuildOptions['detail']='district'){
 if(choice.kind==='legacy')return null;
 return choice.kind==='tower'?buildResidentialTower(b,art,{recipe:choice.recipe,detail}):buildApartmentBuilding(b,art,{recipe:choice.recipe,detail});
}

/** Deterministic local facade-repeat suppression, O(n log n + local neighbours).
 * Oldest IDs are assigned first so adding later infill does not restyle existing neighbours.
 * Selection remains appearance-only; no Building records are changed.
 */
export function resolveApartmentArchitectures(buildings:readonly Building[],mode:ApartmentEra='mixed',retained:ReadonlySet<number>=new Set<number>()){
 const result=new Map<number,ApartmentArchitecture>(),cells=new Map<string,Array<{b:Building;choice:ApartmentArchitecture}>>(),cell=48;
 for(const b of [...buildings].sort((a,b)=>a.id-b.id)){
  let choice=selectApartmentArchitecture(b,mode,retained.has(b.id));
  if(choice.kind==='legacy'){result.set(b.id,choice);continue;}
  const x=Math.floor(b.x/cell),z=Math.floor(b.z/cell),neighbours:Array<{b:Building;choice:ApartmentArchitecture}>=[];
  for(let dx=-1;dx<=1;dx++)for(let dz=-1;dz<=1;dz++)neighbours.push(...cells.get(`${x+dx}:${z+dz}`)??[]);
  const near=neighbours.filter(n=>Math.hypot(n.b.x-b.x,n.b.z-b.z)<cell&&n.choice.kind===choice.kind).sort((a,c)=>Math.hypot(a.b.x-b.x,a.b.z-b.z)-Math.hypot(c.b.x-b.x,c.b.z-b.z)).slice(0,12);
  const pool:readonly string[]=choice.kind==='tower'?RESIDENTIAL_TOWER_IDS:choice.era==='contemporary'?newer:older;
  const first=pool.indexOf(choice.recipe);let best=Infinity,recipe=choice.recipe;
  for(let i=0;i<pool.length;i++){
   const candidate=pool[(first+i)%pool.length],score=near.reduce((n,v)=>n+(v.choice.kind!=='legacy'&&v.choice.recipe===candidate?1/(1+Math.hypot(v.b.x-b.x,v.b.z-b.z)):0),0);
   if(score<best){best=score;recipe=candidate as typeof recipe;if(score===0)break;}
  }
  choice={...choice,recipe} as ApartmentArchitecture;result.set(b.id,choice);
  const key=`${x}:${z}`,list=cells.get(key)??[];list.push({b,choice});cells.set(key,list);
 }
 return result;
}
