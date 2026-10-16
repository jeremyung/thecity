import { it, expect, vi } from 'vitest';
import { RiverWater } from '../web/src/render/water';
it('restores the original water material and owns the temporary reflection probe', () => {
  const water = new RiverWater([
    { x: 0, z: -50 },
    { x: 0, z: 50 },
  ]);
  const normal = water.mesh.material;
  expect(water.toggleReflectionForReview()).toBe(true);
  const probe = water.mesh.material;
  expect(probe).not.toBe(normal);
  if (Array.isArray(probe) || Array.isArray(normal)) throw new Error('Expected one water material');
  const disposeNormal = vi.spyOn(normal, 'dispose'),
    disposeProbe = vi.spyOn(probe, 'dispose');
  expect(water.toggleReflectionForReview()).toBe(false);
  expect(water.mesh.material).toBe(normal);
  expect(water.toggleReflectionForReview()).toBe(true);
  expect(water.mesh.material).toBe(probe);
  water.dispose();
  expect(disposeNormal).toHaveBeenCalledTimes(1);
  expect(disposeProbe).toHaveBeenCalledTimes(1);
});
