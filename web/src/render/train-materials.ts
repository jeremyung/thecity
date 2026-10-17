import * as THREE from 'three/webgpu';
import { attribute, color, float, mix, positionGeometry, select, uv, vec3 } from 'three/tsl';
import type { ArtMaterials } from './materials';
import { streamedTexture, wheelPosition } from './vehicle-materials';

const ATLAS_URL = '/art/trains/train-atlas.png';
const MASK_URL = '/art/trains/train-mask.png';

/**
 * One material for every train car. Livery is baked into the atlas, so unlike
 * the road vehicles there is no per-instance paint: the mask only carries glass
 * (B) and lamps (G). Wheels roll from the shared `wheel`/`vstate` attributes.
 */
export function trainMaterial(art: ArtMaterials) {
  return art.shared('train-atlas', () => {
    const atlas = streamedTexture(ATLAS_URL, THREE.SRGBColorSpace, art, [200, 200, 200, 255]);
    const masks = streamedTexture(MASK_URL, THREE.NoColorSpace, art, [0, 0, 0, 255]);
    const base = atlas.sample(uv());
    const mask = masks.sample(uv());
    const state = attribute<'vec4'>('vstate', 'vec4');
    const part = attribute<'float'>('vpart', 'float');
    const glass = mask.b.mul(select(part.equal(2), float(0), float(1)));
    const lamp = mask.g.mul(select(part.equal(3), float(0), float(1)));
    const front = positionGeometry.z.mul(attribute<'float'>('trainFacing', 'float')).greaterThan(0);
    const lampColor = select(front, color('#fff0c8'), color('#ff3b26'));
    const lampPower = select(front, art.night.mul(2.2), art.night.mul(1.3).add(state.w.mul(1.6)));
    const sky = art.glassRadiance();
    const pair = {
      anime: new THREE.MeshToonNodeMaterial({ gradientMap: art.toonRamp }),
      miniature: new THREE.MeshStandardNodeMaterial({ roughness: 0.4, metalness: 0.05 }),
    };
    const door = attribute<'vec2'>('doorTravel', 'vec2');
    const opening = attribute<'float'>('doorOpening', 'float');
    const slide = select(door.x.mul(opening).greaterThan(0), opening.abs().mul(door.y), float(0));
    const position = wheelPosition().add(vec3(0, 0, slide));
    for (const [style, material] of Object.entries(pair) as [
      keyof typeof pair,
      THREE.MeshStandardNodeMaterial | THREE.MeshToonNodeMaterial,
    ][]) {
      material.colorNode = base.rgb;
      material.positionNode = position;
      const emission = lampColor
        .mul(lamp)
        .mul(lampPower)
        .add(sky.mul(glass).mul(style === 'anime' ? 0.55 : 1));
      (material as unknown as { emissiveNode: typeof emission }).emissiveNode = emission;
    }
    pair.miniature.roughnessNode = mix(
      mix(float(0.42), float(0.1), mask.b),
      mix(float(0.24), float(0.06), mask.b),
      art.wet,
    );
    return pair;
  });
}
