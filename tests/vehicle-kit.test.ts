import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import {
  buildLiveryGeometry,
  buildVehicleGeometry,
  CAR_BODIES,
  vehicleSpec,
  type VehicleBody,
} from '../web/src/render/vehicle-kit';

const bodies: VehicleBody[] = [...CAR_BODIES, 'bus'];

function faces(g: THREE.BufferGeometry) {
  const p = g.getAttribute('position'),
    n = g.getAttribute('normal'),
    out: { face: THREE.Vector3; normal: THREE.Vector3; area: number }[] = [];
  for (let i = 0; i < p.count; i += 3) {
    const [a, b, c] = [0, 1, 2].map((k) => new THREE.Vector3().fromBufferAttribute(p, i + k));
    const cross = b.clone().sub(a).cross(c.clone().sub(a));
    const normal = [0, 1, 2].reduce(
      (s, k) => s.add(new THREE.Vector3().fromBufferAttribute(n, i + k)),
      new THREE.Vector3(),
    );
    out.push({
      face: cross.clone().normalize(),
      normal: normal.normalize(),
      area: cross.length() / 2,
    });
  }
  return out;
}

describe('vehicle kit', () => {
  it('interleaves all static vehicle vertex attributes into one lossless buffer', () => {
    const g = buildVehicleGeometry('taxi');
    const layout = [
      ['position', 3, 0],
      ['normal', 3, 3],
      ['uv', 2, 6],
      ['vpart', 1, 8],
      ['lateral', 1, 9],
      ['wheel', 4, 10],
      ['tinted', 1, 14],
    ] as const;
    const position = g.getAttribute('position');
    expect(position).toBeInstanceOf(THREE.InterleavedBufferAttribute);
    const data = (position as THREE.InterleavedBufferAttribute).data;
    expect(data.stride).toBe(15);
    expect(data.array).toHaveLength(position.count * data.stride);
    for (const [name, size, offset] of layout) {
      const attribute = g.getAttribute(name);
      expect(attribute).toBeInstanceOf(THREE.InterleavedBufferAttribute);
      const interleaved = attribute as THREE.InterleavedBufferAttribute;
      expect(interleaved.data).toBe(data);
      expect(interleaved.itemSize).toBe(size);
      expect(interleaved.offset).toBe(offset);
      expect(interleaved.count).toBe(position.count);
      for (let vertex = 0; vertex < attribute.count; vertex++) {
        for (let component = 0; component < size; component++) {
          expect(attribute.getComponent(vertex, component)).toBe(
            data.array[vertex * data.stride + offset + component],
          );
        }
      }
    }
    g.dispose();
  });

  it.each(bodies)('%s is an outward-wound, budgeted body at its manifest size', (body) => {
    const g = buildVehicleGeometry(body),
      spec = vehicleSpec(body);
    const tris = faces(g).filter((f) => f.area > 1e-6);
    const outward = tris.filter((f) => f.face.dot(f.normal) > 0).reduce((s, f) => s + f.area, 0);
    const total = tris.reduce((s, f) => s + f.area, 0);
    expect(outward / total).toBeGreaterThan(0.98);
    expect(tris.length).toBeLessThan(body === 'bus' ? 2500 : 1200);
    g.computeBoundingBox();
    const size = g.boundingBox!.getSize(new THREE.Vector3());
    expect(size.z).toBeCloseTo(spec.length, 1);
    expect(size.x).toBeLessThanOrEqual(spec.width + 1e-3);
    expect(g.boundingBox!.min.y).toBeGreaterThanOrEqual(-1e-3);
    g.dispose();
  });

  it('marks only wheel vertices with an axle and steers only front wheels on cars', () => {
    for (const body of bodies) {
      const g = buildVehicleGeometry(body),
        part = g.getAttribute('vpart'),
        wheel = g.getAttribute('wheel');
      const axles = new Set<number>(),
        steered = new Set<number>();
      for (let i = 0; i < part.count; i++) {
        const isWheel = part.getX(i) === 2;
        expect(wheel.getZ(i) > 0).toBe(isWheel);
        if (isWheel) {
          axles.add(wheel.getY(i));
          if (wheel.getW(i) !== 0) steered.add(wheel.getY(i));
        }
      }
      expect(axles.size).toBe(2);
      expect(steered.size).toBe(body === 'bus' ? 0 : 1);
      if (body !== 'bus') expect([...steered][0]).toBe(Math.max(...axles));
      g.dispose();
    }
  });

  it('keeps livery lettering readable from both sides', () => {
    const g = buildLiveryGeometry('van', { z0: -1.5, z1: 1, y0: 0.7, y1: 1.5 });
    const p = g.getAttribute('position'),
      uv = g.getAttribute('uv'),
      n = g.getAttribute('normal');
    for (let i = 0; i < p.count; i++) {
      const sign = Math.sign(n.getX(i));
      expect(Math.sign(p.getX(i))).toBe(sign);
      // u grows toward the viewer's right: +z from the -x side, -z from the +x side.
      const u = uv.getX(i),
        z = p.getZ(i);
      expect(u).toBeCloseTo(sign < 0 ? (z + 1.5) / 2.5 : (1 - z) / 2.5, 5);
    }
    for (const f of faces(g)) expect(f.face.dot(f.normal)).toBeGreaterThan(0.99);
    g.dispose();
  });
});
