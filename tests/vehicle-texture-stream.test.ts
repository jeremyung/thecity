import { afterEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { streamed } from '../web/src/render/vehicle-materials';

class FakeBitmap { closed = false; close() { this.closed = true; } }

/** A loader that holds each load's callback until the test delivers it. */
function heldLoader() {
  const pending: Array<(bitmap: unknown) => void> = [];
  const loader = { setOptions() { return loader; }, load(_url: string, done: (bitmap: unknown) => void) { pending.push(done); } };
  return { loader: loader as unknown as THREE.ImageBitmapLoader, deliver: (bitmap: unknown) => pending.shift()!(bitmap) };
}

describe('streamed vehicle textures', () => {
  afterEach(() => vi.unstubAllGlobals());

  function setup() {
    vi.stubGlobal('ImageBitmap', FakeBitmap);
    vi.stubGlobal('createImageBitmap', () => undefined);
    vi.stubGlobal('fetch', () => undefined);
    const art = new ArtMaterials(), held = heldLoader();
    const node = streamed('/atlas.png', THREE.SRGBColorSpace, art, [0, 0, 0, 255], held.loader);
    return { art, node, ...held };
  }

  it('installs a bitmap that loads first, then disposes it and closes its bitmap on teardown', () => {
    const { art, node, deliver } = setup();
    const placeholder = node.value as THREE.Texture, onPlaceholder = vi.fn();
    placeholder.addEventListener('dispose', onPlaceholder);
    const bitmap = new FakeBitmap();
    deliver(bitmap);
    const loaded = node.value as THREE.Texture, onLoaded = vi.fn();
    expect(loaded).not.toBe(placeholder);
    expect(loaded.image).toBe(bitmap);
    expect(onPlaceholder).toHaveBeenCalledOnce();
    loaded.addEventListener('dispose', onLoaded);
    art.dispose();
    expect(onLoaded).toHaveBeenCalledOnce();
    expect(bitmap.closed).toBe(true);
  });

  it('closes a bitmap that arrives after disposal and leaves the retired node alone', () => {
    const { art, node, deliver } = setup();
    const placeholder = node.value as THREE.Texture;
    art.dispose();
    const late = new FakeBitmap();
    deliver(late);
    expect(late.closed).toBe(true);
    expect(node.value).toBe(placeholder);
  });

  it('streams independently into a recreated art set after renderer recovery', () => {
    const first = setup();
    first.art.dispose();
    const second = setup();
    const late = new FakeBitmap(), fresh = new FakeBitmap();
    first.deliver(late);
    second.deliver(fresh);
    expect(late.closed).toBe(true);
    expect((second.node.value as THREE.Texture).image).toBe(fresh);
    expect(fresh.closed).toBe(false);
    second.art.dispose();
    expect(fresh.closed).toBe(true);
  });
});
