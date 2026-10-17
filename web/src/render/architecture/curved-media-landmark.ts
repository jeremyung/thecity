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
 const metal=art.get('#26353e',.45),silver=art.get('#c6d0ce'),wall=art.get('#647b80',.7),roof=art.get('#4d575a');
 const active=b.stage==='open'&&b.business_active!==false,enabled=active&&mode!=='off'&&detail!=='far'&&Boolean(options.signage?.feature),lived=windowLived(b);
 const glass=()=>art.shared('curved-media-glass',()=>windowMaterials('shop',{night:art.night,occupancy:art.occupancy},art.toonRamp,'#688b99'));
 const plate=(width:number,depth:number,radius:number,y:number,h:number,mat:THREE.Material)=>{
  const pts=corporateOutline(width,depth,radius,segments),shape=new THREE.Shape(pts.map(p=>new THREE.Vector2(p.x,-p.z)));
  const geo=new THREE.ExtrudeGeometry(shape,{depth:h,bevelEnabled:false,steps:1});geo.setIndex(Array.from({length:geo.getAttribute('position').count},(_,i)=>i));geo.rotateX(-Math.PI/2);geo.translate(0,y,0);kit.add(geo,mat);
 };
 plate(w+.4,d+.4,r+.2,.08,.2,silver);
 for(let f=0;f<framed;f++){
  const y=base+f*fh,scale=f<podium?1:.88,pw=w*scale,pd=d*scale,pr=r*scale,points=corporateOutline(pw,pd,pr,segments);
  plate(pw,pd,pr,y,.18,silver);
  if(f>=installed){for(const p of points.filter((_,i)=>i%(segments+1)===0))kit.box(p.x,y+fh/2,p.z,.2,fh,.2,metal);continue;}
  plate(pw-.5,pd-.5,Math.max(.2,pr-.25),y+.18,fh-.22,wall);
  for(let e=0;e<points.length;e++){
   const a=points[e],c=points[(e+1)%points.length],dx=c.x-a.x,dz=c.z-a.z,len=Math.hypot(dx,dz);if(len<.001)continue;
   const count=Math.max(1,Math.ceil(len/(detail==='close'?2:detail==='district'?3.5:8))),angle=Math.atan2(-dz,dx);
   for(let i=0;i<count;i++){
    const t=(i+.5)/count,x=a.x+dx*t,z=a.z+dz*t,n=corporateGlassNormal(x,z,pw,pd,pr),g=new THREE.PlaneGeometry(len/count,2.85);
    tagWindowPane(g,paneSeed(b.appearance_seed,x,y,z),active?lived:0,len/count,2.85,'shop');g.rotateY(angle);
    const normals=g.getAttribute('normal'),uv=g.getAttribute('uv');for(let v=0;v<normals.count;v++){const q=(i+uv.getX(v))/count,normal=corporateGlassNormal(a.x+dx*q,a.z+dz*q,pw,pd,pr);normals.setXYZ(v,normal.x,0,normal.z);}
    g.translate(x+n.x*.02,y+1.6,z+n.z*.02);kit.add(g,glass());
    if(detail!=='far')kit.box(a.x+dx*i/count,y+1.6,a.z+dz*i/count,.055,2.85,.08,metal,angle);
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
  if(mode==='animated'&&options.signage?.feature==='screen')kit.add(g,mediaArtworkSurface(art,campaign));
  else{const c=billboardCell(campaign*3),uv=g.getAttribute('uv');for(let i=0;i<uv.count;i++)uv.setXY(i,c.u0+uv.getX(i)*(c.u1-c.u0),c.v0+uv.getY(i)*(c.v1-c.v0));kit.add(g,streetSignSurface(art));}
  panels=1;
 }
 if(installed>0){const ex=((architecturalChoice(b.appearance_seed,761)%3)-1)*w*.15;
  kit.box(ex,base+1.4,d/2+.04,2.8,2.8,.1,metal);kit.windowContext={seed:b.appearance_seed,lived:active?1:0};kit.box(ex,base+1.4,d/2+.1,2.5,2.6,.03,art.window('shop'));
  kit.box(ex,base+3,d/2+.25,4.2,.14,.45,silver);
  if(active)addStorefrontSign(kit,art,{x:ex,y:base+3.35,z:d/2+.12,width:5.2,height:.5,tenant:15+campaign,active});
 }
 if(complete&&installed===b.floors){plate(tw,td,tr,top,.25,silver);plate(tw*.76,td*.76,tr*.76,top+.25,1.6,roof);}
 if(b.stage==='foundation')plate(w,d,r,.28,.3,silver);
 if(b.stage==='demolition')addDemolitionSite(kit,b,art);
 const g=kit.finish();g.userData.businessArchitecture={recipe:options.recipe.id,family:options.recipe.family,mediaLandmark:'curved',detail,installedFloors:installed,framedFloors:framed,roofInstalled:complete&&installed===b.floors,displayPanels:panels,campaign,tenantIds:[],bands:[{from:0,count:podium,use:'shop'}]};return g;
}
