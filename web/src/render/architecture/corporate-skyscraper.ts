import * as THREE from 'three/webgpu';
import type {Building} from '../../types';
import type {ArtMaterials} from '../materials';
import {paneSeed,tagWindowPane,windowLived,windowMaterials} from '../building-surfaces';
import {Kit} from './geometry-kit';
import {apartmentConstruction,addDemolitionSite} from './lifecycle';
import type {BusinessBuildOptions} from './business';
import {addStorefrontSign} from './business-signs';
type Point={x:number;z:number};
/** Clockwise parcel-local outlines, reused for slabs, glazing, framing and crowns. */
export function corporateOutline(w:number,d:number,r:number,segments:number):Point[]{
 const points:Point[]=[];
 for(const [x,z,start] of [[w/2-r,d/2-r,-90],[w/2-r,-d/2+r,0],[-w/2+r,-d/2+r,90],[-w/2+r,d/2-r,180]])for(let n=0;n<=segments;n++){
  const a=(start+n*90/segments)*Math.PI/180;points.push({x:x+r*Math.cos(a),z:z-r*Math.sin(a)});
 }
 return points;
}
/** Analytic outward normals keep reflection directions continuous across curved chords. */
export function corporateGlassNormal(x:number,z:number,w:number,d:number,r:number):THREE.Vector3{
 const cx=Math.max(-w/2+r,Math.min(w/2-r,x)),cz=Math.max(-d/2+r,Math.min(d/2-r,z));
 return new THREE.Vector3(x-cx,0,z-cz).normalize();
}
export function buildCorporateSkyscraper(b:Building,art:ArtMaterials,options:BusinessBuildOptions):THREE.Group{
 const r=options.recipe,form=r.towerForm!,detail=options.detail??'district',kit=new Kit(),fh=3.25,base=.58;
 const {installed,framed,complete}=apartmentConstruction(b),w=b.width-1,d=b.depth-2.2,podium=Math.min(6,Math.max(3,Math.floor(b.floors*.1)));
 const metal=art.get('#303b40',.45),silver=art.get('#cecdc2'),core=art.get(r.wall,r.roughness),roof=art.get('#585f5b');
 const tint=form==='rounded'?'#9baeb4':form==='chamfered'?'#497784':form==='crown'?'#476773':'#688b99';
 const glass=()=>art.shared(`corporate-glass/${tint}`,()=>windowMaterials('office',{night:art.night,occupancy:art.occupancy},art.toonRamp,tint));
 const segments=detail==='close'?6:detail==='district'?4:2,lived=windowLived(b);
 const plates=(f:number)=>{
  if(f<podium)return [{w,d,x:0,z:0,r:Math.min(w,d)*.12}];
  const step=form==='setback'?Math.floor(Math.max(0,f-podium)/(Math.max(1,b.floors-podium)/3))*.065:0;
  const tw=w*(.79-step),td=d*(.79-step);
  if(form==='split')return [{w:tw*.43,d:td,x:-tw*.285,z:0,r:.2},...(f<b.floors-2?[{w:tw*.43,d:td*.94,x:tw*.285,z:-td*.03,r:.2}]:[])];
  return [{w:tw,d:td,x:0,z:form==='setback'?-d*step*.2:0,r:Math.min(tw,td)*(form==='rounded'?.2:form==='chamfered'?.13:form==='crown'?.1:.035)}];
 };
 const outline=(p:ReturnType<typeof plates>[number])=>corporateOutline(p.w,p.d,p.r,form==='chamfered'?1:segments).map(v=>({x:v.x+p.x,z:v.z+p.z}));
 const slabTemplates=new Map<string,THREE.BufferGeometry>();
 const slab=(points:Point[],y:number,h:number,mat:THREE.Material)=>{
  const key=`${h}/${points.map(p=>`${p.x},${p.z}`).join(';')}`;
  let template=slabTemplates.get(key);
  if(!template){
   const shape=new THREE.Shape(points.map(p=>new THREE.Vector2(p.x,-p.z)));
   template=new THREE.ExtrudeGeometry(shape,{depth:h,bevelEnabled:false,steps:1});
   template.setIndex(Array.from({length:template.getAttribute('position').count},(_,i)=>i));template.rotateX(-Math.PI/2);slabTemplates.set(key,template);
  }
  kit.add(template.clone().translate(0,y,0),mat);
 };
 const edge=(a:Point,c:Point,y:number,h:number,thick:number,mat:THREE.Material)=>{
  const dx=c.x-a.x,dz=c.z-a.z;kit.box((a.x+c.x)/2,y,(a.z+c.z)/2,Math.hypot(dx,dz),h,thick,mat,Math.atan2(-dz,dx));
 };
 kit.box(0,.09,0,b.width-.1,.18,b.depth-.1,silver);
 for(let f=0;f<framed;f++)for(const p of plates(f)){
  const points=outline(p),y=base+f*fh,isInstalled=f<installed;
  slab(points,y,.16,f<podium?silver:metal);
  if(!isInstalled){for(const v of points.filter((_,i)=>i%((form==='chamfered'?1:segments)+1)===0))kit.box(v.x,y+fh/2,v.z,.25,fh,.25,silver);continue;}
  // A recessed solid floorplate prevents sky showing through unoccupied rooms.
  slab(points.map(v=>({x:p.x+(v.x-p.x)*.965,z:p.z+(v.z-p.z)*.965})),y+.16,fh-.2,core);
  for(let e=0;e<points.length;e++){
   const a=points[e],c=points[(e+1)%points.length],dx=c.x-a.x,dz=c.z-a.z,len=Math.hypot(dx,dz);if(len<.001)continue;
   const theta=Math.atan2(-dz,dx),nx=-dz/len,nz=dx/len;
   const count=Math.max(1,Math.ceil(len/(detail==='close'?1.8:detail==='district'?3.6:9)));
   const use=f<podium?'shop':'office',ph=f<podium?2.86:2.8;
   for(let n=0;n<count;n++){
    const t=(n+.5)/count,x=a.x+dx*t+nx*.018,z=a.z+dz*t+nz*.018,pw=len/count;
    const g=new THREE.PlaneGeometry(pw,ph);tagWindowPane(g,paneSeed(b.appearance_seed,x,y,z),lived,pw,ph,use);g.rotateY(theta);
    if(form!=='chamfered'){
     const normals=g.getAttribute('normal'),uv=g.getAttribute('uv');
     for(let v=0;v<normals.count;v++){
      const t=(n+uv.getX(v))/count,normal=corporateGlassNormal(a.x+dx*t-p.x,a.z+dz*t-p.z,p.w,p.d,p.r);
      normals.setXYZ(v,normal.x,normal.y,normal.z);
     }
    }
    g.translate(x,y+1.65,z);kit.add(g,glass());
    kit.box(a.x+dx*n/count+nx*.045,y+1.65,a.z+dz*n/count+nz*.045,.055,ph,.09,metal,theta);
   }
   edge(a,c,y+3.12,form==='rounded'?.14:.09,.12,form==='rounded'?silver:metal);
  }
  if(f===0&&b.stage==='open'){
   const ex=((b.appearance_seed%3)-1)*w*.2;
   kit.box(ex,y+1.4,d/2+.075,2.4,2.6,.07,metal);kit.windowContext={seed:b.appearance_seed,lived};kit.box(ex,y+1.4,d/2+.12,2.15,2.4,.04,art.window('shop'));
   kit.box(ex,y+2.9,d/2+.4,4,.13,.75,silver);
   addStorefrontSign(kit,art,{x:ex,y:y+2.73,z:d/2+.16,width:2.1,height:.26,tenant:12+b.appearance_seed%6,active:b.business_active!==false});
  }
 }
 const roofInstalled=complete&&installed===b.floors;
 if(roofInstalled)for(const p of plates(b.floors-1)){
  const top=base+b.floors*fh,points=outline(p);slab(points,top,.2,metal);
  // Recessed mechanical crown: bounded to three metres above the engine envelope.
  const crown=points.map(v=>({x:p.x+(v.x-p.x)*.83,z:p.z+(v.z-p.z)*.83}));
  slab(crown,top+.2,form==='crown'?2.7:1.6,roof);
  if(detail!=='far')for(let n=0;n<(form==='crown'?8:4);n++)for(let i=0;i<crown.length;i++)edge(crown[i],crown[(i+1)%crown.length],top+.4+n*.3,.055,.08,silver);
  kit.box(p.x,top+1.1,p.z,p.w*.32,1.7,p.d*.3,metal);
 }
 if(b.stage==='demolition')addDemolitionSite(kit,b,art);
 if(b.stage==='foundation')kit.box(0,.24,0,w,.3,d,silver);
 const group=kit.finish();for(const template of slabTemplates.values())template.dispose();group.userData.businessArchitecture={recipe:r.id,family:r.family,towerForm:form,detail,installedFloors:installed,framedFloors:framed,roofInstalled,entryX:((b.appearance_seed%3)-1)*w*.2,tenantIds:[],bands:[{from:0,count:podium,use:'shop'},{from:podium,count:b.floors-podium,use:'office'}]};return group;
}
