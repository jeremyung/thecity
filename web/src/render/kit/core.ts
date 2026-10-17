import * as THREE from 'three/webgpu';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { ArtMaterials, shopSignLabel } from '../materials';
import { boxGeometry } from '../box-geometry';
import {
  paneSeed,
  tagCladding,
  tagWindowPane,
  windowActivity,
  type Cladding,
} from '../building-surfaces';
import { type Building } from '../../types';

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

/**
 * Master-plan lots share lot lines, so the body keeps a fixed side margin for
 * eaves, sills and the foundation lip instead of a proportional inset.
 * Neighbours end up about 0.9 m apart, close to Tokyo's 0.5 m-per-side norm.
 * Depth keeps its proportional front setback and rear yard.
 */

export function partyWallBodyWidth(
  width: number,
  anchor = false,
  floors = 2,
  kind = 'home',
): number {
  if (anchor || floors >= 14) return Math.max(6, width * (anchor ? 0.66 : 0.84));
  // Narrow generated offices and apartments need their side margin for the foundation,
  // roof coping and facade details. A minimum six-metre body can exceed the lot.
  if ((kind === 'office' || (kind === 'apartment' && floors >= 4)) && floors < 12)
    return width - 1.3;
  // Apartment and office families carry balconies and entry reveals.
  const lowrise = floors < 4 && (kind === 'home' || kind === 'shop' || kind === 'cafe');
  // The planner admits 5 m sites. Do not let the legacy 6 m body floor
  // exceed those parcels before the low-rise family selects its envelope.
  if (lowrise && width < 6) return width - 0.9;
  return Math.max(6, width - (lowrise ? 0.9 : 1.3));
}

export function partyWallBodyDepth(depth: number): number {
  return Math.max(6, depth * 0.84);
}

export const pick = (seed: number, count: number) => Math.abs(Math.trunc(seed)) % count;

/** About half of non-anchor two- and three-floor homes receive restrained modern service details. */

export function isModernHomeVariant(
  b: Pick<Building, 'kind' | 'floors' | 'appearance_seed' | 'anchor'>,
): boolean {
  return (
    !b.anchor &&
    b.kind === 'home' &&
    (b.floors === 2 || b.floors === 3) &&
    ((b.appearance_seed >>> 2) & 1) === 0
  );
}

export function zForSide(x: number, w: number, d: number) {
  return x < 0 ? d / 2 + 0.62 : -d / 2 - 0.62;
}

export function windowPane(art: ArtMaterials, b: Pick<Building, 'kind'>) {
  return art.window(windowActivity(b.kind));
}

export function addRod(
  kit: Kit,
  from: { x: number; y: number; z: number },
  to: { x: number; y: number; z: number },
  radius: number,
  material: THREE.Material,
) {
  const start = new THREE.Vector3(from.x, from.y, from.z);
  const end = new THREE.Vector3(to.x, to.y, to.z);
  const direction = end.clone().sub(start);
  const geometry = new THREE.CylinderGeometry(radius, radius, direction.length(), 6);
  geometry.applyQuaternion(
    new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0, 1, 0), direction.normalize()),
  );
  geometry.translate((start.x + end.x) / 2, (start.y + end.y) / 2, (start.z + end.z) / 2);
  kit.add(geometry, material);
}

export function baseValue(top: number, floors: number, index: number) {
  return Math.min(top - 0.3, 3.2 + (index % Math.max(1, Math.min(3, floors - 1))) * 3.25);
}

export function renderRenovationScaffold(
  kit: Kit,
  w: number,
  d: number,
  h: number,
  material: THREE.Material,
  caution: THREE.Material,
) {
  for (const x of [-w / 2 - 0.45, w / 2 + 0.45]) {
    for (const z of [-d / 2 - 0.45, d / 2 + 0.45]) kit.box(x, h / 2, z, 0.07, h, 0.07, material);
    for (let y = 1.2; y < h; y += 2.4) kit.box(x, y, 0, 0.06, 0.06, d + 1, material);
  }
  kit.box(0, 1.5, d / 2 + 1.55, w + 0.8, 2.8, 0.13, caution);
}

export function seedInt(seed: number) {
  return Math.trunc(seed) >>> 0;
}

export function finishBuilding(
  kit: Kit,
  b: Building,
  w: number,
  d: number,
  height: number,
  art: ArtMaterials,
  completed: boolean,
) {
  const group = kit.finish();
  if (completed && (b.anchor || b.kind === 'shop' || b.kind === 'cafe')) {
    const { material, v0, v1 } = art.shopSign(shopSignLabel(b));
    const signW = Math.min(w * 0.7, b.anchor ? 5.5 : 4.8);
    const signH = signW / 3.7;
    const plane = new THREE.PlaneGeometry(signW, signH),
      uvs = plane.getAttribute('uv');
    for (let i = 0; i < uvs.count; i++) uvs.setY(i, v0 + uvs.getY(i) * (v1 - v0));
    const sign = new THREE.Mesh(plane, material);
    const signY =
      height < 5.2 ? height - signH / 2 - 0.06 : Math.max(4.45, Math.min(height - 0.25, 4.85));
    sign.position.set(0, signY, Math.min(d / 2 + 0.31, b.depth / 2 - 0.03));
    group.add(sign);
  }
  group.userData.buildingId = b.id;
  group.traverse((o) => (o.userData.buildingId = b.id));
  return group;
}

export function disposeGroup(group: THREE.Object3D) {
  group.traverse((o) => {
    if (o instanceof THREE.Mesh) {
      if (o instanceof THREE.InstancedMesh) o.dispose();
      o.geometry.dispose();
      if (o.userData.ownedMaterial) (o.material as THREE.Material).dispose();
    }
    if (o.userData.ownedTexture) (o.userData.ownedTexture as THREE.Texture).dispose();
  });
  group.removeFromParent();
}
