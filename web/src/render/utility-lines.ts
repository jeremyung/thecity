import * as THREE from 'three/webgpu';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import type { ArtMaterials } from './materials';
import { roadWidth } from './road-elevation';
import { streetLightPlacements } from './street-lighting';
import { distanceToSegment, SpatialCells } from './tree-occlusion';
import type { Building, Point, World } from '../types';

/**
 * Overhead utility lines on local streets and lanes: concrete poles on one
 * side of each street, three sagging power lines and a telecom cable between
 * neighbors, transformer cans, and service drops to nearby houses. Arterials
 * stay clear, as most wide Tokyo roads carry their cables underground.
 */
export interface PolePlacement {
  x:number; z:number;
  /** Road bearing (Three.js yaw); local +Z runs along the street. */
  heading:number; roadId:number; along:number; transformer:boolean;
  /** Straight street line the pole belongs to, shared by collinear segments, and its position along that line. */
  line:string; lineT:number;
}

export const POLE_SPACING = 30;
/** Longest wire span; consecutive poles farther apart along a line stay unlinked. */
export const MAX_SPAN = 45;
export const MAX_POLES = 1500;
/** Trunk sits just outside the carriageway edge, inside the shallow lot setback. */
export function poleOffset(kind:string){return roadWidth(kind)/2+.25;}
const LAMP_CLEARANCE = 2.5, DOOR_CLEARANCE = 1.5, STOP_CLEARANCE = 4, END_MARGIN = 5;
const CELL = 24;

function hash(a:number,b:number){
  let h=Math.imul(a^0x9e3779b1,0x85ebca6b)^Math.imul(b^0x27d4eb2f,0xc2b2ae35);
  h^=h>>>15;h=Math.imul(h,0x2c1b3c6d);h^=h>>>12;
  return (h>>>0)/4294967296;
}

/** Deterministic pole positions for open, connected local streets and lanes. */
export function utilityPolePlacements(world:Pick<World,'nodes'|'roads'|'building_access'|'bus_lines'>):PolePlacement[]{
  const nodes=new Map(world.nodes.map(node=>[node.id,node]));
  const finite=(...points:Point[])=>points.every(p=>Number.isFinite(p.x)&&Number.isFinite(p.z));
  const segments=world.roads.flatMap(road=>{const a=nodes.get(road.from),b=nodes.get(road.to);return a&&b&&road.stage!=='survey'&&finite(a,b)?[{road,a,b}]:[];});
  const lamps=new SpatialCells<Point>(CELL),doors=new SpatialCells<readonly [Point,Point]>(CELL),stops=new SpatialCells<Point>(CELL),roads=new SpatialCells<(typeof segments)[number]>(CELL);
  for(const lamp of streetLightPlacements(world))lamps.insert(lamp,lamp.x,lamp.z,LAMP_CLEARANCE);
  for(const access of world.building_access??[]){const [door,curb]=access.points;if(!finite(door,curb))continue;doors.insertBox(access.points,Math.min(door.x,curb.x)-DOOR_CLEARANCE,Math.min(door.z,curb.z)-DOOR_CLEARANCE,Math.max(door.x,curb.x)+DOOR_CLEARANCE,Math.max(door.z,curb.z)+DOOR_CLEARANCE);}
  for(const stop of (world.bus_lines??[]).flatMap(line=>line.stop_positions??[]))if(finite(stop))stops.insert(stop,stop.x,stop.z,STOP_CLEARANCE);
  for(const segment of segments){const r=roadWidth(segment.road.kind)/2+1,{a,b}=segment;roads.insertBox(segment,Math.min(a.x,b.x)-r,Math.min(a.z,b.z)-r,Math.max(a.x,b.x)+r,Math.max(a.z,b.z)+r);}
  const clear=(p:Point,roadId:number)=>!lamps.at(p.x,p.z).some(l=>Math.hypot(l.x-p.x,l.z-p.z)<LAMP_CLEARANCE)
    &&!doors.at(p.x,p.z).some(([door,curb])=>distanceToSegment(p,door,curb)<DOOR_CLEARANCE)
    &&!stops.at(p.x,p.z).some(s=>Math.hypot(s.x-p.x,s.z-p.z)<STOP_CLEARANCE)
    &&!roads.at(p.x,p.z).some(other=>other.road.id!==roadId&&distanceToSegment(p,other.a,other.b)<roadWidth(other.road.kind)/2+1);
  const result:PolePlacement[]=[];
  for(const {road,a,b} of [...segments].sort((x,y)=>x.road.id-y.road.id)){
    if(road.stage!=='open'||!road.connected||(road.kind!=='local'&&road.kind!=='lane'))continue;
    const dx=b.x-a.x,dz=b.z-a.z,length=Math.hypot(dx,dz),available=length-2*END_MARGIN;
    if(available<0)continue;
    // Collinear segments of one street share a line key, so poles keep one side
    // of the street and wires can run through junctions along it.
    const flip=dx<-1e-6||(Math.abs(dx)<=1e-6&&dz<0)?-1:1,ux=dx/length*flip,uz=dz/length*flip;
    const lineAngle=Math.round(Math.atan2(ux,uz)/.05),lineOffset=Math.round((a.x*uz-a.z*ux)/3);
    const line=`${lineAngle}:${lineOffset}`,side=(hash(lineAngle,lineOffset)<.5?-1:1)*flip;
    const heading=Math.atan2(dx,dz),offset=poleOffset(road.kind);
    const nx=dz/length*side,nz=-dx/length*side;
    const intervals=Math.max(0,Math.round(available/POLE_SPACING)),spacing=intervals?available/intervals:0;
    for(let i=0;i<=intervals;i++){
      const base=intervals?END_MARGIN+i*spacing:length/2;
      // A blocked slot slides up to 3 m along the street before it is skipped.
      for(const shift of [0,3,-3]){
        const along=Math.min(length-END_MARGIN,Math.max(END_MARGIN,base+shift)),t=along/length;
        const p={x:a.x+dx*t+nx*offset,z:a.z+dz*t+nz*offset};
        if(!clear(p,road.id))continue;
        result.push({...p,heading,roadId:road.id,along,transformer:hash(road.id,i)<.34,line,lineT:p.x*ux+p.z*uz});
        break;
      }
      if(result.length>=MAX_POLES)return result;
    }
  }
  return result;
}

const POLE_HEIGHT=10;
/** Wire attachment points in pole-local space: x across the street, y up. */
export const WIRE_POINTS:ReadonlyArray<{x:number,y:number,radius:number}>=[
  {x:-.7,y:9.5,radius:.018},{x:0,y:9.55,radius:.018},{x:.7,y:9.5,radius:.018},{x:.28,y:6.2,radius:.035},
];

function poleGeometry(){
  const parts=[
    new THREE.CylinderGeometry(.12,.17,POLE_HEIGHT,8).translate(0,POLE_HEIGHT/2,0),
    new THREE.BoxGeometry(1.7,.1,.1).translate(0,9.4,0),
    new THREE.BoxGeometry(1.2,.1,.1).translate(0,8.5,0),
    new THREE.BoxGeometry(.5,.07,.07).translate(.22,6.2,0),
    ...[-.7,0,.7].map(x=>new THREE.CylinderGeometry(.04,.05,.16,6).translate(x,9.53,0)),
    // Step bolts up the lower shaft.
    ...[2.6,3.4,4.2,5].map((y,i)=>new THREE.BoxGeometry(.32,.04,.04).translate(0,y,0).rotateY(i%2?Math.PI/2:0)),
  ];
  const merged=mergeGeometries(parts,false);
  parts.forEach(part=>part.dispose());
  if(!merged)throw new Error('Could not merge utility pole');
  return merged;
}

function local(p:PolePlacement,x:number,y:number){
  // Three.js yaw maps local +X to (cos h, -sin h) in world X/Z.
  return new THREE.Vector3(p.x+Math.cos(p.heading)*x,y,p.z-Math.sin(p.heading)*x);
}

/** Thin four-sided tube along a polyline, written straight into shared arrays. */
function addCable(points:THREE.Vector3[],radius:number,positions:number[],normals:number[],indices:number[]){
  const up=new THREE.Vector3(0,1,0);
  for(let i=0;i<points.length-1;i++){
    const a=points[i],b=points[i+1],dir=b.clone().sub(a).normalize();
    const side=new THREE.Vector3().crossVectors(dir,up);if(side.lengthSq()<1e-6)side.set(1,0,0);side.normalize();
    const lift=new THREE.Vector3().crossVectors(side,dir).normalize();
    const base=positions.length/3;
    for(const end of [a,b])for(const n of [side,lift,side.clone().negate(),lift.clone().negate()]){
      positions.push(end.x+n.x*radius,end.y+n.y*radius,end.z+n.z*radius);normals.push(n.x,n.y,n.z);
    }
    for(let k=0;k<4;k++){const k1=(k+1)%4;indices.push(base+k,base+4+k,base+k1,base+k1,base+4+k,base+4+k1);}
  }
}

/** Catenary-like sag between two attachment points. */
export function sagPoints(a:THREE.Vector3,b:THREE.Vector3,steps=8){
  const span=a.distanceTo(b),sag=.25+span*.009,points:THREE.Vector3[]=[];
  for(let i=0;i<=steps;i++){const t=i/steps;points.push(a.clone().lerp(b,t).setY(a.y+(b.y-a.y)*t-4*sag*t*(1-t)));}
  return points;
}

/** Street-facing front point of a building, at a height a service drop can reach. */
function buildingDropPoint(b:Building){
  const rotation=b.rotation,front=b.depth*.42;
  return new THREE.Vector3(b.x+Math.sin(rotation)*front,Math.min(.58+Math.max(1,b.floors)*3.25-.6,6),b.z+Math.cos(rotation)*front);
}

/** Spans between consecutive poles along each street line, up to MAX_SPAN apart. */
export function poleSpans(poles:readonly PolePlacement[]){
  const byLine=new Map<string,PolePlacement[]>();
  for(const pole of poles){const list=byLine.get(pole.line)??[];list.push(pole);byLine.set(pole.line,list);}
  const spans:Array<[PolePlacement,PolePlacement]>=[];
  for(const list of byLine.values()){
    list.sort((a,b)=>a.lineT-b.lineT);
    for(let i=0;i<list.length-1;i++){const gap=Math.hypot(list[i+1].x-list[i].x,list[i+1].z-list[i].z);if(gap>1&&gap<=MAX_SPAN)spans.push([list[i],list[i+1]]);}
  }
  return spans;
}

/** Instanced poles and transformers plus one merged wire mesh. */
export function buildUtilityLines(world:Pick<World,'nodes'|'roads'|'buildings'|'building_access'|'bus_lines'>,art:ArtMaterials):THREE.Group{
  const root=new THREE.Group();root.name='utility-lines';
  const poles=utilityPolePlacements(world);
  if(!poles.length)return root;
  const dummy=new THREE.Object3D();
  const shafts=new THREE.InstancedMesh(poleGeometry(),art.get('#a5a397'),poles.length);
  const transformers=poles.filter(p=>p.transformer);
  const cans=new THREE.InstancedMesh(new THREE.CylinderGeometry(.3,.3,.9,10),art.get('#8f9894',.6),Math.max(1,transformers.length));
  poles.forEach((p,i)=>{dummy.position.set(p.x,0,p.z);dummy.rotation.set(0,p.heading,0);dummy.updateMatrix();shafts.setMatrixAt(i,dummy.matrix);});
  transformers.forEach((p,i)=>{const at=local(p,-.42,7.3);dummy.position.copy(at);dummy.rotation.set(0,p.heading,0);dummy.updateMatrix();cans.setMatrixAt(i,dummy.matrix);});
  cans.count=transformers.length;
  for(const mesh of [shafts,cans]){mesh.castShadow=true;mesh.receiveShadow=true;mesh.computeBoundingSphere();root.add(mesh);}

  const positions:number[]=[],normals:number[]=[],indices:number[]=[];
  for(const [a,b] of poleSpans(poles))for(const w of WIRE_POINTS)addCable(sagPoints(local(a,w.x,w.y),local(b,w.x,w.y)),w.radius,positions,normals,indices);
  // Service drops from the telecom bracket to the nearest house front on the pole's side.
  const houses=world.buildings.filter(b=>b.stage==='open'&&(b.kind==='home'||b.kind==='shop'||b.kind==='cafe')&&b.floors<=3);
  const cells=new SpatialCells<Building>(CELL);for(const b of houses)cells.insert(b,b.x,b.z,12);
  for(const pole of poles){
    if(hash(pole.roadId,Math.round(pole.along))>.6)continue;
    const from=local(pole,WIRE_POINTS[3].x,WIRE_POINTS[3].y);
    let best:THREE.Vector3|null=null,bestDistance=10;
    for(const b of cells.at(pole.x,pole.z)){const to=buildingDropPoint(b),d=Math.hypot(to.x-from.x,to.z-from.z);if(d<bestDistance&&d>1.5){best=to;bestDistance=d;}}
    if(best)addCable(sagPoints(from,best,5),.012,positions,normals,indices);
  }
  if(indices.length){
    const geometry=new THREE.BufferGeometry();
    geometry.setAttribute('position',new THREE.Float32BufferAttribute(positions,3));
    geometry.setAttribute('normal',new THREE.Float32BufferAttribute(normals,3));
    geometry.setIndex(indices);geometry.computeBoundingSphere();
    const wires=new THREE.Mesh(geometry,art.get('#2c3133',.6));wires.name='utility-wires';
    root.add(wires);
  }
  return root;
}
