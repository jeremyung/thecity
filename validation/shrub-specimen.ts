import {hybridShrub} from './shrub-hybrid';
import * as THREE from 'three/webgpu';
import {mergeGeometries} from 'three/addons/utils/BufferGeometryUtils.js';
import {ArtMaterials} from '../web/src/render/materials';

// Experimental asset, intentionally not imported by the city.
function shrubGeometry(seed:number){
 let state=seed;const random=()=>((state=(Math.imul(state,1664525)+1013904223)>>>0)/4294967296);
 const parts:THREE.BufferGeometry[]=[];
 for(let i=0;i<38;i++){
  const a=random()*Math.PI*2,r=Math.sqrt(random());
  const x=Math.cos(a)*r*.78,z=Math.sin(a)*r*.50;
  const y=.34+(1-r*r)*.48+random()*.12;
  const g=new THREE.IcosahedronGeometry(1,0);
  g.scale(.20+random()*.16,.15+random()*.13,.17+random()*.13);
  g.rotateY(random()*6.28);g.translate(x,y,z);parts.push(g);
 }
 const result=mergeGeometries(parts,false)!;for(const g of parts)g.dispose();result.computeBoundingSphere();return result;
}
const verge=new URLSearchParams(location.search).has('verge');
const hybrid=verge||new URLSearchParams(location.search).has('hybrid');
const entries:any[]=[];
for(const style of ['anime','miniature'] as const){
 const art=new ArtMaterials();art.setStyle(style);
 const renderer=new THREE.WebGPURenderer({canvas:document.querySelector(`#${style}`) as HTMLCanvasElement,antialias:true,samples:4});renderer.setSize(580,480,false);renderer.setPixelRatio(1);await renderer.init();
 const scene=new THREE.Scene();scene.background=new THREE.Color('#d5dbd2');
 scene.add(new THREE.HemisphereLight('#f5f4db','#5a6655',2.0));const sun=new THREE.DirectionalLight('#fff0cc',2.5);sun.position.set(10,18,7);scene.add(sun);
 function box(x:number,y:number,z:number,w:number,h:number,d:number,color:string){const mesh=new THREE.Mesh(new THREE.BoxGeometry(w,h,d),art.get(color));mesh.position.set(x,y,z);scene.add(mesh);}
 box(0,-.15,5,100,.3,10,'#778064');
 box(0,-.50,-2.45,100,1,3,'#9b9e8b');
 box(0,-.85,-14,100,.10,20,'#668d91'); // Flat water stand-in: bank geometry comparison only.
 box(0,-.08,0,100,.16,1.9,'#65704c');
 box(0,.04,3.55,100,.12,5,'#bbb6a3');
 for(let x=-45;x<=45;x+=3)box(x,.73,1,.08,1.46,.08,'#80877b');
 box(0,1.28,1,90,.055,.065,'#80877b');box(0,.64,1,90,.035,.065,'#80877b');
 const parts=hybrid?hybridShrub(512):null;const geo=parts?.core??shrubGeometry(512);const mat=art.get('#506443',.92);
 const offsets=[0,.48,1.06,1.57,2.21,2.87,3.47,4.06,4.7,5.34,6.05,6.65];
 const count=verge?72:32;
 const mesh=new THREE.InstancedMesh(geo,mat,count);const pose=new THREE.Object3D();
 for(let i=0;i<count;i++){
  const x=verge?(Math.floor(i/12)-3)*12+offsets[i%12]:(i-16)*2.6;
  pose.position.set(x,0,-.1+Math.sin(i*8.3)*.13);pose.rotation.y=i*1.9;
  const scale=verge?.70+(i%5)*.065:.85+(i%5)*.06;
  pose.scale.set(scale,scale*(.8+(i%3)*.1),scale);pose.updateMatrix();mesh.setMatrixAt(i,pose.matrix);
  mesh.setColorAt(i,new THREE.Color().setRGB(.84+(i%3)*.07,.9+(i%4)*.03,.78+(i%5)*.035));
 }
 scene.add(mesh);const plantMeshes=[mesh];
 if(parts){const edge=new THREE.InstancedMesh(parts.edge,art.get('#fffffe'),count);for(let i=0;i<count;i++){mesh.getMatrixAt(i,pose.matrix);edge.setMatrixAt(i,pose.matrix);edge.setColorAt(i,new THREE.Color('#536e40'));}scene.add(edge);plantMeshes.push(edge);}
 const camera=new THREE.PerspectiveCamera(2*Math.atan(24/100)*180/Math.PI,580/480,.1,2000);
 entries.push({scene,renderer,camera,plantMeshes});
}
async function render(){const d=Number((document.querySelector('#distance') as HTMLSelectElement).value);for(const e of entries){for(const mesh of e.plantMeshes)mesh.visible=(document.querySelector('#planting') as HTMLInputElement).checked;e.camera.position.set(d*.20,d*.65,d*.733);e.camera.lookAt(0,0,0);await e.renderer.renderAsync(e.scene,e.camera);}document.querySelector('#status')!.textContent=`Rendered ${d} m · ${verge?'6 planted patches / 72 canopy units':'32 isolated shrubs'} per view · ${hybrid?'Hybrid · 196 triangles/shrub · two instanced draws':'Opaque · 760 triangles/shrub · one instanced draw'} per style · WebGPU · seed 512`;}
document.querySelector('#planting')!.addEventListener('change',render);document.querySelector('#distance')!.addEventListener('change',render);await render();
