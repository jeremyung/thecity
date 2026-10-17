import * as THREE from 'three/webgpu';
import {color,positionWorld,mx_noise_float,vec3,smoothstep} from 'three/tsl';
import type {ArtMaterials} from '../materials';
import type {Kit} from './geometry-kit';
import {architecturalChoice} from './variation';
export function retailVisionGlass(art:ArtMaterials){return art.shared('flagship-retail-glass',()=>{
 const anime=new THREE.MeshToonNodeMaterial({color:'#bdccc9',gradientMap:art.toonRamp,transparent:true,opacity:.18,depthWrite:false});
 const miniature=new THREE.MeshPhysicalNodeMaterial({color:'#adc1c1',transparent:true,opacity:.24,depthWrite:false,roughness:.14,metalness:0,ior:1.48,envMapIntensity:.65});
 for(const m of [anime,miniature]){m.userData.corporateGlass=true;m.userData.retailVisionGlass=true;m.userData.reflectionSurface={version:1,role:'vision-glass'};}
 return {anime,miniature};
});}
/** Small coherent tenant modules in facade-local metres. Negative Z is inside. */
export function addRetailInterior(kit:Kit,art:ArtMaterials,m:{x:number;y:number;z:number;angle:number;width:number;seed:number;floor:number;kind?:number}){
 const tx=Math.cos(m.angle),tz=-Math.sin(m.angle),nx=Math.sin(m.angle),nz=Math.cos(m.angle);
 const box=(x:number,y:number,z:number,w:number,h:number,d:number,mat:THREE.Material)=>kit.box(m.x+tx*x+nx*z,m.y+y,m.z+tz*x+nz*z,w,h,d,mat,m.angle);
 const wood=art.get('#8c6e51'),cream=art.get('#d7ceba'),steel=art.get('#39474b',.45),fabric=art.get('#806f59');
 const kind=m.kind??architecturalChoice(m.seed,810,m.floor)%3,pw=Math.max(.8,Math.min(2.8,m.width-.2));
 // A visible ceiling and warm luminaire provide depth without another light object.
 box(0,2.91,-1.2,pw,.08,2.6,cream);box(0,2.85,-1.15,pw*.65,.025,.12,art.get('#ead8b5',.5,true,'shop'));
 if(kind===0){
  box(0,.42,-1.25,pw*.7,.8,.7,wood);box(0,.85,-1.25,pw*.78,.07,.85,cream);
  for(let i=0;i<3;i++){const x=(i-1)*pw*.2;box(x,1.02,-1.25,pw*.13,.25,.32,art.get(['#c4b092','#637d7b','#a47662'][i]));}
  box(0,1.45,-2.4,pw,.09,.45,wood);box(0,.82,-2.4,pw,.09,.45,wood);
 }else if(kind===1){
  box(0,.82,-1.4,pw*.6,.08,.7,wood);box(0,.42,-1.4,.08,.8,.08,steel);
  for(const side of [-1,1]){box(side*pw*.4,.48,-1.4,.4,.08,.42,fabric);box(side*pw*.4,.75,-1.65,.4,.55,.06,fabric);box(side*pw*.4,.24,-1.4,.055,.45,.055,steel);}
  box(-.1,.91,-1.4,.12,.13,.12,cream);box(.2,.89,-1.3,.15,.05,.18,cream);
 }else{
  for(let shelf=0;shelf<4;shelf++){
   const y=.45+shelf*.48;box(0,y,-1.8,pw,.065,.4,wood);
   for(let i=0;i<5;i++)box((i-2)*pw*.17,y+.18,-1.8,pw*.12,.25+(i%3)*.045,.24,art.get(['#a76f55','#576e76','#c2b08b','#747d5c'][i%4]));
  }
  for(const side of [-1,1])box(side*pw*.49,1.2,-1.8,.065,2.1,.45,steel);
 }
}
/** Two escalator silhouettes behind the main entrance, separate from circulation space. */
export function addFlagshipAtrium(kit:Kit,art:ArtMaterials,front:number,base:number,xOffset=0){
 const metal=art.get('#39474b',.45),tread=art.get('#7d8989');
 for(const side of [-1,1]){
  const x=xOffset+side*1.05;
  const body=new THREE.BoxGeometry(1.45,.2,5.2).rotateX(-.675).translate(x,base+1.625,front-4);kit.add(body,tread);
  for(const edge of [-1,1]){const rail=new THREE.BoxGeometry(.06,.6,5.2).rotateX(-.675).translate(x+edge*.7,base+2.05,front-4);kit.add(rail,metal);}
  for(let i=0;i<12;i++)kit.box(x,base+.1+i*(3.25/11),front-6.03+i*(4.06/11),1.25,.07,.35,metal);
 }
}

export function flagshipCladding(art:ArtMaterials){return art.shared('flagship-aluminium',()=>{
 const grain=mx_noise_float(positionWorld.mul(vec3(.7,.16,.7))),base=smoothstep(.2,1.4,positionWorld.y).oneMinus();
 const anime=new THREE.MeshToonNodeMaterial({color:'#b4bcb9',gradientMap:art.toonRamp}),miniature=new THREE.MeshStandardNodeMaterial({color:'#b4bcb9',roughness:.5,metalness:.12});
 for(const m of [anime,miniature]){m.colorNode=color('#b4bcb9').mul(grain.mul(.016).add(.99)).mul(base.mul(grain.add(1).mul(.018)).oneMinus());}
 miniature.roughnessNode=grain.mul(.025).add(.5);return {anime,miniature};
});}
