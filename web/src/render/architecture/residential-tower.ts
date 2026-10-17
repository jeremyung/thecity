import {windowLived} from '../building-surfaces';
import {apartmentConstruction,addDemolitionSite} from './lifecycle';
import * as THREE from 'three/webgpu';
import type {Building} from '../../types';
import type {ArtMaterials} from '../materials';
import {Kit} from './geometry-kit';
import {architecturalChoice} from './variation';
import {applyBuildingWeathering} from './character';
import type {ApartmentBuildOptions} from './recipes';
import {RESIDENTIAL_TOWERS,RESIDENTIAL_TOWER_IDS,type ResidentialTowerRecipe} from './tower-recipes';
export {RESIDENTIAL_TOWERS,type ResidentialTowerRecipe} from './tower-recipes';
export interface ResidentialTowerOptions {recipe?:ResidentialTowerRecipe;detail?:ApartmentBuildOptions['detail']}
type Point=[number,number];
/** Chamfered corners are true geometry, shared by slabs, glazing and guard rails. */
function outline(w:number,d:number,cut:number):Point[]{const x=w/2,z=d/2,c=Math.min(cut,w*.15,d*.15);return [[-x+c,-z],[x-c,-z],[x,-z+c],[x,z-c],[x-c,z],[-x+c,z],[-x,z-c],[-x,-z+c]];}
function prism(kit:Kit,points:Point[],y:number,h:number,material:THREE.Material){
 const values:number[]=[];
 const tri=(a:number[],b:number[],c:number[])=>values.push(...a,...b,...c);
 for(let i=0;i<points.length;i++){
  const a=points[i],b=points[(i+1)%points.length],al=[a[0],y,a[1]],au=[a[0],y+h,a[1]],bl=[b[0],y,b[1]],bu=[b[0],y+h,b[1]];
  tri(al,au,bu);tri(al,bu,bl);tri([0,y+h,0],bu,au);tri([0,y,0],al,bl);
 }
 const geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.Float32BufferAttribute(values,3));geometry.setAttribute('uv',new THREE.Float32BufferAttribute(new Float32Array(values.length/3*2),2));geometry.setIndex(Array.from({length:values.length/3},(_,i)=>i));geometry.computeVertexNormals();kit.add(geometry,material);
}
function edgeBox(kit:Kit,a:Point,b:Point,y:number,h:number,thickness:number,material:THREE.Material,offset=0){
 const dx=b[0]-a[0],dz=b[1]-a[1],length=Math.hypot(dx,dz);
 kit.box((a[0]+b[0])/2+dz/length*offset,y,(a[1]+b[1])/2-dx/length*offset,length,h,thickness,material,-Math.atan2(dz,dx));
}
export function supportsResidentialTower(b:Building){return b.kind==='apartment'&&!b.anchor&&Number.isFinite(b.width)&&Number.isFinite(b.depth)&&b.width>=14&&b.depth>=14&&Number.isInteger(b.floors)&&b.floors>=12&&b.floors<=60&&Number.isFinite(b.progress)&&['foundation','structure','surface','finishing','open','renovation','demolition'].includes(b.stage);}
/** Five composable residential tower recipes. Local-space geometry, shared palette, no engine mutation. */
export function buildResidentialTower(b:Building,art:ArtMaterials,options:ResidentialTowerOptions={}){
 if(!supportsResidentialTower(b))return null;
 const recipe=options.recipe??RESIDENTIAL_TOWER_IDS[architecturalChoice(b.appearance_seed,151)%RESIDENTIAL_TOWER_IDS.length],design=RESIDENTIAL_TOWERS[recipe],dark=recipe==='ribbon'||recipe==='terrace',detail=options.detail??'district';
 const kit=new Kit();kit.windowContext={seed:b.appearance_seed,lived:windowLived(b)};
 const base=.58,fh=3.25,podium=design.podium,w=b.width-2,d=b.depth-2;
 const seed=b.appearance_seed>>>0,profile=architecturalChoice(seed,155)%3,shaftW=w*(dark?.82+profile*.025:.85+profile*.018),shaftD=d*(dark?.86-profile*.015:.88-profile*.012),cut=(recipe==='lantern'?2.6:dark?1.3:.9)+profile*.2;
 const {complete,installed,framed}=apartmentConstruction(b);
 const wall=art.get(design.wall),slab=art.get(design.slab),metal=art.get(design.metal,.45),shadow=art.get('#28383c'),roof=art.get('#666f68');
 const glass=art.get('#59717a',.24),panes=[art.window('home')];
 const stone=art.get(design.stone),warm=art.get('#c8aa7a',.6,b.stage==='open'&&b.occupancy>0,'home'),leaves=art.get('#536a4c');
 const outer=outline(shaftW,shaftD,cut),podiumOutline=outline(w,d,1.2);
 prism(kit,podiumOutline,.03,.18,stone);
 if(installed>0){
  const lobbyHeight=Math.min(podium,installed)*fh;
  prism(kit,outline(w-1.1,d-1.1,1.1),base,lobbyHeight,shadow);
  // Glazed double-height lobby with a warm room backing and slender vertical mullions.
  for(let i=0;i<podiumOutline.length;i++){
   const a=podiumOutline[i],c=podiumOutline[(i+1)%podiumOutline.length],dx=c[0]-a[0],dz=c[1]-a[1],len=Math.hypot(dx,dz),count=Math.max(1,Math.ceil(len/2.2));
   edgeBox(kit,a,c,base+lobbyHeight/2,lobbyHeight-.18,.055,glass,-.18);
   for(let j=0;j<=count;j++)kit.box(a[0]+dx*j/count,base+lobbyHeight/2,a[1]+dz*j/count,.075,lobbyHeight,.075,metal);
  }
  kit.box(0,base+Math.min(2.6,lobbyHeight/2),d/2-.48,w*.55,Math.min(4.8,lobbyHeight-.2),.08,warm);
  // The engine walking endpoint is front-centre for every seed and rotation.
  const entry=0,front=d/2;
  kit.box(entry,base+1.4,front-.08,2.15,2.8,.07,shadow);
  kit.box(entry,base+1.4,front-.02,1.93,2.6,.035,glass);
  kit.box(entry,base+1.4,front+.025,.045,2.6,.045,metal);
  kit.box(entry,base+3.05,front+.35,Math.min(5,w*.48),.09,.95,metal);
  kit.box(entry,.35,front+.29,3.05,.25,.9,stone);
  for(const x of [-w/2+.85,w/2-.85]){
   kit.box(x,.42,front-.35,1.25,.5,.65,stone);
   for(let j=0;j<4;j++){const leaf=new THREE.IcosahedronGeometry(.31,0);leaf.setIndex(Array.from({length:leaf.getAttribute('position').count},(_,i)=>i));leaf.scale(1,1.4,1);leaf.translate(x+(j%2-.5)*.45,.95+Math.floor(j/2)*.2,front-.35);kit.add(leaf,leaves);}
  }
  if(installed>=podium)prism(kit,podiumOutline,base+podium*fh-.12,.22,slab);
 }
 for(let f=podium;f<installed;f++){
  const y=base+f*fh,crown=f>=b.floors-2,taper=recipe==='terrace'?(f>=b.floors*.8?.77:f>=b.floors*.6?.88:1):crown?(dark?.91:.95):1;
  const ow=shaftW*taper,od=shaftD*taper,balcony=recipe==='lantern'?.48:dark?1.25+profile*.12:1.18+profile*.08,inner=outline(ow-2*balcony,od-2*balcony,Math.max(.35,cut-.5)),ring=outline(ow,od,cut);
  prism(kit,inner,y+.12,fh-.14,shadow);
  prism(kit,ring,y,.2,slab);
  if(dark)prism(kit,ring,y+.2,.13,metal);
  for(let i=0;i<ring.length;i++){
   const a=ring[i],c=ring[(i+1)%ring.length];
   if(recipe!=='lantern'||i%2===0&&f%5<2){edgeBox(kit,a,c,y+.79,.94,.045,glass,-.1);edgeBox(kit,a,c,y+1.29,.045,.065,metal,-.1);}
   const ia=inner[i],ic=inner[(i+1)%inner.length],dx=ic[0]-ia[0],dz=ic[1]-ia[1],length=Math.hypot(dx,dz),bays=Math.max(1,Math.floor(length/(detail==='far'?6:3)));
   for(let j=0;j<bays;j++){
    const aa:Point=[ia[0]+dx*j/bays,ia[1]+dz*j/bays],cc:Point=[ia[0]+dx*(j+1)/bays,ia[1]+dz*(j+1)/bays];
    const v=architecturalChoice(seed,153,f*113+i*13+j);
    edgeBox(kit,aa,cc,y+1.65,2.52,.045,panes[v%panes.length],.025);
    if(detail!=='far'){
     kit.box(aa[0],y+1.65,aa[1],.07,2.7,.07,metal);
     if(detail==='close')edgeBox(kit,aa,cc,y+1.63,.035,.065,metal,.06);
    }
   }
   // Guard posts and privacy dividers follow unit boundaries, not random floor clutter.
   if(detail!=='far')for(let j=0;j<=bays;j++){
    const t=j/bays,x=a[0]+(c[0]-a[0])*t,z=a[1]+(c[1]-a[1])*t;
    kit.box(x,y+.8,z,.045,.99,.045,metal);
    if(i%2===0&&j>0&&j<bays&&j%2===0){
     const inside:Point=[ia[0]+dx*t,ia[1]+dz*t];edgeBox(kit,inside,[x,z],y+1.19,1.82,.045,wall);
    }
   }
  }
  // Light tower: full-height paired piers and selective vertical facade divisions.
  if(recipe==='frame')for(const x of [-ow/2+1.2,ow/2-1.2])for(const z of [-od/2+.22,od/2-.22])kit.box(x,y+fh/2,z,.5,fh,.5,wall);
  if(recipe==='frame')for(const z of [-od/2+.2,od/2-.2])kit.box(0,y+fh/2,z,.3,fh,.35,wall);
  if(recipe==='fins'){
   for(const z of [-od/2+.18,od/2-.18])for(let x=-ow/2+1.4;x<ow/2-.8;x+=2.6)kit.box(x,y+fh/2,z,.19,fh,.65,wall);
   for(const x of [-ow/2+.18,ow/2-.18])for(let z=-od/2+1.4;z<od/2-.8;z+=2.6)kit.box(x,y+fh/2,z,.65,fh,.19,wall);
  }
  if(recipe==='lantern'){
   for(const x of [-ow/2+.5,ow/2-.5])for(const z of [-od/2+.5,od/2-.5])kit.box(x,y+fh/2,z,.18,fh,.18,metal);
   if(f%5===0)prism(kit,ring,y+.22,.28,wall);
   for(const z of [-od/2+.15,od/2-.15])kit.box(0,y+fh/2,z,.42,fh,.52,wall);
   for(const x of [-ow/2+.15,ow/2-.15])kit.box(x,y+fh/2,0,.52,fh,.42,wall);
  }
  if(recipe==='terrace'&&f>podium&&(f===Math.ceil(b.floors*.6)||f===Math.ceil(b.floors*.8))){
   for(const x of [-ow*.3,ow*.3]){kit.box(x,y+.4,od/2-.55,1.4,.55,.5,wall);kit.box(x,y+.8,od/2-.55,1.25,.36,.38,leaves);}
  }
  // One restrained sky-terrace break rather than an identical extruded shaft.
  if(dark&&f===b.floors-5-profile){prism(kit,outline(ow+.9,od+.9,cut),y-.04,.22,slab);}
 }
 if(!complete)for(let f=installed;f<framed;f++){
  const y=base+f*fh,p=f<podium?podiumOutline:outer;
  prism(kit,p,y,.14,slab);
  for(const x of [-shaftW/2+1.4,shaftW/2-1.4])for(const z of [-shaftD/2+1.4,shaftD/2-1.4])kit.box(x,y+fh/2,z,.32,fh,.32,wall);
 }
 if(complete){
  const top=base+b.floors*fh,cw=shaftW*(recipe==='terrace'?.77:dark?.91:.95),cd=shaftD*(recipe==='terrace'?.77:dark?.91:.95);
  prism(kit,outline(cw,cd,cut),top,.23,slab);
  prism(kit,outline(cw*.62,cd*.62,.6),top+.23,1.9,roof);
  for(const z of [-cd*.36,cd*.36]){
   kit.box(0,top+1.2,z,cw*.78,1.65,.09,metal);
   if(detail!=='far')for(let x=-cw*.36;x<cw*.38;x+=.55)kit.box(x,top+1.2,z,.04,1.8,.18,wall);
  }
  if(recipe==='fins'||recipe==='lantern')for(const x of [-cw*.42,cw*.42])kit.box(x,top+1.6,0,.22,2.8,cd*.8,wall);
  for(let i=0;i<3;i++)kit.box(-cw*.22+i*cw*.2,top+.48,0,cw*.13,.5,cd*.2,metal);
 }
 if(b.stage==='demolition')addDemolitionSite(kit,b,art);
 const group=kit.finish();const metadata={recipe,label:RESIDENTIAL_TOWERS[recipe].label,detail,width:b.width,depth:b.depth,floors:b.floors,installedFloors:installed,framedFloors:framed,podiumFloors:podium,roofInstalled:complete,entryX:0,seed,shaftWidth:shaftW,shaftDepth:shaftD,profile};
 group.userData={buildingId:b.id,residentialTower:metadata};group.traverse(o=>o.userData.buildingId=b.id);
 applyBuildingWeathering(group,{profile:'fresh',strength:.025,phase:seed%997});return group;
}
