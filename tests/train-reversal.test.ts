import * as THREE from 'three/webgpu';
import { describe, expect, it } from 'vitest';
import { CityTrains } from '../web/src/render/trains';
import type { World } from '../web/src/types';
import { COACH_SPACING } from '../web/src/render/rail-profile';

const art = { get: (color: string) => new THREE.MeshStandardMaterial({ color }) };

function world(id: number, route: number[], nodes: { id: number; x: number; z: number }[]): World {
  return {
    seed: 1, tick: 0, trains: [{ id, route, progress: 0 }],
    nodes: nodes.map(node => ({ ...node, kind: 'rail' })),
  } as World;
}

function render(trains: CityTrains, yaw: number) {
  trains.begin();
  trains.set(0, 42, 12, 80, yaw);
  trains.finish(1);
}

function matrices(meshes: THREE.InstancedMesh[]) {
  const matrix = new THREE.Matrix4();
  return meshes.map(mesh => {
    mesh.getMatrixAt(0, matrix);
    return matrix.toArray().map(value => Number(value.toFixed(6)));
  });
}

function lampPose(mesh: THREE.InstancedMesh) {
  const matrix = new THREE.Matrix4();
  mesh.getMatrixAt(0, matrix);
  const position = new THREE.Vector3();
  const rotation = new THREE.Quaternion();
  matrix.decompose(position, rotation, new THREE.Vector3());
  mesh.geometry.computeBoundingBox();
  mesh.geometry.boundingBox!.getCenter(position).applyMatrix4(matrix);
  return { position, rotation };
}

describe('commuter train terminal reversal', () => {
  it('keeps the physical consist fixed and swaps endpoint lamps with travel direction', () => {
    const trains = new CityTrains(art);
    try {
      trains.setWorld(world(42, [1, 2], [{ id: 1, x: 0, z: 0 }, { id: 2, x: 0, z: 200 }]));

      render(trains, 0);
      const forwardBodies = trains.batches.map(batch => matrices(batch.meshes));
      const forwardWarm = lampPose(trains.lamps.meshes[0]);
      const forwardRed = lampPose(trains.lamps.meshes[1]);

      for (const turn of [Math.PI / 4, Math.PI / 2, 3 * Math.PI / 4]) {
        render(trains, turn);
        expect(trains.batches.map(batch => matrices(batch.meshes))).toEqual(forwardBodies);
      }
      render(trains, Math.PI);
      const reverseBodies = trains.batches.map(batch => matrices(batch.meshes));
      expect(reverseBodies).toEqual(forwardBodies);

      const reverseWarm = lampPose(trains.lamps.meshes[0]);
      const reverseRed = lampPose(trains.lamps.meshes[1]);
      expect(forwardWarm.position.z).toBeCloseTo(80 + COACH_SPACING + 7.559, 4);
      expect(forwardRed.position.z).toBeCloseTo(80 - COACH_SPACING - 7.559, 4);
      expect(reverseWarm.position.z).toBeCloseTo(forwardRed.position.z, 4);
      expect(reverseRed.position.z).toBeCloseTo(forwardWarm.position.z, 4);
      expect(Math.abs(forwardWarm.rotation.angleTo(reverseWarm.rotation))).toBeCloseTo(Math.PI, 4);
      expect(Math.abs(forwardRed.rotation.angleTo(reverseRed.rotation))).toBeCloseTo(Math.PI, 4);
      expect(trains.lamps.ids).toEqual([42]);
    } finally {
      trains.dispose();
    }
  });

  it('clears cached route headings when the world is replaced', () => {
    const trains = new CityTrains(art);
    try {
      trains.setWorld(world(42, [1, 2], [{ id: 1, x: 0, z: 0 }, { id: 2, x: 0, z: 200 }]));
      // Reuse the actor id in a replacement world with no usable route. The supplied
      // travel heading must remain the fallback instead of inheriting the old route.
      trains.setWorld(world(42, [7, 8], [{ id: 7, x: 0, z: 0 }]));
      render(trains, Math.PI / 2);
      const matrix = new THREE.Matrix4();
      trains.batches[0].meshes[0].getMatrixAt(0, matrix);
      const rotation = new THREE.Quaternion().setFromRotationMatrix(matrix);
      expect(Math.abs(rotation.angleTo(new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), Math.PI / 2)))).toBeCloseTo(0, 4);
    } finally {
      trains.dispose();
    }
  });
});
