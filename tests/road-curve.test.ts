import {expect,it} from 'vitest';
import {bezierRoad,sweepRoad} from '../web/src/render/road-curve';
import {Kit,disposeGroup} from '../web/src/render/kit';
import {Mesh,MeshBasicMaterial} from 'three/webgpu';
// Adjacent authoritative cubics, with a common tangent at (100,0).
const a={x:0,z:0},joint={x:100,z:0},b={x:200,z:30};
const c={x:80,z:-2},d={x:120,z:2};
const first=()=>bezierRoad(a,{x:30,z:0},c,joint);
it('shares both swept edges at an authoritative bend, including reversed storage',()=>{
 for(const reversed of [false,true]){
 const left=first(),right=reversed?bezierRoad(b,{x:170,z:21},d,joint):bezierRoad(joint,d,{x:170,z:21},b);
 const p=left.at(left.length),q=right.at(reversed?right.length:0),sign=reversed?-1:1;
 for(const offset of [-10.4,0,10.4]){expect(p.x+p.uz*offset).toBeCloseTo(q.x+q.uz*offset*sign,8);expect(p.z-p.ux*offset).toBeCloseTo(q.z-q.ux*offset*sign,8);}
 }
});
it('builds finite indexed curved slabs and clips construction at the requested arc distance',()=>{
 const path=first(),kit=new Kit(),mat=new MeshBasicMaterial();
 sweepRoad(kit,path,0,path.length/2,0,11,20,.3,mat);const group=kit.finish();group.traverse(o=>{if(o instanceof Mesh){expect(o.geometry.index).not.toBeNull();const p=o.geometry.getAttribute('position');for(let i=0;i<p.count;i++){expect(Number.isFinite(p.getX(i))).toBe(true);expect(p.getX(i)).toBeLessThan(55);}}});disposeGroup(group);mat.dispose();
});

it('joins actual deck, median and barrier mesh sections without wedges',()=>{
 const material=new MeshBasicMaterial();
 // Check emitted Float32 vertices, not just analytical centerline samples.
 const section=(path:ReturnType<typeof bezierRoad>,end:boolean,offset:number,width:number,height:number)=>{
  const kit=new Kit();sweepRoad(kit,path,0,path.length,offset,11,width,height,material);
  const group=kit.finish(),p=path.at(end?path.length:0),vertices=new Map<string,number[]>();
  group.traverse(o=>{if(o instanceof Mesh){const a=o.geometry.getAttribute('position');
   for(let i=0;i<a.count;i++){
    const x=a.getX(i),y=a.getY(i),z=a.getZ(i);
    if(Math.abs((x-p.x)*p.ux+(z-p.z)*p.uz)<1e-4){
     const v=[x,y,z];vertices.set(v.map(n=>n.toFixed(4)).join(','),v);
    }
   }
  }});disposeGroup(group);
  return [...vertices.values()].sort((a,b)=>a[0]-b[0]||a[1]-b[1]||a[2]-b[2]);
 };
 try{for(const reversed of [false,true]){
  const right=reversed?bezierRoad(b,{x:170,z:21},d,joint):bezierRoad(joint,d,{x:170,z:21},b);
  for(const [offset,width,height] of [[0,20.8,.3],[0,.65,.76],[-10.1,.3,1.2],[10.1,.3,1.2]]){
   const leftSection=section(first(),true,offset,width,height);
   const rightSection=section(right,reversed,reversed?-offset:offset,width,height);
   expect(leftSection).toHaveLength(4);expect(rightSection).toHaveLength(4);
   leftSection.forEach((v,i)=>v.forEach((n,j)=>expect(n).toBeCloseTo(rightSection[i][j],4)));
  }
 }}finally{material.dispose();}
});
