import * as THREE from 'three/webgpu';
import { color, float, smoothstep, uv } from 'three/tsl';
import type { ArtMaterials } from './materials';
import type { World } from '../types';

export interface StreetLightPlacement {
  x:number;
  z:number;
  /** Road-level base elevation used by the rendered road and sidewalk. */
  y:number;
  /** Three.js yaw for the short arm pointing from the sidewalk into the road. */
  heading:number;
  roadId:number;
}

const MAX_LIGHTS=1_000;
const EDGE_MARGIN=10;
const TARGET_SPACING=31;
const MIN_SPACING=28;
const MAX_SPACING=34;

/** Deterministic pedestrian-side placements for streets with a finished deck. */
export function streetLightPlacements(world:Pick<World,'nodes'|'roads'>):StreetLightPlacement[]{
  const nodes=new Map(world.nodes.map(node=>[node.id,node]));
  const result:StreetLightPlacement[]=[];
  const roads=[...world.roads].sort((a,b)=>a.id-b.id);
  for(const road of roads){
    if(road.stage!=='open'||!road.connected||(road.kind!=='local'&&road.kind!=='arterial'))continue;
    const a=nodes.get(road.from),b=nodes.get(road.to);
    if(!a||!b||![a.x,a.z,b.x,b.z].every(Number.isFinite))continue;
    const dx=b.x-a.x,dz=b.z-a.z,length=Math.hypot(dx,dz);
    if(!Number.isFinite(length)||length<2*EDGE_MARGIN)continue;
    const laneWidth=road.kind==='arterial'?14:6.5;
    const angle=Math.atan2(dx,dz),normalX=Math.cos(angle),normalZ=-Math.sin(angle);
    const available=length-2*EDGE_MARGIN;
    let intervals=0,spacing=0,span=0;
    if(available<MIN_SPACING){
      // A single midpoint light is still safe on a short segment; endpoint
      // clearance is preserved and there is no inter-light spacing to violate.
      intervals=0;
    }else{
      intervals=Math.max(1,Math.round(available/TARGET_SPACING));
      spacing=available/intervals;
      if(spacing<MIN_SPACING||spacing>MAX_SPACING){
        intervals=Math.max(1,Math.floor(available/TARGET_SPACING));
        spacing=TARGET_SPACING;
      }
      span=intervals*spacing;
    }
    const firstDistance=intervals===0?length/2:(length-span)/2;
    const count=intervals===0?1:intervals+1;
    for(let index=0;index<count&&result.length<MAX_LIGHTS;index++){
      const distance=firstDistance+index*spacing;
      if(distance<EDGE_MARGIN-1e-6||length-distance<EDGE_MARGIN-1e-6)continue;
      const t=distance/length;
      const side=(index%2===0?-1:1) as -1|1;
      const offset=laneWidth/2+.65;
      const inwardX=-side*normalX,inwardZ=-side*normalZ;
      result.push({
        x:a.x+dx*t+side*normalX*offset,
        z:a.z+dz*t+side*normalZ*offset,
        y:.3,
        heading:Math.atan2(-inwardZ,inwardX),
        roadId:road.id,
      });
    }
    if(result.length>=MAX_LIGHTS)break;
  }
  return result;
}

/** Build three instanced meshes and one shared night-driven pavement glow. */
export function buildStreetLighting(world:Pick<World,'nodes'|'roads'>,art:ArtMaterials):THREE.Group{
  const root=new THREE.Group();
  root.name='street-lighting';
  const placements=streetLightPlacements(world);
  if(!placements.length)return root;
  const roadKinds=new Map(world.roads.map(road=>[road.id,road.kind]));
  const roadById=new Map(world.roads.map(road=>[road.id,road]));
  const nodeById=new Map(world.nodes.map(node=>[node.id,node]));

  const poleMaterial=art.get('#536164',.78);
  const poleGeometry=new THREE.CylinderGeometry(.075,.105,5,6);
  poleGeometry.translate(0,2.5,0);
  const poles=new THREE.InstancedMesh(poleGeometry,poleMaterial,placements.length);
  poles.name='street-lamp-poles';
  poles.castShadow=false;poles.receiveShadow=true;

  const armGeometry=new THREE.BoxGeometry(.84,.075,.075);
  const arms=new THREE.InstancedMesh(armGeometry,poleMaterial,placements.length);
  arms.name='street-lamp-arms';

  const lampGeometry=new THREE.BoxGeometry(.34,.12,.24);
  const lampMaterial=new THREE.MeshStandardNodeMaterial({color:'#8d8068',roughness:.66,emissive:'#ffd08a',emissiveIntensity:.5});
  const lampEmission=color('#ffd08a').mul(art.night);
  (lampMaterial as unknown as {emissiveNode:typeof lampEmission}).emissiveNode=lampEmission;
  const lamps=new THREE.InstancedMesh(lampGeometry,lampMaterial,placements.length);
  lamps.name='street-lamp-heads';
  lamps.userData.ownedMaterial=true;

  const poolGeometry=new THREE.PlaneGeometry(2,2,1,1);
  poolGeometry.rotateX(-Math.PI/2);
  const poolMaterial=new THREE.MeshBasicNodeMaterial({transparent:true,depthWrite:false,side:THREE.DoubleSide,blending:THREE.AdditiveBlending,toneMapped:false});
  const radius=uv().sub(.5).length().mul(2);
  const radialFalloff=float(.2).mul(smoothstep(.12,1,radius).oneMinus());
  poolMaterial.colorNode=color('#ffd19a');
  poolMaterial.opacityNode=radialFalloff.mul(art.night);
  poolMaterial.userData.streetLightNight=art.night;
  const pools=new THREE.InstancedMesh(poolGeometry,poolMaterial,placements.length);
  pools.name='street-lamp-light-pools';
  pools.renderOrder=2;
  pools.userData.ownedMaterial=true;

  const dummy=new THREE.Object3D();
  for(let i=0;i<placements.length;i++){
    const p=placements[i];
    const baseY=p.y+.175; // top of the 0.35 m sidewalk slab
    dummy.position.set(p.x,baseY,p.z);dummy.rotation.set(0,0,0);dummy.scale.setScalar(1);dummy.updateMatrix();poles.setMatrixAt(i,dummy.matrix);
    const inwardX=Math.cos(p.heading),inwardZ=-Math.sin(p.heading);
    dummy.position.set(p.x+inwardX*.34,baseY+4.92,p.z+inwardZ*.34);
    dummy.rotation.set(0,p.heading,0);dummy.updateMatrix();arms.setMatrixAt(i,dummy.matrix);
    dummy.position.set(p.x+inwardX*.72,baseY+4.94,p.z+inwardZ*.72);
    dummy.rotation.set(0,p.heading,0);dummy.updateMatrix();lamps.setMatrixAt(i,dummy.matrix);
    const road=roadById.get(p.roadId),a=road?nodeById.get(road.from):undefined,b=road?nodeById.get(road.to):undefined;
    const dx=a&&b?b.x-a.x:0,dz=a&&b?b.z-a.z:0,length=Math.hypot(dx,dz);
    const angle=Math.atan2(dx,dz),normalX=Math.cos(angle),normalZ=-Math.sin(angle);
    const projection=length>0?THREE.MathUtils.clamp(((p.x-(a?.x??0))*dx+(p.z-(a?.z??0))*dz)/(length*length),0,1):0;
    const signedOffset=length>0?((p.x-(a?.x??0))*dz-(p.z-(a?.z??0))*dx)/length:0;
    const side=signedOffset<0?-1:1;
    const laneWidth=roadKinds.get(p.roadId)==='arterial'?14:9;
    const transverseRadius=Math.min(1.8,laneWidth*.17);
    // Center the glow within the carriageway near the curb. The full transverse
    // footprint remains inside the rendered street, away from the sidewalk slab.
    const inwardDistance=laneWidth/2-transverseRadius-.08;
    const poolX=(a?.x??p.x)+dx*projection+side*normalX*inwardDistance;
    const poolZ=(a?.z??p.z)+dz*projection+side*normalZ*inwardDistance;
    dummy.position.set(poolX,.325,poolZ);
    // The baked plane lies in XZ: X is transverse, Z is road-parallel.
    dummy.rotation.set(0,angle,0);
    dummy.scale.set(transverseRadius,1,4);dummy.updateMatrix();pools.setMatrixAt(i,dummy.matrix);
  }
  poles.instanceMatrix.needsUpdate=true;arms.instanceMatrix.needsUpdate=true;lamps.instanceMatrix.needsUpdate=true;pools.instanceMatrix.needsUpdate=true;
  root.add(poles,arms,lamps,pools);
  return root;
}
