import type { Material } from 'three/webgpu';
import { Kit } from './kit';
import type { Point } from '../types';
import { bezierRoad, sweepRoad, type RoadCurve } from './road-curve';
import { CURRENT_EXPRESSWAY_PROFILE, type ExpresswayProfile } from './expressway-profile';
import { EXPRESSWAY_DECK } from './road-elevation';

/** Deck-local furniture for completed expressways; no ground-road junction furniture. */
export function addExpresswayFurniture(
  kit: Kit,
  a: Point,
  b: Point,
  materials: {
    paint: Material;
    concrete: Material;
    metal: Material;
    lamp: Material;
  },
  deckHeight = EXPRESSWAY_DECK,
  curve?: RoadCurve,
  profile: ExpresswayProfile = CURRENT_EXPRESSWAY_PROFILE,
) {
  const dx = b.x - a.x,
    dz = b.z - a.z;
  const path =
      curve ??
      bezierRoad(a, { x: a.x + dx / 3, z: a.z + dz / 3 }, { x: b.x - dx / 3, z: b.z - dz / 3 }, b),
    length = path.length;
  if (length < 1) return;
  const width = profile.width,
    surface = deckHeight + 0.15;
  const box = (
    across: number,
    along: number,
    height: number,
    w: number,
    h: number,
    d: number,
    material: Material,
  ) => {
    if (d > 0.5)
      sweepRoad(kit, path, along - d / 2, along + d / 2, across, surface + height, w, h, material);
    else {
      const p = path.at(along);
      kit.box(
        p.x + p.uz * across,
        surface + height,
        p.z - p.ux * across,
        w,
        h,
        d,
        material,
        Math.atan2(p.ux, p.uz),
      );
    }
  };
  // Two lanes in each direction, with shoulders and a narrow central barrier.
  box(0, length / 2, 0.38, profile.median_width, 0.76, length, materials.concrete);
  for (const side of [-1, 1]) {
    for (const offset of [profile.inner_mark, profile.outer_mark])
      box(side * offset, length / 2, 0.025, 0.15, 0.035, length, materials.paint);
    for (let along = 3; along < length - 1; along += 9) {
      const dashLength = Math.min(3, length - along);
      box(
        side * profile.lane_mark,
        along + dashLength / 2,
        0.025,
        0.14,
        0.035,
        dashLength,
        materials.paint,
      );
    }
    // Mount on the deck inside the sound barrier. Arms reach inward over lanes.
    for (let along = 18; along < length - 4; along += 36) {
      box(side * (width / 2 - 0.55), along, 3.3, 0.12, 6.6, 0.12, materials.metal);
      box(side * (width / 2 - 1.3), along, 6.55, 1.6, 0.12, 0.12, materials.metal);
      box(side * (width / 2 - 2), along, 6.5, 0.65, 0.16, 0.32, materials.lamp);
    }
  }
}
