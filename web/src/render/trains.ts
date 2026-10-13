import * as THREE from 'three/webgpu';
import { buildCommuterCoach, buildCommuterDoorLeaf } from './train-kit';
import { COACH_SPACING, RAIL_HEAD_Y } from './rail-profile';
import type { World, RailPlatform } from '../types';
import { Kit } from './kit';
import { TRAIN_DOOR_STRIDE } from '../movement';

/** Bounded shared coach geometry; the engine still owns train movement. */
export class CityTrains {
  readonly root = new THREE.Group();
  readonly batches: { meshes: THREE.InstancedMesh[]; ids: number[] }[] = [];
  readonly lamps: { meshes: THREE.InstancedMesh[]; ids: number[] } = { meshes: [], ids: [] };
  readonly doors: { meshes: THREE.InstancedMesh[]; ids: number[] } = { meshes: [], ids: [] };
  private routeYaw = new Map<number, number>();
  private doorSamples = new Map<number, {from:number;to:number;terminal:number}>();
  private platforms = new Map<number, RailPlatform>();
  private hasDoorMovement = false;
  private pose = new THREE.Object3D();
  constructor(art: Parameters<typeof buildCommuterCoach>[0]) {
    for (const role of ['lead', 'middle', 'tail'] as const) {
      const source = buildCommuterCoach(art, role, false, false), meshes: THREE.InstancedMesh[] = [];
      for (const child of source.children) {
        const authored = child as THREE.Mesh;
        const mesh = new THREE.InstancedMesh(authored.geometry, authored.material, 8);
        mesh.count = 0;
        mesh.castShadow = mesh.receiveShadow = true;
        mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
        meshes.push(mesh);
        this.root.add(mesh);
      }
      this.batches.push({ meshes, ids: [] });
    }
    for (const color of ['#ffcf76', '#e5352f']) {
      const kit = new Kit();
      for (const x of [-.83, .83]) kit.box(x, 1.17, 7.559, .18, .17, .04, art.get(color, .3, true));
      const source = kit.finish(false).children[0] as THREE.Mesh;
      const mesh = new THREE.InstancedMesh(source.geometry, source.material, 8);
      mesh.count = 0;
      mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
      this.lamps.meshes.push(mesh); this.root.add(mesh);
    }
    for (const child of buildCommuterDoorLeaf(art).children) {
      const source = child as THREE.Mesh;
      const mesh = new THREE.InstancedMesh(source.geometry, source.material, 8 * 36);
      mesh.count = 0; mesh.castShadow = mesh.receiveShadow = true;
      mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
      this.doors.meshes.push(mesh); this.root.add(mesh);
    }
  }
  setWorld(world: World) {
    this.routeYaw.clear();
    const nodes = new Map(world.nodes.map(node => [node.id, node]));
    this.platforms = new Map((world.rail_platforms ?? []).map(p => [p.node_id, p]));
    for (const train of world.trains) {
      const a = nodes.get(train.route[0]), b = nodes.get(train.route[1]);
      if (a && b && Math.hypot(b.x - a.x, b.z - a.z) > .01)
        this.routeYaw.set(train.id, Math.atan2(b.x - a.x, b.z - a.z));
      // Bootstrap a paused restore; later world updates must not rewind movement state.
      if (!this.hasDoorMovement) {
        const opening = Math.max(0, Math.min(1, train.door_open_fraction ?? 0));
        this.doorSamples.set(train.id, {from:opening, to:opening, terminal:train.door_terminal ?? -1});
      }
    }
  }
  setMovement(data: Float32Array) {
    const next = new Map<number, {from:number;to:number;terminal:number}>();
    for (let i = 0; i + TRAIN_DOOR_STRIDE <= data.length; i += TRAIN_DOOR_STRIDE) {
      const id = data[i], terminal = data[i + 1], to = Math.max(0, Math.min(1, data[i + 2]));
      const old = this.doorSamples.get(id);
      next.set(id, {from:this.hasDoorMovement && old ? old.to : to, to, terminal:terminal < 0 && old ? old.terminal : terminal});
    }
    this.doorSamples = next;
    this.hasDoorMovement = true;
  }
  begin() { for (const batch of [...this.batches, this.lamps, this.doors]) batch.ids.length = 0; }
  set(index: number, id: number, x: number, z: number, yaw: number, alpha = 1) {
    if (index < 0 || index >= 8) return;
    const bodyYaw = this.routeYaw.get(id) ?? yaw;
    const direction = Math.cos(yaw - bodyYaw) >= 0 ? 1 : -1;
    const sample = this.doorSamples.get(id), platform = sample ? this.platforms.get(sample.terminal) : undefined;
    // Keep doors closed away from the platform, including skipped fast-forward samples.
    const stoppedAtPlatform = platform && Math.hypot(x - platform.x - platform.ux * 23.5, z - platform.z - platform.uz * 23.5) < .03;
    const opening = sample && stoppedAtPlatform ? THREE.MathUtils.lerp(sample.from, sample.to, Math.min(1, Math.max(0, alpha))) : 0;
    const platformSide = platform ? Math.sign(platform.uz * Math.cos(bodyYaw) + platform.ux * Math.sin(bodyYaw)) : 0;
    this.batches.forEach((batch, coach) => {
      const offset = (1 - coach) * COACH_SPACING;
      this.pose.position.set(x + Math.sin(bodyYaw) * offset, RAIL_HEAD_Y, z + Math.cos(bodyYaw) * offset);
      this.pose.rotation.set(0, bodyYaw, 0);
      this.pose.updateMatrix();
      for (const mesh of batch.meshes) mesh.setMatrixAt(index, this.pose.matrix);
      batch.ids[index] = id;
      let leafIndex = index * 36 + coach * 12;
      for (const side of [-1, 1]) for (const doorZ of [-4.5, 0, 4.5]) for (const leaf of [-1, 1]) {
        const along = offset + doorZ + leaf * (.28 + (side === platformSide ? opening * .56 : 0));
        this.pose.position.set(x + Math.sin(bodyYaw) * along + Math.cos(bodyYaw) * side * 1.5,
          RAIL_HEAD_Y, z + Math.cos(bodyYaw) * along - Math.sin(bodyYaw) * side * 1.5);
        this.pose.rotation.set(0, bodyYaw + (side < 0 ? Math.PI : 0), 0); this.pose.updateMatrix();
        for (const mesh of this.doors.meshes) mesh.setMatrixAt(leafIndex, this.pose.matrix);
        this.doors.ids[leafIndex++] = id;
      }
    });
    this.lamps.meshes.forEach((mesh, lamp) => {
      const end = lamp === 0 ? direction : -direction;
      this.pose.position.set(x + Math.sin(bodyYaw) * end * COACH_SPACING, RAIL_HEAD_Y, z + Math.cos(bodyYaw) * end * COACH_SPACING);
      this.pose.rotation.set(0, bodyYaw + (end < 0 ? Math.PI : 0), 0);
      this.pose.updateMatrix(); mesh.setMatrixAt(index, this.pose.matrix);
    });
    this.lamps.ids[index] = id;
  }
  finish(count: number) {
    for (const batch of [...this.batches, this.lamps]) for (const mesh of batch.meshes) {
      mesh.count = Math.max(0, Math.min(count, 8));
      mesh.instanceMatrix.needsUpdate = true;
      mesh.computeBoundingSphere();
    }
    for (const mesh of this.doors.meshes) {
      mesh.count = Math.max(0, Math.min(count, 8)) * 36;
      mesh.instanceMatrix.needsUpdate = true; mesh.computeBoundingSphere();
    }
  }
  reset() { this.routeYaw.clear(); this.doorSamples.clear(); this.platforms.clear(); this.hasDoorMovement = false; this.begin(); this.finish(0); }
  dispose() {
    this.root.removeFromParent();
    for (const batch of [...this.batches, this.lamps, this.doors]) for (const mesh of batch.meshes) {
      mesh.geometry.dispose(); mesh.dispose();
    }
    // Materials belong to the shared ArtMaterials cache.
  }
}
