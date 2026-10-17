import { expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { snapShadowAnchor } from '../web/src/render/shadow-anchor';

it('holds the light-plane grid fixed for subtexel camera motion without changing depth', () => {
  const offset = new THREE.Vector3(200, 350, -160);
  const z = offset.clone().normalize();
  const x = new THREE.Vector3(0, 1, 0).cross(z).normalize();
  const y = z.clone().cross(x).normalize();
  const target = x.clone().multiplyScalar(12).addScaledVector(y, -8).addScaledVector(z, 25);
  const a = snapShadowAnchor(target, offset, 512, 512, 2048, 2048, new THREE.Vector3());
  const b = snapShadowAnchor(
    target.clone().addScaledVector(x, 0.05).addScaledVector(y, 0.04),
    offset,
    512,
    512,
    2048,
    2048,
    new THREE.Vector3(),
  );
  expect(a.distanceTo(b)).toBeLessThan(1e-10);
  expect(a.dot(z)).toBeCloseTo(target.dot(z), 10);
  expect(a.clone().sub(target).length()).toBeLessThanOrEqual(Math.SQRT2 * 0.125);
});

it('supports rectangular maps and crosses grid cells in whole texel increments', () => {
  const offset = new THREE.Vector3(0, 0, 100);
  const result = snapShadowAnchor(
    new THREE.Vector3(0.31, 0.43, 12),
    offset,
    100,
    200,
    100,
    400,
    new THREE.Vector3(),
  );
  expect(result.toArray()).toEqual([0, 0.5, 12]);
  const crossed = snapShadowAnchor(
    new THREE.Vector3(0.6, 0.43, 12),
    offset,
    100,
    200,
    100,
    400,
    new THREE.Vector3(),
  );
  expect(crossed.x - result.x).toBe(1);
});
