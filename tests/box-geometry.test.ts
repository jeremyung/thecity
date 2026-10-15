import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { boxGeometry } from '../web/src/render/box-geometry';

function reference(x: number, y: number, z: number, w: number, h: number, d: number, rotation: number) {
  return new THREE.BoxGeometry(w, h, d).rotateY(rotation).translate(x, y, z);
}
function expectSame(actual: THREE.BufferGeometry, expected: THREE.BufferGeometry) {
  for (const key of ['position', 'normal', 'uv']) {
    expect(Array.from(actual.getAttribute(key).array as ArrayLike<number>), key)
      .toEqual(Array.from(expected.getAttribute(key).array as ArrayLike<number>));
  }
  expect(Array.from(actual.index!.array)).toEqual(Array.from(expected.index!.array));
  expect(actual.groups).toEqual(expected.groups);
  expect(actual.boundingBox).toEqual(expected.boundingBox);
  expect(actual.boundingSphere).toEqual(expected.boundingSphere);
}

describe('boxGeometry', () => {
  it('matches positive-dimension BoxGeometry arrays for randomized transforms', () => {
    let state = 87231;
    const random = () => { state = (Math.imul(state, 1664525) + 1013904223) >>> 0; return state / 0x100000000; };
    for (let i = 0; i < 500; i++) {
      const dimension = () => [Number.MIN_VALUE, 1e-5, .1, 1, 3.75, 10.125, 1e5][Math.floor(random() * 7)];
      const offset = () => [0, -100000, 1e-5, .375, 98765.125][Math.floor(random() * 5)];
      const rotation = i % 4 === 0 ? [0, Math.PI / 2, Math.PI, -Math.PI / 2][i / 4 % 4] : (random() - .5) * Math.PI * 8;
      const input = [offset(), offset(), offset(), dimension(), dimension(), dimension(), rotation] as const;
      const actual = boxGeometry(...input), expected = reference(...input);
      expectSame(actual, expected);
      actual.dispose(); expected.dispose();
    }
  });

  it('preserves Three behavior for zero, negative, and nonfinite dimensions', () => {
    for (const dimension of [0, -0, -1, -20, Infinity, -Infinity, NaN]) {
      for (const axis of [0, 1, 2]) {
        const dimensions: [number, number, number] = [2, 3, 4]; dimensions[axis] = dimension;
        const input = [2.75, -3.5, 1e6, ...dimensions, .713] as const;
        const actual = boxGeometry(...input), expected = reference(...input);
        expectSame(actual, expected);
        actual.dispose(); expected.dispose();
      }
    }
  });

  it('keeps result buffers independent', () => {
    const a = boxGeometry(0, 0, 0, 2, 3, 4), b = boxGeometry(0, 0, 0, 2, 3, 4);
    expect(a.getAttribute('position').array).not.toBe(b.getAttribute('position').array);
    expect(a.getAttribute('normal').array).not.toBe(b.getAttribute('normal').array);
    expect(a.getAttribute('uv').array).not.toBe(b.getAttribute('uv').array);
    expect(a.index!.array).not.toBe(b.index!.array);
    (a.getAttribute('position').array as Float32Array)[0] = 123;
    expect((b.getAttribute('position').array as Float32Array)[0]).not.toBe(123);
    a.dispose(); b.dispose();
  });
});
