import {flagshipCampaignSurface,flagshipPosterSurface,campaignCell} from './media-campaigns';
import {retailVisionGlass,addRetailInterior,addFlagshipAtrium,flagshipCladding} from './flagship-interiors';
import * as THREE from 'three/webgpu';
import type {Building} from '../../types';
import type {ArtMaterials} from '../materials';
import type {BusinessBuildOptions} from './business';
import {Kit} from './geometry-kit';
import {corporateOutline,corporateGlassNormal} from './corporate-skyscraper';
import {apartmentConstruction,addDemolitionSite} from './lifecycle';
import {paneSeed,tagWindowPane,windowMaterials,windowLived} from '../building-surfaces';
import {mediaArtworkSurface,streetSignSurface,billboardCell} from '../signage';
import {architecturalChoice} from './variation';
import {addStorefrontSign} from './business-signs';
type Point={x:number;z:number};
/** Rounded front including both corner returns, ordered left to right. */
export function curvedMediaFront(w:number,d:number,r:number,segments=12):Point[]{
 const points:Point[]=[];
 for(const side of [-1,1])for(let i=0;i<=segments;i++){
  const a=(side<0?Math.PI-i/segments*Math.PI/2:Math.PI/2-i/segments*Math.PI/2);
  points.push({x:side*(w/2-r)+Math.cos(a)*r,z:d/2-r+Math.sin(a)*r});
 }return points;
}
/** Continuous panorama UVs and analytic normals around a rounded building front. */
export function curvedMediaRibbon(w:number,d:number,r:number,h:number,segments=12,offset=.04){
 const points=curvedMediaFront(w,d,r,segments),lengths=[0];
 for(let i=1;i<points.length;i++)lengths.push(lengths[i-1]+Math.hypot(points[i].x-points[i-1].x,points[i].z-points[i-1].z));
 const position:number[]=[],normal:number[]=[],uv:number[]=[],indices:number[]=[];
 points.forEach((p,i)=>{const n=corporateGlassNormal(p.x,p.z,w,d,r);for(const y of [0,h]){position.push(p.x+n.x*offset,y,p.z+n.z*offset);normal.push(n.x,0,n.z);uv.push(lengths[i]/lengths.at(-1)!,y/h);}if(i<points.length-1){const a=i*2;indices.push(a,a+2,a+1,a+2,a+3,a+1);}});
 const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.Float32BufferAttribute(position,3));g.setAttribute('normal',new THREE.Float32BufferAttribute(normal,3));g.setAttribute('uv',new THREE.Float32BufferAttribute(uv,2));g.setIndex(indices);return g;
}
export function buildCurvedMediaLandmark(b:Building,art:ArtMaterials,options:BusinessBuildOptions):THREE.Group{
 const kit=new Kit(),detail=options.detail??'district',mode=options.displays??'animated',segments=detail==='close'?12:detail==='district'?8:4;
 const {installed,framed,complete}=apartmentConstruction(b),fh=3.25,base=.58,w=b.width-1.4,d=b.depth-2,r=Math.min(w,d)*.31,podium=Math.min(3,Math.max(2,Math.floor(b.floors*.25)));
 const metal=art.get('#26353e',.45),silver=flagshipCladding(art),wall=art.get('#647b80',.7),roof=art.get('#4d575a');
 const entryX=0; // Primary entrance follows the authoritative front-centre access axis.
 const active=b.stage==='open'&&b.business_active!==false,enabled=active&&mode!=='off'&&detail!=='far'&&Boolean(options.signage?.feature),lived=windowLived(b);
 const glass=()=>art.shared('curved-media-glass',()=>windowMaterials('shop',{night:art.night,occupancy:art.occupancy},art.toonRamp,'#688b99'));
 const plateTemplates=new Map<string,THREE.BufferGeometry>();
 const plate=(width:number,depth:number,radius:number,y:number,h:number,mat:THREE.Material,atrium=false)=>{
  const key=`${width}/${depth}/${radius}/${h}/${atrium}`,cached=plateTemplates.get(key);
  if(cached){kit.add(cached.clone().translate(0,y,0),mat);return;}
  const pts=corporateOutline(width,depth,radius,segments),shape=new THREE.Shape(pts.map(p=>new THREE.Vector2(p.x,-p.z)));
  if(atrium){const hole=new THREE.Path();hole.moveTo(entryX-2.5,-(d/2-7));hole.lineTo(entryX-2.5,-(d/2-1));hole.lineTo(entryX+2.5,-(d/2-1));hole.lineTo(entryX+2.5,-(d/2-7));hole.closePath();shape.holes.push(hole);}
  const geo=new THREE.ExtrudeGeometry(shape,{depth:h,bevelEnabled:false,steps:1});geo.setIndex(Array.from({length:geo.getAttribute('position').count},(_,i)=>i));geo.rotateX(-Math.PI/2);plateTemplates.set(key,geo);kit.add(geo.clone().translate(0,y,0),mat);
 };
 plate(w+.4,d+.4,r+.2,.08,.5,silver);
 for(let f=0;f<framed;f++){
  const y=base+f*fh,scale=f<podium?1:.88,pw=w*scale,pd=d*scale,pr=r*scale,points=corporateOutline(pw,pd,pr,segments);
  plate(pw,pd,pr,y,.18,silver,f===1&&detail!=='far');
  if(f>=installed){for(const p of points.filter((_,i)=>i%(segments+1)===0))kit.box(p.x,y+fh/2,p.z,.2,fh,.2,metal);continue;}
  if(f<podium)plate(pw-.9,pd*.52,Math.min(pr*.6,pd*.2),y+.18,fh-.22,wall);
  else plate(pw-.5,pd-.5,Math.max(.2,pr-.25),y+.18,fh-.22,wall);
  for(let e=0;e<points.length;e++){
   const a=points[e],c=points[(e+1)%points.length],dx=c.x-a.x,dz=c.z-a.z,len=Math.hypot(dx,dz);if(len<.001)continue;
   const count=Math.max(1,Math.ceil(len/(detail==='close'?2:detail==='district'?3.5:8))),angle=Math.atan2(-dz,dx);
   for(let i=0;i<count;i++){
    const t=(i+.5)/count,x=a.x+dx*t,z=a.z+dz*t,n=corporateGlassNormal(x,z,pw,pd,pr),g=new THREE.PlaneGeometry(len/count,2.85);
    tagWindowPane(g,paneSeed(b.appearance_seed,x,y,z),active?lived:0,len/count,2.85,'shop');g.rotateY(angle);
    const normals=g.getAttribute('normal'),uv=g.getAttribute('uv');for(let v=0;v<normals.count;v++){const q=(i+uv.getX(v))/count,normal=corporateGlassNormal(a.x+dx*q,a.z+dz*q,pw,pd,pr);normals.setXYZ(v,normal.x,0,normal.z);}
    g.translate(x+n.x*.02,y+1.6,z+n.z*.02);kit.add(g,f<podium?retailVisionGlass(art):glass());
    if(f<podium&&detail!=='far'&&active&&n.z>.65&&!(f===0&&Math.abs(x-entryX)<3.4))addRetailInterior(kit,art,{x,y,z,angle,width:len/count,seed:b.appearance_seed,floor:f,kind:(f+Math.floor((x+pw/2)/7))%3});
    if(detail!=='far')kit.box(a.x+dx*i/count,y+1.6,a.z+dz*i/count,.085,2.85,.16,metal,angle);
   }
  }
  if(f<podium&&active&&detail!=='far'){
   const band=curvedMediaRibbon(pw,pd,pr,.055,segments,.035).translate(0,y+2.95,0);kit.add(band,art.get('#f5cfa0',.5,true,'shop'));
  }
 }
 const top=base+installed*fh,tw=w*.88,td=d*.88,tr=r*.88,bottom=base+podium*fh+.2,h=top-4.3-bottom,campaign=architecturalChoice(b.appearance_seed,763)%4;
 let panels=0;
 if(enabled&&h>=2){
  const g=curvedMediaRibbon(tw,td,tr,h,segments,.11),back=curvedMediaRibbon(tw,td,tr,h,segments,.065).translate(0,bottom,0);kit.add(back,metal);
  g.translate(0,bottom,0);
  if(mode==='animated'&&options.signage?.feature==='screen')kit.add(g,flagshipCampaignSurface(art,campaign));
  else{const c=campaignCell(campaign,0),uv=g.getAttribute('uv');for(let i=0;i<uv.count;i++)uv.setXY(i,c.u0+uv.getX(i)*(c.u1-c.u0),c.v0+uv.getY(i)*(c.v1-c.v0));kit.add(g,flagshipPosterSurface(art));}
  // Recessed LED housing, service lip and cabinet frame follow the same curvature.
  for(const y of [bottom-.14,bottom+h])kit.add(curvedMediaRibbon(tw+.16,td+.16,tr+.08,.14,segments,.16).translate(0,y,0),silver);
  const ends=curvedMediaFront(tw,td,tr,segments);for(const p of [ends[0],ends.at(-1)!])kit.box(p.x,bottom+h/2,p.z,.16,h+.28,.2,metal);
  panels=1;
 }
 if(installed>0){const ex=entryX;
  for(const side of [-1,1]){kit.box(ex+side*1.35,base+1.58,d/2+.04,.1,2.8,.13,metal);kit.box(ex,base+1.58+side*1.4,d/2+.04,2.8,.08,.13,metal);}
  const door=new THREE.PlaneGeometry(2.5,2.7);tagWindowPane(door,paneSeed(b.appearance_seed,ex,base,d/2),active?1:0,2.5,2.7,'shop');door.translate(ex,base+1.58,d/2+.11);kit.add(door,retailVisionGlass(art));
  kit.box(ex,base+1.58,d/2+.15,.06,2.7,.05,metal);
  for(let step=0;step<4;step++){const depth=.88-step*.2;kit.box(ex,(step+1)*.19/2,d/2+.06+depth/2,3.3,(step+1)*.19,depth,silver);}
  kit.box(ex,base+3,d/2+.35,6.4,.22,.85,silver);
  if(active&&detail!=='far')addFlagshipAtrium(kit,art,d/2,base,ex);
  if(active&&detail==='close'){
   kit.box(8.1,1.1,d/2+.25,.9,2.1,.5,art.get('#d7dedb'));kit.box(8.02,1.28,d/2+.52,.6,1.05,.025,art.get('#344e60'));
   for(let row=0;row<3;row++)for(let col=0;col<3;col++)kit.box(7.84+col*.18,.95+row*.27,d/2+.54,.1,.17,.025,art.get(['#b98164','#a8bfae','#c9b973'][col]));
   const pot=new THREE.CylinderGeometry(.35,.28,.48,10).translate(-8.1,.28,d/2+.35);kit.add(pot,art.get('#80644e'));
   const bush=new THREE.SphereGeometry(.4,8,6).scale(1,.85,1).translate(-8.1,.73,d/2+.35);kit.add(bush,art.get('#536f55'));
  }
  if(active)addStorefrontSign(kit,art,{x:ex,y:base+3.35,z:d/2+.12,width:5.2,height:.5,tenant:15+campaign,active});
 }
 if(complete&&installed===b.floors){
  plate(tw,td,tr,top,.25,silver);plate(tw*.76,td*.76,tr*.76,top+.25,1.6,roof);
  if(detail!=='far'){
   for(let i=0;i<9;i++)kit.box(0,top+.4+i*.15,td*.38,tw*.67,.055,.1,metal);
   for(const side of [-1,1]){kit.box(side*tw*.22,top+.65,-td*.12,tw*.17,.8,td*.19,silver);for(let i=0;i<5;i++)kit.box(side*tw*.22,top+.32+i*.13,-td*.22,tw*.15,.04,.03,metal);}
   const terrace=curvedMediaRibbon(w,d,r,.12,segments,.02).translate(0,base+podium*fh+.05,0);kit.add(terrace,metal);
  }
 }
 if(b.stage==='foundation')plate(w,d,r,.28,.3,silver);
 if(b.stage==='demolition')addDemolitionSite(kit,b,art);
 const g=kit.finish();for(const template of plateTemplates.values())template.dispose();g.traverse(o=>{if(o instanceof THREE.Mesh&&(o.material as THREE.Material).transparent)o.castShadow=false;});g.userData.businessArchitecture={recipe:options.recipe.id,family:options.recipe.family,mediaLandmark:'curved',entryX,detail,installedFloors:installed,framedFloors:framed,roofInstalled:complete&&installed===b.floors,displayPanels:panels,campaign,tenantIds:[],bands:[{from:0,count:podium,use:'shop'}]};return g;
}
