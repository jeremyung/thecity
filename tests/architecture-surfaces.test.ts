import { expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
// @ts-expect-error Node built-in types are not included in the app TypeScript config.
import { readFileSync } from 'node:fs';
import { ApartmentResources } from '../web/src/render/architecture/resources';
import {
  ARCHITECTURE_SURFACES,
  ARCHITECTURE_TEXTURES,
} from '../web/src/render/architecture/surface-catalog';
import { APARTMENT_RECIPES } from '../web/src/render/architecture/recipes';
import { RESIDENTIAL_TOWERS } from '../web/src/render/architecture/tower-recipes';
it('ships every bounded surface with a verified, compact runtime asset', () => {
  const manifest = JSON.parse(readFileSync('public/art/apartment-pilot/manifest.json', 'utf8'));
  expect(ARCHITECTURE_TEXTURES).toHaveLength(12);
  expect(Object.keys(APARTMENT_RECIPES)).toHaveLength(10);
  expect(Object.keys(RESIDENTIAL_TOWERS)).toHaveLength(5);
  let mipBytes = 0;
  for (const name of ARCHITECTURE_TEXTURES) {
    const entry = manifest.textures[name],
      bytes = readFileSync(entry.path);
    expect(bytes.readUInt32BE(16)).toBe(entry.width);
    expect(bytes.readUInt32BE(20)).toBe(entry.height);
    let size = entry.width;
    while (size) {
      mipBytes += size * size * 4;
      size = Math.floor(size / 2);
    }
  }
  expect(mipBytes).toBeLessThan(6.1 * 1024 * 1024);
});
it('keeps different textures in distinct batches, but reuses each finish across colors', () => {
  const r = new ApartmentResources();
  try {
    const a = r.art.get(APARTMENT_RECIPES.gallery.wallTints[0]),
      b = r.art.get(APARTMENT_RECIPES.gallery.wallTints[1]),
      c = r.art.get(APARTMENT_RECIPES.walkup.wallTints[0]);
    expect(r.batch(a).material).toBe(r.batch(b).material);
    expect(r.batch(a).material).not.toBe(r.batch(c).material);
    for (const binding of ARCHITECTURE_SURFACES) {
      const m = r.art.get(binding.hex, binding.roughness);
      expect(r.owns(m)).toBe(true);
      if (binding.texture) expect(ARCHITECTURE_TEXTURES).toContain(m.userData.apartmentTexture);
    }
  } finally {
    r.dispose();
  }
});
it('loads each shared image once, caches loading, and disposes every map once', async () => {
  const maps: THREE.Texture[] = [];
  const loader = vi
    .spyOn(THREE.TextureLoader.prototype, 'loadAsync')
    .mockImplementation(async () => {
      const m = new THREE.Texture<HTMLImageElement>();
      maps.push(m);
      return m;
    });
  const r = new ApartmentResources();
  try {
    const first = r.loadTextures();
    expect(r.loadTextures()).toBe(first);
    expect(await first).toBe(true);
    expect(loader).toHaveBeenCalledTimes(12);
    const disposals = maps.map((m) => vi.spyOn(m, 'dispose'));
    r.dispose();
    r.dispose();
    for (const spy of disposals) expect(spy).toHaveBeenCalledOnce();
  } finally {
    r.dispose();
    loader.mockRestore();
  }
});
it('reports a partial load honestly and cleans up the successful maps', async () => {
  const m = new THREE.Texture<HTMLImageElement>(),
    dispose = vi.spyOn(m, 'dispose');
  const loader = vi
    .spyOn(THREE.TextureLoader.prototype, 'loadAsync')
    .mockImplementation(async (url) => {
      if (String(url).includes('/old-concrete.png')) return m;
      throw Error('missing fixture');
    });
  const r = new ApartmentResources();
  try {
    expect(await r.loadTextures()).toBe(false);
    r.dispose();
    expect(dispose).toHaveBeenCalledOnce();
  } finally {
    r.dispose();
    loader.mockRestore();
  }
});
it('preserves all five tower material families through city chunk batching and style switches', async () => {
  const { BuildingChunks } = await import('../web/src/render/building-chunks');
  const { architectureCityStudy } = await import('../web/src/render/architecture/city-study');
  const { selectApartmentArchitecture, buildSelectedApartment } =
    await import('../web/src/render/architecture/era');
  const { disposeGroup } = await import('../web/src/render/kit');
  const r = new ApartmentResources(),
    chunks = new BuildingChunks(
      (m) => (r.owns(m) ? r.batch(m) : r.art.buildingBatch(m)),
      (release) => release(),
    );
  try {
    for (const b of architectureCityStudy(32, true).buildings) {
      const choice = selectApartmentArchitecture(b, 'contemporary');
      const createGroup = () => {
        const g = buildSelectedApartment(b, r.art, choice, 'district')!;
        g.position.set(b.x, 0, b.z);
        return g;
      };
      chunks.set(b.id, createGroup(), { createGroup, disposeGroup });
    }
    let n = 0;
    while (chunks.pendingCount && n++ < 10000) chunks.update();
    expect(chunks.pendingCount).toBe(0);
    const textures = new Set<string>();
    chunks.root.traverse((o) => {
      if (!(o instanceof THREE.Mesh)) return;
      const m = o.material as THREE.Material;
      if (!r.owns(m)) return;
      textures.add(m.userData.apartmentTexture);
      expect(r.variant(m, 'anime').userData.artKey).toBe(m.userData.artKey);
      if (m.userData.apartmentSurface && m.userData.apartmentRole !== 'glass')
        expect(o.geometry.getAttribute('architectureWear')).toBeDefined();
    });
    for (const name of [
      'new-bronze-metal',
      'new-white-precast',
      'new-honed-limestone',
      'new-charcoal-ceramic',
      'new-ivory-porcelain',
    ])
      expect(textures).toContain(name);
  } finally {
    chunks.clear();
    r.dispose();
  }
}, 20000);
