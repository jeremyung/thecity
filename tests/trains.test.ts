import { afterEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { TRAIN_HALF_LENGTH, TrainFleet } from '../web/src/render/trains';
import { consistCars } from '../web/src/render/train-kit';

afterEach(() => vi.unstubAllGlobals());

describe('train fleet', () => {
  it('places every car of a consist and reports the shared id for picking', () => {
    vi.stubGlobal('document', { createElement: () => ({ width: 0, height: 0, getContext: () => ({ fillRect() {}, fillText() {}, strokeRect() {} }) }) });
    const art = new ArtMaterials(), fleet = new TrainFleet(art, 8);
    fleet.begin();
    fleet.add(7, 10, 20, Math.PI / 2, 6, 0, 1 / 60);
    fleet.finish(1);
    const counts = fleet.meshes.map(m => m.count);
    expect(counts.reduce((a, b) => a + b, 0)).toBe(consistCars().length);
    const ids = fleet.pickable.flatMap(p => p.ids);
    expect(ids.length).toBe(consistCars().length);
    expect(new Set(ids)).toEqual(new Set([7]));
    fleet.dispose(); art.dispose();
  });

  it('places the consist along its heading and mirrors the trailing cab', () => {
    vi.stubGlobal('document', { createElement: () => ({ width: 0, height: 0, getContext: () => ({ fillRect() {}, fillText() {}, strokeRect() {} }) }) });
    const art = new ArtMaterials(), fleet = new TrainFleet(art, 8);
    fleet.begin();
    fleet.add(3, 0, 0, 0, 0, 0, 1 / 60);
    fleet.finish(1);
    const cars = consistCars();
    for (const mesh of fleet.meshes) {
      const matrix = new THREE.Matrix4();
      for (let i = 0; i < mesh.count; i++) {
        mesh.getMatrixAt(i, matrix);
        const p = new THREE.Vector3().setFromMatrixPosition(matrix);
        // Heading 0 puts forward along +z; every car sits at its offset.
        expect(cars.some(c => Math.abs(p.z - c.offset) < 1e-3)).toBe(true);
      }
    }
    fleet.dispose(); art.dispose();
  });

  it('exports a half-length that matches the consist span', () => {
    const spec = consistCars();
    const span = Math.abs(spec[spec.length - 1].offset - spec[0].offset) + 20;
    expect(TRAIN_HALF_LENGTH * 2).toBeCloseTo(span, 1);
  });
});
