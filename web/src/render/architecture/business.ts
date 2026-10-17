import {districtFrontage} from './district-frontage';
import {buildCorporateSkyscraper} from './corporate-skyscraper';
import {businessEnvelope,businessFacadeParts,businessRoofServices,fabricAwning} from './business-components';
import * as THREE from 'three/webgpu';
import type {Building} from '../../types';
import type {ArtMaterials} from '../materials';
import {windowLived} from '../building-surfaces';
import {addBladeSign,addFacadeFeature,addRooftopBillboard,type SignageContext} from '../signage';
import {Kit} from './geometry-kit';
import {apartmentConstruction,addDemolitionSite} from './lifecycle';
import {architecturalChoice} from './variation';
import {applyBuildingWeathering} from './character';
import {addStorefrontSign,addPriceCard,STOREFRONT_TENANTS} from './business-signs';
import {supportsBusiness,businessCanHostAdvertising,type BusinessRecipe,type DisplayMode} from './business-recipes';
export interface BusinessBuildOptions {recipe:BusinessRecipe;detail?:'close'|'district'|'far';displays?:DisplayMode;signage?:SignageContext}
/** Local Y-up geometry; city owns placement/batching and shared material lifetime. */
export function buildBusinessBuilding(b:Building,art:ArtMaterials,options:BusinessBuildOptions):THREE.Group|null{
 if(!supportsBusiness(b))return null;
 if(options.recipe.towerForm)return buildCorporateSkyscraper(b,art,options);
 const r=options.recipe,detail=options.detail??'district',displays=options.displays??'animated',seed=b.appearance_seed;
 const kit=new Kit(),base=.58,fh=3.25,w=b.width-1,d=(b.depth-2.2)*(r.form==='slab'?.78:1),front=d/2,back=-d/2;
 const {installed,framed,complete}=apartmentConstruction(b),active=b.stage==='open'&&b.business_active!==false;
 const lived=windowLived(b),wall=art.get(r.wall,r.roughness),slab=art.get('#cecdc2'),metal=art.get('#303b40',.45),roof=art.get('#585f5b'),dark=art.get('#404947'),wood=art.get('#755d49'),plant=art.get('#536a4c');
 const retail=Math.min(b.floors,r.retailFloors),separate=r.family==='mixed-use'||Boolean(r.district)&&r.family==='office',entrySide=architecturalChoice(seed,310)%2?-1:1;
 const lobbyW=separate?Math.min(2.2,w*.28):r.family==='office'?Math.min(3,w*.4):0;
 const lobbyX=entrySide*(w/2-lobbyW/2),retailW=w-lobbyW,retailX=separate?-entrySide*lobbyW/2:0;
 const flagship=!r.district&&['screen','curtain','department'].includes(r.form);
 const tenantCount=flagship?1:r.form==='media'?Math.max(1,Math.min(2,Math.floor(retailW/5))):Math.max(1,Math.min(5,Math.floor(retailW/(r.form==='arcade'||r.form==='loft'?2.7:4))));
 const weights=Array.from({length:tenantCount},(_,i)=>r.era==='older'?.75+(architecturalChoice(seed,320,i)%6)*.1:1),weightSum=weights.reduce((a,c)=>a+c,0);
 let frontageCursor=retailX-retailW/2;
 const slots=weights.map(weight=>{const width=retailW*weight/weightSum,x=frontageCursor+width/2;frontageCursor+=width;return {x,width};});
 const tenantPool=b.kind==='cafe'?[0,2,5,10,11]:Array.from({length:STOREFRONT_TENANTS.length},(_,i)=>i);
 const tenantStart=architecturalChoice(seed,311)%tenantPool.length;
 const tenantIds=Array.from({length:tenantCount},(_,i)=>tenantPool[(tenantStart+i*7)%tenantPool.length]);
 const pane=(x:number,y:number,z:number,pw:number,ph:number,use:'home'|'office'|'shop',side=false)=>{
  kit.windowContext={seed,lived:use==='shop'?(active?1:0):lived};
  kit.box(x,y,z,side?.055:pw,ph,side?pw:.055,art.window(use));
 };
 const planter=(x:number,y:number,z:number,pw:number)=>{kit.box(x,y+.2,z,pw,.4,.5,slab);kit.box(x,y+.48,z,pw-.08,.35,.4,plant);};
 kit.box(0,.09,0,b.width-.1,.18,b.depth-.1,slab);
 const floorEnvelope=(f:number)=>businessEnvelope(b,r,f);
 for(let f=0;f<installed;f++){
  const y=base+f*fh,{fw,fd,step}=floorEnvelope(f),fz=fd/2,rz=-fd/2;
  const use=f<retail?'shop':r.upper;
  kit.box(0,y+.1,0,fw,.2,fd,slab);
  // Walls surround real recessed panes: an opaque core gives occupied rooms depth.
  kit.box(0,y+fh/2,0,fw-.45,fh-.2,fd-.45,wall);
  if(f<retail){
   for(let i=0;i<tenantCount;i++){
    const {x,width}=slots[i],pw=width-.3;
    pane(x,y+1.38,fz+.015,pw,2.25,'shop');
    kit.box(x-pw/2,y+1.45,fz+.08,.09,2.7,.12,metal);kit.box(x,y+.3,fz+.08,pw,.12,.13,slab);
    kit.box(x,y+1.36,fz+.08,.07,2.1,.12,metal);
    if(b.stage==='open'){
     if(f===0||r.form==='tenant'||r.form==='media')addStorefrontSign(kit,art,{x,y:y+2.88,z:fz+(flagship?.4:.115),width:flagship?Math.min(6,pw):pw,height:flagship?.42:.57,tenant:tenantIds[i],active});
     if(flagship&&detail!=='far')for(const side of [-1,1])kit.box(x+side*Math.min(1,pw*.18),y+1.25,fz+.1,.065,2.25,.12,metal);
     if(!active){kit.box(x,y+1.35,fz+.14,pw,2.1,.08,art.get('#8c9386'));if(detail==='close')for(let k=0;k<10;k++)kit.box(x,y+.43+k*.18,fz+.19,pw,.018,.025,metal);}
     if(detail!=='far'&&r.awning)fabricAwning(kit,art,x,y+2.48,fz,pw,tenantIds[i],detail);
     if(detail==='close'&&active){
      // Small exterior displays sit beside, rather than across, each shop door.
      const prop=tenantIds[i]%3,px=x-pw*.3;
      if(prop===0){kit.box(px,y+.58,fz+.32,.48,1.05,.12,wood);kit.box(px,y+.63,fz+.395,.39,.73,.025,dark);}
      else if(prop===1){kit.box(px,y+.35,fz+.3,.55,.6,.4,wood);for(let k=0;k<3;k++)kit.box(px+(k-1)*.13,y+.76,fz+.3,.1,.23,.24,art.get(['#c1a16a','#9daea9','#b7b0a3'][k]));}
      else planter(px,y,fz+.35,.5);
      if(r.form==='loft'||r.form==='arcade'){
       const rackX=x-pw*.28;kit.box(rackX,y+.75,fz+.3,Math.min(1.1,pw*.4),1.35,.4,wood);
       for(let row=0;row<3;row++)for(let col=0;col<4;col++)kit.box(rackX+(col-1.5)*Math.min(.2,pw*.08),y+.35+row*.4,fz+.53,.13,.29,.12,art.get(['#c1a16a','#9daea9','#b7b0a3','#a4654f'][(row+col+tenantIds[i])%4]));
       addPriceCard(kit,art,rackX,y+1.48,fz+.62,tenantIds[i]);
      }
      if(r.id==='corner-cafe'){kit.cylinder(x+pw*.3,y+.72,fz+.7,.25,.09,wood);kit.cylinder(x+pw*.3,y+.38,fz+.7,.05,.66,metal);}
     }
    }
   }
   if(separate){pane(lobbyX,y+1.4,fz+.03,lobbyW-.24,2.5,r.upper);kit.box(lobbyX,y+2.85,fz+.08,lobbyW-.12,.18,.2,metal);}
  }else{
   const bays=Math.max(1,Math.floor((fw-.55)/r.bay)),pitch=(fw-.55)/bays;
   const glazed=['curtain','screen','media','fins'].includes(r.form);
   const ph=r.form==='punched'?1.65:r.form==='ribbon'||r.form==='slab'?1.65:r.form==='loft'?2.2:glazed?2.8:2.45;
   for(let i=0;i<bays;i++){
    const x=-fw/2+.275+(i+.5)*pitch,pw=pitch*(r.form==='punched'?.63:r.form==='department'?.65:glazed?.98:.88);
    pane(x,y+1.8,fz+.016,pw,ph,use);
    pane(x,y+1.8,rz-.016,pw,ph,use);
    if(detail!=='far'){
     for(const z of [fz+.08,rz-.08]){
      const frame=r.era==='older'?art.get('#636b69',.45):metal;
      for(const side of [-1,1]){kit.box(x+side*pw/2,y+1.8,z,.065,ph+.12,.1,frame);kit.box(x,y+1.8+side*ph/2,z,pw,.065,.12,frame);}
      if(r.era==='older'){kit.box(x,y+1.8,z,.055,ph,.12,frame);if(r.form==='loft'||r.form==='tenant')kit.box(x,y+1.9,z,pw,.045,.12,frame);}
     }
     if(r.form==='frame')kit.box(x-pitch/2,y+fh/2,fz+.2,.22,fh,.4,slab);
     if(r.form==='curtain')kit.box(x,y+1.8,fz+.08,.06,ph,.12,metal);
     if(r.upper==='home'&&(r.form==='shop-house'||r.form==='podium'||r.form==='corner')){
      kit.box(x,y+.3,fz+.38,pw+.13,.13,.75,slab);kit.box(x,y+.88,fz+.74,pw,.85,.06,metal);
     }
    }
   }
   if(r.family==='office'&&f===0){pane(lobbyX,y+1.55,fz+.04,Math.min(2.5,fw-.8),2.75,'office');kit.box(lobbyX,y+3,fz+.38,Math.min(3.2,lobbyW+.3),.16,.85,metal);if(b.stage==='open')addStorefrontSign(kit,art,{x:lobbyX,y:y+2.78,z:fz+.15,width:Math.max(.8,lobbyW-.2),height:.3,tenant:12+architecturalChoice(seed,316)%6,active});}
  }
  if(b.stage==='open'&&detail!=='far'&&r.form==='tenant'&&f>=retail){
   addStorefrontSign(kit,art,{x:entrySide*(fw/2-.62),y:y+2.95,z:fz+.14,width:1,height:.32,tenant:architecturalChoice(seed,315,f)%24,active});
  }
  if(f===0&&b.stage==='open'&&detail!=='far'){
   // A quiet rear service entrance and metered plant stay away from street access.
   kit.box(-entrySide*(fw/2-.65),y+1.1,rz-.06,.85,2.1,.09,metal);
   kit.box(entrySide*(fw/2-.65),y+.65,rz-.2,.85,1.1,.32,dark);
   if(r.form==='arcade')for(const x of [...slots.map(slot=>slot.x-slot.width/2),retailX+retailW/2])kit.box(x,y+1.3,fz+.8,.13,2.6,.13,metal);
   if(r.form==='corner')addStorefrontSign(kit,art,{x:fw/2+.08,y:y+2.88,z:0,width:Math.min(4,fd-.8),height:.57,tenant:tenantIds[0],active,side:true});
  }
  businessFacadeParts(kit,art,r,{width:fw,depth:fd,y,floor:f,floors:b.floors,detail});
  // Side facades use the same floor schedule, with a quiet rear service zone.
  const sideBays=Math.max(1,Math.floor((fd-1.4)/r.bay)),sidePitch=(fd-1.4)/sideBays;
  for(const side of [-1,1])for(let j=0;j<sideBays;j++){
   const z=-fd/2+.7+(j+.5)*sidePitch;
   pane(side*(fw/2+.012),y+1.8,z,sidePitch*.7,f<retail?2.1:1.8,use,true);
   if(detail==='close')kit.box(side*(fw/2+.08),y+.86,z,.13,.08,sidePitch*.75,metal);
  }
  if(r.form==='ribbon'||r.form==='curtain')kit.box(0,y+3.1,0,fw+.08,.3,fd+.08,metal);
  if(detail!=='far'&&step>0&&f>0&&floorEnvelope(f-1).step<step)planter(0,y,Math.min(d/2-.3,fz+.45),fw*.65);
 }
 for(let f=installed;f<framed;f++){
  const y=base+f*fh,{fw,fd}=floorEnvelope(f);kit.box(0,y,0,fw,.2,fd,slab);
  for(const x of [-fw/2+.25,fw/2-.25])for(const z of [-fd/2+.25,fd/2-.25])kit.box(x,y+fh/2,z,.3,fh,.3,slab);
 }
 const roofInstalled=complete&&installed===b.floors;
 if(roofInstalled){
  const {fw,fd}=floorEnvelope(installed-1),top=base+installed*fh;kit.box(0,top,0,fw+.1,.23,fd+.1,slab);
  for(const side of [-1,1]){kit.box(side*(fw/2-.1),top+.4,0,.16,.8,fd,wall);kit.box(0,top+.4,side*(fd/2-.1),fw,.8,.16,wall);}
  businessRoofServices(kit,art,r,fw,fd,top,detail,seed);
  if(businessCanHostAdvertising(r)&&r.era==='older'&&active&&detail!=='far'&&displays!=='off'&&(options.signage?.proximity??0)>0&&b.floors>=4&&b.floors<12&&architecturalChoice(seed,312)%3===0)addRooftopBillboard(kit,art,metal,{x:0,z:0,width:fw,depth:fd,top},seed);
 }
 let display:string|undefined;
 if(active&&detail!=='far'&&displays!=='off'){
  const feature=businessCanHostAdvertising(r)?options.signage?.feature:undefined;
  // Displays replace a reserved opaque upper frontage bay, keeping shop and lobby doors clear.
  if(feature&&installed>=3){display=feature==='screen'&&displays==='animated'?'screen':'ad';addFacadeFeature(kit,art,metal,display as 'screen'|'ad',{frontZ:front+.04,width:Math.max(3,Math.min(w-.4,w*.65)),bottom:base+fh+ .35,height:r.form==='media'&&display==='screen'?Math.min(5.9,(installed-1)*fh-.5):2.35,seed,program:seed%3});}
  if((r.form==='tenant'||r.family==='commercial'&&r.era==='older')&&installed>=2&&b.depth/2-front>=.55)addBladeSign(kit,art,metal,{x:-entrySide*(w/2-.22),frontZ:front+.05,maxDepth:b.depth/2-front-.12,bottom:base+fh,maxHeight:Math.min(12,(installed-1)*fh-.3),panels:Math.min(installed-1,4),seed});
 }
 if(complete&&r.era==='older'&&detail!=='far')kit.cylinder(-w/2+.12,base+installed*fh/2,front+.23,.045,installed*fh,metal,6);
 if(b.stage==='demolition')addDemolitionSite(kit,b,art);
 if(b.stage==='foundation')kit.box(0,.24,0,w,.3,d,slab);
 if(b.stage==='renovation'&&detail!=='far')for(const x of [-w/2+.2,w/2-.2]){kit.box(x,base+installed*fh/2,front+.32,.07,installed*fh,.07,metal);for(let f=0;f<installed;f++)kit.box(0,base+f*fh,front+.32,w,.06,.14,metal);}
 districtFrontage(kit,art,b,r,detail,Boolean(display),slots);
 const group=kit.finish();
 const weather={profile:r.era==='older'?'maintained':'fresh',strength:b.stage==='open'?(r.era==='older'?.15+architecturalChoice(seed,313)%4*.08:.015+architecturalChoice(seed,313)%3*.025):.025,phase:architecturalChoice(seed,314)%997};applyBuildingWeathering(group,weather);
 group.userData.businessArchitecture={recipe:r.id,family:r.family,district:r.district,detail,installedFloors:installed,framedFloors:framed,roofInstalled,display,tenantIds,entryX:lobbyX,retailEntrances:slots.map(slot=>slot.x),frontageSlots:slots,bands:[{from:0,count:retail,use:'shop'},{from:retail,count:b.floors-retail,use:r.upper}].filter(v=>v.count>0)};
 return group;
}
