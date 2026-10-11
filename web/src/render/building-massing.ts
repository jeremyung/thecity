import type { Building } from '../types';

export interface BuildingMass {
  x: number;
  z: number;
  width: number;
  depth: number;
  baseFloor: number;
  floors: number;
  role: 'main' | 'core';
}

export interface MidriseMassing {
  family: 'walkup' | 'setback-office';
  masses: BuildingMass[];
}

type MidriseBuilding = Pick<Building, 'kind' | 'floors' | 'width' | 'depth' | 'appearance_seed' | 'anchor'>;

/** Review candidate only: retain small-lot clearances while reducing oversized
 * percentage setbacks on wider midrise parcels. Does not change authoritative lots. */
export function compactMidriseEnvelope(b: MidriseBuilding, width: number, depth: number) {
  if (!Number.isFinite(b.width) || !Number.isFinite(b.depth)
      || b.width < 8 || b.depth < 8 || !midriseMassing(b, width, depth)) return { width, depth };
  return { width: Math.max(width, b.width - 1.6), depth: Math.max(depth, b.depth - 1.8) };
}

/**
 * Compose a deterministic midrise silhouette inside the supplied building
 * envelope. Floor counts and world-state dimensions remain authoritative; this
 * helper only provides visual mass groupings for the renderer.
 */
export function midriseMassing(b: MidriseBuilding, w: number, d: number): MidriseMassing | null {
  if (b.anchor || (b.kind !== 'apartment' && b.kind !== 'office')
      || !Number.isInteger(b.floors) || b.floors < 4 || b.floors > 11
      || !Number.isFinite(w) || !Number.isFinite(d) || w < 6 || d < 6) {
    return null;
  }

  const seed = b.appearance_seed >>> 0;
  const side = (seed & 1) === 0 ? -1 : 1;
  if (b.kind === 'apartment') {
    const coreRatio = 0.20 + ((seed >>> 2) % 5) * 0.01;
    const coreWidth = w * coreRatio;
    const mainWidth = w - coreWidth;
    const mainX = -side * coreWidth * 0.5;
    const lowerFloors = b.floors - 2;
    const capWidthRatio = 0.78 + ((seed >>> 6) % 5) * 0.01;
    const capDepthRatio = 0.76 + ((seed >>> 9) % 7) * 0.01;
    const capShiftX = 0.035 + ((seed >>> 12) % 4) * 0.01;
    const capShiftZ = 0.04 + ((seed >>> 14) % 5) * 0.01;

    return {
      family: 'walkup',
      masses: [
        {
          x: side * (w / 2 - coreWidth / 2), z: 0,
          width: coreWidth, depth: d, baseFloor: 0, floors: b.floors, role: 'core',
        },
        {
          x: mainX, z: 0,
          width: mainWidth, depth: d, baseFloor: 0, floors: lowerFloors, role: 'main',
        },
        {
          x: mainX - side * mainWidth * capShiftX,
          z: -d * capShiftZ,
          width: mainWidth * capWidthRatio,
          depth: d * capDepthRatio,
          baseFloor: lowerFloors, floors: 2, role: 'main',
        },
      ],
    };
  }

  const podiumFloors = 2;
  const shaftFloors = Math.max(0, b.floors - 4);
  const shaftWidth = w * (0.76 + ((seed >>> 2) % 5) * 0.01);
  const shaftDepth = d * (0.78 + ((seed >>> 5) % 7) * 0.01);
  const shaftX = side * w * (0.05 + ((seed >>> 8) % 5) * 0.01);
  const crownWidthRatio = 0.78 + ((seed >>> 11) % 7) * 0.01;
  const crownDepthRatio = 0.78 + ((seed >>> 14) % 7) * 0.01;
  const crownShiftX = side * shaftWidth * (0.015 + ((seed >>> 17) % 3) * 0.005);

  const masses: BuildingMass[] = [
    { x: 0, z: 0, width: w, depth: d, baseFloor: 0, floors: podiumFloors, role: 'main' },
  ];

  if (shaftFloors > 0) {
    masses.push({
      x: shaftX, z: 0,
      width: shaftWidth, depth: shaftDepth,
      baseFloor: podiumFloors, floors: shaftFloors, role: 'main',
    });
  }

  const upperBaseFloor = b.floors - 2;
  const upperWidth = shaftFloors > 0 ? shaftWidth : w * (0.62 + ((seed >>> 2) % 5) * 0.01);
  const upperDepth = shaftFloors > 0 ? shaftDepth : d * (0.68 + ((seed >>> 5) % 7) * 0.01);
  const upperX = shaftFloors > 0 ? shaftX : side * w * 0.08;
  masses.push({
    x: upperX + crownShiftX,
    z: -(shaftFloors > 0 ? shaftDepth : d) * 0.035,
    width: upperWidth * crownWidthRatio,
    depth: upperDepth * crownDepthRatio,
    baseFloor: upperBaseFloor,
    floors: 2,
    role: 'main',
  });

  return { family: 'setback-office', masses };
}
