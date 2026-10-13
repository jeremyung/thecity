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
  const target = kit, side = (seed >>> 2) & 1 ? -1 : 1;
  kit = {
    box: (x,y,z,width,height,depth,material) => target.box(x*side,y,z,width,height,depth,material),
    cylinder: (x,y,z,radius,height,material,segments) => target.cylinder(x*side,y,z,radius,height,material,segments),
  };

  const roof = top + .53;
  const small = Math.min(w, d);
  // Below this span even the supported tank lacks comfortable parapet clearance.
  if (small < 3.2) return;
  const radius = Math.max(.24, Math.min(.73, w * .18, d * .18));
  const padW = Math.max(.58, radius * 2 + .2);
  const padD = Math.max(.58, radius * 2 + .2);
  const tankX = -Math.min(w * .22, w / 2 - padW / 2 - .08);
  const tankZ = -Math.min(d * .2, d / 2 - padD / 2 - .08);
  const tankBase = top + .81;
  const tankHeight = Math.min(1.02, Math.max(.52, d * .2));

  // Concrete plinth and four short legs carry the tank directly from roof level.
  kit.box(tankX, top + .77, tankZ, padW, .08, padD, concrete);
  for (const dx of [-1, 1]) for (const dz of [-1, 1]) {
    kit.box(tankX + dx * radius * .68, top + .67, tankZ + dz * radius * .68, .075, .28, .075, iron);
  }
  kit.cylinder(tankX, tankBase + tankHeight / 2, tankZ, radius, tankHeight, paleConcrete, 10);
  kit.cylinder(tankX, tankBase + tankHeight + .035, tankZ, radius + .025, .07, iron, 10);

  // Narrow roofs retain the supported tank without crowding roof access.
  if (small < 4.6) return;

  // Layout is deterministic and preserves separation between service masses.

  const hvacX = w * .22;
  const hvacZ = -d * .2;
  const unitW = Math.min(1.22, w * .27);
  const unitD = Math.min(.9, d * .23);
  const unitBottom = roof + .12;
  const unitH = .43;
  const unitY = unitBottom + unitH / 2;
  // Two vibration rails provide clear contact with the roof surface.
  for (const xOffset of [-.31, .31]) {
    kit.box(hvacX + xOffset * unitW, roof + .06, hvacZ, .12, .12, unitD + .12, iron);
  }
  kit.box(hvacX, unitY, hvacZ, unitW, unitH, unitD, paleConcrete);
  // Service grille on the street-facing side, with visible slats and an upper fan.
  const faceZ = hvacZ + unitD / 2 + .018;
  kit.box(hvacX, unitY, faceZ, unitW * .72, unitH * .65, .035, dark);
  for (let i = 0; i < 3; i++) {
    kit.box(hvacX, unitY - .1 + i * .1, faceZ + .025, unitW * .62, .025, .025, iron);
  }
  kit.box(hvacX, unitBottom + unitH + .025, hvacZ, unitW * .72, .05, unitD * .72, iron);
  kit.cylinder(hvacX, unitBottom + unitH + .057, hvacZ, Math.min(.2, unitW * .16), .018, dark, 8);
  // A restrained deterministic spoke makes the fan legible from above.
  const spoke = (seed >>> 3) % 2 ? 0 : .045;
  kit.box(hvacX + spoke, unitBottom + unitH + .069, hvacZ, .025, .012, unitD * .45, paleConcrete);

  const headhouseX = -w * .22;
  const headhouseZ = d * .22;
  const headhouseW = Math.min(2.4, w * .4);
  const headhouseD = Math.min(1.8, d * .32);
  const headhouseH = 2.1;
  const headhouseY = roof + headhouseH / 2;
  kit.box(headhouseX, headhouseY, headhouseZ, headhouseW, headhouseH, headhouseD, concrete);
  // A human-scale access door and shallow coping restore a readable stair/elevator overrun silhouette.
  const headhouseFaceZ = headhouseZ + headhouseD / 2 + .018;
  const doorW = Math.min(.78, headhouseW * .52);
  kit.box(headhouseX, roof + .9, headhouseFaceZ, doorW, 1.8, .035, dark);
  kit.box(headhouseX, roof + .9, headhouseFaceZ + .022, doorW * .9, 1.72, .018, paleConcrete);
  kit.box(headhouseX + doorW * .34, roof + .9, headhouseFaceZ + .04, .025, .12, .025, iron);
  kit.box(headhouseX + headhouseW * .3, roof + 1.62, headhouseFaceZ, headhouseW * .24, .2, .035, dark);
  for (let i = 0; i < 3; i++) {
    kit.box(headhouseX + headhouseW * .3, roof + 1.56 + i * .055, headhouseFaceZ + .024, headhouseW * .18, .018, .018, paleConcrete);
  }
  kit.box(headhouseX, roof + headhouseH + .04, headhouseZ, headhouseW + .12, .08, headhouseD + .12, iron);

  // Capped risers sit beside the headhouse with a visible clearance gap.
  const pipeX = headhouseX + headhouseW / 2 + .18;
  for (const dz of [-.1, 0, .1]) {
    const height = .28 + (dz === 0 ? .08 : 0);
    const pipeZ = headhouseZ + dz;
    kit.cylinder(pipeX, roof + height / 2, pipeZ, .035, height, iron, 6);
    kit.cylinder(pipeX, roof + height, pipeZ, .055, .045, concrete, 6);
  }
}
