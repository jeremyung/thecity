import {windowLived} from '../building-surfaces';
import {apartmentConstruction,addDemolitionSite} from './lifecycle';
import {entranceLayout} from './character';
import * as THREE from 'three/webgpu';
import {circulationLayout} from './circulation';
import { Kit } from './geometry-kit';
import type { ArtMaterials } from '../materials';
import type { Building } from '../../types';
import { placeComponent as component } from './components';
import {apartmentFeatures,architecturalChoice} from './variation';
import {householdDressing,hangingLaundry,parkedBicycle} from './dressing';
import { apartmentRecipe, APARTMENT_RECIPE_VERSION, type ApartmentBuildOptions } from './recipes';
type Materials=Record<'wall'|'concrete'|'metal'|'glass'|'roof',THREE.Material>;
/** Reference-driven review recipe: continuous loggias, circulation spine, service rear. */
export function apartmentPilot(b:Building,art:ArtMaterials,options:ApartmentBuildOptions={}){
 const recipe=apartmentRecipe(b,options.recipe),detail=options.detail??'close';
 const kit=new Kit(),base=.58,fh=3.25;kit.windowContext={seed:b.appearance_seed,lived:windowLived(b)};
 const circulation={...circulationLayout(b.width,b.depth)};
 if(recipe.circulation==='internal')circulation.external=false;
 const {bodyWidth:w,bodyDepth:d,centerZ:cz}=circulation,seed=b.appearance_seed>>>0;
 const features=apartmentFeatures(seed),entrance=entranceLayout(b,recipe),entryX=entrance.x;
 const contemporary=recipe.circulation==='internal';
 if(contemporary){features.shutters=false;features.windows=1;features.roof=1;features.balconyScreens=0;}
 const coreWidth=recipe.frontCore&&w>=7.2?Math.min(2.1,Math.max(1.6,w*.17)):0;
 const livingWidth=w-coreWidth,center=-coreWidth/2;
 const front=cz+d/2,rear=cz-d/2,edge=front+recipe.balconyDepth;
 const stairStart=features.stairSide<0?-w/2+.9:w/2-.9-circulation.stairLength;
 const landingXs=[stairStart-circulation.landingWidth/2,stairStart+circulation.stairLength+circulation.landingWidth/2];
 const left=-w/2+.14,right=w/2-coreWidth-.14;
 const bayCount=Math.max(1,Math.round((right-left)/recipe.bayWidth)),pitch=(right-left)/bayCount;
 const bays=Array.from({length:bayCount},(_,i)=>left+(i+.5)*pitch);
 const coreX=w/2-coreWidth/2;
 const m:Materials={wall:art.get(recipe.wallTints[seed%recipe.wallTints.length],recipe.wallRoughness??.85),concrete:art.get(recipe.concreteTint),metal:art.get(contemporary?'#303b40':'#636b69',.45),glass:art.get('#59717a',.24),roof:art.get(recipe.roofTint)};
 const dark=art.get('#404947'),joint=art.get('#aaa99f'),tile=art.get(recipe.guardTint),stone=art.get('#8e9289');
 const fabric=[art.get('#d5c8b0'),art.get('#9daea9'),art.get('#b7b0a3')];
 const pane=['#59717a','#40565f','#7c8c8b','#526c73'];
 const plant=art.get('#536a4c'),pot=art.get('#967d62');
 const {complete,installed,framed}=apartmentConstruction(b);
 const variation=(f:number,i:number)=>architecturalChoice(seed,20,f*97+i);
 const line=(a:THREE.Vector3,c:THREE.Vector3,r:number,material:THREE.Material)=>{
  const delta=c.clone().sub(a),geo=new THREE.CylinderGeometry(r,r,delta.length(),6);
  geo.applyQuaternion(new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0,1,0),delta.clone().normalize()));
  geo.translate(...a.clone().add(c).multiplyScalar(.5).toArray());kit.add(geo,material);
 };
 const planter=(x:number,y:number,z:number)=>{
  const profile=architecturalChoice(seed,91,Math.round(x*19+y*7));
  if(profile%2)kit.cylinder(x,y+.16,z,.17,.32,pot,8);else kit.box(x,y+.14,z,.5,.28,.28,pot);
  for(let j=0;j<2+profile%3;j++){
   const leaf=new THREE.IcosahedronGeometry(.16,0);leaf.scale(.6+(profile%4)*.14,1.1+(profile%3)*.3,.8);leaf.translate(x+(j-1)*.13,y+.37+(j%2)*.04,z);kit.add(leaf,plant);
  }
 };
 kit.box(0,.025,0,b.width-.08,.05,b.depth-.08,stone);
 kit.box(0,.15,cz,w+.25,.3,d+.25,stone);
 kit.box(0,.11,b.depth/2-.13,b.width-.1,.16,.18,m.concrete);
 for(let x=-b.width/2+.6;x<b.width/2-.1;x+=.6)kit.box(x,.054,edge+.36,.012,.009,Math.max(.1,b.depth/2-edge-.2),joint);
 for(let f=0;f<installed;f++){
  const y=base+f*fh;
  const front=cz+d/2-(f>=b.floors-2?recipe.upperSetback:0),rear=cz-d/2;
  const floorDepth=front-rear,floorCenter=(front+rear)/2,edge=front+recipe.balconyDepth;
  kit.box(0,y+.08,floorCenter,w-.3,.16,floorDepth-.3,m.concrete);
  // Deep wall reveals surround fixed-size doors; the balcony sits beyond the shell.
  const openings=f===0?[{x:entryX,width:1.7,low:0,high:2.35}]:bays.map(x=>({x,width:1.92,low:.18,high:2.52}));
  let cursor=-w/2;
  for(const opening of openings){
   const a=opening.x-opening.width/2,c=opening.x+opening.width/2;
   if(a>cursor)kit.box((cursor+a)/2,y+fh/2,front,a-cursor,fh,.34,m.wall);
   if(opening.low>0)kit.box(opening.x,y+opening.low/2,front,opening.width,opening.low,.34,m.wall);
   kit.box(opening.x,y+(opening.high+fh)/2,front,opening.width,fh-opening.high,.34,m.wall);
   cursor=c;
  }
  if(cursor<w/2)kit.box((cursor+w/2)/2,y+fh/2,front,w/2-cursor,fh,.34,m.wall);
  for(const side of [-1,1]){
   kit.box(side*(w/2-.14),y+fh/2,floorCenter,.28,fh,floorDepth,m.wall);
   // Two coherent utility-window columns break the previously blank gable.
   for(const z of (contemporary?[rear+1.05]:features.utilityColumns===1?[cz]:[rear+1.05,front-1.1])){
    const x=side*(w/2+.015);
    kit.box(x,y+1.9,z,.055,1.12,.66,dark);
    kit.box(x+side*.035,y+1.9,z,.035,.96,.51,art.window('home'));
    kit.box(x+side*.06,y+1.9,z,.045,1.02,.035,m.metal);
    kit.box(x+side*.07,y+1.32,z,.16,.08,.81,m.concrete);
   }
   // Constrained vertical expansion joints, rather than stripes on every floor.
   kit.box(side*(w/2+.004),y+fh/2,cz-d*.12,.012,fh,.023,joint);
  }
  // Rear openings are actual unit doors, not window components on an unbroken wall.
  if(contemporary){kit.box(0,y+fh/2,rear,w,fh,.28,m.wall);}
  else {
  let rearCursor=-w/2;
  for(const x of bays){
   const a=x-.45,c=x+.45;
   if(a>rearCursor)kit.box((rearCursor+a)/2,y+fh/2,rear,a-rearCursor,fh,.28,m.wall);
   kit.box(x,y+2.72,rear,.9,1.06,.28,m.wall);
   kit.box(x,y+1.22,rear+.04,.83,2.24,.07,dark);
   kit.box(x,y+1.22,rear-.06,.76,2.15,.045,m.metal);
   if(detail!=='far'){kit.box(x+.26,y+1.18,rear-.095,.035,.16,.04,stone);kit.box(x,y+2.36,rear-.16,.22,.09,.025,m.concrete);}
   rearCursor=c;
  }
  if(rearCursor<w/2)kit.box((rearCursor+w/2)/2,y+fh/2,rear,w/2-rearCursor,fh,.28,m.wall);
  }
  if(f===0){
   // Recessed lobby with a masonry plinth and address/mailbox fixtures.
   component(kit,'entrance',entryX,y+1.14,front-.16,{...m,glass:art.window('home')});
   for(const x of [-w/2+.48,w/2-.48])kit.box(x,y+.48,front+.02,.7,.96,.38,stone);
   const canopyWidth=Math.min(w-.12,recipe.canopyWidth);
   kit.box(entryX,y+2.85,front+.55,canopyWidth,contemporary?.07:features.canopy===1?.08:.16,1.55,contemporary||features.canopy===1?m.metal:m.concrete);
   if(features.canopy===2)for(const x of [-canopyWidth/2+.1,canopyWidth/2-.1])kit.box(entryX+x,y+1.44,front+1.15,.085,2.8,.085,m.metal);
   if(features.canopy===1)for(const x of [-canopyWidth*.38,canopyWidth*.38])line(new THREE.Vector3(entryX+x,y+3.15,front+.1),new THREE.Vector3(entryX+x,y+2.9,front+1.15),.025,m.metal);
   kit.box(entryX,y+2.75,front+.65,canopyWidth-.15,.045,1.25,dark);
   kit.box(entryX,.43,front+.48,Math.min(w-.2,2.35),.2,1.35,stone);
   kit.box(entryX,.27,front+.89,Math.min(w-.2,2.65),.13,.72,m.concrete);
   component(kit,'mailboxes',Math.max(-w/2+.47,Math.min(w/2-.47,entryX+(entryX>0?-1.38:1.38))),y+1.36,front+.22,m);
   kit.box(Math.max(-w/2+.4,Math.min(w/2-.4,entryX+(entryX>0?1.28:-1.28))),y+1.7,front+.2,.42,.22,.035,dark);
   for(let i=0;i<3;i++)kit.box(Math.max(-w/2+.4,Math.min(w/2-.4,entryX+(entryX>0?1.28:-1.28)))-.13+i*.1,y+1.7,front+.225,.045,.075,.012,m.concrete);
   // Inset paving joints and two planted edges frame the actual entrance.
   for(const x of [entryX-2.1,entryX+2.1])planter(Math.max(-w/2+.3,Math.min(w/2-.3,x)),.1,edge-.12);
   // Retain the authoritative center-front connection; cross the paved apron to the door.
   const dz=front+1.1-(b.depth/2-.65);
   kit.box(entryX/2,.065,(front+1.1+b.depth/2-.65)/2,1.1,.025,Math.hypot(entryX,dz),stone,Math.atan2(entryX,dz));
   kit.box(0,.065,b.depth/2-.35,1.1,.025,.62,stone);
  }else{
   // A continuous cantilever and substantial panel guard make the building read as units.
   const balconyWidth=livingWidth-.08,balconyDepth=recipe.balconyDepth+.22;
   const spans=recipe.balcony==='individual'?bays.map(x=>({x,width:Math.min(2.55,pitch-.32)})):[{x:center,width:balconyWidth}];
   for(const span of spans){
    kit.box(span.x,y+.1,front+balconyDepth/2-.12,span.width,.23,balconyDepth,m.concrete);
    if(recipe.guard==='solid'){
     kit.box(span.x,y+.57,edge,span.width,.73,.12,tile);
     kit.box(span.x,y+.97,edge,span.width+.04,.075,.18,m.concrete);
    }else if(recipe.guard==='glass')kit.box(span.x,y+.69,edge,span.width,.94,.045,m.glass);
    else{
     kit.box(span.x,y+.56,edge,span.width,.035,.035,m.metal);
     for(let x=span.x-span.width/2+.04;x<span.x+span.width/2;x+=detail==='far'?.65:detail==='district'?.32:.17)kit.box(x,y+.68,edge,.025,1.01,.025,m.metal);
    }
    kit.box(span.x,y+1.18,edge,span.width,.035,.035,m.metal);
    if(recipe.accent==='fins')for(const side of [-1,1])kit.box(span.x+side*(span.width/2-.04),y+fh/2,front+.45,.09,fh,.95,m.metal);
    if(recipe.accent==='frames')kit.box(span.x,y+fh-.13,edge,span.width,.12,.13,m.metal);
    for(const x of [span.x-span.width/2+.04,span.x+span.width/2-.04]){
     kit.box(x,y+.68,front+recipe.balconyDepth/2,.045,1.05,recipe.balconyDepth,m.metal);
    }
   }
   if(!recipe.balcony)for(let i=1;i<bayCount;i++){
    const x=left+i*pitch;
    kit.box(x,y+1.06,front+.64,.065,1.78,1.04,m.concrete);
    kit.box(x,y+1.98,front+.64,.075,.055,1.04,m.metal);
   }
   if(!recipe.balcony)for(const x of [-w/2+.075,right+.06])kit.box(x,y+fh/2,front+.62,.15,fh,1.43,m.wall);
   for(let i=0;i<bays.length;i++){
    const x=bays[i],v=variation(f,i),glass=art.window('home');
    if(features.windows===0&&detail!=='far')component(kit,'balcony-door',x,y+1.34,front-.2,{...m,glass});
    else {
     // Same structural opening; different fenestration system, never scaled authored meshes.
     kit.box(x,y+1.34,front-.2,1.84,2.28,.065,glass);
     for(const dx of [-.94,.94])kit.box(x+dx,y+1.34,front-.16,.055,2.37,.09,m.metal);
     for(const dy of [.17,2.51])kit.box(x,y+dy,front-.16,1.92,.055,.09,m.metal);
     const divisions=features.windows===1?3:2;
     for(let k=1;k<divisions;k++)kit.box(x-.94+k*1.88/divisions,y+1.34,front-.145,.045,2.3,.09,m.metal);
     if(features.windows===2)kit.box(x,y+1.92,front-.14,1.88,.045,.09,m.metal);
    }
    if(features.shutters){
     kit.box(x,y+2.64,front-.1,2.04,.22,.21,m.concrete);
     if(v%4===0){kit.box(x,y+2.17,front-.09,1.87,.72,.055,m.metal);for(let k=0;k<5;k++)kit.box(x,y+1.85+k*.14,front-.055,1.87,.018,.02,dark);}
    }
    if(recipe.balcony!=='juliet'&&detail!=='far'&&features.balconyScreens===1&&i%2===0)kit.box(x,y+.64,edge-.08,Math.min(pitch-.2,1.7),.68,.035,fabric[v%fabric.length]);
    if(recipe.balcony!=='juliet'&&detail!=='far'&&features.balconyScreens===2&&f>1&&i===0)for(let k=0;k<5;k++)kit.box(x-.64+k*.3,y+1.58,edge-.08,.07,1.12,.06,m.metal);
    // Production room mapping supplies curtains and depth without hidden duplicate meshes.
    if(!contemporary&&detail!=='far'&&recipe.balcony!=='juliet'&&v%3===0){
     const acX=x+Math.min(pitch/2-.48,recipe.balcony==='individual'?.75:1.05);
     component(kit,'ac',acX,y+.55,front+.43,m);
     for(const dx of [-.25,.25])kit.box(acX+dx,y+.24,front+.43,.09,.1,.44,m.metal);
     if(detail==='close')line(new THREE.Vector3(acX+.35,y+.55,front+.2),new THREE.Vector3(acX+.35,y+1.4,front+.13),.025,stone);
    }
    const household=householdDressing(seed,f,i);
    if(b.stage==='open'&&detail!=='far'&&recipe.balcony!=='juliet'&&household.plants)planter(x+.45,y+.24,edge-.31);
    if(detail==='close'&&recipe.balcony!=='juliet'&&household.storage)kit.box(x-Math.min(.65,pitch/2-.3),y+.45,front+.48,.42,.52,.37,fabric[(v+1)%fabric.length]);
    if(detail==='close'&&recipe.balcony!=='juliet'&&household.laundry&&(!contemporary||v%3===0))hangingLaundry(kit,x+household.offset,y+1.83,front+.64,household,fabric,m.metal);
   }
   // Fine, restrained masonry joints establish panel scale in close views.
   if(!recipe.balcony&&recipe.guard==='solid'&&detail==='close')for(let x=left+.4;x<right-.1;x+=.45)kit.box(x,y+.55,edge+.066,.009,.67,.008,joint);
   if(!recipe.balcony&&recipe.guard==='solid')kit.box(center,y+.56,edge+.068,balconyWidth,.01,.008,joint);
   if(recipe.upperSetback&&f===b.floors-2){
    kit.box(center,y+.1,edge+recipe.upperSetback/2,livingWidth-.1,.14,recipe.upperSetback,m.concrete);
    kit.box(center,y+1.15,edge+recipe.upperSetback,livingWidth-.1,.04,.04,m.metal);
    for(const x of [left+.45,right-.45])planter(x,y+.18,edge+.35);
   }
  }
  // Full-height circulation spine, with narrow glazing on its own rhythm.
  if(coreWidth){
   kit.box(coreX,y+fh/2,front+.63,coreWidth,fh,1.38,m.concrete);
   kit.box(coreX,y+1.83,front+1.34,.48,1.55,.06,dark);
   kit.box(coreX,y+1.83,front+1.38,.35,1.38,.025,m.glass);
   kit.box(coreX,y+1.83,front+1.405,.025,1.38,.025,m.metal);
   kit.box(w/2-.13,y+fh/2,edge+.1,.08,fh,.08,m.metal);
  }
  if(contemporary){
   // Integrated core and screened service elevation, rather than a bolted-on steel stair.
   const coreX=w/2-Math.min(1.1,w*.22),coreW=Math.min(2.2,w*.44);
   kit.box(coreX,y+fh/2,rear-.42,coreW,fh,.9,m.concrete);
   kit.box(coreX,y+1.8,rear-.9,.38,1.45,.035,dark);
   kit.box(coreX,y+1.8,rear-.925,.27,1.32,.02,m.glass);
   for(const x of bays)if(Math.abs(x-coreX)>coreW/2+.4){
    kit.box(x,y+1.8,rear-.16,.62,.96,.035,dark);
    if(detail!=='far')for(let k=0;k<6;k++)kit.box(x,y+1.39+k*.15,rear-.2,.65,.035,.045,m.metal);
   }
  }else{
  // Service rear: access gallery, meters, risers and one aligned stair flight per story.
  if(detail!=='far')component(kit,'meter-bank',-w/2+.5,y+1.65,rear-.2,m,Math.PI);
  for(const x of [-w/2+.18,w/2-.18]){
   kit.box(x,y+fh/2,front+.12,.065,fh,.065,m.metal);
   kit.box(x,y+.3,front+.18,.065,.08,.26,m.metal);
   if(detail==='close')for(const h of [.65,2.6])kit.box(x,y+h,front+.12,.13,.045,.09,m.metal);
  }
  kit.box(0,y+.08,circulation.galleryZ,w-.18,.14,circulation.galleryDepth,m.concrete);
  if(f>0){
   const gaps=circulation.external?landingXs.map(x=>[x-circulation.landingWidth/2,x+circulation.landingWidth/2]).sort((a,b)=>a[0]-b[0]):[];
   let cursor=-w/2+.1;
   for(const [a,c] of [...gaps,[w/2-.1,w/2-.1]]){
    if(a>cursor)for(const height of [.65,1.16])kit.box((cursor+a)/2,y+height,rear-.97,a-cursor,.035,.04,m.metal);
    cursor=Math.max(cursor,c);
   }
   for(let x=-w/2+.16;x<w/2;x+=1)if(!gaps.some(([a,c])=>x>a&&x<c))kit.box(x,y+.65,rear-.97,.03,1.05,.03,m.metal);
  }
  if(circulation.external){
   const length=circulation.stairLength,start=stairStart;
   const reverse=(f+(features.stairSide>0?1:0))%2===1,sz=circulation.stairZ;
   // Each end has a separate landing, supported continuously to the foundation.
   for(const x of landingXs){
    kit.box(x,y+.08,rear-1.43,circulation.landingWidth,.14,.9,m.concrete);
    if(f===0)kit.box(x,base+installed*fh/2,sz-.35,.085,installed*fh,.085,m.metal);
    kit.box(x,y+1.13,sz-.43,circulation.landingWidth,.04,.04,m.metal);
   }
   if(f<installed-1){
    const stairMaterial=features.stairFinish?m.metal:m.concrete,steps=detail==='far'?8:18;
    for(let step=0;step<steps;step++){
     const x=start+(step+.5)*length/steps,rise=(reverse?steps-step:step+1)*fh/steps;
     kit.box(x,y+rise,sz,length/steps+.012,.075,.82,stairMaterial);
    }
    for(const z of [sz-.4,sz+.4]){
     line(new THREE.Vector3(start,y+(reverse?fh:0)-.06,z),new THREE.Vector3(start+length,y+(reverse?0:fh)-.06,z),.055,m.metal);
     line(new THREE.Vector3(start,y+(reverse?fh:0)+1,z),new THREE.Vector3(start+length,y+(reverse?0:fh)+1,z),.024,m.metal);
     if(detail!=='far')for(let step=0;step<18;step+=3){const x=start+(step+.5)*length/18,rise=(reverse?18-step:step+1)*fh/18;kit.box(x,y+rise+.5,z,.027,1,.027,m.metal);}
    }
   }
  }

  }
 }
 if(!complete){
  for(let f=installed;f<framed;f++){
   const y=base+f*fh;kit.box(0,y+.06,cz,w,.12,d,m.concrete);
   for(const x of [-w/2+.15,w/2-.15])for(const z of [front-.15,rear+.15])kit.box(x,y+fh/2,z,.22,fh,.22,m.concrete);
  }
 }
 if(complete){
  const top=base+b.floors*fh,roofFront=edge-recipe.upperSetback+.12,roofRear=rear-.12;
  kit.box(0,top,(roofFront+roofRear)/2,w+.15,.23,roofFront-roofRear,m.roof);
  for(const z of [roofFront,roofRear]){
   kit.box(0,top+.37,z,w+.18,.72,.16,m.wall);
   kit.box(0,top+.76,z,w+.22,.075,.23,m.concrete);
  }
  for(const x of [-w/2,w/2])kit.box(x,top+.37,(roofFront+roofRear)/2,.15,.72,roofFront-roofRear,m.wall);
  const hatchX=contemporary?Math.max(0,w/2-1.05):coreWidth?coreX:0;
  kit.box(hatchX,top+.83,cz+.3,Math.min(1.55,w*.42),1.66,2.05,m.concrete);
  kit.box(hatchX,top+1.7,cz+.3,Math.min(1.75,w*.47),.12,2.25,m.metal);
  kit.box(hatchX,top+.72,cz+1.34,.65,1.3,.04,dark);
  // A bounded roof access path connects the headhouse to the rear landing.
  if(circulation.external){
   const accessX=features.stairSide<0?landingXs[0]:landingXs[1];
   kit.box(accessX,top+.08,rear-.5,.7,.14,1.15,m.concrete);
   for(const dx of [-.24,.24])kit.box(accessX+dx,top-.3,rear-.18,.035,2.1,.035,m.metal);
   if(detail!=='far')for(let k=0;k<6;k++)kit.box(accessX,top-1.2+k*.28,rear-.18,.5,.025,.035,m.metal);
  }
  const equipmentX=-w/2+.8;
  for(let i=0;i<Math.min(3,bayCount);i++){
   const x=equipmentX+i*1.05;
   kit.box(x,top+.18,rear+1.12,.95,.17,.65,m.concrete);
   if(detail==='far')kit.box(x,top+.55,rear+1.15,.8,.65,.45,m.metal);else component(kit,'ac',x,top+.55,rear+1.15,m);
  }
  if(features.roof!==1&&w>=6.2){
  kit.cylinder(-w/2+.75,top+.88,front-.65,.43,1.25,m.concrete,12);
  kit.cylinder(-w/2+.75,top+1.54,front-.65,.46,.08,m.metal,12);
  kit.box(-w/2+.75,top+.2,front-.65,1.04,.23,1.04,stone);
  line(new THREE.Vector3(-w/2+.75,top+.38,front-.65),new THREE.Vector3(-w/2+.75,top+.38,rear+.35),.035,m.metal);
  }
  if(features.roof===1)for(let i=0;i<2;i++){
   kit.box(-w/2+.8+i*.95,top+.42,front-.65,.78,.7,1.15,m.metal);
   for(let k=0;k<4;k++)kit.box(-w/2+.8+i*.95,top+.25+k*.12,cz+.09,.65,.025,.025,dark);
  }
  if(features.roof===2){
   const mastX=w/2-.65;kit.cylinder(mastX,top+1.25,rear+.7,.025,2.5,m.metal,6);
   for(const dy of [.7,1.1,1.5])kit.box(mastX,top+dy,rear+.7,.9,.025,.025,m.metal);
  }
  kit.box(0,top+.14,cz,.34,.035,d-.3,stone);
 }
 // Bikes belong beside the entrance, outside the central door/step route.
 if(complete&&detail!=='far'&&b.width>=10){
  const count=1+architecturalChoice(seed,70)%2;
  for(let i=0;i<count;i++)parkedBicycle(kit,entrance.parkingSide*(w/2-1.05),.06,front+1.35+i*.4,m.metal,dark,fabric[(seed+i)%fabric.length]);
 }
 if(b.stage==='renovation')for(const x of [-w/2+.15,w/2-.15])kit.box(x,base+2,edge+.14,.07,4,.07,m.metal);
 if(b.stage==='demolition')addDemolitionSite(kit,b,art);
 const group=kit.finish();group.userData={buildingId:b.id,apartmentPilot:{width:w,depth:d,floors:b.floors,installedFloors:installed,roofInstalled:complete,bays:bayCount,seed,recipeVersion:APARTMENT_RECIPE_VERSION,recipe:recipe.id,features,detail,balconyDepth:recipe.balconyDepth,coreWidth,circulation,entrance}};
 group.traverse(o=>o.userData.buildingId=b.id);
 return group;
}
