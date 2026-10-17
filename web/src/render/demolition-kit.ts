import type * as THREE from 'three/webgpu';
import type { BuildingMass } from './building-massing';

interface Boxes {
  box(
    x: number,
    y: number,
    z: number,
    w: number,
    h: number,
    d: number,
    material: THREE.Material,
  ): void;
}

/** Keep the original setbacks while removing upper floors. Shared materials and
 * deterministic debris keep this bounded, with no particle or physics system. */
export function renderDemolition(
  kit: Boxes,
  masses: BuildingMass[],
  options: {
    progress: number;
    floors: number;
    parcelW: number;
    parcelD: number;
    wall: THREE.Material;
    concrete: THREE.Material;
    dark: THREE.Material;
    scaffold: THREE.Material;
    caution: THREE.Material;
  },
) {
  const { floors, parcelW, parcelD, wall, concrete, dark, scaffold, caution } = options;
  const progress = Math.max(0, Math.min(1, options.progress));
  const remaining = Math.ceil(floors * (1 - progress));
  const floorHeight = 3.25,
    base = 0.58;
  for (const mass of masses) {
    const count = Math.max(0, Math.min(mass.floors, remaining - mass.baseFloor));
    for (let f = 0; f < count; f++) {
      const level = mass.baseFloor + f;
      const y = base + level * floorHeight;
      const exposed = level === remaining - 1 && progress > 0;
      kit.box(mass.x, y + 0.12, mass.z, mass.width, 0.24, mass.depth, concrete);
      // The working floor is stripped to columns; lower floors retain facade
      // panels and dark, unoccupied window bands rather than emissive glazing.
      for (const side of [-1, 1]) {
        if (!exposed) {
          kit.box(
            mass.x,
            y + 1.6,
            mass.z + side * (mass.depth / 2 - 0.1),
            mass.width,
            3,
            0.2,
            wall,
          );
          kit.box(
            mass.x + side * (mass.width / 2 - 0.1),
            y + 1.6,
            mass.z,
            0.2,
            3,
            mass.depth,
            wall,
          );
          kit.box(
            mass.x,
            y + 1.8,
            mass.z + side * (mass.depth / 2 + 0.015),
            mass.width * 0.88,
            1.3,
            0.03,
            dark,
          );
          kit.box(
            mass.x + side * (mass.width / 2 + 0.015),
            y + 1.8,
            mass.z,
            0.03,
            1.3,
            mass.depth * 0.88,
            dark,
          );
        }
        for (const zSide of [-1, 1]) {
          kit.box(
            mass.x + side * (mass.width / 2 - 0.2),
            y + 1.5,
            mass.z + zSide * (mass.depth / 2 - 0.2),
            0.32,
            3,
            0.32,
            concrete,
          );
        }
      }
    }
  }
  // Opaque site hoarding stays inside the parcel, including the cleared phase.
  for (const side of [-1, 1]) {
    kit.box(side * (parcelW / 2 - 0.18), 1, 0, 0.12, 2, parcelD - 0.3, scaffold);
    kit.box(0, 1, side * (parcelD / 2 - 0.18), parcelW - 0.3, 2, 0.12, scaffold);
    kit.box(side * parcelW * 0.3, 1.5, parcelD / 2 - 0.1, parcelW * 0.16, 0.22, 0.05, caution);
  }
  if (progress > 0) {
    for (let i = 0; i < 12; i++) {
      const x = (((i * 7) % 11) / 10 - 0.5) * parcelW * 0.65;
      const z = (((i * 3) % 7) / 6 - 0.5) * parcelD * 0.65;
      kit.box(x, 0.42, z, 0.4 + (i % 3) * 0.18, 0.25 + (i % 2) * 0.2, 0.5, concrete);
    }
  }
}
