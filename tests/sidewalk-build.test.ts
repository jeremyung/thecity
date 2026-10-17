import * as THREE from 'three/webgpu';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { describe, expect, it, vi } from 'vitest';
import type { Road, WorldNode } from '../web/src/types';
import { arterialSidewalkPolygons } from '../web/src/render/arterial-sidewalks';
import { sidewalkGeometry } from '../web/src/render/sidewalk-kit';
import { SidewalkBuild } from '../web/src/render/sidewalk-build';

const node = (id: number, x: number, z: number) => ({ id, x, z, kind: 'junction' }) as WorldNode;
const road = (id: number, from: number, to: number) =>
  ({
    id,
    from,
    to,
    kind: 'arterial' as const,
    stage: 'open',
    progress: 1,
    connected: true,
    length: 1,
    lanes: 2,
  }) as Road;
const world = (count = 1) => {
  const nodes: WorldNode[] = [],
    roads: Road[] = [];
  for (let i = 0; i < count; i++) {
    nodes.push(node(i * 2 + 1, i * 60, 0), node(i * 2 + 2, i * 60 + 40, 0));
    roads.push(road(i + 1, i * 2 + 1, i * 2 + 2));
  }
  return { nodes, roads };
};
function drain(job: SidewalkBuild) {
  for (let n = 0; n < 10000; n++) if (job.step()) return;
  throw new Error('job did not finish');
}
function geometryArrays(g: THREE.BufferGeometry) {
  return {
    attributes: Object.fromEntries(
      Object.entries(g.attributes).map(([k, a]) => [
        k,
        Array.from((a as THREE.BufferAttribute).array),
      ]),
    ),
    index: g.index && Array.from(g.index.array),
  };
}

describe('SidewalkBuild', () => {
  it('matches the existing extrude-and-merge output and transfers it once', () => {
    const input = world(3),
      expectedParts = arterialSidewalkPolygons(input).map(sidewalkGeometry);
    const expected = mergeGeometries(expectedParts, false)!;
    const job = new SidewalkBuild(input);
    drain(job);
    const actual = job.takeGeometry()!;
    expect(geometryArrays(actual)).toEqual(geometryArrays(expected));
    expect(job.takeGeometry()).toBeNull();
    const release = vi.spyOn(actual, 'dispose');
    job.dispose();
    expect(release).not.toHaveBeenCalled();
    actual.dispose();
    release.mockRestore();
    expected.dispose();
    expectedParts.forEach((g) => g.dispose());
  });

  it('finishes empty input with null output', () => {
    const job = new SidewalkBuild({ nodes: [], roads: [] });
    drain(job);
    expect(job.takeGeometry()).toBeNull();
    job.dispose();
  });

  it('cancels during clipping without producing geometry', () => {
    const job = new SidewalkBuild(world(20));
    expect(job.step()).toBe(false);
    job.dispose();
    expect(job.step()).toBe(false);
    expect(job.takeGeometry()).toBeNull();
  });

  it('cancels during extrusion and disposes every fragment it owns', () => {
    const dispose = vi.spyOn(THREE.BufferGeometry.prototype, 'dispose');
    const job = new SidewalkBuild(world(20));
    while (!job['polygons']) job.step();
    job.step(); // at most sixteen fragments
    expect(job['fragments']).toHaveLength(16);
    const fragments = [...job['fragments']];
    expect(dispose.mock.calls).toHaveLength(0);
    job.dispose();
    job.dispose();
    expect(dispose.mock.instances).toEqual(fragments);
    dispose.mockRestore();
  });

  it('cancels during merge and disposes the merge destination and source fragments', () => {
    const dispose = vi.spyOn(THREE.BufferGeometry.prototype, 'dispose');
    const job = new SidewalkBuild(world(300));
    // Clipping and extrusion are separately bounded phases; 300 simple roads fit in these drains.
    for (let i = 0; !job['merger'] && i < 10000; i++) job.step();
    expect(job['merger']).not.toBeNull();
    const fragments = [...job['fragments']];
    // Cancel the confirmed unfinished merge, before its final copy step.
    expect(job['merger']!.done).toBe(false);
    job.dispose();
    job.dispose();
    expect(dispose.mock.instances).toHaveLength(fragments.length + 1);
    expect(new Set(dispose.mock.instances).size).toBe(fragments.length + 1);
    for (const fragment of fragments) expect(dispose.mock.instances).toContain(fragment);
    dispose.mockRestore();
  });

  it('disposes completed output that has not been transferred', () => {
    const dispose = vi.spyOn(THREE.BufferGeometry.prototype, 'dispose');
    const job = new SidewalkBuild(world());
    drain(job);
    const before = dispose.mock.calls.length;
    job.dispose();
    job.dispose();
    expect(dispose.mock.calls).toHaveLength(before + 1);
    expect(job.takeGeometry()).toBeNull();
    dispose.mockRestore();
  });
});
