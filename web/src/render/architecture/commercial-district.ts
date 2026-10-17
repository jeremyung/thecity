import type {Building} from '../../types';
import type {BusinessRecipe} from './business-recipes';
import {architecturalChoice} from './variation';
export const COMMERCIAL_DISTRICTS=[['standard','Standard city'],['tokyo-mix','Tokyo district mix'],['shibuya','Shibuya · fashion & media'],['roppongi','Roppongi · dining & nightlife'],['akihabara','Akihabara · electronics & games'],['shinjuku','Shinjuku · entertainment'],['ameyoko','Ameyoko · market street']] as const;
export type CommercialDistrict=typeof COMMERCIAL_DISTRICTS[number][0];
export type DenseDistrict=Exclude<CommercialDistrict,'standard'|'tokyo-mix'>;
export function parseCommercialDistrict(value:unknown):CommercialDistrict{return COMMERCIAL_DISTRICTS.some(([id])=>id===value)?value as CommercialDistrict:'standard';}
export function districtAt(b:Building,mode:CommercialDistrict,worldSeed=0):DenseDistrict|undefined{
 if(mode==='standard')return;
 if(mode!=='tokyo-mix')return mode;
 // Shared 160 m cells make coherent neighbourhoods, independent of occupancy and time.
 return (['shibuya','roppongi','akihabara','shinjuku','ameyoko'] as const)[architecturalChoice(Math.imul(Math.floor(b.x/160),73856093)^Math.imul(Math.floor(b.z/160),19349663)^worldSeed,520)%5];
}
/** Presentation grammar only; authoritative parcel, height, use and capacity stay intact. */
export function applyCommercialDistrict(b:Building,r:BusinessRecipe,mode:CommercialDistrict,worldSeed=0):BusinessRecipe{
 const district=districtAt(b,mode,worldSeed);
 if(!district||r.mediaLandmark||r.towerForm||b.anchor||b.floors>12||b.width>36||b.depth>40||!['shop','cafe','office'].includes(b.kind)||b.kind==='office'&&(b.floors<3||b.floors>10))return r;
 const forms:Record<DenseDistrict,readonly BusinessRecipe['form'][]>= {shibuya:['media','tenant','frame'],roppongi:['tenant','ribbon','frame'],akihabara:['media','tenant','department'],shinjuku:['tenant','media','frame'],ameyoko:['loft','arcade','shop-house']};
 const market=district==='ameyoko',nightlife=district==='roppongi'||district==='shinjuku';
 return {...r,id:`${r.id}/${district}`,district,label:`${district} · ${r.label}`,form:forms[district][architecturalChoice(b.appearance_seed,535)%3],bay:market?2.4:2.8,
  retailFloors:Math.min(b.floors,market?1:district==='akihabara'?3:2),awning:market||district==='shibuya',
  wall:r.era==='older'?'#c9bdad':nightlife?'#626762':'#d9dfdf'};
}
