import type { Point } from '../types';
export type BridgeSpan={a:Point;b:Point};
/** Elevated rail viaduct lift above the legacy at-grade rail height. */
export const RAIL_DECK=6;
/** Expressway deck height above ground. */
export const EXPRESSWAY_DECK=11;
/** Rendered carriageway width by road kind. */
/**
 * Rendered carriageway width by road kind. The hierarchy is deliberately
 * legible: expressway 20 m (elevated), arterial and its bridges 16 m with
 * 2.5 m sidewalks, local 6.5 m shared surface, alley (lane) 4 m.
 */
export function roadWidth(kind:string){return kind==='rail'?7:kind==='expressway'?20:kind==='arterial'||kind==='bridge'?16:kind==='local'?6.5:kind==='lane'?4:9;}
/** Arterial sidewalk width on each side. */
export const ARTERIAL_SIDEWALK=2.5;
/** Deck joins street level at both ends, with bounded approach ramps. */
export function bridgeDeckHeight(distance:number,length:number){
  const ramp=Math.min(12,length*.22);
  return .3+1.7*Math.max(0,Math.min(1,distance/ramp,(length-distance)/ramp));
}
export function bridgeLift(point:Point,spans:BridgeSpan[]){
  let lift=0;
  for(const {a,b}of spans){const dx=b.x-a.x,dz=b.z-a.z,length=Math.hypot(dx,dz);if(length<1)continue;
    const along=((point.x-a.x)*dx+(point.z-a.z)*dz)/length;
    const across=Math.abs((point.x-a.x)*dz-(point.z-a.z)*dx)/length;
    if(along>=0&&along<=length&&across<=6.4)lift=Math.max(lift,bridgeDeckHeight(along,length)-.3);
  }
  return lift;
}
/** Vehicle axles follow the same deck profile used by the visible road mesh. */
export function bridgeVehiclePose(point:Point,yaw:number,wheelbase:number,spans:BridgeSpan[]){
  const dx=Math.sin(yaw)*wheelbase/2,dz=Math.cos(yaw)*wheelbase/2;
  const front=bridgeLift({x:point.x+dx,z:point.z+dz},spans),back=bridgeLift({x:point.x-dx,z:point.z-dz},spans);
  return{lift:(front+back)/2,pitch:Math.atan2(front-back,wheelbase)};
}

/** A pier must not occupy a lower crossing's carriageway or rail clearance. */
export type CrossingDeck = { a:Point; b:Point; height:number; width:number };
export function expresswayPierClear(point:Point,a:Point,b:Point,height:number,crossings:readonly CrossingDeck[]){
  const dx=b.x-a.x,dz=b.z-a.z,length=Math.hypot(dx,dz);
  if(length<1)return false;
  return !crossings.some(c=>{
    if(c.height>=height)return false;
    const cx=c.b.x-c.a.x,cz=c.b.z-c.a.z,cl=Math.hypot(cx,cz);
    // Parallel streets below the viaduct keep their central pier reservation.
    if(cl<1||Math.abs(dx*cz-dz*cx)/(length*cl)<.2)return false;
    const t=Math.max(0,Math.min(1,((point.x-c.a.x)*cx+(point.z-c.a.z)*cz)/(cl*cl)));
    return Math.hypot(point.x-c.a.x-t*cx,point.z-c.a.z-t*cz)<c.width/2+2;
  });
}
