import * as THREE from 'three/webgpu';
import { Kit } from './kit';
import type { ArtMaterials } from './materials';
import { ARTERIAL_SIDEWALK, roadWidth } from './road-elevation';
import { streetLightPlacements } from './street-lighting';
import { distanceToSegment, SpatialCells } from './tree-occlusion';
import type { Parcel, Point, World } from '../types';

/** One instanced tree outside the seeded groves: x/z trunk base, s grove-scale size. */
export interface PlantedTree { x:number; z:number; s:number; color:string }

export const STREET_TREE_SPACING=9;
/** Gap at segment ends; corners are cleared per side by the crossing-street keep-out. */
const STREET_TREE_END_MARGIN=2;
const STREET_TREE_DEAD_END_MARGIN=12;
/** Trunks stand in the outer part of the 2.5 m arterial sidewalk, clear of the lamp line. */
export const STREET_TREE_OFFSET=roadWidth('arterial')/2+ARTERIAL_SIDEWALK*.6;
const STREET_TREE_LAMP_CLEARANCE=3;
const STREET_TREE_DOOR_CLEARANCE=2;
const STREET_TREE_STOP_CLEARANCE=7;
export const MAX_STREET_TREES=900;
const STREET_TREE_COLORS=['#7f9a5e','#8ca664','#6f8c5c','#97a86a'];
const PARK_TREE_COLORS=['#6d8a5f','#5f7e5c','#7d955f'];
const BLOSSOM_COLORS=['#e5bdba','#edcfc4'];
const SHRINE_TREE_COLORS=['#4f6e57','#5a7658','#46664f'];

function hash(x:number,z:number,salt=0){
  let h=Math.imul(Math.round(x*10)^0x9e3779b1,0x85ebca6b)^Math.imul(Math.round(z*10)^salt,0xc2b2ae35);
  h^=h>>>16;h=Math.imul(h,0x7feb352d);h^=h>>>15;
  return (h>>>0)/4294967296;
}

const OBSTACLE_CELL=24;

/**
 * Deterministic ginkgo/zelkova rows on both arterial sidewalks. Trees keep
 * clear of junctions, cross streets, lamps, building entries, bus stops and
 * the elevated rail and expressway corridors.
 */
export function streetTreePlacements(world:Pick<World,'nodes'|'roads'|'building_access'|'bus_lines'>):PlantedTree[]{
  const nodes=new Map(world.nodes.map(node=>[node.id,node]));
  const finite=(...points:Point[])=>points.every(p=>Number.isFinite(p.x)&&Number.isFinite(p.z));
  const segments=world.roads.flatMap(road=>{const a=nodes.get(road.from),b=nodes.get(road.to);return a&&b&&road.stage!=='survey'&&finite(a,b)?[{road,a,b}]:[];});
  // Obstacles register in every grid cell their clearance reaches, so each
  // candidate tests only nearby lamps, entries, stops and crossing roads.
  const lamps=new SpatialCells<Point>(OBSTACLE_CELL),doors=new SpatialCells<readonly [Point,Point]>(OBSTACLE_CELL);
  const stops=new SpatialCells<Point>(OBSTACLE_CELL),crossings=new SpatialCells<(typeof segments)[number]>(OBSTACLE_CELL);
  for(const lamp of streetLightPlacements(world))lamps.insert(lamp,lamp.x,lamp.z,STREET_TREE_LAMP_CLEARANCE);
  for(const access of world.building_access??[]){const [door,curb]=access.points;if(!finite(door,curb))continue;const r=STREET_TREE_DOOR_CLEARANCE;doors.insertBox(access.points,Math.min(door.x,curb.x)-r,Math.min(door.z,curb.z)-r,Math.max(door.x,curb.x)+r,Math.max(door.z,curb.z)+r);}
  for(const stop of (world.bus_lines??[]).flatMap(line=>line.stop_positions??[]))if(finite(stop))stops.insert(stop,stop.x,stop.z,STREET_TREE_STOP_CLEARANCE);
  for(const segment of segments){const {a,b}=segment,r=crossingClearance(segment.road.kind);crossings.insertBox(segment,Math.min(a.x,b.x)-r,Math.min(a.z,b.z)-r,Math.max(a.x,b.x)+r,Math.max(a.z,b.z)+r);}
  // Master-plan arterials split at every node, and most nodes are T-junctions
  // with a street on one side only. Corners are cleared per side by the
  // crossing keep-out below; only dead ends keep a full margin.
  const degree=new Map<number,number>();
  for(const {road} of segments)if(road.kind!=='rail'&&road.kind!=='expressway')for(const id of [road.from,road.to])degree.set(id,(degree.get(id)??0)+1);
  const margin=(id:number)=>(degree.get(id)??0)>1?STREET_TREE_END_MARGIN:STREET_TREE_DEAD_END_MARGIN;
  const continues=(road:(typeof segments)[number],other:(typeof segments)[number])=>{
    if(other.road.kind!=='arterial'||![other.road.from,other.road.to].some(id=>id===road.road.from||id===road.road.to))return false;
    const ux=road.b.x-road.a.x,uz=road.b.z-road.a.z,vx=other.b.x-other.a.x,vz=other.b.z-other.a.z;
    return Math.abs(ux*vx+uz*vz)>.97*Math.hypot(ux,uz)*Math.hypot(vx,vz);
  };
  const result:PlantedTree[]=[];
  for(const segment of [...segments].sort((x,y)=>x.road.id-y.road.id)){
    const {road,a,b}=segment;
    if(road.kind!=='arterial'||road.stage!=='open'||!road.connected)continue;
    const dx=b.x-a.x,dz=b.z-a.z,length=Math.hypot(dx,dz);
    const startMargin=margin(road.from),available=length-startMargin-margin(road.to);
    if(available<0)continue;
    const intervals=Math.floor(available/STREET_TREE_SPACING),first=startMargin+(available-intervals*STREET_TREE_SPACING)/2;
    const normalX=dz/length,normalZ=-dx/length;
    for(let index=0;index<=intervals;index++){
      const t=(first+index*STREET_TREE_SPACING)/length;
      for(const side of [-1,1]){
        const p={x:a.x+dx*t+side*normalX*STREET_TREE_OFFSET,z:a.z+dz*t+side*normalZ*STREET_TREE_OFFSET};
        if(lamps.at(p.x,p.z).some(lamp=>Math.hypot(lamp.x-p.x,lamp.z-p.z)<STREET_TREE_LAMP_CLEARANCE))continue;
        if(doors.at(p.x,p.z).some(([door,curb])=>distanceToSegment(p,door,curb)<STREET_TREE_DOOR_CLEARANCE))continue;
        if(stops.at(p.x,p.z).some(stop=>Math.hypot(stop.x-p.x,stop.z-p.z)<STREET_TREE_STOP_CLEARANCE))continue;
        if(crossings.at(p.x,p.z).some(other=>other.road.id!==road.id&&!continues(segment,other)&&distanceToSegment(p,other.a,other.b)<crossingClearance(other.road.kind)))continue;
        const r=hash(p.x,p.z,road.id);
        result.push({...p,s:1.35+r*.3,color:STREET_TREE_COLORS[Math.floor(r*STREET_TREE_COLORS.length)%STREET_TREE_COLORS.length]});
        if(result.length>=MAX_STREET_TREES)return result;
      }
    }
  }
  return result;
}

/**
 * Trunk keep-out around another road's centerline: open sight lines at street
 * corners and room under the 7 m rail viaduct. The expressway deck is 11 m up
 * on single central piers, so ~6 m street trees only avoid the pier line.
 */
function crossingClearance(kind:string){return kind==='expressway'?3:roadWidth(kind)/2+(kind==='rail'?3:4);}

/** Protected master-plan lots with nothing built on them are the city's pocket parks. */
export function pocketParks(world:Pick<World,'parcels'|'buildings'>):Parcel[]{
  const built=new Set(world.buildings.map(building=>building.parcel_id));
  return world.parcels.filter(parcel=>parcel.protected&&!built.has(parcel.id)&&parcel.width>=6&&parcel.depth>=6&&[parcel.x,parcel.z,parcel.width,parcel.depth].every(Number.isFinite)).sort((a,b)=>a.id-b.id);
}

/** Maps lot-local coordinates (+Z toward the frontage street) into the world. */
class LotFrame {
  private readonly cos:number;
  private readonly sin:number;
  constructor(private kit:Kit,private x:number,private z:number,private rotation:number){this.cos=Math.cos(rotation);this.sin=Math.sin(rotation);}
  point(lx:number,lz:number){return {x:this.x+lx*this.cos+lz*this.sin,z:this.z-lx*this.sin+lz*this.cos};}
  place(geometry:THREE.BufferGeometry,lx:number,y:number,lz:number,material:THREE.Material,yaw=0){
    geometry.rotateY(yaw+this.rotation);const p=this.point(lx,lz);geometry.translate(p.x,y,p.z);this.kit.add(geometry,material);
  }
  box(lx:number,y:number,lz:number,w:number,h:number,d:number,material:THREE.Material,yaw=0,tiltX=0){
    const geometry=new THREE.BoxGeometry(w,h,d);if(tiltX)geometry.rotateX(tiltX);this.place(geometry,lx,y,lz,material,yaw);
  }
  post(lx:number,y:number,lz:number,radius:number,h:number,material:THREE.Material){this.place(new THREE.CylinderGeometry(radius,radius,h,6),lx,y,lz,material);}
}

/**
 * Neighborhood pocket park furniture on a vacant protected lot: gravel ground,
 * a low curb and mesh fence with a street-facing entry, a hedge, playground
 * equipment scaled to the lot, benches, a lamp and a lit vending machine.
 * Returns the park's trees separately so they share the instanced tree batches.
 */
export function addPocketPark(kit:Kit,parcel:Parcel,art:ArtMaterials):PlantedTree[]{
  const w=parcel.width,d=parcel.depth,frame=new LotFrame(kit,parcel.x,parcel.z,parcel.rotation??0);
  const seed=hash(parcel.x,parcel.z,parcel.id);
  const gravel=art.get('#cbbd98'),curb=art.get('#b4b1a5'),fence=art.get('#5b7866',.7),hedge=art.get('#4d7057');
  const timber=art.get('#866a52'),steel=art.get('#7d8780',.6),red=art.get('#c05a43',.6),yellow=art.get('#d9b24f',.6),blue=art.get('#4f7e9b',.6),sand=art.get('#dccca0');
  const innerW=w-.6,innerD=d-.6,entry=Math.min(3.2,innerW*.4);
  frame.box(0,.06,0,innerW,.12,innerD,gravel);
  // Curb and 1.1 m mesh fence; the street side keeps a centered entry.
  for(const side of [-1,1]){
    frame.box(side*innerW/2,.15,0,.22,.3,innerD,curb);
    frame.box(side*innerW/2,1.05,0,.06,.06,innerD,fence);
    frame.box((side*(innerW+entry))/4,.15,innerD/2,(innerW-entry)/2,.3,.22,curb);
    frame.box((side*(innerW+entry))/4,1.05,innerD/2,(innerW-entry)/2,.06,.06,fence);
    frame.post(side*entry/2,.5,innerD/2+.05,.09,.7,curb);
  }
  frame.box(0,.15,-innerD/2,innerW,.3,.22,curb);
  frame.box(0,1.05,-innerD/2,innerW,.06,.06,fence);
  const posts:Array<[number,number]>=[];
  for(let x=-innerW/2;x<=innerW/2+1e-3;x+=innerW/Math.max(1,Math.round(innerW/2.2)))posts.push([x,-innerD/2]);
  for(let z=-innerD/2;z<=innerD/2+1e-3;z+=innerD/Math.max(1,Math.round(innerD/2.2)))posts.push([-innerW/2,z],[innerW/2,z]);
  for(const [x,z]of posts)if(Math.abs(z-innerD/2)>.01||Math.abs(x)>entry/2)frame.box(x,.6,z,.06,1,.06,fence);
  // A clipped hedge along the rear fence screens the neighbors' back walls.
  frame.box(0,.45,-innerD/2+.65,innerW-1.2,.75,.9,hedge);
  const trees:PlantedTree[]=[];
  const blossom=seed<.3;
  const addTree=(lx:number,lz:number,index:number)=>{const p=frame.point(lx,lz),r=hash(p.x,p.z,index);trees.push({...p,s:1.6+r*.7,color:blossom&&index===0?BLOSSOM_COLORS[Math.floor(r*2)]:PARK_TREE_COLORS[Math.floor(r*PARK_TREE_COLORS.length)%PARK_TREE_COLORS.length]});};
  addTree(-innerW/2+1.8,-innerD/2+1.9,0);addTree(innerW/2-1.8,-innerD/2+1.9,1);
  if(innerD>13)addTree((seed<.5?-1:1)*(innerW/2-1.6),1,2);
  // Swing frame in the rear half, slide or sandbox opposite, scaled to the lot.
  const flip=seed<.5?-1:1,playZ=-innerD*.12;
  if(innerW>=7){
    const swingX=-flip*innerW*.22,span=Math.min(3,innerW*.32);
    for(const end of [-1,1])for(const leg of [-1,1])frame.box(swingX+end*span/2,1.15,playZ+leg*.45,.09,2.3,.09,red,0,leg*.2);
    frame.box(swingX,2.2,playZ,span+.2,.1,.1,red);
    for(const seat of [-1,1]){const x=swingX+seat*span/4;frame.box(x,1.35,playZ,.03,1.7,.03,steel);frame.box(x,.5,playZ,.5,.06,.24,yellow);}
  }
  if(innerW>=7&&innerD>=9){
    const slideX=flip*innerW*.22;
    for(const x of [-.4,.4])for(const z of [-.4,.4])frame.post(slideX+x,.75,playZ-1+z,.05,1.5,blue);
    frame.box(slideX,1.5,playZ-1,1,.1,1,blue);
    frame.box(slideX,.85,playZ+.55,.7,.06,2.6,yellow,0,.55);
    for(let step=0;step<4;step++)frame.box(slideX,.3+step*.36,playZ-1.6,.8,.05,.12,steel);
  }else{
    const sandX=flip*innerW*.2;
    frame.box(sandX,.1,playZ,2.2,.08,2.2,sand);
    for(const side of [-1,1]){frame.box(sandX+side*1.15,.17,playZ,.16,.22,2.46,timber);frame.box(sandX,.17,playZ+side*1.15,2.3,.22,.16,timber);}
  }
  // Benches face the play area from the street half of the lot.
  for(const side of [-1,1]){
    if(innerW<6&&side>0)continue;
    const x=side*(innerW/2-.75),z=innerD*.2;
    frame.box(x,.45,z,.45,.06,1.6,timber);frame.box(x-side*.2,.7,z,.06,.42,1.6,timber);
    for(const end of [-.65,.65])frame.box(x,.23,z+end,.42,.44,.06,steel);
  }
  // Lamp and a vending machine beside the entry, both lit after dark.
  const lampX=entry/2+.8;
  frame.post(lampX,1.9,innerD/2-.6,.06,3.8,steel);
  frame.box(lampX,3.85,innerD/2-.6,.3,.22,.3,art.get('#ffd38a',.5,true));
  const vendX=-(entry/2+1.1);
  frame.box(vendX,.95,innerD/2-.75,.95,1.8,.7,art.get('#e4e6e2',.55));
  frame.box(vendX,1.15,innerD/2-.39,.78,1.1,.03,art.get('#f4efd8',.5,true,'shop'));
  return trees;
}

/**
 * Chinju-no-mori shrine grove on the master plan's protected land: a dense
 * evergreen canopy, a stone approach and a vermilion torii facing the city.
 */
export function addShrineGrove(kit:Kit,polygon:readonly Point[],art:ArtMaterials):PlantedTree[]{
  if(polygon.length<3||!polygon.every(p=>Number.isFinite(p.x)&&Number.isFinite(p.z)))return [];
  const minX=Math.min(...polygon.map(p=>p.x)),maxX=Math.max(...polygon.map(p=>p.x)),minZ=Math.min(...polygon.map(p=>p.z)),maxZ=Math.max(...polygon.map(p=>p.z));
  const cx=(minX+maxX)/2,cz=(minZ+maxZ)/2;
  // The approach runs from the edge nearest the city center toward the hall.
  const towardX=-cx,towardZ=-cz,eastWest=Math.abs(towardX)*(maxZ-minZ)>Math.abs(towardZ)*(maxX-minX);
  const rotation=eastWest?(towardX>0?Math.PI/2:-Math.PI/2):(towardZ>0?0:Math.PI);
  const halfAlong=eastWest?(maxX-minX)/2:(maxZ-minZ)/2,halfAcross=eastWest?(maxZ-minZ)/2:(maxX-minX)/2;
  const frame=new LotFrame(kit,cx,cz,rotation);
  const stone=art.get('#b9b4a6'),vermilion=art.get('#c4472f',.6),black=art.get('#2f2b29'),hall=art.get('#8a6a4e'),roof=art.get('#4d4e4c'),moss=art.get('#6f8462');
  frame.box(0,.06,halfAlong*.3,3,.12,halfAlong*1.4,stone);
  frame.box(0,.05,0,halfAcross*2-2,.06,halfAlong*2-2,moss);
  const gateZ=halfAlong-3;
  for(const side of [-1,1])frame.post(side*2.2,2.4,gateZ,.22,4.8,vermilion);
  frame.box(0,4.95,gateZ,6.6,.32,.5,black);frame.box(0,4.6,gateZ,6,.3,.4,vermilion);frame.box(0,3.9,gateZ,5.2,.22,.3,vermilion);
  const hallZ=-halfAlong*.3;
  frame.box(0,.4,hallZ,8,.8,6,stone);frame.box(0,2.2,hallZ,6.4,2.8,4.6,hall);
  frame.box(0,4.1,hallZ+1.2,8,.25,4,roof,0,-.45);frame.box(0,4.1,hallZ-1.2,8,.25,4,roof,0,.45);
  for(const side of [-1,1])frame.post(side*1.2,.7,gateZ-3,.25,1.4,stone);
  const trees:PlantedTree[]=[];
  for(let x=-halfAcross+3;x<=halfAcross-3;x+=6.5)for(let z=-halfAlong+3;z<=halfAlong-3;z+=6.5){
    const r=hash(x+cx,z+cz,0x5a1),lx=x+(r-.5)*3,lz=z+(hash(z,x,0x5a2)-.5)*3;
    if(Math.abs(lx)<4.5&&lz>hallZ-5)continue;
    const p=frame.point(lx,lz);
    trees.push({...p,s:2.6+r*1.1,color:SHRINE_TREE_COLORS[Math.floor(r*SHRINE_TREE_COLORS.length)%SHRINE_TREE_COLORS.length]});
  }
  return trees;
}

/** Signature of the inputs that place parks and street trees. */
export function parkSignature(world:World){
  return `${pocketParks(world).map(p=>`${p.id}:${p.x}:${p.z}:${p.width}:${p.depth}:${p.rotation??0}`).join(',')}|${world.building_access?.length??0}|${(world.bus_lines??[]).map(line=>line.stop_positions?.length??0).join(',')}`;
}
