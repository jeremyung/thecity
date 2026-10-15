import * as THREE from 'three/webgpu';

// Positive dimensions share BoxGeometry's one-segment topology. The cached
// template is never mutated; every returned geometry owns copied attributes.
const unit = new THREE.BoxGeometry(2, 2, 2);
const unitPosition = unit.getAttribute('position');
const unitNormal = unit.getAttribute('normal');
const unitUv = unit.getAttribute('uv');
const unitIndex = unit.index!;

/** Equivalent to new BoxGeometry(w,h,d).rotateY(rotation).translate(x,y,z). */
export function boxGeometry(x: number, y: number, z: number, w: number, h: number, d: number, rotation = 0): THREE.BufferGeometry {
  // Match Three's own behavior for dimensions whose sign or finiteness can
  // affect face normals and construction arithmetic.
  if (!(w > 0 && h > 0 && d > 0 && Number.isFinite(w) && Number.isFinite(h) && Number.isFinite(d))) {
    return new THREE.BoxGeometry(w, h, d).rotateY(rotation).translate(x, y, z);
  }

  const geometry = new THREE.BufferGeometry();
  const positions = new Float32Array(unitPosition.count * 3);
  const halfW = w / 2, halfH = h / 2, halfD = d / 2;
  for (let i = 0; i < unitPosition.count; i++) {
    positions[i * 3] = unitPosition.getX(i) * halfW;
    positions[i * 3 + 1] = unitPosition.getY(i) * halfH;
    positions[i * 3 + 2] = unitPosition.getZ(i) * halfD;
  }
  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  geometry.setAttribute('normal', unitNormal.clone());
  geometry.setAttribute('uv', unitUv.clone());
  geometry.setIndex(unitIndex.clone());
  for (const group of unit.groups) geometry.addGroup(group.start, group.count, group.materialIndex);
  return geometry.rotateY(rotation).translate(x, y, z);
}
