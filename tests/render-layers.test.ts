import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { RENDER_LAYERS } from '../web/src/render/layers';
import type { LayerHost } from '../web/src/render/layer';

describe('render layer registry', () => {
  it('creates uniquely named layers with scene roots from host services only', () => {
    const host = {
      art: { get: () => new THREE.MeshBasicMaterial() },
      retire: vi.fn(),
      isBuildingVisible: () => true,
    } as unknown as LayerHost;
    const layers = RENDER_LAYERS.map((create) => create(host));
    expect(new Set(layers.map((layer) => layer.name)).size).toBe(layers.length);
    for (const layer of layers) expect(layer.root).toBeInstanceOf(THREE.Object3D);
  });
});
