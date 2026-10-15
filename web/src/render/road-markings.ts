import type { Point, Road, World } from '../types';
import { roadWidth } from './road-elevation';
export type MarkingTrim = { start: number; end: number };
type Arm = { road: Road; node: number; end: Point };
/** Keep straight-road paint outside the full width of connected ground-road approaches. */
export function roadMarkingTrims(world:Pick<World,'nodes'|'roads'>):Map<number,MarkingTrim> {
  const nodes=new Map(world.nodes.map(n=>[n.id,n])),incident=new Map<number,Arm[]>();
  for(const road of world.roads){
    if(road.stage!=='open'||road.kind==='rail'||road.kind==='expressway')continue;
    const a=nodes.get(road.from),b=nodes.get(road.to);if(!a||!b||Math.hypot(a.x-b.x,a.z-b.z)<1e-6)continue;
    for(const [node,end] of [[a,b],[b,a]]){const list=incident.get(node.id)??[];list.push({road,node:node.id,end});incident.set(node.id,list);}
  }
  const result=new Map<number,MarkingTrim>();
  for(const [id,arms] of incident){
    const node=nodes.get(id)!;
    for(const arm of arms){
      if(arm.road.kind!=='arterial'&&arm.road.kind!=='local')continue;
      const dx=arm.end.x-node.x,dz=arm.end.z-node.z,length=Math.hypot(dx,dz),ux=dx/length,uz=dz/length;
      const halfPaint=arm.road.kind==='arterial'?roadWidth(arm.road.kind)/4+.07:roadWidth(arm.road.kind)/2-.45+.07;
      let trim=0;
      for(const other of arms){
        if(other.road.id===arm.road.id)continue;
        const ox=other.end.x-node.x,oz=other.end.z-node.z,ol=Math.hypot(ox,oz),vx=ox/ol,vz=oz/ol;
        const sine=Math.abs(ux*vz-uz*vx);if(sine<1e-5)continue; // continuous collinear street
        const halfOther=roadWidth(other.road.kind)/2+(other.road.kind==='bridge'?1.4:0);
        // Intersect the finite neighbor rectangle with this road's paint band.
        // Using only the angle would erase long stretches beside short acute arms.
        let polygon=[[-halfOther,0],[halfOther,0],[halfOther,ol],[-halfOther,ol]].map(([side,along])=>{
          const x=vx*along-vz*side,z=vz*along+vx*side;
          return {x:x*ux+z*uz,z:-x*uz+z*ux};
        });
        for(const sign of [-1,1]){
          const out:Point[]=[];
          for(let i=0;i<polygon.length;i++){
            const a=polygon[i],b=polygon[(i+1)%polygon.length],ca=halfPaint-sign*a.z,cb=halfPaint-sign*b.z;
            if(ca>=0)out.push(a);
            if((ca>=0)!==(cb>=0)){const t=ca/(ca-cb);out.push({x:a.x+(b.x-a.x)*t,z:a.z+(b.z-a.z)*t});}
          }
          polygon=out;
        }
        if(polygon.length)trim=Math.max(trim,Math.min(length,Math.max(0,...polygon.map(p=>p.x))+.2));
      }
      const entry=result.get(arm.road.id)??{start:0,end:0};entry[arm.road.from===id?'start':'end']=trim;result.set(arm.road.id,entry);
    }
  }
  return result;
}
