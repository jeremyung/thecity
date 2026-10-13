import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { buildTrainCar, consistCars, consistHalfLength, trainClass, type TrainCar } from '../web/src/render/train-kit';

const roles: TrainCar[] = ['cab', 'trailer', 'motor'];

function faces(g: THREE.BufferGeometry) {
  const p = g.getAttribute('position'), n = g.getAttribute('normal'), out: { face: THREE.Vector3; normal: THREE.Vector3; area: number }[] = [];
  for (let i = 0; i < p.count; i += 3) {
    const [a, b, c] = [0, 1, 2].map(k => new THREE.Vector3().fromBufferAttribute(p, i + k));
    const cross = b.clone().sub(a).cross(c.clone().sub(a));
    const normal = [0, 1, 2].reduce((s, k) => s.add(new THREE.Vector3().fromBufferAttribute(n, i + k)), new THREE.Vector3());
    out.push({ face: cross.clone().normalize(), normal: normal.normalize(), area: cross.length() / 2 });
  }
  return out;
}

describe('train kit', () => {
  it('builds an outward-wound, budgeted car at its manifest size', () => {
    const spec = trainClass();
    for (const role of roles) {
      const g = buildTrainCar('commuter', role);
      const tris = faces(g).filter(f => f.area > 1e-6);
      const outward = tris.filter(f => f.face.dot(f.normal) > 0).reduce((s, f) => s + f.area, 0);
      const total = tris.reduce((s, f) => s + f.area, 0);
      expect(outward / total).toBeGreaterThan(.97);
      expect(tris.length).toBeLessThan(2000);
      g.computeBoundingBox();
      const size = g.boundingBox!.getSize(new THREE.Vector3());
      expect(size.z).toBeCloseTo(spec.length, 1);
      expect(size.x).toBeLessThanOrEqual(spec.width + .7); // pantograph/AC width
      expect(g.boundingBox!.min.y).toBeGreaterThanOrEqual(-1e-3);
      g.dispose();
    }
  });

  it('gives every car four unsteered wheels across two bogies', () => {
    for (const role of roles) {
      const g = buildTrainCar('commuter', role), part = g.getAttribute('vpart'), wheel = g.getAttribute('wheel');
      const axles = new Set<number>();
      for (let i = 0; i < part.count; i++) {
        const isWheel = part.getX(i) === 2;
        expect(wheel.getZ(i) > 0).toBe(isWheel);
        if (isWheel) { axles.add(wheel.getY(i)); expect(wheel.getW(i)).toBe(0); }
      }
      expect(axles.size).toBe(4);
      g.dispose();
    }
  });

  it('has a front cap only on the cab, and a consist that matches the engine half-length', () => {
    const spec = trainClass();
    const capVerts = (role: TrainCar) => {
      const g = buildTrainCar('commuter', role), part = g.getAttribute('vpart');
      let n = 0;
      for (let i = 0; i < part.count; i++) if (part.getX(i) === 0 && g.getAttribute('normal').getZ(i) > .9) n++;
      g.dispose();
      return n;
    };
    expect(capVerts('cab')).toBeGreaterThan(0);
    expect(capVerts('trailer')).toBe(0);
    const cars = consistCars();
    expect(cars.map(c => c.role)).toEqual(spec.consist);
    expect(cars[0].leading).toBe(true);
    expect(cars[cars.length - 1].trailing).toBe(true);
    // Mirrors engine/src/lib.rs TRAIN_HALF_LENGTH.
    expect(consistHalfLength()).toBeCloseTo(40.9, 2);
  });
});
