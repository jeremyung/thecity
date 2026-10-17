import * as THREE from 'three/webgpu';
import type { Building } from '../../types';
import { architecturalChoice } from './variation';
import type { ApartmentRecipe } from './recipes';

export function buildingWeathering(
  b: Pick<Building, 'appearance_seed' | 'stage'>,
  recipe: ApartmentRecipe,
) {
  const value = architecturalChoice(b.appearance_seed, 112),
    modern = recipe.circulation === 'internal';
  const level = modern ? [0, 0, 1, 1, 2][value % 5] : [0, 1, 1, 2, 3][value % 5];
  const strength = ['open', 'renovation'].includes(b.stage)
    ? (modern ? [0.015, 0.08, 0.18, 0.3] : [0.03, 0.2, 0.48, 0.72])[level]
    : 0.025;
  return {
    profile: ['open', 'renovation'].includes(b.stage)
      ? ['fresh', 'maintained', 'weathered', 'exposed'][level]
      : 'fresh',
    strength,
    phase: (value >>> 8) % 997,
  };
}
/** Per-vertex profile survives chunk transforms; no material or texture per building. */
export function applyBuildingWeathering(
  group: THREE.Group,
  weather: ReturnType<typeof buildingWeathering>,
) {
  group.traverse((o) => {
    if (!(o instanceof THREE.Mesh)) return;
    const material = o.material as THREE.Material;
    if (!material.userData.apartmentSurface || material.userData.apartmentRole === 'glass') return;
    const values = new Float32Array(o.geometry.getAttribute('position').count * 2);
    for (let i = 0; i < values.length; i += 2) {
      values[i] = weather.strength;
      values[i + 1] = weather.phase;
    }
    o.geometry.setAttribute('architectureWear', new THREE.BufferAttribute(values, 2));
  });
}
export function entranceLayout(
  b: Pick<Building, 'width' | 'appearance_seed'>,
  recipe: ApartmentRecipe,
) {
  const w = b.width - 1.3,
    core = recipe.frontCore && w >= 7.2 ? Math.min(2.1, Math.max(1.6, w * 0.17)) : 0;
  const center = -core / 2,
    range = Math.max(0, (w - core) / 2 - 1.8);
  const choice = architecturalChoice(b.appearance_seed, 113) % 3;
  const x = w < 6 ? 0 : center + (choice - 1) * range;
  return {
    x,
    placement: w < 6 ? 'center' : (['left', 'center', 'right'] as const)[choice],
    parkingSide: x < -0.4 ? 1 : -1,
  };
}
