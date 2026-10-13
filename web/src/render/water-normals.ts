import * as THREE from 'three/webgpu';

/**
 * Seamless ripple normal map, generated instead of shipped as an image.
 * The height field is a sum of sine waves with integer wave vectors, so it
 * tiles exactly; amplitudes fall with frequency like a small capillary
 * spectrum. Encoding: r = x slope, g = z slope, b = up (all mapped to 0..255).
 */
export function createWaterNormalData(size = 256, seed = 0x2f6e2b1): Uint8Array {
  let state = seed >>> 0;
  const random = () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 0x1_0000_0000;
  };
  const waves: Array<{ kx: number; ky: number; amplitude: number; phase: number }> = [];
  while (waves.length < 72) {
    const kx = Math.round((random() * 2 - 1) * 32), ky = Math.round((random() * 2 - 1) * 32);
    const k = Math.hypot(kx, ky);
    // No very long components: a few dominant waves read as parallel bands.
    if (k < 5 || k > 34) continue;
    waves.push({ kx, ky, amplitude: 1 / Math.pow(k, 1.25), phase: random() * Math.PI * 2 });
  }
  const slopes = new Float32Array(size * size * 2);
  let maxSlope = 0;
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      let dx = 0, dy = 0;
      for (const wave of waves) {
        const angle = Math.PI * 2 * (wave.kx * x + wave.ky * y) / size + wave.phase;
        const derivative = Math.cos(angle) * wave.amplitude * Math.PI * 2;
        dx += derivative * wave.kx;
        dy += derivative * wave.ky;
      }
      const i = (y * size + x) * 2;
      slopes[i] = dx; slopes[i + 1] = dy;
      maxSlope = Math.max(maxSlope, Math.abs(dx), Math.abs(dy));
    }
  }
  const data = new Uint8Array(size * size * 4);
  const scale = 1.6 / maxSlope;
  for (let i = 0; i < size * size; i += 1) {
    const nx = -slopes[i * 2] * scale, nz = -slopes[i * 2 + 1] * scale;
    const length = Math.hypot(nx, 1, nz);
    data[i * 4] = Math.round((nx / length * 0.5 + 0.5) * 255);
    data[i * 4 + 1] = Math.round((nz / length * 0.5 + 0.5) * 255);
    data[i * 4 + 2] = Math.round((1 / length * 0.5 + 0.5) * 255);
    data[i * 4 + 3] = 255;
  }
  return data;
}

export function createWaterNormalTexture(size = 256): THREE.DataTexture {
  const texture = new THREE.DataTexture(createWaterNormalData(size), size, size, THREE.RGBAFormat);
  texture.wrapS = texture.wrapT = THREE.RepeatWrapping;
  texture.magFilter = THREE.LinearFilter;
  texture.minFilter = THREE.LinearMipmapLinearFilter;
  texture.generateMipmaps = true;
  texture.anisotropy = 4;
  texture.needsUpdate = true;
  return texture;
}
