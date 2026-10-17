import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { instanceCapacity, reuseInstances } from '../web/src/render/reuse-instances';

describe('persistent static instances', () => {
  it('updates transforms/colors and shrink counts without replacing GPU buffers', () => {
    const geometry = new THREE.BoxGeometry(),
      material = new THREE.MeshBasicMaterial();
    const previous = new THREE.InstancedMesh(geometry, material, 8);
    previous.setColorAt(0, new THREE.Color('red'));
    previous.computeBoundingSphere();
    const matrices = previous.instanceMatrix,
      colors = previous.instanceColor;
    const next = new THREE.InstancedMesh(geometry, material, 2);
    next.setMatrixAt(0, new THREE.Matrix4().makeTranslation(10, 2, 3));
    next.setMatrixAt(1, new THREE.Matrix4().makeTranslation(-4, 5, 6));
    next.setColorAt(0, new THREE.Color('blue'));
    next.setColorAt(1, new THREE.Color('green'));
    const dispose = vi.fn();
    next.addEventListener('dispose', dispose);
    expect(reuseInstances(previous, next)).toBe(previous);
    expect(previous.instanceMatrix).toBe(matrices);
    expect(previous.instanceColor).toBe(colors);
    expect(previous.count).toBe(2);
    expect(previous.boundingSphere).toBeNull();
    expect(Array.from(previous.instanceMatrix.array.slice(0, 32))).toEqual(
      Array.from(next.instanceMatrix.array),
    );
    expect(Array.from(previous.instanceColor!.array.slice(0, 6))).toEqual(
      Array.from(next.instanceColor!.array),
    );
    expect(dispose).toHaveBeenCalledOnce();
    previous.computeBoundingSphere();
    expect(previous.boundingSphere!.center.x).toBeCloseTo(3);
    previous.dispose();
    geometry.dispose();
    material.dispose();
  });
  it('replaces insufficient or incompatible buffers rather than dropping instances', () => {
    const g = new THREE.BoxGeometry(),
      m = new THREE.MeshBasicMaterial();
    const old = new THREE.InstancedMesh(g, m, 2),
      grown = new THREE.InstancedMesh(g, m, instanceCapacity(3));
    grown.count = 3;
    expect(reuseInstances(old, grown)).toBe(grown);
    const colored = new THREE.InstancedMesh(g, m, 2);
    colored.setColorAt(0, new THREE.Color('red'));
    expect(reuseInstances(old, colored)).toBe(colored);
    const otherMaterial = new THREE.MeshBasicMaterial(),
      other = new THREE.InstancedMesh(g, otherMaterial, 1);
    expect(reuseInstances(old, other)).toBe(other);
    for (const mesh of [old, grown, colored, other]) mesh.dispose();
    g.dispose();
    m.dispose();
    otherMaterial.dispose();
  });
});
