import * as THREE from 'three/webgpu';
import { describe, expect, it } from 'vitest';
import { IncrementalBuildingContours } from '../web/src/render/incremental-building-contours';

function oracle(group: THREE.Group, minSq = 12.25, angle = 42) {
  const values: number[] = [];
  group.updateMatrixWorld(true);
  group.traverse((o) => {
    if (!(o instanceof THREE.Mesh)) return;
    const edges = new THREE.EdgesGeometry(o.geometry, angle);
    edges.applyMatrix4(o.matrixWorld);
    const p = edges.getAttribute('position');
    for (let i = 0; i < p.count; i += 2) {
      const dx = p.getX(i + 1) - p.getX(i),
        dy = p.getY(i + 1) - p.getY(i),
        dz = p.getZ(i + 1) - p.getZ(i);
      if (dx * dx + dy * dy + dz * dz < minSq) continue;
      for (let j = 0; j < 2; j++) values.push(p.getX(i + j), p.getY(i + j), p.getZ(i + j));
    }
    edges.dispose();
  });
  return new Float32Array(values);
}
function run(group: THREE.Group, quota = 7) {
  const job = new IncrementalBuildingContours(group);
  while (!job.step(quota)) expect(job.processedWork).toBeLessThanOrEqual(quota);
  return job.result!;
}

describe('IncrementalBuildingContours', () => {
  it('matches EdgesGeometry ordering and float32 world transforms for indexed and nonindexed meshes', () => {
    const group = new THREE.Group(),
      a = new THREE.Mesh(new THREE.BoxGeometry(7, 4, 5)),
      b = new THREE.Mesh(new THREE.SphereGeometry(4, 8, 5).toNonIndexed());
    a.position.set(100000.25, 2, -7);
    a.rotation.set(0.2, 0.4, 0.1);
    a.scale.set(2, 0.5, 1.3);
    b.position.set(-9, 3, 8);
    b.rotation.y = 0.6;
    group.add(a, b);
    expect(run(group)).toEqual(oracle(group));
  });
  it('preserves duplicate and degenerate face semantics', () => {
    const g = new THREE.BufferGeometry();
    g.setAttribute(
      'position',
      new THREE.Float32BufferAttribute(
        [0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0, 0, 5, 0, 0, 0, 5, 0, 1, 1, 1, 1, 1, 1, 8, 0, 0],
        3,
      ),
    );
    g.setIndex([0, 1, 2, 3, 5, 4, 6, 7, 8]);
    const group = new THREE.Group();
    group.add(new THREE.Mesh(g));
    expect(run(group, 2)).toEqual(oracle(group));
  });
  it('bounds actual work per step and supports cancellation without touching inputs', () => {
    const g = new THREE.BoxGeometry(10, 10, 10),
      group = new THREE.Group();
    group.add(new THREE.Mesh(g));
    const task = new IncrementalBuildingContours(group);
    for (let n = 0; n < 5; n++) {
      task.step(3);
      expect(task.processedWork).toBeLessThanOrEqual(3);
    }
    task.dispose();
    expect(task.result).toBeNull();
    expect(g.getAttribute('position').count).toBe(24);
  });
  it('drains unmatched edges for an open single triangle', () => {
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute([0, 0, 0, 8, 0, 0, 0, 8, 0], 3));
    const group = new THREE.Group();
    group.add(new THREE.Mesh(g));
    expect(run(group, 1)).toEqual(oracle(group));
  });
  it('matches Float64 local coordinates after EdgesGeometry Float32 rounding', () => {
    const g = new THREE.BufferGeometry();
    g.setAttribute(
      'position',
      new THREE.BufferAttribute(
        new Float64Array([0.123456789, 0, 0, 8.87654321, 0, 0, 0, 8.123456789, 0]),
        3,
      ),
    );
    const group = new THREE.Group();
    group.position.set(200000.12, 3, 1);
    group.add(new THREE.Mesh(g));
    expect(run(group, 2)).toEqual(oracle(group));
  });
  it('supports interleaved positions and keeps morph meshes on the exact path', () => {
    const g = new THREE.BufferGeometry();
    g.setAttribute(
      'position',
      new THREE.InterleavedBufferAttribute(
        new THREE.InterleavedBuffer(new Float32Array([0, 0, 0, 9, 0, 0, 0, 9, 0]), 3),
        3,
        0,
      ),
    );
    g.morphAttributes.position = [
      new THREE.Float32BufferAttribute([0, 0, 0, 10, 0, 0, 0, 10, 0], 3),
    ];
    const group = new THREE.Group();
    group.add(new THREE.Mesh(g));
    expect(run(group, 2)).toEqual(oracle(group));
  });
  it('uses all transformed box corners for a rotated elongated bound and returns a stable result buffer', () => {
    const g = new THREE.BoxGeometry(20, 0.2, 0.2),
      group = new THREE.Group(),
      mesh = new THREE.Mesh(g);
    mesh.rotation.set(0.7, 0.3, 0.9);
    group.add(mesh);
    const task = new IncrementalBuildingContours(group);
    while (!task.step(5)) expect(task.processedWork).toBeLessThanOrEqual(5);
    const first = task.result;
    expect(first).toEqual(oracle(group));
    expect(task.result).toBe(first);
  });
  it('returns the same empty output for bounds-prefiltered small meshes', () => {
    const group = new THREE.Group();
    group.add(new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1)));
    expect(run(group)).toEqual(oracle(group));
  });
});
