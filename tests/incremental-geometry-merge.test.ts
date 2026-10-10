import * as THREE from 'three/webgpu';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { describe, expect, it } from 'vitest';
import { IncrementalGeometryMerge } from '../web/src/render/incremental-geometry-merge';

function box(x: number, color: number): THREE.BufferGeometry {
  const geometry = new THREE.BoxGeometry(2, 3, 4);
  geometry.translate(x, 1.5, -x * 0.25);
  const positions = geometry.getAttribute('position');
  const colors = new Float32Array(positions.count * 3);
  for (let i = 0; i < positions.count; i++) {
    colors[i * 3] = color; colors[i * 3 + 1] = color * 0.5; colors[i * 3 + 2] = 1 - color;
  }
  geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  return geometry;
}

function finish(merger: IncrementalGeometryMerge, v = 17, i = 23): THREE.BufferGeometry {
  while (!merger.done) merger.step(v, i);
  return merger.result!;
}

describe('IncrementalGeometryMerge', () => {
  it('matches Three merge attributes, indices, and triangle order across bounded slices', () => {
    const sources = [box(-3, 0.2), box(8, 0.8), box(22, 0.45)];
    const expected = mergeGeometries(sources, false)!;
    const merger = new IncrementalGeometryMerge(sources);
    expect(merger.result).toBeNull();
    while (!merger.done) {
      const beforeVertices = merger.processedVertices, beforeIndices = merger.processedIndices;
      merger.step(11, 13);
      expect(merger.processedVertices - beforeVertices).toBeLessThanOrEqual(11);
      expect(merger.processedIndices - beforeIndices).toBeLessThanOrEqual(13);
    }
    const actual = merger.result!;
    expect(actual.index!.array).toEqual(expected.index!.array);
    for (const name of Object.keys(expected.attributes)) {
      expect(actual.getAttribute(name).array).toEqual(expected.getAttribute(name).array);
    }
    expect(actual.boundingBox!.min.toArray()).toEqual([ -4, 0, -7.5 ]);
    expect(actual.boundingSphere!.center.toArray()).toEqual(actual.boundingBox!.getCenter(new THREE.Vector3()).toArray());
    expect(actual.boundingSphere!.radius).toBeCloseTo(actual.boundingBox!.getSize(new THREE.Vector3()).length() / 2);
    sources.forEach(source => source.dispose());
    expected.dispose();
    merger.dispose();
  });

  it('widens mixed 16/32-bit source indices when accumulated offsets exceed 65535', () => {
    const large = new THREE.BufferGeometry();
    const positions = new Float32Array(65536 * 3);
    for (let i = 0; i < 65536; i++) positions[i * 3] = i;
    large.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    large.setIndex(new THREE.BufferAttribute(new Uint16Array([65533, 65534, 65535]), 1));
    const small = new THREE.BufferGeometry();
    small.setAttribute('position', new THREE.Float32BufferAttribute([0, 0, 0, 1, 0, 0, 0, 1, 0], 3));
    small.setIndex(new THREE.BufferAttribute(new Uint32Array([0, 1, 2]), 1));

    const merged = finish(new IncrementalGeometryMerge([large, small]), 4096, 2);
    expect(merged.index!.array).toBeInstanceOf(Uint32Array);
    expect(Array.from(merged.index!.array.slice(-3))).toEqual([65536, 65537, 65538]);
    merged.dispose(); large.dispose(); small.dispose();
  });

  it('supports unindexed inputs without changing sequential triangle order', () => {
    const a = box(0, 0.2).toNonIndexed();
    const b = box(9, 0.7).toNonIndexed();
    const expected = mergeGeometries([a, b], false)!;
    const actual = finish(new IncrementalGeometryMerge([a, b]), 19, 0);
    expect(actual.index).toBeNull();
    expect(actual.getAttribute('position').array).toEqual(expected.getAttribute('position').array);
    expect(actual.getAttribute('color').array).toEqual(expected.getAttribute('color').array);
    actual.dispose(); expected.dispose(); a.dispose(); b.dispose();
  });

  it('rejects incompatible or empty schemas before starting and validates quotas', () => {
    expect(() => new IncrementalGeometryMerge([])).toThrow(/at least one/i);
    const a = box(0, 0.2), b = box(1, 0.4);
    b.deleteAttribute('uv');
    expect(() => new IncrementalGeometryMerge([a, b])).toThrow(/incompatible attributes/i);
    const c = box(2, 0.6);
    c.setAttribute('color', new THREE.Uint8BufferAttribute(new Uint8Array(c.getAttribute('position').count * 3), 3, true));
    expect(() => new IncrementalGeometryMerge([a, c])).toThrow(/incompatible layout/i);
    const merger = new IncrementalGeometryMerge([a]);
    expect(() => merger.step(-1, 0)).toThrow(/nonnegative/i);
    expect(() => merger.step(1.5, 0)).toThrow(/nonnegative/i);
    merger.dispose(); a.dispose(); b.dispose(); c.dispose();
  });

  it('cancellation disposes only the destination and leaves source geometries usable', () => {
    const source = box(4, 0.5);
    const sourcePosition = source.getAttribute('position').array;
    let sourceDisposed = false;
    source.addEventListener('dispose', () => { sourceDisposed = true; });
    const merger = new IncrementalGeometryMerge([source]);
    merger.step(2, 3);
    merger.dispose();
    expect(merger.result).toBeNull();
    expect(() => merger.step(1, 1)).toThrow(/disposed/i);
    expect(sourceDisposed).toBe(false);
    expect(source.getAttribute('position').array).toBe(sourcePosition);
    expect(source.getAttribute('position').count).toBeGreaterThan(0);
    source.dispose();
  });

  it('does not scan indices in the constructor and reads at most the step quota', () => {
    const source = box(2, 0.4);
    const index = source.index!;
    const originalGetX = index.getX.bind(index);
    let reads = 0;
    index.getX = (i: number) => { reads++; return originalGetX(i); };
    const merger = new IncrementalGeometryMerge([source]);
    expect(reads).toBe(0);
    merger.step(0, 2);
    expect(reads).toBe(2);
    merger.step(0, 5);
    expect(reads).toBe(7);
    merger.dispose(); source.dispose();
  });

  it('validates index values only as their bounded copy slice is processed', () => {
    const source = box(0, 0.3);
    source.index!.setX(0, source.getAttribute('position').count + 1);
    const merger = new IncrementalGeometryMerge([source]);
    expect(() => merger.step(0, 1)).toThrow(/out-of-range index/i);
    merger.dispose(); source.dispose();
  });
});
