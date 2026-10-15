import { afterEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';

function installCanvasForStorefrontAtlas() {
  const context = {
    fillStyle: '',
    strokeStyle: '',
    lineWidth: 1,
    font: '',
    textAlign: 'start',
    textBaseline: 'alphabetic',
    fillRect: vi.fn(),
    fillText: vi.fn(),
    strokeRect: vi.fn(),
    beginPath: vi.fn(),
    ellipse: vi.fn(),
    fill: vi.fn(),
    stroke: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    closePath: vi.fn(),
  };
  vi.stubGlobal('document', {
    createElement: () => ({ width: 1, height: 1, getContext: () => context }),
  });
}

afterEach(() => vi.unstubAllGlobals());

describe('storefront poster atlas', () => {
  it('shares one 512-square atlas across style materials and disposes it once', () => {
    installCanvasForStorefrontAtlas();
    const art = new ArtMaterials();
    const anime = art.storefrontPoster();
    const atlas = (anime as THREE.MeshToonNodeMaterial).map as THREE.CanvasTexture;
    const dispose = vi.spyOn(atlas, 'dispose');

    expect(atlas.image).toMatchObject({ width: 512, height: 512 });
    expect(anime.userData.artKey).toBe('storefront-poster-atlas');
    expect('emissiveNode' in anime ? anime.emissiveNode : null).toBeNull();
    expect(art.storefrontPoster()).toBe(anime);

    art.setStyle('miniature');
    const miniature = art.storefrontPoster();
    expect(miniature).not.toBe(anime);
    expect((miniature as THREE.MeshStandardNodeMaterial).map).toBe(atlas);
    expect(miniature.userData.artKey).toBe('storefront-poster-atlas');
    expect((miniature as THREE.MeshStandardNodeMaterial).emissiveNode ?? null).toBeNull();
    expect(art.storefrontPoster()).toBe(miniature);

    art.dispose();
    expect(dispose).toHaveBeenCalledTimes(1);
  });

  it('keeps both shared style variants refreshable for unchanged shadow receivers in the same render', () => {
    installCanvasForStorefrontAtlas();
    const art = new ArtMaterials();
    const atlasAnime = art.storefrontPoster();
    art.setStyle('miniature');
    const atlasMiniature = art.storefrontPoster();
    const target = new THREE.RenderTarget(16, 16);
    target.depthTexture = new THREE.DepthTexture(16, 16);
    const light = new THREE.DirectionalLight();
    light.castShadow = true;
    light.shadow.map = target;
    const nodeFrame = { renderId: 9, renderer: { getMRT: () => null } } as any;

    for (const material of [atlasAnime, atlasMiniature]) {
      const geometry = new THREE.PlaneGeometry(1, 1);
      const first = new THREE.Mesh(geometry, material);
      const second = new THREE.Mesh(geometry, material);
      first.receiveShadow = true;
      second.receiveShadow = true;
      expect(first.material).toBe(second.material);

      for (const object of [first, second]) {
        const observer = material.setupObserver({ material, object, context: {} } as any);
        const renderObject = {
          object,
          material,
          geometry,
          bundle: null,
          context: {},
          lightsNode: { getLights: () => [light] },
        } as any;
        const versionBefore = material.version;

        expect(observer.needsRefresh(renderObject, nodeFrame)).toBe(true);
        expect(observer.needsRefresh(renderObject, nodeFrame)).toBe(true);
        target.depthTexture!.needsUpdate = true;
        expect(observer.needsRefresh(renderObject, nodeFrame)).toBe(true);
        expect(material.version).toBe(versionBefore);
      }

      geometry.dispose();
    }

    art.dispose();
    target.dispose();
    target.depthTexture?.dispose();
  });
});
