import * as THREE from 'three/webgpu';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { boxGeometry } from '../box-geometry';
import { paneSeed, tagWindowPane, tagCladding, type Cladding } from '../building-surfaces';
// Every repeatable detail is baked into one geometry per shared material.
export class Kit {
  private parts = new Map<THREE.Material, THREE.BufferGeometry[]>();
  private offset = new THREE.Vector3();
  /** The building whose window panes this kit draws: seeds each room and says whether anyone is in. */
  windowContext = { seed: 0, lived: 1 };

  withOffset<T>(x: number, y: number, z: number, build: () => T): T {
    const previous = this.offset;
    this.offset = previous.clone().add(new THREE.Vector3(x, y, z));
    try {
      return build();
    } finally {
      this.offset = previous;
    }
  }

  box(
    x: number,
    y: number,
    z: number,
    w: number,
    h: number,
    d: number,
    mat: THREE.Material,
    rotation = 0,
  ) {
    const geo = boxGeometry(x, y, z, w, h, d, rotation);
    // Keep room attributes on the optimized box path, including nested mass offsets.
    if (mat.userData.windowPane)
      tagWindowPane(
        geo,
        paneSeed(this.windowContext.seed, x + this.offset.x, y + this.offset.y, z + this.offset.z),
        this.windowContext.lived,
        Math.max(w, d),
        h,
        mat.userData.windowActivity,
      );
    this.add(geo, mat);
  }
  cylinder(
    x: number,
    y: number,
    z: number,
    r: number,
    h: number,
    mat: THREE.Material,
    segments = 8,
  ) {
    const geo = new THREE.CylinderGeometry(r, r, h, segments);
    geo.translate(x, y, z);
    this.add(geo, mat);
  }
  cone(
    x: number,
    y: number,
    z: number,
    radius: number,
    h: number,
    mat: THREE.Material,
    segments = 6,
  ) {
    const geo = new THREE.ConeGeometry(radius, h, segments);
    geo.translate(x, y, z);
    this.add(geo, mat);
  }
  add(geo: THREE.BufferGeometry, mat: THREE.Material) {
    if (mat.userData.windowPane && !geo.hasAttribute('windowInfo')) {
      geo.computeBoundingBox();
      const bounds = geo.boundingBox!,
        center = bounds.getCenter(new THREE.Vector3()),
        size = bounds.getSize(new THREE.Vector3());
      const position = geo.getAttribute('position'),
        uv = new Float32Array(position.count * 2);
      for (let i = 0; i < position.count; i++) {
        uv[i * 2] = (position.getX(i) - bounds.min.x) / Math.max(size.x, 0.001);
        uv[i * 2 + 1] = (position.getY(i) - bounds.min.y) / Math.max(size.y, 0.001);
      }
      geo.setAttribute('uv', new THREE.BufferAttribute(uv, 2));
      tagWindowPane(
        geo,
        paneSeed(
          this.windowContext.seed,
          center.x + this.offset.x,
          center.y + this.offset.y,
          center.z + this.offset.z,
        ),
        this.windowContext.lived,
        Math.max(size.x, size.z),
        size.y,
        mat.userData.windowActivity,
      );
    }
    const cladding = (
      mat.userData.artBuildingClass as { cladding?: Exclude<Cladding, 'plaster'> } | undefined
    )?.cladding;
    if (cladding) tagCladding(geo, cladding);
    if (this.offset.lengthSq() > 0) geo.translate(this.offset.x, this.offset.y, this.offset.z);
    const list = this.parts.get(mat) ?? [];
    list.push(geo);
    this.parts.set(mat, list);
  }
  finish(cast = true) {
    const group = new THREE.Group();
    for (const [mat, parts] of this.parts) {
      const merged = mergeGeometries(parts, false);
      if (merged) {
        const mesh = new THREE.Mesh(merged, mat);
        mesh.castShadow = cast;
        mesh.receiveShadow = true;
        group.add(mesh);
      }
      parts.forEach((g) => g.dispose());
    }
    this.parts.clear();
    return group;
  }
  /** Cancel unfinished geometry; materials remain owned by the shared palette. */
  dispose() {
    for (const parts of this.parts.values()) for (const geometry of parts) geometry.dispose();
    this.parts.clear();
  }
}
