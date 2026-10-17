import * as THREE from 'three/webgpu';
import { color, float, smoothstep, uv } from 'three/tsl';
import type { ArtMaterials } from './materials';
import type { World } from '../types';
import { roadWidth } from './road-elevation';
import type { StreetLightSource } from './nearby-street-lights';

export interface StreetLightPlacement {
  x: number;
  z: number;
  /** Road-level base elevation used by the rendered road and sidewalk. */
  y: number;
  /** Three.js yaw for the short arm pointing from the sidewalk into the road. */
  heading: number;
  roadId: number;
}

export interface StreetLightingData {
  count: number;
  /** Pole, arm, lamp and pavement-pool matrices, each packed as count × 16 floats. */
  matrices: [Float32Array, Float32Array, Float32Array, Float32Array];
  sources: StreetLightSource[];
}

const MAX_LIGHTS = 1_000;
const EDGE_MARGIN = 10;
const TARGET_SPACING = 31;
const MIN_SPACING = 28;
const MAX_SPACING = 34;

/** Deterministic pedestrian-side placements for streets with a finished deck. */
export function streetLightPlacements(
  world: Pick<World, 'nodes' | 'roads'>,
): StreetLightPlacement[] {
  const nodes = new Map(world.nodes.map((node) => [node.id, node]));
  const result: StreetLightPlacement[] = [];
  const roads = [...world.roads].sort((a, b) => a.id - b.id);
  for (const road of roads) {
    if (
      road.stage !== 'open' ||
      !road.connected ||
      (road.kind !== 'local' && road.kind !== 'arterial')
    )
      continue;
    const a = nodes.get(road.from),
      b = nodes.get(road.to);
    if (!a || !b || ![a.x, a.z, b.x, b.z].every(Number.isFinite)) continue;
    const dx = b.x - a.x,
      dz = b.z - a.z,
      length = Math.hypot(dx, dz);
    if (!Number.isFinite(length) || length < 2 * EDGE_MARGIN) continue;
    const laneWidth = roadWidth(road.kind);
    const angle = Math.atan2(dx, dz),
      normalX = Math.cos(angle),
      normalZ = -Math.sin(angle);
    const available = length - 2 * EDGE_MARGIN;
    let intervals = 0,
      spacing = 0,
      span = 0;
    if (available < MIN_SPACING) {
      intervals = 0;
    } else {
      intervals = Math.max(1, Math.round(available / TARGET_SPACING));
      spacing = available / intervals;
      if (spacing < MIN_SPACING || spacing > MAX_SPACING) {
        intervals = Math.max(1, Math.floor(available / TARGET_SPACING));
        spacing = TARGET_SPACING;
      }
      span = intervals * spacing;
    }
    const firstDistance = intervals === 0 ? length / 2 : (length - span) / 2;
    const count = intervals === 0 ? 1 : intervals + 1;
    for (let index = 0; index < count && result.length < MAX_LIGHTS; index++) {
      const distance = firstDistance + index * spacing;
      if (distance < EDGE_MARGIN - 1e-6 || length - distance < EDGE_MARGIN - 1e-6) continue;
      const t = distance / length;
      const side = (index % 2 === 0 ? -1 : 1) as -1 | 1;
      const offset = laneWidth / 2 + 0.65;
      const inwardX = -side * normalX,
        inwardZ = -side * normalZ;
      result.push({
        x: a.x + dx * t + side * normalX * offset,
        z: a.z + dz * t + side * normalZ * offset,
        y: 0.3,
        heading: Math.atan2(-inwardZ, inwardX),
        roadId: road.id,
      });
    }
    if (result.length >= MAX_LIGHTS) break;
  }
  return result;
}

/** Prepare immutable instance transforms without allocating or mutating scene objects. */
export function prepareStreetLighting(world: Pick<World, 'nodes' | 'roads'>): StreetLightingData {
  const placements = streetLightPlacements(world),
    count = placements.length;
  const matrices: [Float32Array, Float32Array, Float32Array, Float32Array] = [
    new Float32Array(count * 16),
    new Float32Array(count * 16),
    new Float32Array(count * 16),
    new Float32Array(count * 16),
  ];
  const roadKinds = new Map(world.roads.map((road) => [road.id, road.kind]));
  const roadById = new Map(world.roads.map((road) => [road.id, road]));
  const nodeById = new Map(world.nodes.map((node) => [node.id, node]));
  const dummy = new THREE.Object3D();
  const sources: StreetLightSource[] = [];
  const write = (mesh: number, index: number) => {
    dummy.updateMatrix();
    matrices[mesh].set(dummy.matrix.elements, index * 16);
  };
  for (let i = 0; i < count; i++) {
    const p = placements[i];
    const baseY = roadKinds.get(p.roadId) === 'arterial' ? p.y + 0.175 : 0;
    dummy.position.set(p.x, baseY, p.z);
    dummy.rotation.set(0, 0, 0);
    dummy.scale.setScalar(1);
    write(0, i);
    const inwardX = Math.cos(p.heading),
      inwardZ = -Math.sin(p.heading);
    dummy.position.set(p.x + inwardX * 0.34, baseY + 4.92, p.z + inwardZ * 0.34);
    dummy.rotation.set(0, p.heading, 0);
    write(1, i);
    dummy.position.set(p.x + inwardX * 0.72, baseY + 4.94, p.z + inwardZ * 0.72);
    write(2, i);
    sources.push({
      key: `${p.roadId}:${p.x}:${p.z}`,
      x: dummy.position.x,
      y: dummy.position.y - 0.08,
      z: dummy.position.z,
      targetX: p.x + inwardX * 2.5,
      targetZ: p.z + inwardZ * 2.5,
    });
    const road = roadById.get(p.roadId),
      a = road ? nodeById.get(road.from) : undefined,
      b = road ? nodeById.get(road.to) : undefined;
    const dx = a && b ? b.x - a.x : 0,
      dz = a && b ? b.z - a.z : 0,
      length = Math.hypot(dx, dz);
    const angle = Math.atan2(dx, dz),
      normalX = Math.cos(angle),
      normalZ = -Math.sin(angle);
    const projection =
      length > 0
        ? THREE.MathUtils.clamp(
            ((p.x - (a?.x ?? 0)) * dx + (p.z - (a?.z ?? 0)) * dz) / (length * length),
            0,
            1,
          )
        : 0;
    const signedOffset =
      length > 0 ? ((p.x - (a?.x ?? 0)) * dz - (p.z - (a?.z ?? 0)) * dx) / length : 0;
    const side = signedOffset < 0 ? -1 : 1;
    const laneWidth = roadWidth(roadKinds.get(p.roadId) ?? 'local');
    const transverseRadius = Math.min(1.8, laneWidth * 0.17);
    const inwardDistance = laneWidth / 2 - transverseRadius - 0.08;
    const poolX = (a?.x ?? p.x) + dx * projection + side * normalX * inwardDistance;
    const poolZ = (a?.z ?? p.z) + dz * projection + side * normalZ * inwardDistance;
    dummy.position.set(poolX, 0.325, poolZ);
    dummy.rotation.set(0, angle, 0);
    dummy.scale.set(transverseRadius, 1, 4);
    write(3, i);
  }
  return { count, matrices, sources };
}

/** Owns stable scene objects; road candidates only replace their instance data at commit. */
export class PersistentStreetLighting {
  readonly root = new THREE.Group();
  readonly meshes: [
    THREE.InstancedMesh,
    THREE.InstancedMesh,
    THREE.InstancedMesh,
    THREE.InstancedMesh,
  ];
  private disposed = false;
  constructor(art: ArtMaterials) {
    this.root.name = 'street-lighting';
    const poleMaterial = art.get('#536164', 0.78);
    const poleGeometry = new THREE.CylinderGeometry(0.075, 0.105, 5, 6);
    poleGeometry.translate(0, 2.5, 0);
    const poles = new THREE.InstancedMesh(poleGeometry, poleMaterial, MAX_LIGHTS);
    poles.name = 'street-lamp-poles';
    poles.castShadow = false;
    poles.receiveShadow = true;
    const armGeometry = new THREE.BoxGeometry(0.84, 0.075, 0.075);
    const arms = new THREE.InstancedMesh(armGeometry, poleMaterial, MAX_LIGHTS);
    arms.name = 'street-lamp-arms';
    const lampGeometry = new THREE.BoxGeometry(0.34, 0.12, 0.24);
    const lampMaterial = new THREE.MeshStandardNodeMaterial({
      color: '#8d8068',
      roughness: 0.66,
      emissive: '#ffd08a',
      emissiveIntensity: 0.5,
    });
    const lampEmission = color('#ffd08a').mul(art.night);
    (lampMaterial as unknown as { emissiveNode: typeof lampEmission }).emissiveNode = lampEmission;
    const lamps = new THREE.InstancedMesh(lampGeometry, lampMaterial, MAX_LIGHTS);
    lamps.name = 'street-lamp-heads';
    lamps.userData.ownedMaterial = true;
    const poolGeometry = new THREE.PlaneGeometry(2, 2, 1, 1);
    poolGeometry.rotateX(-Math.PI / 2);
    const poolMaterial = new THREE.MeshBasicNodeMaterial({
      transparent: true,
      depthWrite: false,
      side: THREE.DoubleSide,
      blending: THREE.AdditiveBlending,
      toneMapped: false,
    });
    const radius = uv().sub(0.5).length().mul(2);
    const radialFalloff = float(0.2).mul(smoothstep(0.12, 1, radius).oneMinus());
    poolMaterial.colorNode = color('#ffd19a');
    poolMaterial.opacityNode = radialFalloff.mul(art.night);
    poolMaterial.userData.streetLightNight = art.night;
    const pools = new THREE.InstancedMesh(poolGeometry, poolMaterial, MAX_LIGHTS);
    pools.name = 'street-lamp-light-pools';
    pools.renderOrder = 2;
    pools.userData.ownedMaterial = true;
    this.meshes = [poles, arms, lamps, pools];
    for (const mesh of this.meshes) {
      mesh.count = 0;
      mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    }
    this.root.add(...this.meshes);
  }
  commit(data: StreetLightingData) {
    if (this.disposed) throw new Error('Cannot commit street lighting after disposal');
    if (
      !Number.isInteger(data.count) ||
      data.count < 0 ||
      data.count > MAX_LIGHTS ||
      data.matrices.some((matrix) => matrix.length !== data.count * 16)
    )
      throw new Error('Invalid street lighting instance data');
    for (let index = 0; index < this.meshes.length; index++) {
      const mesh = this.meshes[index],
        target = mesh.instanceMatrix.array as Float32Array;
      target.set(data.matrices[index], 0);
      mesh.count = data.count;
      mesh.instanceMatrix.needsUpdate = true;
      mesh.computeBoundingSphere();
    }
    this.root.userData.streetLightSources = data.sources;
  }
  reset() {
    if (this.disposed) return;
    for (const mesh of this.meshes) {
      mesh.count = 0;
      mesh.instanceMatrix.needsUpdate = true;
      mesh.computeBoundingSphere();
    }
    this.root.userData.streetLightSources = [];
  }
  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    this.root.removeFromParent();
    for (const mesh of this.meshes) {
      mesh.geometry.dispose();
      if (mesh.userData.ownedMaterial) {
        (mesh.material as THREE.Material).dispose();
        delete mesh.userData.ownedMaterial;
      }
      mesh.dispose();
      mesh.removeFromParent();
    }
    this.root.userData.streetLightSources = [];
  }
}

/** Compatibility builder for isolated previews and existing callers. */
export function buildStreetLighting(
  world: Pick<World, 'nodes' | 'roads'>,
  art: ArtMaterials,
): THREE.Group {
  const data = prepareStreetLighting(world);
  if (!data.count) {
    const empty = new THREE.Group();
    empty.name = 'street-lighting';
    empty.userData.streetLightSources = data.sources;
    return empty;
  }
  const lighting = new PersistentStreetLighting(art);
  lighting.commit(data);
  lighting.root.userData.streetLightingOwner = lighting;
  return lighting.root;
}
