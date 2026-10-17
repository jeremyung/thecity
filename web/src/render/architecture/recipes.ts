import {architecturalChoice} from './variation';
import type {Building} from '../../types';

export const APARTMENT_RECIPE_VERSION=11;
export const APARTMENT_RECIPE_IDS=['gallery','walkup','terrace','compact','brick','modern','urban','ceramic','limestone','courtyard'] as const;
export type ApartmentRecipeId=typeof APARTMENT_RECIPE_IDS[number];
export interface ApartmentBuildOptions {recipe?:ApartmentRecipeId|'auto';detail?:'close'|'district'|'far'}
export interface ApartmentRecipe {
 id:ApartmentRecipeId;label:string;bayWidth:number;guard:'solid'|'rail'|'glass';frontCore:boolean;
 upperSetback:number;balconyDepth:number;canopyWidth:number;
 wallRoughness?:number;circulation?:'internal';accent?:'frames'|'fins';
 balcony?:'individual'|'juliet';finish?:'tile'|'brick';
 wallTints:readonly string[];concreteTint:string;roofTint:string;guardTint:string;
}
export const APARTMENT_RECIPES:Readonly<Record<ApartmentRecipeId,ApartmentRecipe>>={
 gallery:{id:'gallery',label:'Gallery apartment',bayWidth:3.15,guard:'solid',frontCore:true,upperSetback:0,balconyDepth:1.26,canopyWidth:3.6,wallTints:['#e2dfd2','#d4d6ce','#d9d0be'],concreteTint:'#c4c0b4',roofTint:'#757b74',guardTint:'#b7b2a3'},
 walkup:{id:'walkup',label:'Open-rail walk-up',bayWidth:2.8,guard:'rail',frontCore:false,upperSetback:0,balconyDepth:1.26,canopyWidth:2.4,wallTints:['#c9bdad','#b7b7a9','#bfb9b0'],concreteTint:'#b9b7ac',roofTint:'#666f68',guardTint:'#b7b2a3'},
 terrace:{id:'terrace',label:'Setback apartment',bayWidth:3.3,guard:'solid',frontCore:false,upperSetback:.75,balconyDepth:1.26,canopyWidth:3.1,wallTints:['#d5d8ce','#c8cfc6','#ded6c8'],concreteTint:'#cecdc2',roofTint:'#8c9386',guardTint:'#c3bcac'},
 compact:{id:'compact',label:'Compact Juliet windows',bayWidth:2.6,guard:'rail',frontCore:false,upperSetback:0,balconyDepth:.3,canopyWidth:2.1,balcony:'juliet',wallTints:['#dad8cf','#c0c4c0'],concreteTint:'#b9b7ac',roofTint:'#666f68',guardTint:'#b7b2a3'},
 brick:{id:'brick',label:'Brick individual balconies',bayWidth:3.5,guard:'solid',frontCore:false,upperSetback:0,balconyDepth:1.26,canopyWidth:2.8,balcony:'individual',finish:'brick',wallTints:['#986b56','#aa7c61'],concreteTint:'#b9b7ac',roofTint:'#666f68',guardTint:'#986b56'},
 modern:{id:'modern',label:'Contemporary glass and tile',bayWidth:3.7,guard:'glass',frontCore:false,upperSetback:.45,balconyDepth:1.26,canopyWidth:3.2,finish:'tile',circulation:'internal',accent:'frames',wallTints:['#e7e8e3','#d9dfdf'],concreteTint:'#cecdc2',roofTint:'#757b74',guardTint:'#c3bcac'},
 urban:{id:'urban',label:'Contemporary screened residences',wallRoughness:.55,bayWidth:3.3,guard:'glass',frontCore:false,upperSetback:0,balconyDepth:1.05,canopyWidth:3.2,balcony:'individual',circulation:'internal',accent:'fins',wallTints:['#a3947e','#897c69'],concreteTint:'#cecdc2',roofTint:'#666f68',guardTint:'#c3bcac'},
 ceramic:{id:'ceramic',label:'White ceramic framed residences',bayWidth:4.1,guard:'glass',frontCore:true,upperSetback:.8,balconyDepth:.95,canopyWidth:3.8,finish:'tile',circulation:'internal',accent:'frames',balcony:'individual',wallTints:['#ece9df','#e1e2da'],concreteTint:'#d5d4c9',roofTint:'#747b75',guardTint:'#c9c9bf'},
 limestone:{id:'limestone',label:'Honed stone screened residences',bayWidth:3.9,guard:'rail',frontCore:false,upperSetback:1.1,balconyDepth:1.15,canopyWidth:3.7,circulation:'internal',accent:'fins',wallTints:['#d8cfbb','#c8bca5'],concreteTint:'#c2beaf',roofTint:'#72786c',guardTint:'#b5ac98'},
 courtyard:{id:'courtyard',label:'Charcoal recessed loggias',bayWidth:3.05,guard:'solid',frontCore:true,upperSetback:.55,balconyDepth:1.2,canopyWidth:2.6,finish:'tile',circulation:'internal',accent:'frames',balcony:'individual',wallTints:['#626762','#747771'],concreteTint:'#a5aaa3',roofTint:'#585f5b',guardTint:'#626762'},
};
/** Stable recipe selection; renderer parameters never mutate authoritative Building state. */
export function apartmentRecipe(b:Pick<Building,'appearance_seed'>,id:ApartmentRecipeId|'auto'='auto'):ApartmentRecipe{
 return APARTMENT_RECIPES[id==='auto'?APARTMENT_RECIPE_IDS[architecturalChoice(b.appearance_seed,101)%APARTMENT_RECIPE_IDS.length]:id];
}
export function supportsApartment(b:Building):boolean{
 return b.kind==='apartment'&&!b.anchor&&Number.isFinite(b.width)&&Number.isFinite(b.depth)
  &&b.width>=5&&b.depth>=8&&Number.isInteger(b.floors)&&b.floors>=4&&b.floors<=11
  &&Number.isFinite(b.progress)&&['foundation','structure','open','renovation','surface','finishing','demolition'].includes(b.stage);
}

export {apartmentPhase} from './lifecycle';
