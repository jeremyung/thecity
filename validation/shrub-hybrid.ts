import * as THREE from 'three/webgpu';
import {mergeGeometries} from 'three/addons/utils/BufferGeometryUtils.js';

/** Experimental shared canopy: 100 opaque triangles plus 96 masked leaf triangles. */
export function hybridShrub(seed:number){
 let state=seed;const random=()=>((state=(Math.imul(state,1664525)+1013904223)>>>0)/4294967296);
 const cores:THREE.BufferGeometry[]=[],leaves:THREE.BufferGeometry[]=[];
 for(let i=0;i<5;i++){
  const a=i*Math.PI*2/5;const g=new THREE.IcosahedronGeometry(1,0);
  g.scale(.25,.24,.24);g.translate(Math.cos(a)*.21,.28+random()*.06,Math.sin(a)*.19);cores.push(g);
 }
 for(let i=0;i<48;i++){
  const a=random()*Math.PI*2,v=random(),r=Math.sqrt(1-v*v);
  const center=new THREE.Vector3(Math.cos(a)*r*.43,.26+v*.32,Math.sin(a)*r*.39);
  const normal=new THREE.Vector3(Math.cos(a)*r,v*.8+.3,Math.sin(a)*r).normalize();
  const g=new THREE.PlaneGeometry(.22+random()*.08,.17+random()*.08);
  g.rotateZ(random()*Math.PI*2);g.applyQuaternion(new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0,0,1),normal));g.translate(center.x,center.y,center.z);
  const colors=[];for(let j=0;j<4;j++)colors.push(.78+v*.16,.84+v*.13,.70+v*.20);
  g.setAttribute('color',new THREE.Float32BufferAttribute(colors,3));leaves.push(g);
 }
 const core=mergeGeometries(cores,false)!,edge=mergeGeometries(leaves,false)!;
 for(const g of [...cores,...leaves])g.dispose();
 core.computeBoundingBox();edge.computeBoundingBox();return {core,edge};
}
