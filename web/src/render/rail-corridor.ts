import * as THREE from 'three/webgpu';
import type { Point, RailPlatform } from '../types';
import { Kit } from './kit';
import { PANTOGRAPH_CONTACT_HEIGHT, RAIL_DECK, RAIL_HEAD_Y } from './rail-profile';

/** Straight elevated commuter corridor; platform envelopes stay unobstructed. */
export function addRailCorridor(kit: Kit, a: Point, b: Point,
  art: { get(color: string, roughness?: number): THREE.Material }, platforms: readonly RailPlatform[] = []) {
  if (![a.x, a.z, b.x, b.z].every(Number.isFinite)) return;
  const dx = b.x - a.x, dz = b.z - a.z, length = Math.hypot(dx, dz);
  if (length < .01) return;
  const yaw = Math.atan2(dx, dz), ux = dx / length, uz = dz / length;
  const wire = art.get('#495957', .65), metal = art.get('#8b9995', .65);
  const insulator = art.get('#b5b8a7', .8), concrete = art.get('#a3a69c');
  const bed = art.get('#5e655e', .95);
  const deckTop = 1 + RAIL_DECK + .15;
  const contact = RAIL_HEAD_Y + PANTOGRAPH_CONTACT_HEIGHT;
  // Terminals face inward, so their platforms occupy opposite world sides.
  const reservations = platforms.flatMap(p => {
    const cross = (p.x - a.x) * uz - (p.z - a.z) * ux;
    const direction = p.ux * ux + p.uz * uz;
    if (Math.abs(cross) > .1 || Math.abs(direction) < .999) return [];
    const start = (p.x - a.x) * ux + (p.z - a.z) * uz;
    const ends = [start - 3 * direction, start + 51 * direction].sort((x, y) => x - y);
    return [{ start: Math.max(0, ends[0]), end: Math.min(length, ends[1]), side: Math.sign(direction) }];
  });
  const box = (x: number, y: number, z: number, w: number, h: number, d: number, mat: THREE.Material) =>
    kit.box(a.x + uz * x + ux * z, y, a.z - ux * x + uz * z, w, h, d, mat, yaw);
  const beam = (x1: number, y1: number, z1: number, x2: number, y2: number, z2: number, thickness: number, mat: THREE.Material) => {
    const from = new THREE.Vector3(a.x + uz * x1 + ux * z1, y1, a.z - ux * x1 + uz * z1);
    const to = new THREE.Vector3(a.x + uz * x2 + ux * z2, y2, a.z - ux * x2 + uz * z2);
    const delta = to.clone().sub(from), pose = new THREE.Object3D();
    pose.position.copy(from).add(to).multiplyScalar(.5);
    pose.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), delta.clone().normalize());
    pose.updateMatrix();
    const geometry = new THREE.BoxGeometry(thickness, delta.length(), thickness);
    geometry.applyMatrix4(pose.matrix); kit.add(geometry, mat);
  };
  // Shallow ballast strip below the sleepers; service edge stays outside trains.
  box(0, deckTop + .025, length / 2, 2.6, .05, length, bed);
  const cuts = [...new Set([0, length, ...reservations.flatMap(r => [r.start, r.end]).filter(v => v > 0 && v < length)])].sort((x, y) => x - y);
  for (let i = 1; i < cuts.length; i++) {
    const z = (cuts[i - 1] + cuts[i]) / 2, run = cuts[i] - cuts[i - 1];
    if (reservations.some(r => r.side < 0 && z >= r.start && z <= r.end)) continue;
    box(-3.42, deckTop + .36, z, .18, .72, run, concrete);
    box(-2.76, deckTop + .045, z, .32, .09, run, concrete);
  }
  // Wire underside touches the authored pantograph shoe; no height discontinuity.
  box(0, contact + .0175, length / 2, .035, .035, length, wire);
  const spans = Math.max(1, Math.ceil(length / 32)), spacing = length / spans;
  for (let i = 0; i < spans; i++) {
    const z = (i + .5) * spacing, top = contact + 1.05;
    const side = reservations.some(r => r.side < 0 && z >= r.start && z <= r.end) ? 1 : -1;
    box(side * 3.1, (deckTop + top) / 2, z, .16, top - deckTop, .16, metal);
    box(side * 3.1, deckTop + .18, z, .48, .36, .48, concrete);
    beam(side * 3.1, top - .12, z, -side * .1, contact + .83, z, .085, metal);
    beam(side * 3.1, top - 1.05, z, side * 1.35, top - .23, z, .065, metal);
    box(side * .65, contact + .87, z, .32, .14, .14, insulator);
    beam(0, contact + .83, z, 0, contact + .035, z, .035, metal);
  }
  // Messenger sag stays above the contact wire, sampled at eight-metre intervals.
  const samples = spans * 4;
  const height = (z: number) => contact + .35 + .5 * Math.pow(Math.sin(Math.PI * z / spacing), 2);
  for (let i = 0; i < samples; i++) {
    const from = length * i / samples, to = length * (i + 1) / samples;
    beam(0, height(from), from, 0, height(to), to, .028, wire);
    if (i % 2 === 1) beam(0, contact + .035, from, 0, height(from), from, .022, wire);
  }
}
