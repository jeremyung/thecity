import { describe, expect, it } from 'vitest';
import * as THREE from 'three';
import { dampFocusDepth, focusDepthFromViewZ } from '../web/src/optics/focus';

describe('camera focus depth', () => {
  it('uses axial camera-space depth for an off-axis target', () => {
    const camera = new THREE.PerspectiveCamera(45, 1, 0.5, 3_500);
    camera.position.set(0, 4, 10);
    const target = new THREE.Vector3(8, 2, -5);
    camera.lookAt(0, 0, 0);
    camera.updateMatrixWorld();
    const viewTarget = target.clone().applyMatrix4(camera.matrixWorldInverse);
    const axialDepth = focusDepthFromViewZ(viewTarget.z, camera.near, camera.far);
    const euclideanDistance = camera.position.distanceTo(target);

    expect(viewTarget.x).toBeGreaterThan(0);
    expect(axialDepth).toBeCloseTo(-viewTarget.z, 10);
    expect(axialDepth).toBeLessThan(euclideanDistance);
  });

  it('keeps depth positive and bounded for behind-camera, near, far, and non-finite inputs', () => {
    expect(focusDepthFromViewZ(4, 0.5, 100)).toBe(0.5);
    expect(focusDepthFromViewZ(-0.1, 0.5, 100)).toBe(0.5);
    expect(focusDepthFromViewZ(-120, 0.5, 100)).toBe(100);
    expect(focusDepthFromViewZ(Number.NaN, 0.5, 100)).toBe(0.5);
    expect(Number.isFinite(focusDepthFromViewZ(Number.POSITIVE_INFINITY))).toBe(true);
  });

  it('damps equivalently across frame rates', () => {
    let at30Hz = 100;
    let at120Hz = 100;
    for (let i = 0; i < 30; i++) at30Hz = dampFocusDepth(at30Hz, 40, 1 / 30);
    for (let i = 0; i < 120; i++) at120Hz = dampFocusDepth(at120Hz, 40, 1 / 120);
    expect(at30Hz).toBeCloseTo(at120Hz, 10);
    expect(at30Hz).toBeLessThan(100);
    expect(at30Hz).toBeGreaterThan(40);
  });

  it('keeps invalid damping values finite and converges over a long frame interval', () => {
    expect(dampFocusDepth(Number.NaN, 35, 0.1)).toBe(35);
    expect(dampFocusDepth(35, Number.NaN, 0.1)).toBe(35);
    expect(dampFocusDepth(35, 80, Number.NaN)).toBe(35);
    expect(dampFocusDepth(35, 80, 1_000)).toBeCloseTo(80, 10);
  });
});
