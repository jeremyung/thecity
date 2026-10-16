import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { streamedTexture } from '../web/src/render/vehicle-materials';

/** Stand-in for ArtMaterials' dispose hooks. */
function fakeArt() {
  const disposers: Array<() => void> = [];
  return {
    onDispose: (release: () => void) => {
      disposers.push(release);
    },
    dispose: () => disposers.splice(0).forEach((release) => release()),
  };
}

/** A loader that holds each request until the test delivers it. */
function heldLoader() {
  const pending: Array<{
    url: string;
    onLoad: (bitmap: ImageBitmap) => void;
    onError: () => void;
  }> = [];
  return {
    pending,
    load: (url: string, onLoad: (bitmap: ImageBitmap) => void, onError: () => void) => {
      pending.push({ url, onLoad, onError });
    },
  };
}

function bitmap() {
  return { width: 4, height: 2, close: vi.fn() } as unknown as ImageBitmap & {
    close: ReturnType<typeof vi.fn>;
  };
}

function disposeSpy(texture: THREE.Texture) {
  const spy = vi.fn();
  texture.addEventListener('dispose', spy);
  return spy;
}

describe('streamed vehicle textures', () => {
  it('installs a bitmap that loads first, then frees it and closes the bitmap on dispose', () => {
    const art = fakeArt(),
      loader = heldLoader();
    const node = streamedTexture(
      '/atlas.png',
      THREE.SRGBColorSpace,
      art,
      [1, 2, 3, 255],
      loader.load,
    );
    const placeholder = node.value as THREE.Texture,
      placeholderDisposed = disposeSpy(placeholder);
    const image = bitmap();
    loader.pending[0].onLoad(image);
    const loaded = node.value as THREE.Texture;
    expect(loaded).not.toBe(placeholder);
    expect(loaded.image).toBe(image);
    expect(loaded.colorSpace).toBe(THREE.SRGBColorSpace);
    expect(placeholderDisposed).toHaveBeenCalledTimes(1);
    // The live texture keeps its bitmap open so a lost device can re-upload it.
    expect(image.close).not.toHaveBeenCalled();

    const loadedDisposed = disposeSpy(loaded);
    art.dispose();
    expect(loadedDisposed).toHaveBeenCalledTimes(1);
    expect(image.close).toHaveBeenCalledTimes(1);
  });

  it('closes a bitmap that arrives after dispose without creating or installing a texture', () => {
    const art = fakeArt(),
      loader = heldLoader();
    const node = streamedTexture(
      '/mask.png',
      THREE.NoColorSpace,
      art,
      [255, 0, 0, 255],
      loader.load,
    );
    const placeholder = node.value as THREE.Texture,
      placeholderDisposed = disposeSpy(placeholder);
    art.dispose();
    expect(placeholderDisposed).toHaveBeenCalledTimes(1);

    // Texture ids are sequential: probes either side show whether one was created.
    const before = new THREE.Texture().id;
    const late = bitmap();
    loader.pending[0].onLoad(late);
    expect(late.close).toHaveBeenCalledTimes(1);
    expect(node.value).toBe(placeholder);
    expect(new THREE.Texture().id).toBe(before + 1);
    expect(placeholderDisposed).toHaveBeenCalledTimes(1);
  });

  it("keeps a recovered renderer's textures apart from the retired one's late loads", () => {
    const loader = heldLoader();
    const retiredArt = fakeArt();
    const retired = streamedTexture(
      '/atlas.png',
      THREE.SRGBColorSpace,
      retiredArt,
      [200, 200, 200, 255],
      loader.load,
    );
    retiredArt.dispose();
    // Graphics recovery builds a fresh ArtMaterials, which requests the art again.
    const freshArt = fakeArt();
    const fresh = streamedTexture(
      '/atlas.png',
      THREE.SRGBColorSpace,
      freshArt,
      [200, 200, 200, 255],
      loader.load,
    );
    const freshPlaceholder = fresh.value;
    expect(loader.pending).toHaveLength(2);

    const late = bitmap(),
      current = bitmap();
    loader.pending[0].onLoad(late);
    expect(late.close).toHaveBeenCalledTimes(1);
    expect(fresh.value).toBe(freshPlaceholder);
    expect(retired.value.image).not.toBe(late);

    loader.pending[1].onLoad(current);
    expect((fresh.value as THREE.Texture).image).toBe(current);
    expect(current.close).not.toHaveBeenCalled();
    freshArt.dispose();
    expect(current.close).toHaveBeenCalledTimes(1);
  });

  it('keeps the placeholder when the art fails to load, and without a decoder', () => {
    const art = fakeArt(),
      loader = heldLoader();
    const node = streamedTexture(
      '/missing.png',
      THREE.SRGBColorSpace,
      art,
      [200, 200, 200, 255],
      loader.load,
    );
    const placeholder = node.value;
    loader.pending[0].onError();
    expect(node.value).toBe(placeholder);
    const headless = streamedTexture(
      '/atlas.png',
      THREE.SRGBColorSpace,
      art,
      [200, 200, 200, 255],
      null,
    );
    expect((headless.value as THREE.DataTexture).image.width).toBe(1);
    art.dispose();
  });
});
