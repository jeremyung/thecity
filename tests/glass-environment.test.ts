import { expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { GlassEnvironment } from '../web/src/render/glass-environment';
import type { Building } from '../web/src/types';
it('bounds reflection work, reuses targets and leaves borrowed sky resources alive', () => {
  const sky = new THREE.Mesh(new THREE.SphereGeometry(1800), new THREE.MeshBasicNodeMaterial()),
    target = new THREE.RenderTarget(16, 16);
  const fromScene = vi.fn((_scene: THREE.Scene, ..._args: unknown[]) => target),
    dispose = vi.fn(),
    probe = new GlassEnvironment(
      sky,
      () => ({ fromScene, dispose }) as unknown as THREE.PMREMGenerator,
    );
  const world = Array.from(
      { length: 130 },
      (_, i) =>
        ({
          id: i,
          x: i * 5,
          z: -40,
          width: 12,
          depth: 18,
          floors: 20,
          rotation: 0,
          stage: 'open',
          progress: 1,
          kind: 'office',
        }) as Building,
    ),
    camera = new THREE.Vector3(100, 180, 300),
    colors = [new THREE.Color('#aaccee')];
  expect(
    probe.update({} as THREE.WebGPURenderer, 0, world, camera, colors, 0) === target.texture,
  ).toBe(true);
  const scene = fromScene.mock.calls[0][0] as THREE.Scene,
    blocks = scene.children.find((c) => c instanceof THREE.InstancedMesh) as THREE.InstancedMesh;
  expect(blocks.count).toBeLessThanOrEqual(96);
  expect(blocks.count).toBeGreaterThan(0);
  probe.update({} as THREE.WebGPURenderer, 1000, [], camera, colors, 1);
  expect(fromScene).toHaveBeenCalledTimes(1);
  probe.update({} as THREE.WebGPURenderer, 3000, world, camera, colors, 0);
  expect(fromScene).toHaveBeenCalledTimes(1);
  probe.update({} as THREE.WebGPURenderer, 6000, world, camera, [new THREE.Color('#223344')], 1);
  expect(fromScene).toHaveBeenCalledTimes(2);
  expect((fromScene.mock.calls[1] as unknown[])[4]).toMatchObject({
    size: 128,
    renderTarget: target,
  });
  let borrowedDisposed = false;
  sky.geometry.addEventListener('dispose', () => {
    borrowedDisposed = true;
  });
  sky.material.addEventListener('dispose', () => {
    borrowedDisposed = true;
  });
  probe.dispose();
  expect(dispose).toHaveBeenCalledTimes(1);
  expect(borrowedDisposed).toBe(false);
  sky.geometry.dispose();
  sky.material.dispose();
});
