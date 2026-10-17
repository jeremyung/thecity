import * as THREE from 'three/webgpu';
import type {Kit} from './geometry-kit';
import {architecturalChoice} from './variation';

/** A restrained household profile; empty balconies are deliberately common. */
export function householdDressing(seed:number,floor:number,bay:number){
 const value=architecturalChoice(seed,61,floor*131+bay);
 return {laundry:floor%3===architecturalChoice(seed,63,bay)%3&&value%2===0,items:1+((value>>>8)%4),offset:((value>>>13)%7-3)*.07,
  storage:(value>>>18)%9===0,plants:(value>>>23)%5===0,variant:value>>>3};
}
export function hangingLaundry(kit:Kit,x:number,y:number,z:number,profile:ReturnType<typeof householdDressing>,fabric:THREE.Material[],metal:THREE.Material){
 const width=1.12;
 kit.box(x,y,z,width,.025,.025,metal);
 for(const dx of [-width/2,width/2])kit.box(x+dx,y+.09,z-.15,.025,.2,.32,metal);
 for(let j=0;j<profile.items;j++){
  const v=architecturalChoice(profile.variant,62,j),cx=x-width*.4+j*width*.8/Math.max(1,profile.items-1),cloth=fabric[v%fabric.length];
  const kind=(v>>>4)%3,length=.32+((v>>>9)%6)*.065;
  // Towels, shirts with sleeves, and split trouser legs have different silhouettes.
  if(kind===0)kit.box(cx,y-length/2,z,.22+((v>>>16)%3)*.055,length,.025,cloth);
  if(kind===1){
   kit.box(cx,y-.19,z,.27,.38,.035,cloth);
   kit.box(cx-.18,y-.075,z,.13,.15,.035,cloth);kit.box(cx+.18,y-.075,z,.13,.15,.035,cloth);
  }
  if(kind===2)for(const dx of [-.072,.072])kit.box(cx+dx,y-length/2,z,.11,length,.03,cloth);
  for(const dx of [-.07,.07])kit.box(cx+dx,y+.015,z,.015,.045,.04,metal);
 }
}
/** Ground-level cycle parking, with a narrow depth footprint and shared materials. */
export function parkedBicycle(kit:Kit,x:number,y:number,z:number,metal:THREE.Material,rubber:THREE.Material,frame:THREE.Material){
 const bar=(a:number[],b:number[],r:number,mat:THREE.Material)=>{
  const start=new THREE.Vector3(...a),end=new THREE.Vector3(...b),delta=end.clone().sub(start);
  const geo=new THREE.CylinderGeometry(r,r,delta.length(),5);
  geo.applyQuaternion(new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0,1,0),delta.normalize()));geo.translate(...start.add(end).multiplyScalar(.5).toArray());kit.add(geo,mat);
 };
 kit.withOffset(x,y,z,()=>{
  for(const wx of [-.53,.53]){
   const wheel=new THREE.TorusGeometry(.31,.025,5,16);wheel.translate(wx,.33,0);kit.add(wheel,rubber);
   for(let k=0;k<6;k++){const a=k*Math.PI/3;bar([wx,.33,0],[wx+Math.cos(a)*.29,.33+Math.sin(a)*.29,0],.006,metal);}
  }
  const a=[-.53,.33,0],b=[-.12,.35,0],c=[-.24,.85,0],d=[.32,.85,0],e=[.53,.33,0];
  for(const [p,q] of [[a,b],[a,c],[b,c],[b,d],[c,d],[d,e]])bar(p,q,.023,frame);
  bar(c,[-.27,.97,0],.022,metal);kit.box(-.27,.98,0,.29,.055,.15,rubber);
  bar(d,[.29,1.04,0],.02,metal);bar([.29,1.04,-.2],[.29,1.04,.2],.018,metal);
  kit.box(.48,.87,0,.31,.24,.28,metal);kit.box(.48,.91,0,.27,.17,.24,rubber);
  bar([-.12,.35,0],[-.12,.35,.15],.025,metal);kit.box(-.12,.35,.17,.14,.04,.08,rubber);
 });
}
