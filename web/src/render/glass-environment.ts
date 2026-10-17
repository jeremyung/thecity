import * as THREE from 'three/webgpu';
import type { Building } from '../types';
/** One bounded city reflection probe, shared by all corporate panes. The proxy
 * preserves nearby massing without rendering the detailed city six extra times. */
export class GlassEnvironment {
  private scene = new THREE.Scene();
  private generator: THREE.PMREMGenerator | null = null;
  private target: THREE.RenderTarget | null = null;
  private lastAt = -Infinity;
  private signature = '';
  private blocks: THREE.InstancedMesh;
  private ground: THREE.Mesh;
  private sky: THREE.Mesh;
  constructor(
    sky: THREE.Mesh,
    private readonly createGenerator = (renderer: THREE.WebGPURenderer) =>
      new THREE.PMREMGenerator(renderer),
  ) {
    this.sky = new THREE.Mesh(sky.geometry, sky.material);
    this.scene.add(this.sky);
    this.blocks = new THREE.InstancedMesh(
      new THREE.BoxGeometry(1, 1, 1),
      new THREE.MeshBasicNodeMaterial({ color: '#ffffff' }),
      96,
    );
    this.blocks.count = 0;
    this.blocks.frustumCulled = false;
    this.scene.add(this.blocks);
    this.ground = new THREE.Mesh(
      new THREE.PlaneGeometry(2600, 2600),
      new THREE.MeshBasicNodeMaterial({ color: '#434e50' }),
    );
    this.ground.rotation.x = -Math.PI / 2;
    this.scene.add(this.ground);
  }
  update(
    renderer: THREE.WebGPURenderer,
    now: number,
    buildings: readonly Building[],
    camera: THREE.Vector3,
    colors: THREE.Color[],
    night: number,
  ): THREE.Texture | null {
    if (this.target && now - this.lastAt < 2500) return this.target.texture;
    this.lastAt = now;
    const focal = buildings.reduce<Building | undefined>(
      (best, b) =>
        !best ||
        Math.hypot(b.x - camera.x, b.z - camera.z) <
          Math.hypot(best.x - camera.x, best.z - camera.z)
          ? b
          : best,
      undefined,
    );
    const x = Math.round((focal?.x ?? camera.x) / 80) * 80,
      z = Math.round((focal?.z ?? camera.z) / 80) * 80 + 40;
    const nearby = buildings
      .filter((b) => b.stage !== 'foundation' && Math.hypot(b.x - x, b.z - z) < 650)
      .sort((a, b) => Math.hypot(a.x - x, a.z - z) - Math.hypot(b.x - x, b.z - z) || a.id - b.id)
      .slice(0, 96);
    const key = [
      x,
      z,
      Math.round(night * 20),
      ...colors.flatMap((c) => [c.r, c.g, c.b].map((n) => Math.round(n * 20))),
      ...nearby.map(
        (b) =>
          `${b.id}:${b.floors}:${b.width}:${b.depth}:${b.x}:${b.z}:${b.rotation}:${b.stage}:${Math.round(b.progress * 10)}`,
      ),
    ].join('/');
    if (this.target && key === this.signature) return this.target.texture;
    const transform = new THREE.Object3D(),
      tint = new THREE.Color();
    let count = 0;
    for (const b of nearby) {
      const height = Math.max(
        0.8,
        b.floors *
          3.25 *
          (b.stage === 'open' || b.stage === 'renovation'
            ? 1
            : b.stage === 'demolition'
              ? 1 - b.progress
              : b.progress),
      );
      // Do not put the capture point inside a proxy building.
      if (Math.abs(b.x - x) < b.width * 0.7 && Math.abs(b.z - z) < b.depth * 0.7) continue;
      transform.position.set(b.x, height / 2, b.z);
      transform.rotation.y = b.rotation;
      transform.scale.set(b.width, height, b.depth);
      transform.updateMatrix();
      this.blocks.setMatrixAt(count, transform.matrix);
      tint
        .set(b.kind === 'office' ? '#536b79' : b.kind === 'apartment' ? '#b4afa2' : '#928c80')
        .multiplyScalar(1 - night * 0.88);
      this.blocks.setColorAt(count++, tint);
    }
    this.blocks.count = count;
    this.blocks.instanceMatrix.needsUpdate = true;
    if (this.blocks.instanceColor) this.blocks.instanceColor.needsUpdate = true;
    (this.ground.material as THREE.MeshBasicNodeMaterial).color
      .set('#434e50')
      .multiplyScalar(1 - night * 0.9);
    this.ground.position.set(x, -0.1, z);
    this.sky.position.set(x, 40, z);
    this.generator ??= this.createGenerator(renderer);
    this.target = this.generator.fromScene(this.scene, 0.025, 1, 2200, {
      size: 128,
      position: new THREE.Vector3(x, 40, z),
      renderTarget: this.target,
    });
    this.signature = key;
    this.lastAt = now;
    return this.target.texture;
  }
  dispose() {
    this.target?.dispose();
    this.generator?.dispose();
    this.blocks.geometry.dispose();
    (this.blocks.material as THREE.Material).dispose();
    this.ground.geometry.dispose();
    (this.ground.material as THREE.Material).dispose();
    this.target = null;
  }
}
