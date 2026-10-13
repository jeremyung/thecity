import { BoxGeometry, type Material } from 'three/webgpu';
import type { Point } from '../types';
import { Kit } from './kit';

export interface RoadCurve { length:number; at:(distance:number)=>Point & {ux:number;uz:number} }
// Curves arrive from the complete authoritative plan; rendering never infers
// tangents from the subset of roads that construction has already revealed.
export function bezierRoad(a:Point,c:Point,d:Point,b:Point):RoadCurve{
  const evaluate=(t:number)=>{const s=1-t;const dx=3*s*s*(c.x-a.x)+6*s*t*(d.x-c.x)+3*t*t*(b.x-d.x),dz=3*s*s*(c.z-a.z)+6*s*t*(d.z-c.z)+3*t*t*(b.z-d.z),l=Math.hypot(dx,dz)||1;return {x:s*s*s*a.x+3*s*s*t*c.x+3*s*t*t*d.x+t*t*t*b.x,z:s*s*s*a.z+3*s*s*t*c.z+3*s*t*t*d.z+t*t*t*b.z,ux:dx/l,uz:dz/l};};
  const steps=Math.max(16,Math.ceil((Math.hypot(c.x-a.x,c.z-a.z)+Math.hypot(d.x-c.x,d.z-c.z)+Math.hypot(b.x-d.x,b.z-d.z))/1));
  const lengths=[0];let previous=evaluate(0);
  for(let i=1;i<=steps;i++){const p=evaluate(i/steps);lengths.push(lengths[i-1]+Math.hypot(p.x-previous.x,p.z-previous.z));previous=p;}
  const length=lengths[steps];
  return {length,at(distance){const s=Math.max(0,Math.min(length,distance));let lo=0,hi=steps;while(lo+1<hi){const mid=(lo+hi)>>1;if(lengths[mid]<s)lo=mid;else hi=mid;}return evaluate((lo+(s-lengths[lo])/(lengths[hi]-lengths[lo]||1))/steps);}};
}

/** Indexed sweep shares an exact endpoint section with its neighboring road. */
export function sweepRoad(kit:Kit,path:RoadCurve,from:number,to:number,across:number,y:number,width:number,height:number,material:Material){
  if(to-from<.001)return;
  const geometry=new BoxGeometry(width,height,to-from,1,1,Math.max(1,Math.ceil((to-from)/2)));
  const positions=geometry.getAttribute('position');
  for(let i=0;i<positions.count;i++){const p=path.at(from+positions.getZ(i)+(to-from)/2),x=positions.getX(i)+across;positions.setXYZ(i,p.x+p.uz*x,y+positions.getY(i),p.z-p.ux*x);}
  geometry.computeVertexNormals();kit.add(geometry,material);
}
