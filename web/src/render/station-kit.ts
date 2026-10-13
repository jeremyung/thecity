import * as THREE from 'three/webgpu';
import type { RailPlatform } from '../types';
import { Kit } from './kit';
import { stationApproachGeometry } from './station-approach';
import { PLATFORM_EDGE_X } from './rail-profile';

/** Both styles and both terminals use the engine's rail frame and access levels. */
export function buildStationPlatform(layout: RailPlatform, art: { get(color:string, emission?:number, lamp?:boolean):THREE.Material }) {
  const s = new Kit(), top = layout.surface_y, base = top - 2;
  const phase = layout.construction_phase;
  const concrete = art.get('#b9bbb1'), metal = art.get('#94a4a0');
  const finish = () => {
    const group = s.finish();
    group.position.set(layout.x, 0, layout.z);
    group.rotation.y = Math.atan2(layout.ux, layout.uz);
    return group;
  };
  if (phase < 2) {
    for (const z of [2, 23.5, 45]) {
      s.box(5.2, .1, z, 3, .2, 3, art.get('#8b8170'));
      if (phase >= 1) s.box(5.2, base / 2, z, 2.4, base, 2.4, art.get('#9a9d93'));
    }
    return finish();
  }
  s.add(stationApproachGeometry(layout), art.get('#b3ac98'));
  s.box((7.8 + PLATFORM_EDGE_X) / 2, top - 1, 23.5, 7.8 - PLATFORM_EDGE_X, 2, 49, concrete);
  if (phase >= 3) s.box(PLATFORM_EDGE_X + .3, top + .08, 23.5, .45, .1, 48, art.get('#d3b962'));
  if (phase >= 3) {
    s.box(5.5, top + 4, 23.5, 7, .22, 51, art.get('#546d6c'));
    for (let z = 3; z <= 45; z += 10.5) {
      s.cylinder(6.7, top + 1.95, z, .14, 3.9, art.get('#d1d3c5'));
      s.box(5.4, top + 3.55, z, 3.8, .12, .12, metal);
      if (phase === 4) s.box(4.7, top + 3.7, z, 1.7, .08, .3, art.get('#ffdda5', .5, true));
    }
  }
  if (phase === 4) {
    for (const z of [12, 34]) {
      s.box(5.7, top + .6, z, 1.15, .15, 3.1, art.get('#7a6754'));
      s.box(6.2, top + 1, z, .13, .75, 3.1, art.get('#7a6754'));
    }
    s.box(5.6, top + .85, 43, 1.1, 1.7, 1.1, art.get('#557777'));
    s.box(5.58, top + 1.15, 42.42, .72, .85, .04, art.get('#ceddd8', .35, true));
  }
  for (const z of [2, 23.5, 45]) s.box(5.2, base / 2, z, 2.4, base, 2.4, art.get('#9a9d93'));

  // Signed run supports access at either slab end; tread sizes stay positive.
  const run = layout.stair_top_z - layout.stair_bottom_z;
  const rise = top - layout.ground_y, count = layout.stair_steps;
  for (let i = 0; i < count; i++) {
    const y = layout.ground_y + rise * (i + 1) / count;
    s.box(layout.stair_x, y - .12, layout.stair_bottom_z + run * (i + .5) / count,
      2.4, .24, Math.abs(run) / count + .015, concrete);
  }
  // Sloping stringers and handrails, with upright posts at regular intervals.
  const pitch = -Math.atan2(rise, run), slope = Math.hypot(run, rise);
  for (const side of [-1, 1]) {
    const x = layout.stair_x + side * 1.13;
    for (const [height, width] of [[-.28,.18],[1.05,.065]] as const) {
      const beam = new THREE.BoxGeometry(width, .09, slope);
      beam.rotateX(pitch);
      beam.translate(x, (layout.ground_y + top) / 2 + height, (layout.stair_bottom_z + layout.stair_top_z) / 2);
      s.add(beam, metal);
    }
    for (let i = 0; i <= 8; i++) {
      const t = i / 8;
      s.cylinder(x, layout.ground_y + rise * t + .52, layout.stair_bottom_z + run * t, .045, 1.04, metal);
    }
  }
  return finish();
}
