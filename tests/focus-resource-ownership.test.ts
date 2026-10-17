import { expect, it, vi } from 'vitest';
import { Texture } from 'three/webgpu';
import { float, texture, vec4 } from 'three/tsl';
import { signedCoCDof } from '../web/src/render/postprocessing/SignedCoCDepthOfFieldNode.js';

it('releases converted input and nested blur targets once, including before first render', () => {
  const effect = signedCoCDof(vec4(1), float(0)) as any;
  const input = effect._ownedInputTexture;
  const target = vi.spyOn(input.renderTarget, 'dispose');
  const material = vi.spyOn(input._quadMesh.material, 'dispose');
  effect.setup({ getSharedContext: () => ({}) });
  const blur = effect._nearBlur;
  const horizontal = vi.spyOn(blur._horizontalRT, 'dispose');
  const vertical = vi.spyOn(blur._verticalRT, 'dispose');
  effect.setup({ getSharedContext: () => ({}) });
  expect(effect._nearBlur).toBe(blur);
  effect.dispose();
  effect.dispose();
  for (const spy of [target, material, horizontal, vertical]) expect(spy).toHaveBeenCalledTimes(1);
});

it('does not release a borrowed texture or caller-owned RTT', () => {
  const source = new Texture();
  const release = vi.spyOn(source, 'dispose');
  const effect = signedCoCDof(texture(source), 0) as any;
  expect(effect._ownedInputTexture).toBeNull();
  effect.dispose();
  expect(release).not.toHaveBeenCalled();
  const owner = signedCoCDof(vec4(1), 0) as any;
  const borrowed = signedCoCDof(owner.textureNode, 0) as any;
  const target = vi.spyOn(owner.textureNode.renderTarget, 'dispose');
  borrowed.dispose();
  expect(target).not.toHaveBeenCalled();
  owner.dispose();
  expect(target).toHaveBeenCalledTimes(1);
});

it('tolerates missing private input and blur materials without skipping owned targets', () => {
  const effect = signedCoCDof(vec4(1), 0) as any;
  const input = effect._ownedInputTexture;
  input._quadMesh.material.dispose();
  delete input._quadMesh;
  effect.setup({ getSharedContext: () => ({}) });
  const blur = effect._nearBlur;
  blur._material?.dispose();
  delete blur._material;
  const target = vi.spyOn(input.renderTarget, 'dispose');
  const horizontal = vi.spyOn(blur._horizontalRT, 'dispose');
  expect(() => {
    effect.dispose();
    effect.dispose();
  }).not.toThrow();
  expect(target).toHaveBeenCalledTimes(1);
  expect(horizontal).toHaveBeenCalledTimes(1);
});
