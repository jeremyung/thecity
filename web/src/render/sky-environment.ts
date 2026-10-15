import * as THREE from 'three/webgpu';

/**
 * Pre-filtered environment from the live sky dome, so standard (Miniature)
 * surfaces receive sky-coloured ambient and reflect the actual sky. The dome
 * material is shared, so the environment follows its time-of-day uniforms;
 * it is re-filtered only when those colours drift, never every frame.
 */
export class SkyEnvironment {
  private readonly scene = new THREE.Scene();
  private generator: THREE.PMREMGenerator | null = null;
  private target: THREE.RenderTarget | null = null;
  private signature: number[] = [];
  private lastAt = -Infinity;

  constructor(sky: THREE.Mesh) {
    this.scene.add(new THREE.Mesh(sky.geometry, sky.material));
  }

  /** Returns the current environment texture, refreshing it if the sky moved. */
  update(renderer: THREE.WebGPURenderer, now: number, colors: THREE.Color[]): THREE.Texture | null {
    const next = colors.flatMap((c) => [c.r, c.g, c.b]);
    const drift = next.reduce((max, value, i) => Math.max(max, Math.abs(value - (this.signature[i] ?? -1))), 0);
    if (this.target && (drift < 0.02 || now - this.lastAt < 1500)) return this.target.texture;
    this.generator ??= new THREE.PMREMGenerator(renderer);
    this.target = this.generator.fromScene(this.scene, 0.04, 1, 2000, { size: 128, renderTarget: this.target });
    this.signature = next;
    this.lastAt = now;
    return this.target.texture;
  }

  dispose() {
    this.target?.dispose();
    this.generator?.dispose();
    this.target = null;
    this.generator = null;
  }
}
