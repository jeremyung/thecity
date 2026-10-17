import * as THREE from 'three/webgpu';
import type { Point, RailPlatform } from '../types';
import { Kit } from './kit';
import { sweepRoad } from './road-curve';
import type { RailPath } from './rail-path';
import {
  PANTOGRAPH_CONTACT_HEIGHT,
  RAIL_DECK,
  RAIL_HEAD_Y,
  RAIL_PLATFORM_END,
} from './rail-profile';

/** Elevated commuter corridor; platform envelopes stay unobstructed. */
export function addRailCorridor(
  kit: Kit,
  a: Point,
  b: Point,
  art: { get(color: string, roughness?: number): THREE.Material },
  platforms: readonly RailPlatform[] = [],
  path?: RailPath,
) {
  if (![a.x, a.z, b.x, b.z].every(Number.isFinite)) return;
  const dx = b.x - a.x,
    dz = b.z - a.z,
    length = path?.length ?? Math.hypot(dx, dz);
  if (length < 0.01) return;
  const ux = dx / length,
    uz = dz / length;
  const pathPose = (z: number) => path?.at(z) ?? { x: a.x + ux * z, z: a.z + uz * z, ux, uz };
  const wire = art.get('#495957', 0.65),
    metal = art.get('#8b9995', 0.65);
  const insulator = art.get('#b5b8a7', 0.8),
    concrete = art.get('#a3a69c');
  const bed = art.get('#5e655e', 0.95);
  const deckTop = 1 + RAIL_DECK + 0.15;
  const contact = RAIL_HEAD_Y + PANTOGRAPH_CONTACT_HEIGHT;
  // Terminals face inward, so their platforms occupy opposite world sides.
  const reservations = platforms.flatMap((p) => {
    const start = path?.distanceAt(p) ?? (p.x - a.x) * ux + (p.z - a.z) * uz;
    const at = pathPose(start);
    const cross = (p.x - at.x) * at.uz - (p.z - at.z) * at.ux;
    const direction = p.ux * at.ux + p.uz * at.uz;
    if (Math.abs(cross) > 0.1 || Math.abs(direction) < 0.999) return [];
    const ends = [start - 3 * direction, start + (RAIL_PLATFORM_END + 3) * direction].sort(
      (x, y) => x - y,
    );
    return [
      { start: Math.max(0, ends[0]), end: Math.min(length, ends[1]), side: Math.sign(direction) },
    ];
  });
  const box = (
    x: number,
    y: number,
    z: number,
    w: number,
    h: number,
    d: number,
    mat: THREE.Material,
  ) => {
    const p = pathPose(z);
    kit.box(p.x + p.uz * x, y, p.z - p.ux * x, w, h, d, mat, Math.atan2(p.ux, p.uz));
  };
  const run = (
    x: number,
    y: number,
    from: number,
    to: number,
    w: number,
    h: number,
    mat: THREE.Material,
  ) => {
    if (path) sweepRoad(kit, path, from, to, x, y, w, h, mat);
    else box(x, y, (from + to) / 2, w, h, to - from, mat);
  };
  const beam = (
    x1: number,
    y1: number,
    z1: number,
    x2: number,
    y2: number,
    z2: number,
    thickness: number,
    mat: THREE.Material,
  ) => {
    const p = pathPose(z1),
      q = pathPose(z2);
    const from = new THREE.Vector3(p.x + p.uz * x1, y1, p.z - p.ux * x1);
    const to = new THREE.Vector3(q.x + q.uz * x2, y2, q.z - q.ux * x2);
    const delta = to.clone().sub(from),
      pose = new THREE.Object3D();
    pose.position.copy(from).add(to).multiplyScalar(0.5);
    pose.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), delta.clone().normalize());
    pose.updateMatrix();
    const geometry = new THREE.BoxGeometry(thickness, delta.length(), thickness);
    geometry.applyMatrix4(pose.matrix);
    kit.add(geometry, mat);
  };
  // Shallow ballast strip below the sleepers; service edge stays outside trains.
  run(0, deckTop + 0.025, 0, length, 2.6, 0.05, bed);
  const cuts = [
    ...new Set([
      0,
      length,
      ...reservations.flatMap((r) => [r.start, r.end]).filter((v) => v > 0 && v < length),
    ]),
  ].sort((x, y) => x - y);
  for (let i = 1; i < cuts.length; i++) {
    const z = (cuts[i - 1] + cuts[i]) / 2;
    if (reservations.some((r) => r.side < 0 && z >= r.start && z <= r.end)) continue;
    run(-3.42, deckTop + 0.36, cuts[i - 1], cuts[i], 0.18, 0.72, concrete);
    run(-2.76, deckTop + 0.045, cuts[i - 1], cuts[i], 0.32, 0.09, concrete);
  }
  // Wire underside touches the authored pantograph shoe; no height discontinuity.
  run(0, contact + 0.0175, 0, length, 0.035, 0.035, wire);
  const spans = Math.max(1, Math.ceil(length / 32)),
    spacing = length / spans;
  for (let i = 0; i < spans; i++) {
    const z = (i + 0.5) * spacing,
      top = contact + 1.05;
    const side = reservations.some((r) => r.side < 0 && z >= r.start && z <= r.end) ? 1 : -1;
    box(side * 3.1, (deckTop + top) / 2, z, 0.16, top - deckTop, 0.16, metal);
    box(side * 3.1, deckTop + 0.18, z, 0.48, 0.36, 0.48, concrete);
    beam(side * 3.1, top - 0.12, z, -side * 0.1, contact + 0.83, z, 0.085, metal);
    beam(side * 3.1, top - 1.05, z, side * 1.35, top - 0.23, z, 0.065, metal);
    box(side * 0.65, contact + 0.87, z, 0.32, 0.14, 0.14, insulator);
    beam(0, contact + 0.83, z, 0, contact + 0.035, z, 0.035, metal);
  }
  // Messenger sag stays above the contact wire, sampled at eight-metre intervals.
  const samples = spans * 4;
  const height = (z: number) =>
    contact + 0.35 + 0.5 * Math.pow(Math.sin((Math.PI * z) / spacing), 2);
  for (let i = 0; i < samples; i++) {
    const from = (length * i) / samples,
      to = (length * (i + 1)) / samples;
    beam(0, height(from), from, 0, height(to), to, 0.028, wire);
    if (i % 2 === 1) beam(0, contact + 0.035, from, 0, height(from), from, 0.022, wire);
  }
}
