import * as THREE from 'three/webgpu';

/** Height of the backdrop heightfield at (x, z), bilinear over its square grid. */
export function backdropHeightAt(geometry: THREE.BufferGeometry, x: number, z: number): number {
  const positions = geometry.getAttribute('position');
  const columns = Math.round(Math.sqrt(positions.count));
  const min = positions.getX(0),
    max = positions.getX(columns - 1);
  const spacing = (max - min) / (columns - 1);
  const fx = THREE.MathUtils.clamp((x - min) / spacing, 0, columns - 1.0001);
  const fz = THREE.MathUtils.clamp((z - min) / spacing, 0, columns - 1.0001);
  const cx = Math.floor(fx),
    cz = Math.floor(fz),
    tx = fx - cx,
    tz = fz - cz;
  const at = (c: number, r: number) => positions.getY(r * columns + c);
  return THREE.MathUtils.lerp(
    THREE.MathUtils.lerp(at(cx, cz), at(cx + 1, cz), tx),
    THREE.MathUtils.lerp(at(cx, cz + 1), at(cx + 1, cz + 1), tx),
    tz,
  );
}

/**
 * Visual-only forest on the scenic hills: tree crowns wherever the backdrop
 * rises above the height at which its painted colour turns to woodland, so
 * silhouettes and the painted canopy agree. No parcels, routing or growth.
 */
export function backdropForestMatrices(
  geometry: THREE.BufferGeometry,
  seed: number,
  spacing = 21,
  minHeight = 30,
): THREE.Matrix4[] {
  let state = (seed ^ 0x3c6ef372) >>> 0;
  const random = () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 0x1_0000_0000;
  };
  const matrices: THREE.Matrix4[] = [];
  const position = new THREE.Vector3(),
    scale = new THREE.Vector3(),
    rotation = new THREE.Quaternion(),
    up = new THREE.Vector3(0, 1, 0);
  for (let x = -1660; x <= 1660; x += spacing) {
    for (let z = -1660; z <= 1660; z += spacing) {
      const px = x + (random() - 0.5) * spacing,
        pz = z + (random() - 0.5) * spacing;
      const keep = random();
      if (Math.max(Math.abs(px), Math.abs(pz)) < 470) continue;
      const height = backdropHeightAt(geometry, px, pz);
      // Thin out near the tree line instead of stopping at a hard contour.
      if (
        height < minHeight ||
        keep > THREE.MathUtils.smoothstep(height, minHeight, minHeight + 22) * 0.85
      )
        continue;
      const crown = 5 + random() * 4;
      position.set(px, height + crown * 0.7, pz);
      scale.set(crown, crown * (1.3 + random() * 0.6), crown);
      rotation.setFromAxisAngle(up, random() * Math.PI * 2);
      matrices.push(new THREE.Matrix4().compose(position, rotation, scale));
    }
  }
  return matrices;
}
