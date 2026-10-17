import {buildCurvedMediaLandmark} from './curved-media-landmark';
import * as THREE from 'three/webgpu';
import type {Building} from '../../types';
import type {ArtMaterials} from '../materials';
import type {BusinessBuildOptions} from './business';
import {Kit} from './geometry-kit';
import {apartmentConstruction,addDemolitionSite} from './lifecycle';
import {architecturalChoice} from './variation';
import {addStorefrontSign} from './business-signs';
import {mediaArtworkSurface,billboardCell,streetSignSurface} from '../signage';

/** Elliptical front arc, returning around both corners; UVs continuously span the artwork. */
export function landmarkScreen(width:number,height:number,depth:number,segments=32):THREE.BufferGeometry{
 const g=new THREE.PlaneGeometry(width,height,segments,1),p=g.getAttribute('position'),uv=g.getAttribute('uv');
 for(let i=0;i<p.count;i++){
  const angle=(uv.getX(i)-.5)*Math.PI;
  p.setXYZ(i,Math.sin(angle)*width/2,p.getY(i),Math.cos(angle)*depth);
 }
 g.computeVertexNormals();return g;
}
export function buildMediaLandmark(b:Building,art:ArtMaterials,options:BusinessBuildOptions):THREE.Group{
 if(options.recipe.mediaLandmark==='curved')return buildCurvedMediaLandmark(b,art,options);
 const kit=new Kit(),{recipe:r}=options,form=r.mediaLandmark!,detail=options.detail??'district',mode=options.displays??'animated';
 const {installed,framed,complete}=apartmentConstruction(b),base=.58,fh=3.25,w=b.width-1,d=b.depth-4,front=d/2;
 const metal=art.get('#26353e',.45),silver=art.get('#c6d0ce'),wall=art.get(r.wall,.7),roof=art.get('#4d575a');
 const active=b.stage==='open'&&b.business_active!==false,enabled=active&&mode!=='off'&&detail!=='far'&&Boolean(options.signage?.feature);
 const podium=Math.min(3,Math.max(2,Math.floor(b.floors*.25))),screenBottom=base+podium*fh+.2,screenTop=base+installed*fh-(form==='wrap'?4.5:1);
 const pane=(x:number,y:number,z:number,pw:number,ph:number)=>{kit.windowContext={seed:b.appearance_seed,lived:active?1:0};kit.box(x,y,z,pw,ph,.045,art.window('shop'));};
 kit.box(0,.1,0,b.width-.1,.2,b.depth-.1,silver);
 for(let f=0;f<framed;f++){
  const y=base+f*fh;kit.box(0,y,0,w,.16,d,silver);
  if(f>=installed){for(const x of [-w/2+.2,w/2-.2])for(const z of [-d/2+.2,d/2-.2])kit.box(x,y+fh/2,z,.25,fh,.25,silver);continue;}
  kit.box(0,y+fh/2,-.25,w-.5,fh-.2,d-.8,wall);
  const bays=Math.max(3,Math.floor(w/3.2));
  for(let i=0;i<bays;i++){
   const x=(i+.5)*w/bays-w/2;
   if(f<podium){pane(x,y+1.5,front+.015,w/bays-.12,2.75);
    if(detail!=='far')kit.box(x-w/bays/2+.05,y+1.6,front+.08,.065,2.95,.12,metal);
   }else if(form==='stack'||!enabled||form==='wrap'&&f===installed-1)pane(x,y+1.5,front+.015,w/bays-.25,2.5);
   pane(x,y+1.5,-front-.015,w/bays-.3,2.4);
  }
  if(f<podium&&detail!=='far'&&active){
   // Interior shelves and ceiling strips are visible through the glazing.
   kit.box(0,y+2.8,front-.12,w-.6,.07,.15,art.get('#f5cfa0',.5,true,'shop'));
   for(let i=0;i<3;i++)kit.box((i-1)*w*.25,y+1,front-.2,w*.14,.08,.1,silver);
  }
 }
 if(installed>0){
  const ex=((architecturalChoice(b.appearance_seed,761)%3)-1)*w*.22;
  kit.box(ex,base+1.4,front+.1,2.8,2.8,.12,metal);pane(ex,base+1.4,front+.18,2.5,2.6);
  kit.box(ex,base+3,front+.4,4.2,.15,.7,silver);
  if(active)addStorefrontSign(kit,art,{x:0,y:screenBottom-.7,z:front+.16,width:Math.min(w-1,9),height:.75,tenant:architecturalChoice(b.appearance_seed,762)%24,active});
 }
 let panels=0;
 const campaign=architecturalChoice(b.appearance_seed,763)%4;
 const screen=(x:number,bottom:number,width:number,height:number,wrap:boolean,index:number)=>{
  if(height<2)return;
  const arc=wrap?Math.min(1.3,d*.08):0,geometry=wrap?landmarkScreen(width,height,arc,detail==='close'?40:24):new THREE.PlaneGeometry(width,height);
  // Return endpoints stay in front of the structural wall; everything stays within the parcel.
  const z=front+.15;
  const backing=geometry.clone().scale(1.015,1.02,1).translate(x,bottom+height/2,z-.035);kit.add(backing,metal);
  if(mode==='animated'&&options.signage?.feature==='screen')kit.add(geometry.translate(x,bottom+height/2,z),mediaArtworkSurface(art,form==='stack'?(campaign+index)%4:campaign));
  else{const c=billboardCell(campaign*3),tex=geometry.getAttribute('uv');for(let i=0;i<tex.count;i++)tex.setXY(i,c.u0+tex.getX(i)*(c.u1-c.u0),c.v0+tex.getY(i)*(c.v1-c.v0));kit.add(geometry.translate(x,bottom+height/2,z),streetSignSurface(art));}
  panels++;
 };
 if(enabled&&screenTop-screenBottom>=2){
  const h=screenTop-screenBottom;
  if(form==='stack'){
   const count=Math.min(4,Math.max(2,Math.floor(h/5))),gap=.3,ph=(h-(count-1)*gap)/count;
   for(let i=0;i<count;i++)screen(w*.12,screenBottom+i*(ph+gap),w*.67,ph,false,i);
   for(let i=0;i<count;i++)addStorefrontSign(kit,art,{x:-w*.4,y:screenBottom+(i+.5)*h/count,z:front+.2,width:w*.13,height:Math.min(2,h/count-.4),tenant:(campaign*5+i)%24,active});
  }else screen(0,screenBottom,w*(form==='wrap'?.84:.72),h,form==='wrap',0);
 }
 if(form==='frame'&&installed>podium){
  const h=Math.max(0,screenTop-screenBottom);
  for(const x of [-w*.39,w*.39])kit.box(x,screenBottom+h/2,front+.3,.42,h+.5,.3,silver);
  kit.box(0,screenTop+.2,front+.3,w*.8,.4,.3,silver);
 }
 if(form==='wrap'&&installed>podium&&detail!=='far'){
  const top=base+installed*fh;
  for(let i=0;i<=12;i++)kit.box((i/12-.5)*w*.86,top-1.6,front+.12,.055,3,.12,metal);
 }
 // A recognizable structural frame and crown remain when screens are off or culled.
 for(const x of [-w*.47,w*.47])kit.box(x,base+installed*fh/2,front+.22,.3,installed*fh,.18,silver);
 if(complete&&installed===b.floors){const top=base+b.floors*fh;kit.box(0,top,0,w,.25,d,metal);kit.box(0,top+.8,-d*.1,w*.6,1.4,d*.5,roof);
  if(active)addStorefrontSign(kit,art,{x:0,y:top-.55,z:front+.24,width:Math.min(w*.55,9),height:.65,tenant:15+campaign,active});
 }
 if(b.stage==='foundation')kit.box(0,.25,0,w,.3,d,silver);
 if(b.stage==='demolition')addDemolitionSite(kit,b,art);
 const group=kit.finish();group.userData.businessArchitecture={recipe:r.id,family:r.family,mediaLandmark:form,detail,installedFloors:installed,framedFloors:framed,roofInstalled:complete&&installed===b.floors,displayPanels:panels,campaign,tenantIds:[],bands:[{from:0,count:podium,use:'shop'}]};return group;
}
