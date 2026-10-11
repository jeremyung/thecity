import type { Material } from 'three/webgpu';
import { Kit } from './kit';
import type { Point } from '../types';
import { EXPRESSWAY_DECK, roadWidth } from './road-elevation';

/** Deck-local furniture for completed expressways; no ground-road junction furniture. */
export function addExpresswayFurniture(kit: Kit, a: Point, b: Point, materials: {
  paint: Material; concrete: Material; metal: Material; lamp: Material;
}, deckHeight=EXPRESSWAY_DECK) {
  const dx=b.x-a.x,dz=b.z-a.z,length=Math.hypot(dx,dz);
  if(length<1)return;
  const angle=Math.atan2(dx,dz),width=roadWidth('expressway'),surface=deckHeight+.15;
  const box=(across:number,along:number,height:number,w:number,h:number,d:number,material:Material)=>
    kit.box(a.x+dx*along/length+Math.cos(angle)*across,surface+height,
      a.z+dz*along/length-Math.sin(angle)*across,w,h,d,material,angle);
  // Two lanes in each direction, with shoulders and a narrow central barrier.
  box(0,length/2,.38,.65,.76,length,materials.concrete);
  for(const side of [-1,1]){
    for(const offset of [.8,width/2-1.35])box(side*offset,length/2,.025,.15,.035,length,materials.paint);
    for(let along=3;along<length-1;along+=9){
      const dashLength=Math.min(3,length-along);
      box(side*4.7,along+dashLength/2,.025,.14,.035,dashLength,materials.paint);
    }
    // Mount on the deck inside the sound barrier. Arms reach inward over lanes.
    for(let along=18;along<length-4;along+=36){
      box(side*(width/2-.55),along,3.3,.12,6.6,.12,materials.metal);
      box(side*(width/2-1.3),along,6.55,1.6,.12,.12,materials.metal);
      box(side*(width/2-2),along,6.5,.65,.16,.32,materials.lamp);
    }
  }
}
