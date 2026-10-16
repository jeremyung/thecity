import * as THREE from 'three/webgpu';
import { interleaveAttributes } from 'three/addons/utils/BufferGeometryUtils.js';
import { float, smoothstep, uv, vec2 } from 'three/tsl';
import type { ArtMaterials } from './materials';
import { trainMaterial } from './train-materials';
import {
  buildTrainCar,
  buildTrainDoors,
  consistCars,
  consistHalfLength,
  TRAIN_CLASS,
  trainClass,
  type TrainCar,
} from './train-kit';
import { VehicleMotion } from './vehicles';
import { TrainDoorState } from './train-door-state';
import { TrainPlacement } from './train-placement';
import type { World } from '../types';

/** Bind moving actors to infrastructure that has actually reached the GPU. */
export function trainWorldWithInfrastructure(world: World, committed: World | null): World {
  return {
    ...world,
    nodes: committed?.nodes ?? [],
    roads: committed?.roads ?? [],
    rail_platforms: committed?.rail_platforms ?? [],
    rail_alignments: committed?.rail_alignments ?? [],
  };
}

/** Half the consist length; mirrors engine/src/lib.rs TRAIN_HALF_LENGTH. */
export const TRAIN_HALF_LENGTH = consistHalfLength();

interface Slot {
  interior: THREE.InstancedMesh;
  doors: THREE.InstancedMesh;
  opening: THREE.InstancedBufferAttribute;
  mesh: THREE.InstancedMesh;
  state: THREE.InstancedBufferAttribute;
  facing: THREE.InstancedBufferAttribute;
  ids: number[];
  count: number;
  capacity: number;
}

function instanced(geometry: THREE.BufferGeometry, material: THREE.Material, capacity: number) {
  const mesh = new THREE.InstancedMesh(geometry, material, capacity);
  mesh.count = 0;
  mesh.castShadow = true;
  mesh.receiveShadow = true;
  mesh.frustumCulled = false;
  mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
  return mesh;
}

/** WebGPU guarantees eight vertex buffers. Pack immutable mesh attributes
 * together before attaching per-instance state, facing and door movement. */
function packStaticAttributes(geometry: THREE.BufferGeometry) {
  const names = Object.keys(geometry.attributes).filter(
    (name) => !(geometry.getAttribute(name) instanceof THREE.InstancedBufferAttribute),
  );
  const packed = interleaveAttributes(
    names.map((name) => geometry.getAttribute(name) as THREE.BufferAttribute),
  );
  // Three returns an array; the installed declaration incorrectly says a single attribute.
  if (!Array.isArray(packed))
    throw new Error('Train static attributes must share a component type');
  names.forEach((name, i) => geometry.setAttribute(name, packed[i]));
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
  private placement = new TrainPlacement();
  private doorState = new TrainDoorState();

  setWorld(world: World) {
    this.placement.setWorld(world);
    this.doorState.setWorld(world);
  }
  setMovement(data: Float32Array) {
    this.doorState.setMovement(data);
  }

  constructor(
    private art: ArtMaterials,
    readonly capacity = 8,
  ) {
    const material = trainMaterial(art);
    const roles: TrainCar[] = ['cab', 'trailer', 'motor'];
    for (const role of roles) {
      const roleCapacity = this.cars.filter((car) => car.role === role).length * capacity;
      if (roleCapacity === 0) continue;
      const geometry = buildTrainCar(TRAIN_CLASS, role, true);
      const mesh = instanced(geometry, material, roleCapacity);
      const state = new THREE.InstancedBufferAttribute(new Float32Array(roleCapacity * 4), 4);
      state.setUsage(THREE.DynamicDrawUsage);
      geometry.setAttribute('vstate', state);
      const facing = new THREE.InstancedBufferAttribute(new Float32Array(roleCapacity), 1);
      facing.setUsage(THREE.DynamicDrawUsage);
      geometry.setAttribute('trainFacing', facing);
      const opening = new THREE.InstancedBufferAttribute(new Float32Array(roleCapacity), 1);
      opening.setUsage(THREE.DynamicDrawUsage);
      geometry.setAttribute('doorOpening', opening);
      geometry.setAttribute(
        'doorTravel',
        new THREE.BufferAttribute(new Float32Array(geometry.getAttribute('position').count * 2), 2),
      );
      packStaticAttributes(geometry);
      const doorGeometry = buildTrainDoors(TRAIN_CLASS, role);
      packStaticAttributes(doorGeometry);
      doorGeometry.setAttribute('vstate', state);
      doorGeometry.setAttribute('trainFacing', facing);
      doorGeometry.setAttribute('doorOpening', opening);
      const doors = instanced(doorGeometry, material, roleCapacity);
      // A real floor under the aperture prevents open doors revealing track below.
      const spec = trainClass();
      const floor = new THREE.BoxGeometry(spec.width - 0.12, 0.08, spec.length - 0.55);
      floor.translate(0, 1.065 - 0.04, 0);
      const interior = instanced(floor, art.get('#555450', 0.86), roleCapacity);
      this.root.add(interior);
      this.root.add(doors);
      this.slots.set(role, {
        mesh,
        doors,
        interior,
        opening,
        state,
        facing,
        ids: [],
        count: 0,
        capacity: roleCapacity,
      });
      this.root.add(mesh);
    }
    const plane = new THREE.PlaneGeometry(1, 1);
    plane.rotateX(-Math.PI / 2);
    const shade = new THREE.MeshBasicNodeMaterial({
      color: '#1d2526',
      transparent: true,
      depthWrite: false,
    });
    const d = uv().sub(vec2(0.5, 0.5)).mul(vec2(2, 2)).length();
    shade.opacityNode = smoothstep(float(1), float(0.35), d).mul(0.42).sub(art.night.mul(0.12));
    this.shadowCapacity = this.cars.length * capacity;
    this.shadows = new THREE.InstancedMesh(plane, shade, this.shadowCapacity);
    this.shadows.count = 0;
    this.shadows.frustumCulled = false;
    this.shadows.renderOrder = 1;
    this.shadows.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    this.root.add(this.shadows);
  }

  get pickable() {
    return [...this.slots.values()].flatMap((s) => [{ mesh: s.mesh, ids: s.ids }]);
  }
  get doorMeshes() {
    return [...this.slots.values()].map((s) => s.doors);
  }
  get meshes() {
    return [...this.slots.values()].map((s) => s.mesh);
  }

  begin() {
    this.motion.step();
    for (const s of this.slots.values()) {
      s.count = 0;
      s.ids.length = 0;
    }
    this.shadowCount = 0;
  }

  /** Place one consist at its engine centre pose. */
  add(
    id: number,
    x: number,
    z: number,
    yaw: number,
    lift: number,
    pitch: number,
    dt: number,
    alpha = 1,
  ) {
    const spec = trainClass();
    const center = this.placement.center(id, x, z, yaw);
    const bodyYaw = this.placement.at(center, 0).yaw;
    const direction = Math.cos(yaw - bodyYaw) >= 0 ? 1 : -1;
    const m = this.motion.update(id, x, z, yaw, spec.length * 2, dt);
    for (const car of this.cars) {
      const slot = this.slots.get(car.role)!;
      if (slot.count >= slot.capacity) continue;
      const i = slot.count++;
      const body = this.placement.at(center, car.offset);
      const cx = body.x,
        cz = body.z;
      const carYaw = body.yaw + (car.trailing ? Math.PI : 0);
      this.pose.position.set(cx, lift, cz);
      this.pose.rotation.set(-pitch, carYaw, 0, 'YXZ');
      this.pose.scale.setScalar(1);
      this.pose.updateMatrix();
      slot.mesh.setMatrixAt(i, this.pose.matrix);
      slot.doors.setMatrixAt(i, this.pose.matrix);
      slot.interior.setMatrixAt(i, this.pose.matrix);
      slot.opening.setX(i, this.doorState.opening(id, x, z, carYaw, alpha));
      slot.state.setXYZW(i, m.odo, 0, carYaw, m.brake);
      slot.facing.setX(i, direction * (car.trailing ? -1 : 1));
      slot.ids[i] = id;
      if (this.shadowCount < this.shadowCapacity) {
        this.pose.position.y = lift + 0.02;
        this.pose.scale.set(spec.width + 0.4, 1, spec.length + 0.4);
        this.pose.updateMatrix();
        this.shadows.setMatrixAt(this.shadowCount++, this.pose.matrix);
      }
    }
  }

  finish(_count: number) {
    for (const s of this.slots.values()) {
      s.interior.count = s.count;
      s.interior.instanceMatrix.needsUpdate = true;
      s.doors.count = s.count;
      s.doors.instanceMatrix.needsUpdate = true;
      s.opening.needsUpdate = true;
      s.mesh.count = s.count;
      s.mesh.instanceMatrix.needsUpdate = true;
      s.state.needsUpdate = true;
      s.facing.needsUpdate = true;
    }
    this.shadows.count = this.shadowCount;
    this.shadows.instanceMatrix.needsUpdate = true;
  }

  reset() {
    this.begin();
    this.finish(0);
    this.motion.clear();
    this.placement.reset();
    this.doorState.reset();
  }

  dispose() {
    this.root.removeFromParent();
    for (const s of this.slots.values()) {
      s.mesh.geometry.dispose();
      s.mesh.dispose();
      s.doors.geometry.dispose();
      s.doors.dispose();
      s.interior.geometry.dispose();
      s.interior.dispose();
    }
    this.shadows.geometry.dispose();
    (this.shadows.material as THREE.Material).dispose();
    this.shadows.dispose();
  }
}
