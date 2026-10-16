import type * as THREE from 'three/webgpu';
import type { Kit } from './kit';

export interface RoofServicesOptions {
  w: number;
  d: number;
  top: number;
  seed: number;
  concrete: THREE.Material;
  iron: THREE.Material;
  paleConcrete: THREE.Material;
  dark: THREE.Material;
}

/**
 * Add a compact, supported roof-service cluster. Geometry is baked into Kit's
 * shared-material batches; placement is deterministic and has no frame work.
 */
export function renderRoofServices(kit: Pick<Kit, 'box' | 'cylinder'>, m: RoofServicesOptions) {
  const { w, d, top, seed, concrete, iron, paleConcrete, dark } = m;
  if (![w, d, top].every(Number.isFinite) || w <= 0 || d <= 0) return;
  // Mirror the complete service arrangement, retaining all relative clearances.
  // This gives neighboring roofs stable variation without independent prop jitter.
  const target = kit,
    side = (seed >>> 2) & 1 ? -1 : 1;
  kit = {
    box: (x, y, z, width, height, depth, material) =>
      target.box(x * side, y, z, width, height, depth, material),
    cylinder: (x, y, z, radius, height, material, segments) =>
      target.cylinder(x * side, y, z, radius, height, material, segments),
  };

  const roof = top + 0.53;
  const small = Math.min(w, d);
  // Below this span even the supported tank lacks comfortable parapet clearance.
  if (small < 3.2) return;
  const radius = Math.max(0.24, Math.min(0.73, w * 0.18, d * 0.18));
  const padW = Math.max(0.58, radius * 2 + 0.2);
  const padD = Math.max(0.58, radius * 2 + 0.2);
  const tankX = -Math.min(w * 0.22, w / 2 - padW / 2 - 0.08);
  const tankZ = -Math.min(d * 0.2, d / 2 - padD / 2 - 0.08);
  const tankBase = top + 0.81;
  const tankHeight = Math.min(1.02, Math.max(0.52, d * 0.2));

  // Concrete plinth and four short legs carry the tank directly from roof level.
  kit.box(tankX, top + 0.77, tankZ, padW, 0.08, padD, concrete);
  for (const dx of [-1, 1])
    for (const dz of [-1, 1]) {
      kit.box(
        tankX + dx * radius * 0.68,
        top + 0.67,
        tankZ + dz * radius * 0.68,
        0.075,
        0.28,
        0.075,
        iron,
      );
    }
  kit.cylinder(tankX, tankBase + tankHeight / 2, tankZ, radius, tankHeight, paleConcrete, 10);
  kit.cylinder(tankX, tankBase + tankHeight + 0.035, tankZ, radius + 0.025, 0.07, iron, 10);

  // Narrow roofs retain the supported tank without crowding roof access.
  if (small < 4.6) return;

  // Layout is deterministic and preserves separation between service masses.

  const hvacX = w * 0.22;
  const hvacZ = -d * 0.2;
  const unitW = Math.min(1.22, w * 0.27);
  const unitD = Math.min(0.9, d * 0.23);
  const unitBottom = roof + 0.12;
  const unitH = 0.43;
  const unitY = unitBottom + unitH / 2;
  // Two vibration rails provide clear contact with the roof surface.
  for (const xOffset of [-0.31, 0.31]) {
    kit.box(hvacX + xOffset * unitW, roof + 0.06, hvacZ, 0.12, 0.12, unitD + 0.12, iron);
  }
  kit.box(hvacX, unitY, hvacZ, unitW, unitH, unitD, paleConcrete);
  // Service grille on the street-facing side, with visible slats and an upper fan.
  const faceZ = hvacZ + unitD / 2 + 0.018;
  kit.box(hvacX, unitY, faceZ, unitW * 0.72, unitH * 0.65, 0.035, dark);
  for (let i = 0; i < 3; i++) {
    kit.box(hvacX, unitY - 0.1 + i * 0.1, faceZ + 0.025, unitW * 0.62, 0.025, 0.025, iron);
  }
  kit.box(hvacX, unitBottom + unitH + 0.025, hvacZ, unitW * 0.72, 0.05, unitD * 0.72, iron);
  kit.cylinder(
    hvacX,
    unitBottom + unitH + 0.057,
    hvacZ,
    Math.min(0.2, unitW * 0.16),
    0.018,
    dark,
    8,
  );
  // A restrained deterministic spoke makes the fan legible from above.
  const spoke = (seed >>> 3) % 2 ? 0 : 0.045;
  kit.box(
    hvacX + spoke,
    unitBottom + unitH + 0.069,
    hvacZ,
    0.025,
    0.012,
    unitD * 0.45,
    paleConcrete,
  );

  const headhouseX = -w * 0.22;
  const headhouseZ = d * 0.22;
  const headhouseW = Math.min(2.4, w * 0.4);
  const headhouseD = Math.min(1.8, d * 0.32);
  const headhouseH = 2.1;
  const headhouseY = roof + headhouseH / 2;
  kit.box(headhouseX, headhouseY, headhouseZ, headhouseW, headhouseH, headhouseD, concrete);
  // A human-scale access door and shallow coping restore a readable stair/elevator overrun silhouette.
  const headhouseFaceZ = headhouseZ + headhouseD / 2 + 0.018;
  const doorW = Math.min(0.78, headhouseW * 0.52);
  kit.box(headhouseX, roof + 0.9, headhouseFaceZ, doorW, 1.8, 0.035, dark);
  kit.box(headhouseX, roof + 0.9, headhouseFaceZ + 0.022, doorW * 0.9, 1.72, 0.018, paleConcrete);
  kit.box(headhouseX + doorW * 0.34, roof + 0.9, headhouseFaceZ + 0.04, 0.025, 0.12, 0.025, iron);
  kit.box(
    headhouseX + headhouseW * 0.3,
    roof + 1.62,
    headhouseFaceZ,
    headhouseW * 0.24,
    0.2,
    0.035,
    dark,
  );
  for (let i = 0; i < 3; i++) {
    kit.box(
      headhouseX + headhouseW * 0.3,
      roof + 1.56 + i * 0.055,
      headhouseFaceZ + 0.024,
      headhouseW * 0.18,
      0.018,
      0.018,
      paleConcrete,
    );
  }
  kit.box(
    headhouseX,
    roof + headhouseH + 0.04,
    headhouseZ,
    headhouseW + 0.12,
    0.08,
    headhouseD + 0.12,
    iron,
  );

  // Capped risers sit beside the headhouse with a visible clearance gap.
  const pipeX = headhouseX + headhouseW / 2 + 0.18;
  for (const dz of [-0.1, 0, 0.1]) {
    const height = 0.28 + (dz === 0 ? 0.08 : 0);
    const pipeZ = headhouseZ + dz;
    kit.cylinder(pipeX, roof + height / 2, pipeZ, 0.035, height, iron, 6);
    kit.cylinder(pipeX, roof + height, pipeZ, 0.055, 0.045, concrete, 6);
  }
}
