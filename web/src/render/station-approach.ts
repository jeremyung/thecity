import * as THREE from 'three/webgpu';
import type { Point, RailPlatform } from '../types';

/** Mitered outline joins the short, engine-authored access polyline without
 * overlapping coplanar strips. The terminal path has no acute hairpin turns. */
export function stationApproachOutline(platform: RailPlatform): Point[] {
  const points = platform.ground_path, half = platform.ground_width / 2;
  const normals = points.slice(1).map((b,i) => {
    const a=points[i], length=Math.hypot(b.x-a.x,b.z-a.z);
    return {x:-(b.z-a.z)/length,z:(b.x-a.x)/length};
  });
  const side = (sign:number) => points.map((p,i) => {
    const a=normals[Math.max(0,i-1)], b=normals[Math.min(i,normals.length-1)];
    const denominator=1+a.x*b.x+a.z*b.z;
    return {x:p.x+sign*half*(a.x+b.x)/denominator,z:p.z+sign*half*(a.z+b.z)/denominator};
  });
  return [...side(1),...side(-1).reverse()];
}

export function stationApproachGeometry(platform: RailPlatform): THREE.BufferGeometry {
  const shape=new THREE.Shape(stationApproachOutline(platform).map(p=>new THREE.Vector2(p.x,-p.z)));
  const geometry=new THREE.ExtrudeGeometry(shape,{depth:.12,bevelEnabled:false,steps:1});
  // Kit batches use indexed geometry, including when materials are shared.
  geometry.setIndex(Array.from({length:geometry.getAttribute('position').count},(_,i)=>i));
  geometry.rotateX(-Math.PI/2);
  geometry.translate(0,platform.ground_y-.12,0);
  return geometry;
}
