import {architecturalChoice} from './variation';
import * as THREE from 'three/webgpu';
import type {ArtMaterials} from '../materials';
import type {Building} from '../../types';
import type {BusinessRecipe} from './business-recipes';
import type {Kit} from './geometry-kit';
export type BusinessDetail='close'|'district'|'far';
/** The same envelope drives installed floors, construction frames and roof plant. */
export function businessEnvelope(b:Building,r:BusinessRecipe,floor:number){
 const width=b.width-1,depth=(b.depth-2.2)*(r.form==='slab'?.78:1),retail=Math.min(b.floors,r.retailFloors);
 const step=r.form==='screen'&&floor>=Math.ceil(b.floors*.6)?Math.min(.65,width*.1,depth*.1):r.form==='stepped'?Math.min(width*.22,depth*.22,2.2,Math.floor(floor/Math.max(2,Math.ceil(b.floors/3)))*.9):r.form==='podium'&&floor>=retail?Math.min(1.4,width*.15,depth*.15):0;
 return {fw:width-step*2,fd:depth-step*2,step};
}
function beam(kit:Kit,a:THREE.Vector3,b:THREE.Vector3,width:number,mat:THREE.Material){
 const delta=b.clone().sub(a),geometry=new THREE.BoxGeometry(width,delta.length(),width);
 geometry.applyQuaternion(new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0,1,0),delta.normalize()));
 geometry.translate(...a.clone().add(b).multiplyScalar(.5).toArray());kit.add(geometry,mat);
}
/** Reusable sloped fabric canopy; folds and valances are independent of the wall texture. */
export function fabricAwning(kit:Kit,art:ArtMaterials,x:number,y:number,z:number,width:number,tenant:number,detail:BusinessDetail){
 const shade=art.get(['#426c5c','#a4654f','#526c76','#aa783d','#944e59'][tenant%5]);
 const variant=tenant%3,depth=[.62,.78,.88][variant],slope=[.1,.2,.3][variant],edge=.1+depth;
 const geometry=new THREE.BoxGeometry(width,.09,depth);geometry.rotateX(slope);geometry.translate(x,y+.035,z+.1+depth/2);kit.add(geometry,shade);
 kit.box(x,y-.1,z+edge,width,variant===0?.1:.23,.07,shade);
 if(detail==='close'&&tenant%2===0)for(let i=0;i<Math.min(12,Math.floor(width/.25));i++)kit.box(x-width/2+(i+.5)*width/Math.min(12,Math.floor(width/.25)),y-.12,z+edge+.041,.035,.2,.014,art.get('#d5c8b0'));
}
/** Family-specific facade parts. Coarse silhouette pieces survive distant LOD. */
export function businessFacadeParts(kit:Kit,art:ArtMaterials,r:BusinessRecipe,m:{width:number;depth:number;y:number;floor:number;floors:number;detail:BusinessDetail}){
 const {width:w,depth:d,y,floor,detail}=m,front=d/2,metal=art.get('#303b40',.45),pale=art.get('#cecdc2'),stone=art.get('#d8cfbb');
 if(r.form==='fins'){
  const pitch=r.bay*(detail==='far'?2:1),count=Math.max(1,Math.floor(w/pitch));
  for(let i=0;i<=count;i++)for(const side of [-1,1])kit.box(-w/2+.18+i*(w-.36)/count,y+1.65,side*(front+.16),.13,3.15,.45,art.get('#897c69',.55));
  for(const side of [-1,1])kit.box(side*(w/2-.16),y+1.65,0,.26,3.15,d,metal);
 }
 if(r.form==='screen'){
  // A prefabricated lattice module wraps two elevations; no seed-specific image.
  const count=Math.max(1,Math.floor(w/(detail==='close'?1.15:detail==='district'?1.8:3.2))),pitch=(w-.3)/count;
  for(let i=0;i<count;i++){
   const x=-w/2+.15+i*pitch,z=front+.25;
   beam(kit,new THREE.Vector3(x,y+.27,z),new THREE.Vector3(x+pitch,y+2.95,z),.065,pale);
   beam(kit,new THREE.Vector3(x+pitch,y+.27,z),new THREE.Vector3(x,y+2.95,z),.065,pale);
  }
  kit.box(0,y+3.1,front+.24,w,.22,.18,pale);
  if(detail!=='far')for(let z=-d/2+.3;z<d/2-.5;z+=1.5){
   beam(kit,new THREE.Vector3(w/2+.22,y+.27,z),new THREE.Vector3(w/2+.22,y+2.95,Math.min(z+1.5,d/2-.2)),.065,pale);
   beam(kit,new THREE.Vector3(w/2+.22,y+2.95,z),new THREE.Vector3(w/2+.22,y+.27,Math.min(z+1.5,d/2-.2)),.065,pale);
  }
 }
 if(r.form==='department'){
  kit.box(0,y+3.05,front+.18,w+.12,.32,.38,stone);
  const count=Math.max(2,Math.floor(w/3.2));for(let i=0;i<=count;i++)kit.box(-w/2+.2+i*(w-.4)/count,y+1.55,front+.14,.22,2.85,.32,stone);
  if(floor===0)kit.box(0,y+2.75,front+.48,w,.22,.95,metal);
 }
 if(r.form==='slab'){
  kit.box(0,y+3.04,0,w+.12,.35,d+.12,pale);
  for(const side of [-1,1])kit.box(side*(w/2-.22),y+1.6,0,.44,3.15,d,stone);
 }
 if(r.form==='loft'&&detail!=='far'){
  for(const side of [-1,1]){kit.box(side*(w/2-.14),y+1.6,front+.08,.22,3.2,.3,metal);kit.box(0,y+.32,side*front,w,.2,.18,metal);}
 }
 if(r.form==='media'){
  for(const side of [-1,1])kit.box(side*(w/2-.15),y+1.6,front+.1,.28,3.2,.35,metal);
  kit.box(0,y+3.04,front+.12,w,.28,.3,metal);
 }
}
/** Finite roof-service grammar with equipment screens and a distinct mechanical crown. */
export function businessRoofServices(kit:Kit,art:ArtMaterials,r:BusinessRecipe,w:number,d:number,top:number,detail:BusinessDetail,seed=0){
 const variant=architecturalChoice(seed,350)%4,metal=art.get('#303b40',.45),roof=art.get('#585f5b'),pale=art.get('#cecdc2');
 const plantW=Math.min(w*.55,5),plantD=Math.min(d*.45,3),cx=w*.12,cz=-d*.12;
 kit.box(cx,top+.65,cz,plantW,.95,plantD,roof);
 if(detail!=='far')for(let i=0;i<3;i++){
  const x=cx-plantW*.3+i*plantW*.3;kit.cylinder(x,top+1.17,cz,Math.min(.33,plantW*.1),.08,metal,8);
  kit.box(x,top+1.25,cz,.045,.02,Math.min(.65,plantD*.7),pale);
 }
 if(r.era==='older'&&variant===0&&w>=5&&d>=4){
  const x=-w*.25,z=d*.2;for(const dx of [-.45,.45])kit.box(x+dx,top+.6,z,.09,1.2,.09,metal);
  kit.cylinder(x,top+1.8,z,.65,1.3,art.get('#d5c8b0'),10);
 }
 if(r.era==='contemporary'&&variant===1&&detail!=='far'&&w>=8&&d>=8)for(let i=0;i<2;i++){
  const panel=new THREE.BoxGeometry(2.1,.07,1.05);panel.rotateX(-.15);panel.translate(-w*.25+i*2.3,top+.4,d*.2);kit.add(panel,art.get('#40565f',.24));
 }
 if(r.era==='contemporary'){
  const h=r.form==='curtain'||r.form==='fins'?2.1:1.4;
  for(const side of [-1,1]){kit.box(cx+side*(plantW/2+.1),top+h/2,cz,.12,h,plantD+.35,metal);kit.box(cx,top+h/2,cz+side*(plantD/2+.1),plantW+.3,h,.12,metal);}
  if(detail==='close')for(let i=0;i<5;i++)kit.box(cx,top+.2+i*.24,cz+plantD/2+.17,plantW+.3,.06,.04,pale);
 }
}
