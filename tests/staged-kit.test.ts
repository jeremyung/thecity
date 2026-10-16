import * as THREE from 'three/webgpu';
import { describe, expect, it, vi } from 'vitest';
import { Kit, disposeGroup } from '../web/src/render/kit';
import { StagedKit } from '../web/src/render/staged-kit';

function mesh(group: THREE.Group, material: THREE.Material) {
  return group.children.find(
    (child): child is THREE.Mesh => child instanceof THREE.Mesh && child.material === material,
  )!;
}

function drain(kit: StagedKit) {
  for (let i = 0; i < 10000; i++) if (kit.step()) return kit.takeGroup()!;
  throw new Error('staged build did not finish');
}

describe('StagedKit', () => {
  it('preserves first-use material order across deferred and explicit geometry', () => {
    const a = new THREE.MeshBasicMaterial(),
      b = new THREE.MeshBasicMaterial();
    const kit = new StagedKit();
    kit.box(0, 0, 0, 1, 1, 1, a);
    kit.add(new THREE.BoxGeometry(), b);
    const output = drain(kit);
    expect(output.children.map((child) => (child as THREE.Mesh).material)).toEqual([a, b]);
    disposeGroup(output);
    a.dispose();
    b.dispose();
  });
  it('matches Kit vertex and index ordering for primitives, custom geometry, materials and nested offsets', () => {
    const a = new THREE.MeshStandardMaterial(),
      b = new THREE.MeshStandardMaterial();
    const sync = new Kit(),
      staged = new StagedKit();
    const build = (kit: Kit) => {
      kit.box(1, 2, 3, 2, 3, 4, a, 0.3);
      kit.withOffset(10, 20, 30, () => {
        kit.cylinder(1, 2, 3, 0.7, 2, a, 9);
        kit.withOffset(-4, 5, 6, () => kit.cone(2, 3, 4, 0.8, 1.5, b, 7));
        const custom = new THREE.BoxGeometry(0.8, 1.2, 1.6);
        custom.translate(1, 2, 3);
        kit.add(custom, a);
      });
      kit.box(-2, 0.5, 1, 1, 0.2, 2, b);
    };
    build(sync);
    build(staged);
    const expected = sync.finish(),
      actual = drain(staged);
    for (const material of [a, b]) {
      const ea = mesh(expected, material).geometry,
        aa = mesh(actual, material).geometry;
      for (const name of Object.keys(ea.attributes))
        expect(Array.from(aa.getAttribute(name).array)).toEqual(
          Array.from(ea.getAttribute(name).array),
        );
      expect(Array.from(aa.index!.array)).toEqual(Array.from(ea.index!.array));
      expect(mesh(actual, material).castShadow).toBe(true);
      expect(mesh(actual, material).receiveShadow).toBe(true);
    }
    expect(staged.takeGroup()).toBeNull();
    disposeGroup(expected);
    disposeGroup(actual);
    a.dispose();
    b.dispose();
  });

  it('constructs at most 128 commands per step and rejects later mutation', () => {
    const mat = new THREE.MeshBasicMaterial(),
      kit = new StagedKit();
    for (let i = 0; i < 129; i++) kit.box(i, 0, 0, 1, 1, 1, mat);
    expect(kit.step()).toBe(false);
    expect(() => kit.box(0, 0, 0, 1, 1, 1, mat)).toThrow(/no longer accepts/);
    expect(kit.step()).toBe(false);
    const group = drain(kit);
    expect(mesh(group, mat).geometry.getAttribute('position').count).toBe(129 * 24);
    disposeGroup(group);
    mat.dispose();
  });

  it('can be disposed during command construction, merge, or after output without disposing shared materials', () => {
    const mat = new THREE.MeshBasicMaterial();
    const disposeMaterial = vi.spyOn(mat, 'dispose');
    const constructed = new StagedKit();
    constructed.box(0, 0, 0, 1, 1, 1, mat);
    constructed.dispose();
    constructed.dispose();
    const merging = new StagedKit();
    for (let i = 0; i < 300; i++) merging.box(i, 0, 0, 1, 1, 1, mat);
    merging.step();
    merging.step();
    merging.step();
    merging.step();
    merging.dispose();
    merging.dispose();
    const completed = new StagedKit();
    completed.box(0, 0, 0, 1, 1, 1, mat);
    const group = drain(completed);
    completed.dispose();
    expect(disposeMaterial).not.toHaveBeenCalled();
    disposeGroup(group);
    mat.dispose();
  });

  it('throws from finish so callers cannot accidentally drain synchronously', () => {
    expect(() => new StagedKit().finish()).toThrow(/use step\(\) and takeGroup/);
  });
});
