import { expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingKit, disposeGroup } from '../web/src/render/kit';
import type { Building } from '../web/src/types';

it('contains narrow apartments through construction, opening and renovation without losing their entrance',()=>{
 const art=new ArtMaterials();
 try {
 for(const style of ['anime','miniature'] as const){art.setStyle(style);
 for(const depth of [8,16])for(const width of [5,6,7])for(const floors of [4,5,8,11])for(const seed of [32,9812]){
 for(const [stage,progress] of [['structure',.4],['surface',.92],['open',1],['renovation',.5]] as const){
 const b:Building={id:1,parcel_id:1,x:0,z:0,rotation:0,width,depth,floors,kind:'apartment',stage,progress,capacity:20,occupancy:10,business_active:false,built_day:0,name:'Narrow apartment',appearance_seed:seed,history:[],anchor:false,vacant_days:0};
 const group=buildingKit(b,art);group.updateMatrixWorld(true);
 const bounds=new THREE.Box3().setFromObject(group),label=`${style}/${width}x${depth}/${floors}/${seed}/${stage}`;
 expect(bounds.min.x,label).toBeGreaterThanOrEqual(-width/2-1e-6);
 expect(bounds.max.x,label).toBeLessThanOrEqual(width/2+1e-6);
 expect(bounds.min.z,label).toBeGreaterThanOrEqual(-depth/2-1e-6);
 expect(bounds.max.z,label).toBeLessThanOrEqual(depth/2+1e-6);
 if(stage==='open'){
 const hits=new THREE.Raycaster(new THREE.Vector3(0,1.2,10),new THREE.Vector3(0,0,-1)).intersectObject(group,true);
 expect(hits[0],label).toBeDefined();
 expect((hits[0].object as THREE.Mesh).material,label).toBe(art.get('#55757a',.25));
 }
 disposeGroup(group);
 }}}
 }finally{art.dispose()}
},20000);
