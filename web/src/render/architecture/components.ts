import * as THREE from 'three/webgpu';
import { apartmentComponents } from '../generated/apartment-components';
import type { Kit } from './geometry-kit';
export type ComponentName = keyof typeof apartmentComponents;
export type ComponentMaterials = Record<'concrete' | 'metal' | 'glass', THREE.Material>;
const templates = new Map<
  ComponentName,
  Array<{ geometry: THREE.BufferGeometry; material: 'concrete' | 'metal' | 'glass' }>
>();
export function placeComponent(
  kit: Kit,
  name: ComponentName,
  x: number,
  y: number,
  z: number,
  materials: ComponentMaterials,
  yaw = 0,
) {
  let parts = templates.get(name);
  if (!parts) {
    parts = apartmentComponents[name].map((part) => {
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.Float32BufferAttribute(part.positions, 3));
      geometry.setAttribute('normal', new THREE.Float32BufferAttribute(part.normals, 3));
      geometry.setAttribute('uv', new THREE.Float32BufferAttribute(part.uvs, 2));
      geometry.setIndex(Array.from({ length: part.positions.length / 3 }, (_, i) => i));
      return { geometry, material: part.material };
    });
    templates.set(name, parts);
  }
  for (const part of parts)
    kit.add(part.geometry.clone().rotateY(yaw).translate(x, y, z), materials[part.material]);
}

export function disposeApartmentComponentTemplates() {
  for (const parts of templates.values()) for (const part of parts) part.geometry.dispose();
  templates.clear();
}
