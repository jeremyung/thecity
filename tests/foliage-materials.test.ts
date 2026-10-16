import { afterEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';

function installCanvas() {
  const context = {
    fillRect() {},
    beginPath() {},
    moveTo() {},
    quadraticCurveTo() {},
    stroke() {},
    ellipse() {},
    fill() {},
  };
  vi.stubGlobal('document', {
    createElement: () => ({ width: 0, height: 0, getContext: () => context }),
  });
}

afterEach(() => vi.unstubAllGlobals());

describe('foliage material variants', () => {
  it('keeps identical leaf coverage while miniature shadow fill follows per-tree tint', () => {
    installCanvas();
    const art = new ArtMaterials();
    try {
      art.setStyle('anime');
      const anime = art.get('#fffffe', 0.96);
      art.setStyle('miniature');
      const miniature = art.get('#fffffe', 0.96);

      expect(anime).toBeInstanceOf(THREE.MeshToonNodeMaterial);
      expect(miniature).toBeInstanceOf(THREE.MeshStandardNodeMaterial);
      expect(anime.vertexColors).toBe(true);
      expect(miniature.vertexColors).toBe(true);
      expect(anime.opacityNode).toBe(miniature.opacityNode);
      expect(anime.opacityNode?.isNode).toBe(true);
      expect(anime.alphaTest).toBe(0.38);
      expect(miniature.alphaTest).toBe(0.38);
      expect(anime.alphaToCoverage).toBe(true);
      expect(miniature.alphaToCoverage).toBe(true);
      expect(anime.side).toBe(THREE.DoubleSide);
      expect(miniature.side).toBe(THREE.DoubleSide);

      const animeEmission = (anime as unknown as { emissiveNode?: { isNode?: boolean } })
        .emissiveNode;
      const miniatureEmission = (
        miniature as unknown as {
          emissiveNode?: {
            isNode?: boolean;
            traverse?: (visit: (node: { isPropertyNode?: boolean; name?: string }) => void) => void;
          };
        }
      ).emissiveNode;
      expect(miniatureEmission?.isNode).toBe(true);
      expect(miniatureEmission).not.toBe(animeEmission);
      const varyingNames: string[] = [];
      miniatureEmission?.traverse?.((node) => {
        if (node.isPropertyNode && node.name) varyingNames.push(node.name);
      });
      expect(varyingNames).toContain('vInstanceColor');
    } finally {
      art.dispose();
    }
  });
});
