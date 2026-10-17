import * as THREE from 'three/webgpu';
import { color, positionWorld, mx_noise_float, vec3, smoothstep } from 'three/tsl';
import type { ArtMaterials } from '../materials';
import type { Kit } from './geometry-kit';
import { architecturalChoice } from './variation';
export function retailVisionGlass(art: ArtMaterials) {
  return art.shared('flagship-retail-glass', () => {
    const anime = new THREE.MeshToonNodeMaterial({
      color: '#bdccc9',
      gradientMap: art.toonRamp,
      transparent: true,
      opacity: 0.18,
      depthWrite: false,
    });
    const miniature = new THREE.MeshPhysicalNodeMaterial({
      color: '#adc1c1',
      transparent: true,
      opacity: 0.24,
      depthWrite: false,
      roughness: 0.14,
      metalness: 0,
      ior: 1.48,
      envMapIntensity: 0.65,
    });
    for (const m of [anime, miniature]) {
      m.userData.corporateGlass = true;
      m.userData.retailVisionGlass = true;
      m.userData.reflectionSurface = { version: 1, role: 'vision-glass' };
    }
    return { anime, miniature };
  });
}
/** Small coherent tenant modules in facade-local metres. Negative Z is inside. */
export function addRetailInterior(
  kit: Kit,
  art: ArtMaterials,
  m: {
    x: number;
    y: number;
    z: number;
    angle: number;
    width: number;
    seed: number;
    floor: number;
    kind?: number;
  },
) {
  const tx = Math.cos(m.angle),
    tz = -Math.sin(m.angle),
    nx = Math.sin(m.angle),
    nz = Math.cos(m.angle);
  const box = (
    x: number,
    y: number,
    z: number,
    w: number,
    h: number,
    d: number,
    mat: THREE.Material,
  ) => kit.box(m.x + tx * x + nx * z, m.y + y, m.z + tz * x + nz * z, w, h, d, mat, m.angle);
  const wood = art.get('#8c6e51'),
    cream = art.get('#d7ceba'),
    steel = art.get('#39474b', 0.45),
    fabric = art.get('#806f59');
  const kind = m.kind ?? architecturalChoice(m.seed, 810, m.floor) % 3,
    pw = Math.max(0.8, Math.min(2.8, m.width - 0.2));
  // A visible ceiling and warm luminaire provide depth without another light object.
  box(0, 2.91, -1.2, pw, 0.08, 2.6, cream);
  box(0, 2.85, -1.15, pw * 0.65, 0.025, 0.12, art.get('#ead8b5', 0.5, true, 'shop'));
  if (kind === 0) {
    box(0, 0.42, -1.25, pw * 0.7, 0.8, 0.7, wood);
    box(0, 0.85, -1.25, pw * 0.78, 0.07, 0.85, cream);
    for (let i = 0; i < 3; i++) {
      const x = (i - 1) * pw * 0.2;
      box(x, 1.02, -1.25, pw * 0.13, 0.25, 0.32, art.get(['#c4b092', '#637d7b', '#a47662'][i]));
    }
    box(0, 1.45, -2.4, pw, 0.09, 0.45, wood);
    box(0, 0.82, -2.4, pw, 0.09, 0.45, wood);
  } else if (kind === 1) {
    box(0, 0.82, -1.4, pw * 0.6, 0.08, 0.7, wood);
    box(0, 0.42, -1.4, 0.08, 0.8, 0.08, steel);
    for (const side of [-1, 1]) {
      box(side * pw * 0.4, 0.48, -1.4, 0.4, 0.08, 0.42, fabric);
      box(side * pw * 0.4, 0.75, -1.65, 0.4, 0.55, 0.06, fabric);
      box(side * pw * 0.4, 0.24, -1.4, 0.055, 0.45, 0.055, steel);
    }
    box(-0.1, 0.91, -1.4, 0.12, 0.13, 0.12, cream);
    box(0.2, 0.89, -1.3, 0.15, 0.05, 0.18, cream);
  } else {
    for (let shelf = 0; shelf < 4; shelf++) {
      const y = 0.45 + shelf * 0.48;
      box(0, y, -1.8, pw, 0.065, 0.4, wood);
      for (let i = 0; i < 5; i++)
        box(
          (i - 2) * pw * 0.17,
          y + 0.18,
          -1.8,
          pw * 0.12,
          0.25 + (i % 3) * 0.045,
          0.24,
          art.get(['#a76f55', '#576e76', '#c2b08b', '#747d5c'][i % 4]),
        );
    }
    for (const side of [-1, 1]) box(side * pw * 0.49, 1.2, -1.8, 0.065, 2.1, 0.45, steel);
  }
}
/** Two escalator silhouettes behind the main entrance, separate from circulation space. */
export function addFlagshipAtrium(
  kit: Kit,
  art: ArtMaterials,
  front: number,
  base: number,
  xOffset = 0,
) {
  const metal = art.get('#39474b', 0.45),
    tread = art.get('#7d8989');
  for (const side of [-1, 1]) {
    const x = xOffset + side * 1.05;
    const body = new THREE.BoxGeometry(1.45, 0.2, 5.2)
      .rotateX(-0.675)
      .translate(x, base + 1.625, front - 4);
    kit.add(body, tread);
    for (const edge of [-1, 1]) {
      const rail = new THREE.BoxGeometry(0.06, 0.6, 5.2)
        .rotateX(-0.675)
        .translate(x + edge * 0.7, base + 2.05, front - 4);
      kit.add(rail, metal);
    }
    for (let i = 0; i < 12; i++)
      kit.box(
        x,
        base + 0.1 + i * (3.25 / 11),
        front - 6.03 + i * (4.06 / 11),
        1.25,
        0.07,
        0.35,
        metal,
      );
  }
}

export function flagshipCladding(art: ArtMaterials) {
  return art.shared('flagship-aluminium', () => {
    const grain = mx_noise_float(positionWorld.mul(vec3(0.7, 0.16, 0.7))),
      base = smoothstep(0.2, 1.4, positionWorld.y).oneMinus();
    const anime = new THREE.MeshToonNodeMaterial({ color: '#b4bcb9', gradientMap: art.toonRamp }),
      miniature = new THREE.MeshStandardNodeMaterial({
        color: '#b4bcb9',
        roughness: 0.5,
        metalness: 0.12,
      });
    for (const m of [anime, miniature]) {
      m.colorNode = color('#b4bcb9')
        .mul(grain.mul(0.016).add(0.99))
        .mul(base.mul(grain.add(1).mul(0.018)).oneMinus());
    }
    miniature.roughnessNode = grain.mul(0.025).add(0.5);
    return { anime, miniature };
  });
}
