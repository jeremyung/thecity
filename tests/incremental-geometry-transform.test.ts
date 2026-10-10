import * as THREE from 'three/webgpu';
import { describe, expect, it } from 'vitest';
import { IncrementalGeometryTransform } from '../web/src/render/incremental-geometry-transform';

function makeGeometry(indexed = true): THREE.BufferGeometry {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute([
    -1, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 1, 1, 1, 2, 0, 1,
  ], 3));
  geometry.setAttribute('normal', new THREE.Float32BufferAttribute([
    0, 1, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 0, -1, 0, 0, 0, -1,
  ], 3));
  geometry.setAttribute('tangent', new THREE.Float32BufferAttribute([
    1, 0, 0, -1, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 0, 1, 1, 0, 0, -1, 0, 1, 1, 1,
  ], 4));
  geometry.setAttribute('color', new THREE.Float32BufferAttribute([
    0.1, 0.2, 0.3, 0.3, 0.4, 0.5, 0.5, 0.6, 0.7, 0.7, 0.8, 0.9, 1, 0.5, 0.1, 0.2, 0.4, 0.6,
  ], 3));
  geometry.setAttribute('uv', new THREE.Float32BufferAttribute([0, 0, 1, 0, 0.5, 1, 1, 1, 0, 0, 1, 0], 2));
  if (indexed) geometry.setIndex(new THREE.BufferAttribute(new Uint16Array([0, 1, 2, 3, 4, 5]), 1));
  geometry.addGroup(0, 3, 2);
  geometry.setDrawRange(0, indexed ? 6 : 6);
  return geometry;
}

function complete(job: IncrementalGeometryTransform, vertexQuota = 2, indexQuota = 3): THREE.BufferGeometry {
  while (!job.done) job.step(vertexQuota, indexQuota);
  return job.result!;
}

function expectSameAttributes(actual: THREE.BufferGeometry, expected: THREE.BufferGeometry): void {
  expect(Object.keys(actual.attributes).sort()).toEqual(Object.keys(expected.attributes).sort());
  for (const key of Object.keys(expected.attributes)) {
    expect(actual.getAttribute(key).array).toEqual(expected.getAttribute(key).array);
  }
  expect(actual.index!.array).toEqual(expected.index!.array);
}

describe('IncrementalGeometryTransform', () => {
  it('matches clone().applyMatrix4 for transformed positions, normals, tangents, and copied attributes', () => {
    const source = makeGeometry();
    const matrix = new THREE.Matrix4().compose(
      new THREE.Vector3(17, 4, -9),
      new THREE.Quaternion().setFromEuler(new THREE.Euler(0.31, -0.72, 0.18)),
      new THREE.Vector3(1.8, 0.65, 2.3),
    );
    const expected = source.clone().applyMatrix4(matrix);
    expected.computeBoundingBox();
    const job = new IncrementalGeometryTransform(source, matrix);
    while (!job.done) {
      const beforeV = job.processedVertices, beforeI = job.processedIndices;
      job.step(2, 2);
      expect(job.processedVertices - beforeV).toBeLessThanOrEqual(2);
      expect(job.processedIndices - beforeI).toBeLessThanOrEqual(2);
    }
    const actual = job.result!;
    expectSameAttributes(actual, expected);
    expect(actual.groups).toEqual(source.groups);
    expect(actual.drawRange).toEqual(source.drawRange);
    expect(actual.boundingBox!.min.toArray()).toEqual(expected.boundingBox!.min.toArray());
    expect(actual.boundingBox!.max.toArray()).toEqual(expected.boundingBox!.max.toArray());
    expect(actual.boundingSphere!.center.toArray()).toEqual(actual.boundingBox!.getCenter(new THREE.Vector3()).toArray());
    expect(actual.boundingSphere!.radius).toBeCloseTo(actual.boundingBox!.getSize(new THREE.Vector3()).length() / 2);
    actual.dispose(); expected.dispose(); source.dispose(); job.dispose();
  });

  it('overrides colors with the supplied linear RGB while leaving source colors unchanged', () => {
    const source = makeGeometry();
    const originalColor = source.getAttribute('color').array.slice();
    const color = new THREE.Color().setRGB(0.23, 0.47, 0.81);
    const actual = complete(new IncrementalGeometryTransform(source, new THREE.Matrix4().makeTranslation(2, 3, 4), color));
    const colors = actual.getAttribute('color');
    expect(colors.itemSize).toBe(3);
    for (let i = 0; i < colors.count; i++) {
      expect(colors.getX(i)).toBeCloseTo(color.r, 6);
      expect(colors.getY(i)).toBeCloseTo(color.g, 6);
      expect(colors.getZ(i)).toBeCloseTo(color.b, 6);
    }
    expect(source.getAttribute('color').array).toEqual(originalColor);
    actual.dispose(); source.dispose();
  });

  it('synthesizes identity indices incrementally for unindexed geometry, widening at the 0xffff boundary', () => {
    const small = makeGeometry(false);
    const transformed = complete(new IncrementalGeometryTransform(small, new THREE.Matrix4()), 2, 2);
    expect(transformed.index!.array).toBeInstanceOf(Uint16Array);
    expect(Array.from(transformed.index!.array)).toEqual([0, 1, 2, 3, 4, 5]);
    transformed.dispose(); small.dispose();

    const boundary = new THREE.BufferGeometry();
    const positions = new Float32Array(65538 * 3);
    boundary.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    const job = new IncrementalGeometryTransform(boundary, new THREE.Matrix4());
    expect(job.result).toBeNull();
    expect(job.processedVertices).toBe(0);
    expect(job.processedIndices).toBe(0);
    const large = complete(job, 8192, 8192);
    expect(large.index!.array).toBeInstanceOf(Uint32Array);
    expect(large.index!.array[65535]).toBe(65535);
    expect(large.index!.array[65537]).toBe(65537);
    large.dispose(); boundary.dispose();
  });

  it('validates unsupported layouts before destination allocation and index values under quota', () => {
    const invalid = makeGeometry();
    invalid.setAttribute('normal', new THREE.Float32BufferAttribute(new Float32Array(6), 3));
    expect(() => new IncrementalGeometryTransform(invalid, new THREE.Matrix4())).toThrow(/different vertex counts/i);
    invalid.dispose();

    const source = makeGeometry();
    const index = source.index!;
    const oldGetX = index.getX.bind(index);
    let reads = 0;
    index.getX = i => { reads++; return oldGetX(i); };
    const job = new IncrementalGeometryTransform(source, new THREE.Matrix4());
    expect(reads).toBe(0);
    job.step(1, 2);
    expect(reads).toBe(2);
    job.step(1, 1);
    expect(reads).toBe(3);
    job.dispose(); source.dispose();
  });

  it('releases only its partial output on cancellation and rejects further work', () => {
    const source = makeGeometry();
    const originalPositions = source.getAttribute('position').array;
    let sourceDisposed = false;
    source.addEventListener('dispose', () => { sourceDisposed = true; });
    const job = new IncrementalGeometryTransform(source, new THREE.Matrix4().makeTranslation(5, 0, 0));
    job.step(1, 1);
    job.dispose();
    expect(job.result).toBeNull();
    expect(() => job.step(1, 1)).toThrow(/disposed/i);
    expect(sourceDisposed).toBe(false);
    expect(source.getAttribute('position').array).toBe(originalPositions);
    source.dispose();
  });
});
