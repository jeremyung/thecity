import { ActorsLayer } from '../web/src/render/actors-layer';
import { expect, it, vi } from 'vitest';
import { CityRenderer } from '../web/src/render/city';

it('retains borrowed pipeline resources through pending compilation without retiring their replacements', () => {
  const renderer = Object.create(CityRenderer.prototype) as CityRenderer;
  (renderer as any).actors = Object.assign(Object.create(ActorsLayer.prototype), {
    host: renderer,
  });
  const slots = [
    'occlusion',
    'reflections',
    'shafts',
    'pipeline',
    'depthOfField',
    'shine',
    'scenePass',
    'focusedPipeline',
    'unblurredPipeline',
  ] as const;
  const old = slots.map(() => ({ dispose: vi.fn() }));
  const next = slots.map(() => ({ dispose: vi.fn() }));
  Object.assign(renderer, {
    pendingCompiles: 1,
    retiredResources: [],
    ...Object.fromEntries(slots.map((slot, i) => [slot, old[i]])),
  });
  renderer['disposePipeline']();
  for (const slot of slots) expect(renderer[slot]).toBeNull();
  for (const resource of old) expect(resource.dispose).not.toHaveBeenCalled();
  Object.assign(renderer, Object.fromEntries(slots.map((slot, i) => [slot, next[i]])));
  Object.assign(renderer, { pendingCompiles: 0 });
  for (const release of renderer['retiredResources'].splice(0)) release();
  for (const resource of old) expect(resource.dispose).toHaveBeenCalledTimes(1);
  for (const resource of next) expect(resource.dispose).not.toHaveBeenCalled();
  renderer['disposePipeline']();
  for (const resource of next) expect(resource.dispose).toHaveBeenCalledTimes(1);
  expect(renderer['retiredResources']).toHaveLength(0);
});

it('switches focus outputs without rebuilding shared passes and preserves the disabled path', () => {
  const renderer = Object.create(CityRenderer.prototype) as CityRenderer;
  (renderer as any).actors = Object.assign(Object.create(ActorsLayer.prototype), {
    host: renderer,
  });
  const focused = {},
    unblurred = {},
    scenePass = {};
  const rebuild = vi.fn();
  Object.assign(renderer, {
    focusEnabled: false,
    dofEnabled: { value: 0 },
    pipeline: unblurred,
    focusedPipeline: focused,
    unblurredPipeline: unblurred,
    scenePass,
    buildPipeline: rebuild,
  });
  renderer.setFocus(true);
  expect(renderer['pipeline']).toBe(focused);
  expect(renderer['dofEnabled'].value).toBe(1);
  renderer.setFocus(false);
  expect(renderer['pipeline']).toBe(unblurred);
  expect(renderer['dofEnabled'].value).toBe(0);
  expect(renderer['scenePass']).toBe(scenePass);
  expect(rebuild).not.toHaveBeenCalled();
});

it('disposes an active output alias only once', () => {
  const renderer = Object.create(CityRenderer.prototype) as CityRenderer;
  (renderer as any).actors = Object.assign(Object.create(ActorsLayer.prototype), {
    host: renderer,
  });
  const focused = { dispose: vi.fn() },
    unblurred = { dispose: vi.fn() };
  Object.assign(renderer, {
    pendingCompiles: 0,
    retiredResources: [],
    pipeline: focused,
    focusedPipeline: focused,
    unblurredPipeline: unblurred,
  });
  renderer['disposePipeline']();
  expect(focused.dispose).toHaveBeenCalledTimes(1);
  expect(unblurred.dispose).toHaveBeenCalledTimes(1);
});
