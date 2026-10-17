import type {World} from '../../types';
import type {BusinessRecipe} from './business-recipes';
import {districtAt,type CommercialDistrict} from './commercial-district';
import {architecturalChoice} from './variation';
export const MEDIA_LANDMARK_FORMS=['wrap','stack','frame'] as const;
/** Sparse architectural overrides near stations or major road junctions, independent of animation/activity. */
export function resolveMediaLandmarks(world:Pick<World,'nodes'|'roads'|'buildings'|'seed'>,recipes:ReadonlyMap<number,BusinessRecipe>,mode:CommercialDistrict='standard'):Map<number,BusinessRecipe>{
 const result=new Map(recipes),degree=new Map<number,number>();
 for(const road of world.roads){degree.set(road.from,(degree.get(road.from)??0)+1);degree.set(road.to,(degree.get(road.to)??0)+1);}
 const centers=world.nodes.filter(n=>n.kind==='station'||(degree.get(n.id)??0)>=3),chosen:typeof world.buildings=[];
 const candidates=world.buildings.filter(b=>{const r=recipes.get(b.id);return r&&!r.towerForm&&!b.anchor&&r.era==='contemporary'&&['shibuya','shinjuku','akihabara'].includes(r.district??districtAt(b,mode,world.seed)??'')&&['shop','cafe','office'].includes(b.kind)&&b.floors>=6&&b.floors<=20&&b.width>=18&&b.depth>=14;})
  .map(b=>({b,distance:Math.min(...centers.map(n=>Math.hypot(n.x-b.x,n.z-b.z)))})).filter(c=>c.distance<=150).sort((a,b)=>a.distance-b.distance||a.b.id-b.b.id);
 for(const {b} of candidates){if(chosen.some(o=>Math.hypot(o.x-b.x,o.z-b.z)<100))continue;
  const r=recipes.get(b.id)!;result.set(b.id,{...r,district:r.district??districtAt(b,mode,world.seed),id:`${r.id}/landmark`,mediaLandmark:MEDIA_LANDMARK_FORMS[architecturalChoice(b.appearance_seed,760)%3]});chosen.push(b);if(chosen.length===3)break;
 }return result;
}
