import { afterEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { TRAIN_HALF_LENGTH, TrainFleet } from '../web/src/render/trains';
import { consistCars } from '../web/src/render/train-kit';

afterEach(() => vi.unstubAllGlobals());

function fakeDocument() {
  vi.stubGlobal('document', {
    createElement: () => ({
      width: 0,
      height: 0,
      getContext: () => ({ fillRect() {}, fillText() {}, strokeRect() {} }),
    }),
  });
}

const CAR_COUNT = consistCars().length;

describe('train fleet', () => {
  it('sizes each role buffer by its multiplicity, not one per train', () => {
    fakeDocument();
    const art = new ArtMaterials(),
      fleet = new TrainFleet(art, 8);
    fleet.begin();
    for (let i = 0; i < 8; i++) fleet.add(100 + i, i * 120, 0, 0, 6, 0, 1 / 60);
    fleet.finish(8);
    const total = fleet.meshes.reduce((sum, mesh) => sum + mesh.count, 0);
    // Eight four-car consists: two cabs per consist must not be silently dropped.
    expect(total).toBe(CAR_COUNT * 8);
    // The cab role carries two instances per consist, so it must exceed eight.
    expect(fleet.meshes.filter((mesh) => mesh.count === 16)).toHaveLength(1);
    fleet.dispose();
    art.dispose();
  });

  it('resets counts and rewrites transforms every begin(), so city frames do not accumulate', () => {
    fakeDocument();
    const art = new ArtMaterials(),
      fleet = new TrainFleet(art, 4);
    fleet.begin();
    fleet.add(5, 0, 0, 0, 0, 0, 1 / 60);
    fleet.finish(1);
    const before = fleet.meshes.reduce((sum, mesh) => sum + mesh.count, 0);
    // A later frame at a new position must land in slot zero, not a new slot.
    fleet.begin();
    fleet.add(5, 9, 0, 0, 0, 0, 1 / 60);
    fleet.finish(1);
    const after = fleet.meshes.reduce((sum, mesh) => sum + mesh.count, 0);
    expect(before).toBe(CAR_COUNT);
    expect(after).toBe(CAR_COUNT);
    const matrix = new THREE.Matrix4();
    const cab = fleet.meshes.find((mesh) => mesh.count === 2)!;
    cab.getMatrixAt(0, matrix);
    expect(new THREE.Vector3().setFromMatrixPosition(matrix).x).toBeCloseTo(9, 3);
    fleet.dispose();
    art.dispose();
  });

  it('points the leading cab along the heading and the trailing cab backwards', () => {
    fakeDocument();
    const art = new ArtMaterials(),
      fleet = new TrainFleet(art, 2);
    fleet.begin();
    fleet.add(3, 0, 0, 0, 0, 0, 1 / 60);
    fleet.finish(1);
    const cab = fleet.meshes.find((mesh) => mesh.count === 2)!;
    const matrix = new THREE.Matrix4(),
      pos = new THREE.Vector3(),
      xAxis = new THREE.Vector3(),
      yAxis = new THREE.Vector3(),
      zAxis = new THREE.Vector3();
    const forwards: number[] = [];
    for (let i = 0; i < cab.count; i++) {
      cab.getMatrixAt(i, matrix);
      pos.setFromMatrixPosition(matrix);
      matrix.extractBasis(xAxis, yAxis, zAxis);
      // Heading 0 is +z; the front cap is local +z. Forward must point away from centre.
      forwards.push(Math.sign(pos.z) * zAxis.z);
    }
    expect(forwards.length).toBe(2);
    expect(
      forwards.every((f) => f > 0.9),
      `both cabs face outward, got ${forwards}`,
    ).toBe(true);
    fleet.dispose();
    art.dispose();
  });

  it('exports a half-length that matches the consist span', () => {
    const cars = consistCars();
    const span = Math.abs(cars[cars.length - 1].offset - cars[0].offset) + 20;
    expect(TRAIN_HALF_LENGTH * 2).toBeCloseTo(span, 1);
  });
});
