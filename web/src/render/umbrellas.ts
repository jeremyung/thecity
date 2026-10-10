import * as THREE from 'three/webgpu';

const CANOPY_COLORS = ['#b96f5a', '#d2ad68', '#6f8e7c', '#667f96', '#d8cbb0', '#9b879b'] as const;
const SHAFT_COLOR = '#665b50';

/**
 * Rain accessories for the existing actor tuple stream. Pass ArtMaterials.get
 * as `materialForColor` to let the city style switch update umbrella surfaces.
 */
export class Umbrellas {
  readonly root = new THREE.Group();
  readonly capacity: number;

  private readonly canopyGeometry: THREE.BufferGeometry;
  private readonly shaftGeometry: THREE.BufferGeometry;
  private readonly canopyMeshes: THREE.InstancedMesh[] = [];
  private readonly shaftMesh: THREE.InstancedMesh | null;
  private readonly ownedMaterials = new Set<THREE.Material>();
  private readonly sourceIds: Float64Array;
  private readonly sourceOffsets: Int32Array;
  private readonly priorities: Float64Array;
  private readonly colorCounts: Uint16Array;
  private readonly matrixHelper = new THREE.Object3D();
  private disposed = false;
  private opening = 0;
  private lastTime: number | null = null;

  constructor(max = 2_000, materialForColor?: (hex: string) => THREE.Material) {
    this.capacity = Math.max(0, Math.floor(Number.isFinite(max) ? max : 0));
    this.sourceIds = new Float64Array(this.capacity);
    this.sourceOffsets = new Int32Array(this.capacity);
    this.priorities = new Float64Array(this.capacity);
    this.colorCounts = new Uint16Array(CANOPY_COLORS.length);
    this.canopyGeometry = makeCanopyGeometry();
    this.shaftGeometry = new THREE.CylinderGeometry(0.018, 0.018, 0.78, 6, 1);

    const getMaterial = (hex: string) => {
      if (materialForColor) return materialForColor(hex);
      const material = new THREE.MeshStandardMaterial({ color: hex, roughness: 0.82, metalness: 0 });
      this.ownedMaterials.add(material);
      return material;
    };
    if (this.capacity > 0) {
      for (const color of CANOPY_COLORS) {
        const mesh = makeInstanced(this.canopyGeometry, getMaterial(color), this.capacity);
        this.canopyMeshes.push(mesh);
        this.root.add(mesh);
      }
      this.shaftMesh = makeInstanced(this.shaftGeometry, getMaterial(SHAFT_COLOR), this.capacity);
      this.root.add(this.shaftMesh);
    } else {
      this.shaftMesh = null;
    }
  }

  /** Update from [id, x, z, heading, kind] tuples; kind 0 is a pedestrian. */
  update(positions: Float32Array, rain: number, time: number): void {
    if (this.disposed || this.capacity === 0 || !this.shaftMesh) return;
    const rainLevel = clamp01(rain);
    const now = Number.isFinite(time) ? time : 0;
    const dt = this.lastTime === null ? 0 : clamp(now - this.lastTime, 0, 1);
    if (this.lastTime === null) this.opening = rainLevel;
    else this.opening += (rainLevel - this.opening) * (1 - Math.exp(-dt * 2.6));
    this.lastTime = now;
    if (this.opening <= 0.002) {
      for (const mesh of this.canopyMeshes) mesh.count = 0;
      this.shaftMesh.count = 0;
      return;
    }

    let count = 0;
    for (let offset = 0; offset + 4 < positions.length; offset += 5) {
      if (positions[offset + 4] !== 0) continue;
      const id = Math.trunc(positions[offset]);
      const priority = stablePriority(id);
      if (count < this.capacity) {
        let index = count;
        this.sourceIds[index] = id;
        this.sourceOffsets[index] = offset;
        this.priorities[index] = priority;
        count++;
        while (index > 0) {
          const parent = (index - 1) >>> 1;
          if (!worse(this.priorities[index], this.sourceIds[index], this.priorities[parent], this.sourceIds[parent])) break;
          swap(this.sourceIds, this.sourceOffsets, this.priorities, index, parent);
          index = parent;
        }
      } else if (worse(this.priorities[0], this.sourceIds[0], priority, id)) {
        this.sourceIds[0] = id;
        this.sourceOffsets[0] = offset;
        this.priorities[0] = priority;
        let index = 0;
        while (true) {
          const left = index * 2 + 1;
          if (left >= count) break;
          const right = left + 1;
          let worstChild = left;
          if (right < count && worse(this.priorities[right], this.sourceIds[right], this.priorities[left], this.sourceIds[left])) worstChild = right;
          if (!worse(this.priorities[worstChild], this.sourceIds[worstChild], this.priorities[index], this.sourceIds[index])) break;
          swap(this.sourceIds, this.sourceOffsets, this.priorities, index, worstChild);
          index = worstChild;
        }
      }
    }

    this.colorCounts.fill(0);
    for (const mesh of this.canopyMeshes) mesh.count = 0;
    this.shaftMesh.count = 0;
    for (let i = 0; i < count; i++) {
      const offset = this.sourceOffsets[i];
      const id = this.sourceIds[i];
      const x = positions[offset + 1];
      const z = positions[offset + 2];
      const heading = Math.PI / 2 - positions[offset + 3];
      const colorIndex = stableColor(id);
      const canopyIndex = this.colorCounts[colorIndex]++;

      const radius = this.opening;
      const helper = this.matrixHelper;
      helper.position.set(x, 2.32, z);
      helper.rotation.set(0, heading, 0);
      helper.scale.set(radius, 0.72 + this.opening * 0.28, radius);
      helper.updateMatrix();
      const canopy = this.canopyMeshes[colorIndex];
      canopy.setMatrixAt(canopyIndex, helper.matrix);
      canopy.count = canopyIndex + 1;

      helper.position.set(x, 1.93, z);
      helper.rotation.set(0, heading, 0);
      helper.scale.setScalar(1);
      helper.updateMatrix();
      this.shaftMesh.setMatrixAt(i, helper.matrix);
      this.shaftMesh.count = i + 1;
    }
    for (const mesh of this.canopyMeshes) {
      mesh.instanceMatrix.needsUpdate = true;
      if (mesh.count > 0) mesh.computeBoundingSphere();
    }
    this.shaftMesh.instanceMatrix.needsUpdate = true;
    if (this.shaftMesh.count > 0) this.shaftMesh.computeBoundingSphere();
  }

  /** Clear visual and easing state when the renderer switches to another world. */
  reset(): void {
    if (this.disposed) return;
    this.opening = 0;
    this.lastTime = null;
    this.sourceIds.fill(-1);
    this.sourceOffsets.fill(0);
    this.priorities.fill(0);
    this.colorCounts.fill(0);
    for (const mesh of this.canopyMeshes) mesh.count = 0;
    if (this.shaftMesh) this.shaftMesh.count = 0;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.root.removeFromParent();
    for (const mesh of this.canopyMeshes) mesh.dispose();
    this.shaftMesh?.dispose();
    this.canopyGeometry.dispose();
    this.shaftGeometry.dispose();
    for (const material of this.ownedMaterials) material.dispose();
    this.ownedMaterials.clear();
    this.canopyMeshes.length = 0;
  }
}

function makeInstanced(geometry: THREE.BufferGeometry, material: THREE.Material, capacity: number): THREE.InstancedMesh {
  const mesh = new THREE.InstancedMesh(geometry, material, capacity);
  mesh.count = 0;
  mesh.castShadow = false;
  mesh.receiveShadow = false;
  mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
  mesh.frustumCulled = true;
  return mesh;
}

function makeCanopyGeometry(): THREE.BufferGeometry {
  const segments = 8;
  const rings = [
    { radius: 0.55, y: 0 },
    { radius: 0.39, y: 0.14 },
    { radius: 0.19, y: 0.235 },
  ];
  const positions: number[] = [];
  const indices: number[] = [];
  for (const ring of rings) {
    for (let i = 0; i < segments; i++) {
      const angle = (i / segments) * Math.PI * 2;
      positions.push(Math.cos(angle) * ring.radius, ring.y, Math.sin(angle) * ring.radius);
    }
  }
  const topIndex = positions.length / 3;
  positions.push(0, 0.28, 0);
  for (let ring = 0; ring < rings.length - 1; ring++) {
    const base = ring * segments;
    const next = (ring + 1) * segments;
    for (let i = 0; i < segments; i++) {
      const following = (i + 1) % segments;
      indices.push(base + i, next + i, base + following, base + following, next + i, next + following);
    }
  }
  const inner = (rings.length - 1) * segments;
  for (let i = 0; i < segments; i++) indices.push(inner + i, topIndex, inner + ((i + 1) % segments));
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geometry.setIndex(indices);
  geometry.computeVertexNormals();
  geometry.computeBoundingSphere();
  return geometry;
}

function stablePriority(id: number): number {
  let value = id >>> 0;
  value ^= value >>> 16;
  value = Math.imul(value, 0x7feb352d);
  value ^= value >>> 15;
  value = Math.imul(value, 0x846ca68b);
  value ^= value >>> 16;
  return (value >>> 0) / 0x1_0000_0000;
}

function worse(aPriority: number, aId: number, bPriority: number, bId: number): boolean {
  return aPriority > bPriority || aPriority === bPriority && aId > bId;
}

function swap(ids: Float64Array, offsets: Int32Array, priorities: Float64Array, first: number, second: number): void {
  let value = ids[first]; ids[first] = ids[second]; ids[second] = value;
  const offset = offsets[first]; offsets[first] = offsets[second]; offsets[second] = offset;
  value = priorities[first]; priorities[first] = priorities[second]; priorities[second] = value;
}

function stableColor(id: number): number {
  return (stablePriority(id) * CANOPY_COLORS.length) | 0;
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, Number.isFinite(value) ? value : 0));
}
