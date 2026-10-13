import * as THREE from 'three/webgpu';
import { describe, expect, it } from 'vitest';
import { buildCommuterCoach } from '../web/src/render/train-kit';
import { COACH_LENGTH, COACH_WIDTH, RAIL_CENTER_OFFSET } from '../web/src/render/rail-profile';

const art = { get: (color: string) => new THREE.MeshStandardMaterial({ color }) };
const meshes = (group: THREE.Group) => group.children.filter((child): child is THREE.Mesh => child instanceof THREE.Mesh);
const triangles = (group: THREE.Group) => meshes(group).reduce((sum, mesh) => {
  const geometry = mesh.geometry;
  return sum + (geometry.index?.count ?? geometry.getAttribute('position').count) / 3;
}, 0);

describe('commuter coach kit', () => {
  it('keeps authored geometry finite, within its vehicle envelope, and within budgets', () => {
    for (const role of ['lead', 'middle', 'tail'] as const) {
      const coach = buildCommuterCoach(art, role);
      coach.updateMatrixWorld(true);
      const box = new THREE.Box3().setFromObject(coach);
      expect(box.min.x).toBeGreaterThanOrEqual(-COACH_WIDTH / 2 - .45);
      expect(box.max.x).toBeLessThanOrEqual(COACH_WIDTH / 2 + .45);
      expect(box.min.z).toBeGreaterThanOrEqual(-COACH_LENGTH / 2 - .1);
      expect(box.max.z).toBeLessThanOrEqual(COACH_LENGTH / 2 + .1);
      expect(box.min.y).toBeGreaterThanOrEqual(-.02);
      expect(box.max.y).toBeLessThanOrEqual(5.2);
      expect(meshes(coach).length).toBeLessThanOrEqual(12);
      expect(triangles(coach)).toBeLessThanOrEqual(6000);
      for (const mesh of meshes(coach)) {
        const positions = mesh.geometry.getAttribute('position');
        for (let i = 0; i < positions.count; i++) {
          expect(Number.isFinite(positions.getX(i))).toBe(true);
          expect(Number.isFinite(positions.getY(i))).toBe(true);
          expect(Number.isFinite(positions.getZ(i))).toBe(true);
        }
      }
    }
  });

  it('places wheels on the specified gauge and distinguishes cab and powered roof roles', () => {
    const lead = buildCommuterCoach(art, 'lead');
    const middle = buildCommuterCoach(art, 'middle');
    const tail = buildCommuterCoach(art, 'tail');
    const wheelMaterial = new THREE.Color('#252b2d');
    const wheelMesh = meshes(lead).find(mesh => mesh.material instanceof THREE.Material &&
      (mesh.material as THREE.MeshStandardMaterial).color?.equals(wheelMaterial));
    expect(wheelMesh).toBeDefined();
    const wheelBounds = new THREE.Box3().setFromObject(wheelMesh!);
    expect(wheelBounds.min.x).toBeCloseTo(-RAIL_CENTER_OFFSET - .07, 3);
    expect(wheelBounds.max.x).toBeCloseTo(RAIL_CENTER_OFFSET + .07, 3);
    expect(new THREE.Box3().setFromObject(middle).max.y).toBeGreaterThan(new THREE.Box3().setFromObject(lead).max.y);
    const glassData = (group: THREE.Group) => meshes(group)
      .filter(mesh => mesh.material instanceof THREE.MeshStandardMaterial && mesh.material.color.equals(new THREE.Color('#243c45')))
      .flatMap(mesh => Array.from(mesh.geometry.getAttribute('position').array));
    expect(glassData(lead)).not.toEqual(glassData(tail));
  });
});

import { CityTrains } from '../web/src/render/trains';
import { COACH_FLOOR_HEIGHT, COACH_SPACING, PLATFORM_EDGE_X, RAIL_HEAD_Y } from '../web/src/render/rail-profile';

describe('commuter train runtime', () => {
  it('keeps floor at the authoritative platform surface and a small boarding gap', () => {
    expect(RAIL_HEAD_Y + COACH_FLOOR_HEIGHT).toBeCloseTo(8, 8);
    expect(PLATFORM_EDGE_X - COACH_WIDTH / 2).toBeCloseTo(.1, 8);
  });
  it('places, picks and clears all three coaches as one train after a direction change', () => {
    const trains = new CityTrains(art);
    try {
      trains.begin();
      trains.set(0, 42, 30, -20, Math.PI / 2);
      trains.finish(1);
      const matrix = new THREE.Matrix4(), position = new THREE.Vector3();
      for (const [i, batch] of trains.batches.entries()) {
        expect(batch.ids).toEqual([42]);
        for (const mesh of batch.meshes) {
          expect(mesh.count).toBe(1);
          mesh.getMatrixAt(0, matrix); position.setFromMatrixPosition(matrix);
          expect(position.x).toBeCloseTo(30 + (1 - i) * COACH_SPACING, 5);
          expect(position.y).toBeCloseTo(RAIL_HEAD_Y, 5);
          expect(position.z).toBeCloseTo(-20, 5);
          expect(mesh.boundingSphere?.radius).toBeGreaterThan(0);
        }
      }
      trains.begin(); trains.set(0, 42, 30, -20, -Math.PI / 2); trains.finish(1);
      trains.batches[0].meshes[0].getMatrixAt(0, matrix);
      expect(new THREE.Vector3().setFromMatrixPosition(matrix).x).toBeCloseTo(30 - COACH_SPACING, 5);
      trains.reset();
      for (const batch of trains.batches) {
        expect(batch.ids).toEqual([]);
        for (const mesh of batch.meshes) expect(mesh.count).toBe(0);
      }
    } finally { trains.dispose(); }
  });
});
