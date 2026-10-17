import * as THREE from 'three/webgpu';
import { describe, expect, it } from 'vitest';
import { BuildingChunks } from '../web/src/render/building-chunks';

const MIN_EDGE_SQ = 3.5 * 3.5;

/** Frozen pre-optimization outline path: this is the byte-output oracle. */
function legacyOutlines(group: THREE.Group) {
  const values: number[] = [];
  group.updateMatrixWorld(true);
  group.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    const edges = new THREE.EdgesGeometry(object.geometry, 42);
    edges.applyMatrix4(object.matrixWorld);
    const points = edges.getAttribute('position');
    for (let i = 0; i < points.count; i += 2) {
      const dx = points.getX(i + 1) - points.getX(i),
        dy = points.getY(i + 1) - points.getY(i),
        dz = points.getZ(i + 1) - points.getZ(i);
      if (dx * dx + dy * dy + dz * dz < MIN_EDGE_SQ) continue;
      for (let j = 0; j < 2; j++)
        values.push(points.getX(i + j), points.getY(i + j), points.getZ(i + j));
    }
    edges.dispose();
  });
  return new Float32Array(values);
}

function optimizedOutlines(group: THREE.Group, refreshBounds = true) {
  const ink = new THREE.LineBasicNodeMaterial(),
    chunks = new BuildingChunks(
      (m) => m,
      () => {},
      ink,
    );
  const source = legacyOutlines(group);
  let positionReads = 0;
  const restore: Array<() => void> = [];
  group.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    if (refreshBounds) object.geometry.computeBoundingBox();
    const geometry = object.geometry,
      target = geometry as any,
      original = geometry.getAttribute.bind(geometry);
    target.getAttribute = (name: string) => {
      if (name === 'position') positionReads++;
      return original(name);
    };
    restore.push(() => {
      target.getAttribute = original;
    });
  });
  chunks.set(1, group);
  const positionReadsAtSet = positionReads;
  let updates = 0;
  while (chunks.pendingCount && updates++ < 100) chunks.update();
  if (chunks.pendingCount) throw new Error('outline fixture did not drain');
  let result = new Float32Array(0);
  chunks.root.traverse((object) => {
    if (object instanceof THREE.LineSegments)
      result = new Float32Array(object.geometry.getAttribute('position').array as Float32Array);
  });
  for (const restoreGetAttribute of restore) restoreGetAttribute();
  chunks.clear();
  ink.dispose();
  return {
    expected: source,
    actual: result,
    positionReadsAtSet,
    positionReadsAfterDrain: positionReads,
  };
}

function slenderBox(length: number, thickness = 0.01) {
  const group = new THREE.Group(),
    geometry = new THREE.BoxGeometry(length, thickness, thickness),
    material = new THREE.MeshBasicMaterial();
  group.add(new THREE.Mesh(geometry, material));
  return { group, geometry, material };
}

function dispose(group: THREE.Group, geometry: THREE.BufferGeometry, material: THREE.Material) {
  group.removeFromParent();
  geometry.dispose();
  material.dispose();
}

describe('conservative contour outline rejection', () => {
  it('preserves the existing strict 3.5 m edge threshold just below, at and above it', () => {
    for (const length of [3.499, 3.5, 3.501]) {
      const fixture = slenderBox(length),
        { expected, actual, positionReadsAtSet, positionReadsAfterDrain } = optimizedOutlines(
          fixture.group,
        );
      expect(Array.from(actual), `length ${length}`).toEqual(Array.from(expected));
      // Registration must remain cheap; contour extraction runs in update().
      expect(positionReadsAtSet).toBe(0);
      expect(positionReadsAfterDrain).toBeGreaterThan(0);
      if (length < 3.5) expect(actual).toHaveLength(0);
      else expect(actual.length).toBeGreaterThan(0);
      dispose(fixture.group, fixture.geometry, fixture.material);
    }
    const translated = slenderBox(3.4999);
    translated.group.position.x = 300;
    const { expected, actual, positionReadsAtSet, positionReadsAfterDrain } = optimizedOutlines(
      translated.group,
    );
    expect(Array.from(actual)).toEqual(Array.from(expected));
    expect(positionReadsAtSet).toBe(0);
    expect(positionReadsAfterDrain).toBeGreaterThan(0); // Near-threshold extraction still runs after registration.
    dispose(translated.group, translated.geometry, translated.material);
  });

  it('uses rewritten positions even when a shared geometry retains stale small bounds', () => {
    const fixture = slenderBox(1);
    fixture.geometry.computeBoundingBox();
    const positions = fixture.geometry.getAttribute('position');
    for (let i = 0; i < positions.count; i++) positions.setX(i, positions.getX(i) * 8);
    positions.needsUpdate = true;
    expect(fixture.geometry.boundingBox!.max.x).toBe(0.5);
    const { expected, actual } = optimizedOutlines(fixture.group, false);
    expect(expected.length).toBeGreaterThan(0);
    expect(Array.from(actual)).toEqual(Array.from(expected));
    dispose(fixture.group, fixture.geometry, fixture.material);
  });

  it('matches the frozen edge output through nonuniform scale and nested rotations for tiny and retained meshes', () => {
    for (const { dimensions, x } of [
      { dimensions: [0.7, 0.8, 0.9], x: -31 },
      { dimensions: [1.5, 1.3, 1.2], x: 1e7 },
      { dimensions: [3.2, 0.22, 0.16], x: 300 },
    ] as const) {
      const group = new THREE.Group(),
        geometry = new THREE.BoxGeometry(...dimensions),
        material = new THREE.MeshBasicMaterial();
      geometry.computeBoundingBox();
      const before = geometry.boundingBox!.clone();
      group.position.set(x, 2, 47);
      group.rotation.set(0.13, -0.51, 0.08);
      group.scale.set(0.8, 1.35, 1.1);
      const mesh = new THREE.Mesh(geometry, material);
      mesh.position.set(2.4, 0.3, -1.1);
      mesh.rotation.set(0.17, 0.39, -0.11);
      mesh.scale.set(1.7, 0.72, 1.25);
      group.add(mesh);
      const { expected, actual, positionReadsAtSet } = optimizedOutlines(group);
      expect(Array.from(actual)).toEqual(Array.from(expected));
      if (dimensions[0] === 0.7) expect(positionReadsAtSet).toBe(0);
      expect(geometry.boundingBox!.min.toArray()).toEqual(before.min.toArray());
      expect(geometry.boundingBox!.max.toArray()).toEqual(before.max.toArray());
      dispose(group, geometry, material);
    }
  });
});
