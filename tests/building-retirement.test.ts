import {expect,it} from 'vitest';
import * as THREE from 'three/webgpu';
import {BuildingChunks} from '../web/src/render/building-chunks';

function fixture(meshes=1){
  const material=new THREE.MeshBasicMaterial(),ink=new THREE.LineBasicNodeMaterial();
  const chunks=new BuildingChunks(m=>m,dispose=>dispose(),ink);
  const source=(x:number,count:number)=>({
    createGroup(){const group=new THREE.Group();group.position.x=x;for(let i=0;i<count;i++)group.add(new THREE.Mesh(new THREE.BoxGeometry(4,4,4),material));return group;},
    disposeGroup(group:THREE.Group){group.traverse(o=>{if(o instanceof THREE.Mesh)o.geometry.dispose();});},
  });
  const target=source(0,meshes),peer=source(10,1);
  chunks.set(1,target.createGroup(),target);chunks.set(2,peer.createGroup(),peer);
  for(let i=0;i<10000&&chunks.pendingCount;i++)chunks.update();
  expect(chunks.pendingCount).toBe(0);
  return {chunks,target,peer,clear(){chunks.clear();material.dispose();ink.dispose();}};
}

it('retires only target triangles and contour segments while retaining peer picking',()=>{
  const f=fixture();
  const mesh=f.chunks.root.children[0].children.find(o=>o instanceof THREE.Mesh) as THREE.Mesh;
  const indexBefore=Array.from(mesh.geometry.index!.array);
  const spans=mesh.userData.buildingSpans as Array<{first:number;end:number;id:number}>;
  const peerSpan=spans.find(s=>s.id===2)!;
  const lines=f.chunks.root.children[0].children.find(o=>o instanceof THREE.LineSegments) as THREE.LineSegments;
  const before=Array.from(lines.geometry.getAttribute('position').array);
  expect(f.chunks.tryRetireCommittedSource(1)).toBe(true);
  expect(f.chunks.hasVisibleSource(1)).toBe(false);
  expect(f.chunks.isSourceVisible(2,f.peer)).toBe(true);
  expect(Array.from(mesh.geometry.index!.array).slice(peerSpan.first*3,peerSpan.end*3)).toEqual(indexBefore.slice(peerSpan.first*3,peerSpan.end*3));
  for(let i=0;i<peerSpan.first*3;i+=3)expect(mesh.geometry.index!.getX(i)).toBe(mesh.geometry.index!.getX(i+2));
  expect(f.chunks.buildingAt({object:mesh,faceIndex:peerSpan.first,distance:0,point:new THREE.Vector3()} as THREE.Intersection)).toBe(2);
  expect(f.chunks.buildingAt({object:mesh,faceIndex:0,distance:0,point:new THREE.Vector3()} as THREE.Intersection)).toBeUndefined();
  const after=Array.from(lines.geometry.getAttribute('position').array);
  expect(after.slice(before.length/2)).toEqual(before.slice(before.length/2));
  for(let i=0;i<before.length/2;i+=6)expect(after.slice(i,i+3)).toEqual(after.slice(i+3,i+6));
  expect(f.chunks.tryRetireCommittedSource(1)).toBe(true);
  const pendingRanges=mesh.geometry.index!.updateRanges.map(range=>({...range}));
  expect(f.chunks.tryRetireCommittedSource(2)).toBe(true);
  expect(mesh.geometry.index!.updateRanges.slice(0,pendingRanges.length)).toEqual(pendingRanges);
  f.clear();
});

it('leaves an over-quota source untouched for ordinary rebuilding',()=>{
  const f=fixture(1001);
  const mesh=f.chunks.root.children[0].children.find(o=>o instanceof THREE.Mesh) as THREE.Mesh;
  const before=Array.from(mesh.geometry.index!.array),version=mesh.geometry.index!.version;
  expect(f.chunks.tryRetireCommittedSource(1)).toBe(false);
  expect(Array.from(mesh.geometry.index!.array)).toEqual(before);
  expect(mesh.geometry.index!.version).toBe(version);
  expect(f.chunks.isSourceVisible(1,f.target)).toBe(true);
  f.clear();
});
