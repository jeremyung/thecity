import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import * as THREE from 'three/webgpu';
import {BUSINESS_RECIPES,type BusinessRecipe} from '../web/src/render/architecture/business-recipes';
import {CORPORATE_SKYSCRAPERS} from '../web/src/render/architecture/corporate-tower-recipes';
import {buildBusinessBuilding} from '../web/src/render/architecture/business';
import {businessCityStudy} from '../web/src/render/architecture/business-study';
import {ApartmentResources} from '../web/src/render/architecture/resources';
import {BuildingAccessPaths} from '../web/src/render/building-access';
import {disposeGroup} from '../web/src/render/kit';
beforeEach(()=>vi.stubGlobal('document',{createElement:()=>({width:0,height:0,getContext:()=>new Proxy({} as Record<PropertyKey,unknown>,{get:(t,k)=>k==='createLinearGradient'?()=>({addColorStop(){}}):t[k]??(()=>undefined),set:(t,k,v)=>{t[k]=v;return true;}})})}));
afterEach(()=>vi.unstubAllGlobals());
it('connects authoritative walking access to glazed primary doors across recipes, seeds and rotations',()=>{
 const source=businessCityStudy(),owner=new ApartmentResources(),paths=new BuildingAccessPaths(owner.art,release=>release());
 const media:BusinessRecipe[]=(['wrap','stack','frame','curved'] as const).map(mediaLandmark=>({...BUSINESS_RECIPES[0],mediaLandmark}));
 try{for(const recipe of [...BUSINESS_RECIPES,...CORPORATE_SKYSCRAPERS,...media])for(const rotation of [0,Math.PI/2,.73])for(const seed of [0,5]){
  const large=Boolean(recipe.mediaLandmark||recipe.towerForm),b={...source.buildings[0],x:73,z:-41,width:large?38:20,depth:large?30:18,floors:large?28:6,rotation,appearance_seed:seed};
  const label=`${recipe.id}/${recipe.mediaLandmark??''}/${rotation}/${seed}`;
  const group=buildBusinessBuilding(b,owner.art,{recipe,detail:'district',displays:'off'})!;
  expect(group.userData.businessArchitecture.entryX,label).toBe(0);
  group.position.set(b.x,0,b.z);group.rotation.y=rotation;group.updateMatrixWorld(true);
  // Matches engine/src/lib.rs::building_access_segment: the engine endpoint
  // is front-centre, 0.2m beyond the parcel, regardless of architectural seed.
  const outward=new THREE.Vector3(Math.sin(rotation),0,Math.cos(rotation));
  const door=new THREE.Vector3(b.x,0,b.z).addScaledVector(outward,b.depth*.5+.2);
  const curb=door.clone().addScaledVector(outward,8);
  paths.setWorld({...source,buildings:[b],building_access:[{building_id:b.id,road_id:1,points:[{x:door.x,z:door.z},{x:curb.x,z:curb.z}]}]});
  expect(new THREE.Box3().setFromObject(paths.root).containsPoint(door),label).toBe(true);
  // Inspect actual rendered panes, not just entrance metadata. Offset within
  // the walking corridor to avoid a legitimate centre mullion on double doors.
  const origin=door.clone().addScaledVector(new THREE.Vector3(Math.cos(rotation),0,-Math.sin(rotation)),.35);origin.y=1.9;
  const panes:THREE.Mesh[]=[];group.traverse(o=>{if(o instanceof THREE.Mesh){const m=o.material as THREE.Material;if(m.userData.windowPane||m.userData.corporateGlass)panes.push(o);}});
  const hits=new THREE.Raycaster(origin,outward.clone().negate(),0,8).intersectObjects(panes,false);
  expect(hits.length,label).toBeGreaterThan(0);
  const slots=group.userData.businessArchitecture.frontageSlots as {x:number;width:number}[]|undefined;
  if(recipe.family==='mixed-use'&&slots)for(const slot of slots)expect(Math.abs(slot.x)-slot.width/2,label).toBeGreaterThanOrEqual(1.09);
  disposeGroup(group);
 }}finally{paths.reset();owner.dispose();}
},30000);
