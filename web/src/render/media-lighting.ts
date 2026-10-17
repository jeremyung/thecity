import * as THREE from 'three/webgpu';
import type {Building,World,Style} from '../types';
import type {BusinessRecipe,DisplayMode} from './architecture/business-recipes';
import type {SignageContext} from './signage';
import {MEDIA_CAMPAIGNS} from './architecture/media-campaigns';
import {architecturalChoice} from './architecture/variation';
export const MAX_MEDIA_SPILL_LIGHTS=2;
/** Fixed non-shadowing approximation of screen spill. No per-building lights or captures. */
export class MediaScreenLighting{
 readonly root=new THREE.Group();
 readonly lights=Array.from({length:MAX_MEDIA_SPILL_LIGHTS},()=>{
  const light=new THREE.SpotLight('#a8c8ce',0,48,Math.PI/3,.75,2);light.castShadow=false;this.root.add(light,light.target);return light;
 });
 private candidates:Array<Building&{spill:string}>=[];
 setWorld(world:Pick<World,'buildings'>,recipes:ReadonlyMap<number,BusinessRecipe>,signage:ReadonlyMap<number,SignageContext>,mode:DisplayMode){
  this.candidates=mode==='animated'?world.buildings.filter(b=>b.stage==='open'&&b.business_active!==false&&recipes.get(b.id)?.mediaLandmark&&signage.get(b.id)?.feature==='screen').map(b=>({...b,spill:recipes.get(b.id)?.mediaLandmark==='curved'?MEDIA_CAMPAIGNS[architecturalChoice(b.appearance_seed,763)%4].spill:['#28d8ed','#ff56ae','#5ef1c9','#9280ff'][architecturalChoice(b.appearance_seed,763)%4]})):[];
  for(const light of this.lights)light.intensity=0;
 }
 update(camera:THREE.Vector3,time:number,night:number,style:Style,details:ReadonlyMap<number,string>,ready:ReadonlyMap<number,unknown>){
  const near=this.candidates.filter(b=>details.get(b.id)!=='far'&&ready.has(b.id)&&Math.hypot(b.x-camera.x,b.z-camera.z)<170).sort((a,b)=>Math.hypot(a.x-camera.x,a.z-camera.z)-Math.hypot(b.x-camera.x,b.z-camera.z));
  this.lights.forEach((light,i)=>{
   const b=near[i];light.intensity=0;if(!b||night<=.01)return;
   const nx=Math.sin(b.rotation),nz=Math.cos(b.rotation),front=b.depth*.44;
   light.position.set(b.x+nx*(front+1),Math.min(18,b.floors*1.4),b.z+nz*(front+1));
   light.target.position.set(b.x+nx*(front+14),.1,b.z+nz*(front+14));
   light.color.set(b.spill);
   light.intensity=480*night*(style==='anime'?.6:1)*(.985+.015*Math.sin(time*.2));
  });
 }
 reset(){this.candidates=[];for(const l of this.lights)l.intensity=0;}
 dispose(){this.reset();for(const l of this.lights)l.dispose();this.root.clear();}
}
