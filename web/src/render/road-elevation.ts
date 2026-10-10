import type { Point } from '../types';
export type BridgeSpan={a:Point;b:Point};
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
