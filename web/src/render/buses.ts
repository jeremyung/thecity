import * as THREE from 'three/webgpu';
import { Kit, disposeGroup } from './kit';
import { ArtMaterials, signTexture } from './materials';
import type { World } from '../types';

/** Two shared city buses; simulation owns routes, passengers and stop placement. */
export class CityBuses {
  readonly root=new THREE.Group();
  readonly meshes:THREE.InstancedMesh[]=[];
  readonly ids:number[]=[];
  private stops=new THREE.Group();
  private stopSignature='';
  private doorLight:THREE.InstancedMesh;
  private display:THREE.Mesh[]=[];
  private displayText=['',''];
  private matrix=new THREE.Matrix4();
  private pose=new THREE.Object3D();
  private dwellCount=0;
  constructor(private art:ArtMaterials,private retire:(release:()=>void)=>void){
    const kit=new Kit(),cream=art.get('#d8d8bd'),green=art.get('#456f65'),glass=art.get('#425f69',.3),dark=art.get('#304348');
    kit.box(0,1.7,0,2.28,2.35,8.2,cream);
    kit.box(0,1.13,0,2.31,.5,8.22,green);
    kit.box(0,2.93,-.2,2.06,.16,7.8,cream);
    kit.box(0,3.1,-1.4,1.35,.2,1.9,art.get('#9daaa1'));
    for(const side of [-1,1]){
      for(let z=-3.1;z<=2.3;z+=1.25)kit.box(side*1.15,2.22,z,.04,.85,1.07,glass);
      for(const z of [-2.55,2.55]){const wheel=new THREE.CylinderGeometry(.47,.47,.24,12);wheel.rotateZ(Math.PI/2);wheel.translate(side*1.12,.55,z);kit.add(wheel,dark);}
      kit.box(side*.73,.9,4.13,.45,.22,.035,art.get('#ffe6a5',.4,true));
      kit.box(side*.78,1.02,-4.13,.25,.26,.035,art.get('#b75b4a',.5,true));
      kit.box(side*1.28,2.45,3.45,.2,.3,.18,dark);
    }
    kit.box(0,2.27,4.12,2.05,.9,.045,glass);
    kit.box(0,2.26,-4.12,1.8,.7,.045,glass);
    kit.box(-1.167,1.47,2.65,.04,1.75,1.18,dark);
    for(const z of [2.36,2.94])kit.box(-1.19,1.7,z,.025,1.26,.5,glass);
    kit.box(-1.22,1.45,2.65,.04,1.8,.045,cream);
    kit.box(0,.56,4.12,2.2,.15,.15,dark);
    const built=kit.finish();
    for(const object of built.children){const source=object as THREE.Mesh;const mesh=new THREE.InstancedMesh(source.geometry,source.material,2);mesh.count=0;mesh.castShadow=true;mesh.receiveShadow=true;mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);this.meshes.push(mesh);this.root.add(mesh);}
    this.doorLight=new THREE.InstancedMesh(new THREE.BoxGeometry(.028,1.45,.85),art.get('#ebc384',.5,true),2);this.doorLight.count=0;this.root.add(this.doorLight,this.stops);
    for(let i=0;i<2;i++){const display=new THREE.Mesh(new THREE.PlaneGeometry(1.72,.38),new THREE.MeshBasicNodeMaterial({color:'#d9d8ae'}));display.visible=false;this.display.push(display);this.root.add(display);}
  }
  setWorld(world:World){
    const seen=new Set<string>();
    const markers=(world.bus_lines??[]).filter(line=>line.active).flatMap(line=>line.stop_positions??[]).filter(stop=>{
      // Terminal directions can share one physical pole; avoid coincident faces.
      const key=`${stop.node_id}:${stop.x.toFixed(3)}:${stop.z.toFixed(3)}`;
      if(seen.has(key))return false;seen.add(key);return true;
    }).slice(0,10);
    const signature=JSON.stringify(markers);
    if(signature!==this.stopSignature){
      this.stopSignature=signature;const previous=this.stops;previous.removeFromParent();this.retire(()=>disposeGroup(previous));
      const kit=new Kit();
      for(const stop of markers){
        kit.cylinder(stop.x,1.5,stop.z,.065,3,this.art.get('#60746b'),8);
        kit.box(stop.x,2.8,stop.z,.62,.66,.12,this.art.get('#e4debf'),Math.PI/2-stop.heading);
        kit.box(stop.x,2.82,stop.z,.42,.15,.14,this.art.get('#456f65'),Math.PI/2-stop.heading);
      }
      this.stops=kit.finish();this.root.add(this.stops);
    }
  }
  begin(){this.ids.length=0;this.dwellCount=0;}
  set(index:number,id:number,x:number,z:number,yaw:number,world:World|null,lift=0,pitch=0){
    if(index>=2)return;
    this.ids[index]=id;this.pose.position.set(x,lift,z);this.pose.rotation.set(-pitch,yaw,0,'YXZ');this.pose.updateMatrix();this.matrix.copy(this.pose.matrix);
    for(const mesh of this.meshes)mesh.setMatrixAt(index,this.matrix);
    const bus=world?.buses?.find(b=>b.id===id);
    if(bus?.dwelling){this.pose.translateX(-1.205);this.pose.translateY(1.46);this.pose.translateZ(2.65);this.pose.updateMatrix();this.doorLight.setMatrixAt(this.dwellCount++,this.pose.matrix);}
    const destination=world?.nodes.find(node=>node.id===bus?.next_stop);
    const label=destination?.kind==='station'?'駅前':destination?.kind==='market'?'青葉通り':destination?'住宅街':'まちバス';
    const display=this.display[index];
    if(this.displayText[index]!==label){
      this.displayText[index]=label;const mat=display.material as THREE.MeshBasicNodeMaterial,old=mat.map;
      mat.map=signTexture(label,'#263e39','#eadba2');mat.needsUpdate=true;if(old)this.retire(()=>old.dispose());
    }
    display.visible=true;display.position.set(0,2.87,4.151).applyMatrix4(this.matrix);display.quaternion.setFromRotationMatrix(this.matrix);
  }
  finish(count:number){
    for(const mesh of this.meshes){mesh.count=Math.min(count,2);mesh.instanceMatrix.needsUpdate=true;mesh.computeBoundingSphere();}
    this.doorLight.count=this.dwellCount;this.doorLight.instanceMatrix.needsUpdate=true;this.doorLight.computeBoundingSphere();
    for(let i=count;i<2;i++)this.display[i].visible=false;
  }
  reset(){this.begin();this.finish(0);this.stopSignature='';const old=this.stops;old.removeFromParent();this.retire(()=>disposeGroup(old));this.stops=new THREE.Group();this.root.add(this.stops);}
  dispose(){this.root.removeFromParent();this.reset();for(const mesh of [...this.meshes,this.doorLight]){mesh.geometry.dispose();mesh.dispose();}for(const display of this.display){display.geometry.dispose();const mat=display.material as THREE.MeshBasicNodeMaterial;mat.map?.dispose();mat.dispose();}}
}
