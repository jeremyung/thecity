import * as THREE from 'three/webgpu';
import { float, smoothstep, uv, vec2 } from 'three/tsl';
import type { ArtMaterials } from './materials';
import { trainMaterial } from './train-materials';
import { buildTrainCar, consistCars, consistHalfLength, TRAIN_CLASS, trainClass, type TrainCar } from './train-kit';
import { VehicleMotion } from './vehicles';

/** Half the consist length; mirrors engine/src/lib.rs TRAIN_HALF_LENGTH. */
export const TRAIN_HALF_LENGTH = consistHalfLength();

interface Slot { mesh: THREE.InstancedMesh; state: THREE.InstancedBufferAttribute; ids: number[]; count: number; capacity: number }

function instanced(geometry: THREE.BufferGeometry, material: THREE.Material, capacity: number) {
  const mesh = new THREE.InstancedMesh(geometry, material, capacity);
  mesh.count = 0; mesh.castShadow = true; mesh.receiveShadow = true; mesh.frustumCulled = false;
  mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
  return mesh;
}

/**
 * Instanced train consists. The engine reports one movement point per train
 * (kind 2); the fleet expands it into the manifest's cars, places the leading
 * cab at +offset along the heading and mirrors the trailing cab, and rolls every
 * wheel. Each role's instance buffer is sized by how often it appears in the
 * consist, so a cab is not capped at one per train.
 */
export class TrainFleet {
  readonly root = new THREE.Group();
  private slots = new Map<TrainCar, Slot>();
  private shadows: THREE.InstancedMesh;
  private shadowCapacity: number;
  private shadowCount = 0;
  private matrix = new THREE.Matrix4();
  private pose = new THREE.Object3D();
  private motion = new VehicleMotion();
  private cars = consistCars();

  constructor(private art: ArtMaterials, readonly capacity = 8) {
    const material = trainMaterial(art);
    const roles: TrainCar[] = ['cab', 'trailer', 'motor'];
    for (const role of roles) {
      const roleCapacity = this.cars.filter(car => car.role === role).length * capacity;
      if (roleCapacity === 0) continue;
      const geometry = buildTrainCar(TRAIN_CLASS, role);
      const mesh = instanced(geometry, material, roleCapacity);
      const state = new THREE.InstancedBufferAttribute(new Float32Array(roleCapacity * 4), 4);
      state.setUsage(THREE.DynamicDrawUsage);
      geometry.setAttribute('vstate', state);
      this.slots.set(role, { mesh, state, ids: [], count: 0, capacity: roleCapacity });
      this.root.add(mesh);
    }
    const plane = new THREE.PlaneGeometry(1, 1); plane.rotateX(-Math.PI / 2);
    const shade = new THREE.MeshBasicNodeMaterial({ color: '#1d2526', transparent: true, depthWrite: false });
    const d = uv().sub(vec2(.5, .5)).mul(vec2(2, 2)).length();
    shade.opacityNode = smoothstep(float(1), float(.35), d).mul(.42).sub(art.night.mul(.12));
    this.shadowCapacity = this.cars.length * capacity;
    this.shadows = new THREE.InstancedMesh(plane, shade, this.shadowCapacity);
    this.shadows.count = 0; this.shadows.frustumCulled = false; this.shadows.renderOrder = 1;
    this.shadows.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    this.root.add(this.shadows);
  }

  get pickable() { return [...this.slots.values()].flatMap(s => [{ mesh: s.mesh, ids: s.ids }]); }
  get meshes() { return [...this.slots.values()].map(s => s.mesh); }

  begin() { this.motion.step(); for (const s of this.slots.values()) { s.count = 0; s.ids.length = 0; } this.shadowCount = 0; }

  /** Place one consist at its engine centre pose. */
  add(id: number, x: number, z: number, yaw: number, lift: number, pitch: number, dt: number) {
    const spec = trainClass();
    const forwardX = Math.sin(yaw), forwardZ = Math.cos(yaw);
    const m = this.motion.update(id, x, z, yaw, spec.length * 2, dt);
    for (const car of this.cars) {
      const slot = this.slots.get(car.role)!;
      if (slot.count >= slot.capacity) continue;
      const i = slot.count++;
      const cx = x + forwardX * car.offset, cz = z + forwardZ * car.offset;
      const carYaw = car.trailing ? yaw + Math.PI : yaw;
      this.pose.position.set(cx, lift, cz);
      this.pose.rotation.set(-pitch, carYaw, 0, 'YXZ');
      this.pose.scale.setScalar(1);
      this.pose.updateMatrix();
      slot.mesh.setMatrixAt(i, this.pose.matrix);
      slot.state.setXYZW(i, m.odo, 0, carYaw, m.brake);
      slot.ids[i] = id;
      if (this.shadowCount < this.shadowCapacity) {
        this.pose.position.y = lift + .02; this.pose.scale.set(spec.width + .4, 1, spec.length + .4); this.pose.updateMatrix();
        this.shadows.setMatrixAt(this.shadowCount++, this.pose.matrix);
      }
    }
  }

  finish(_count: number) {
    for (const s of this.slots.values()) {
      s.mesh.count = s.count; s.mesh.instanceMatrix.needsUpdate = true; s.state.needsUpdate = true;
    }
    this.shadows.count = this.shadowCount; this.shadows.instanceMatrix.needsUpdate = true;
  }

  reset() { this.begin(); this.finish(0); this.motion.clear(); }

  dispose() {
    this.root.removeFromParent();
    for (const s of this.slots.values()) { s.mesh.geometry.dispose(); s.mesh.dispose(); }
    this.shadows.geometry.dispose(); (this.shadows.material as THREE.Material).dispose(); this.shadows.dispose();
  }
}
