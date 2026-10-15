import * as THREE from 'three/webgpu';
import { describe, expect, it, vi } from 'vitest';
import type { Road, WorldNode } from '../web/src/types';
import { CitySidewalks } from '../web/src/render/sidewalks';
import { SidewalkBuild } from '../web/src/render/sidewalk-build';

const node = (id: number, x: number, z: number): WorldNode => ({ id, x, z, kind: 'road' });
const road = (id: number, from: number, to: number): Road => ({ id, from, to, kind: 'arterial', stage: 'open', progress: 1, connected: true, length: 1, lanes: 2 });
const world = () => ({ nodes: [node(1, 0, 0), node(2, 40, 0)], roads: [road(1, 1, 2)] });
const drain = (owner: CitySidewalks) => { for (let i = 0; i < 1000 && owner.pendingCount; i++) owner.update(); expect(owner.pendingCount).toBe(0); };
const makeOwner = (material = new THREE.MeshBasicMaterial()) => {
  const deferred: (() => void)[] = [];
  const owner = new CitySidewalks(() => material, dispose => deferred.push(dispose));
  return { owner, deferred, material };
};

describe('CitySidewalks', () => {
  it('ignores progress churn and owns a copied snapshot', () => {
    const { owner } = makeOwner();
    const input = world();
    owner.setWorld(input);
    const firstJob = owner['job'];
    input.roads[0].progress = 0.4;
    owner.setWorld(input);
    expect(owner['job']).toBe(firstJob);
    input.nodes[1].x = 500;
    drain(owner);
    const mesh = owner.root.children[0] as THREE.Mesh;
    expect(mesh.geometry.boundingBox!.max.x).toBeLessThan(100);
    owner.reset();
  });

  it('keeps committed geometry visible until atomic replacement and resolves material at commit', () => {
    const oldMaterial = new THREE.MeshBasicMaterial(), activeMaterial = new THREE.MeshBasicMaterial();
    const oldMaterialDispose = vi.spyOn(oldMaterial, 'dispose');
    let current = oldMaterial;
    const deferred: (() => void)[] = [];
    const owner = new CitySidewalks(() => current, dispose => deferred.push(dispose));
    owner.setWorld(world()); drain(owner);
    const previous = owner.root.children[0] as THREE.Mesh;
    const previousSignature = owner['committedSignature'];
    const changed = world(); changed.nodes[1].x = 65;
    owner.setWorld(changed);
    current = activeMaterial;
    expect(owner.root.children).toEqual([previous]);
    expect(owner.pendingCount).toBe(1);
    drain(owner);
    const next = owner.root.children[0] as THREE.Mesh;
    expect(next).not.toBe(previous);
    expect(next.material).toBe(activeMaterial);
    expect(next.castShadow).toBe(true);
    expect(next.receiveShadow).toBe(true);
    expect(previous.parent).toBeNull();
    expect(deferred).toHaveLength(1);
    deferred.shift()!();
    owner.reset();
    expect(oldMaterialDispose).not.toHaveBeenCalled();
    oldMaterial.dispose(); activeMaterial.dispose();
  });

  it('keeps the active same-world build and replaces intermediate queued snapshots', () => {
    const { owner } = makeOwner();
    const a = world(); owner.setWorld(a);
    let commits = 0;
    let committed = owner['committedSignature'];
    for (let frame = 1; frame <= 80; frame++) {
      const changing = world(); changing.nodes[1].x = 40 + frame;
      const active = owner['job'];
      owner.setWorld(changing);
      expect(owner['job']).toBe(active);
      expect(owner.pendingCount).toBe(1);
      expect(owner['queued']?.snapshot.nodes[1].x).toBe(40 + frame);
      owner.update();
      if (owner['committedSignature'] !== committed) {
        commits++;
        committed = owner['committedSignature'];
        const rendered = owner.root.children[0] as THREE.Mesh;
        expect(rendered.geometry.boundingBox!.max.x).toBe(JSON.parse(committed).nodes[1].x);
      }
    }
    drain(owner);
    expect(commits).toBeGreaterThan(1);
    expect(owner['committedSignature']).toBe(JSON.stringify(owner['snapshot'](worldWithX(120))));
    expect((owner.root.children[0] as THREE.Mesh).geometry.boundingBox!.max.x).toBe(120);
    owner.reset();
  });

  it('drops a queued A when the latest request returns to the active A', () => {
    const { owner } = makeOwner();
    const a = world(); owner.setWorld(a);
    const active = owner['job'];
    const b = world(); b.nodes[1].x = 80;
    owner.setWorld(b);
    expect(owner['queued']).not.toBeNull();
    owner.setWorld(world());
    expect(owner['job']).toBe(active);
    expect(owner['queued']).toBeNull();
    expect(owner['signature']).toBe(owner['activeSignature']);
    const expected = owner['activeSignature'];
    drain(owner);
    expect(owner['committedSignature']).toBe(expected);
    expect(owner.pendingCount).toBe(0);
    owner.reset();
  });

  it('starts the latest queued build after commit without stepping it in that frame', () => {
    const { owner } = makeOwner();
    owner.setWorld(world());
    const queued = world(); queued.nodes[1].x = 80;
    owner.setWorld(queued);
    const step = vi.spyOn(SidewalkBuild.prototype, 'step');
    while (owner['job'] === undefined || owner['job'] === null) owner.update();
    let previous = owner['job'];
    for (let i = 0; i < 100 && owner.pendingCount; i++) {
      const before = step.mock.calls.length;
      owner.update();
      const current = owner['job'];
      if (current && current !== previous) {
        expect(step.mock.contexts.slice(before)).not.toContain(current);
      }
      if (current) previous = current;
    }
    step.mockRestore();
    owner.reset();
  });

  it('reset cancels active and queued work and clears both signatures', () => {
    const { owner } = makeOwner();
    const a = world(); owner.setWorld(a);
    const active = owner['job'];
    const b = world(); b.nodes[1].x = 80;
    owner.setWorld(b);
    expect(owner['job']).toBe(active);
    expect(owner['queued']).not.toBeNull();
    owner.reset();
    expect(owner.pendingCount).toBe(0);
    expect(owner['queued']).toBeNull();
    expect(owner['activeSignature']).toBe('');
    expect(owner['committedSignature']).toBe('');
    expect(owner['signature']).toBe('');
    expect(owner.root.children).toHaveLength(0);
    owner.update();
    expect(owner.root.children).toHaveLength(0);
  });

  it('removes prior geometry when a completed replacement is empty', () => {
    const { owner, deferred } = makeOwner();
    owner.setWorld(world()); drain(owner);
    expect(owner.root.children).toHaveLength(1);
    owner.setWorld({ nodes: [], roads: [] }); drain(owner);
    expect(owner.root.children).toHaveLength(0);
    expect(deferred).toHaveLength(1);
    owner.reset();
  });

  it('defers geometry retirement and never disposes its shared material', () => {
    const material = new THREE.MeshBasicMaterial();
    const materialDispose = vi.spyOn(material, 'dispose');
    const { owner, deferred } = makeOwner(material);
    owner.setWorld(world()); drain(owner);
    const geometry = (owner.root.children[0] as THREE.Mesh).geometry;
    const geometryDispose = vi.spyOn(geometry, 'dispose');
    owner.reset();
    expect(geometryDispose).not.toHaveBeenCalled();
    expect(deferred).toHaveLength(1);
    deferred[0]();
    expect(geometryDispose).toHaveBeenCalled();
    expect(materialDispose).not.toHaveBeenCalled();
    geometryDispose.mockRestore(); materialDispose.mockRestore(); material.dispose();
  });
  it('releases a failed replacement and retains the committed mesh', () => {
    const material = new THREE.MeshBasicMaterial();
    let fail = false;
    const owner = new CitySidewalks(() => { if (fail) throw new Error('material failure'); return material; }, release => release());
    owner.setWorld(world()); drain(owner);
    const previous = owner.root.children[0] as THREE.Mesh;
    const previousSignature = owner['committedSignature'];
    const previousRelease = vi.spyOn(previous.geometry, 'dispose');
    const changed = world(); changed.nodes[1].x = 90;
    owner.setWorld(changed); fail = true;
    const queued = world(); queued.nodes[1].x = 110;
    owner.setWorld(queued);
    expect(owner['queued']).not.toBeNull();
    expect(() => drain(owner)).toThrow('material failure');
    expect(owner.pendingCount).toBe(0);
    expect(owner['queued']).toBeNull();
    expect(owner['committedSignature']).toBe(previousSignature);
    expect(owner.root.children).toEqual([previous]);
    expect(previousRelease).not.toHaveBeenCalled();
    owner.reset(); material.dispose();
  });

});

const worldWithX = (x: number) => ({ nodes: [node(1, 0, 0), node(2, x, 0)], roads: [road(1, 1, 2)] });
