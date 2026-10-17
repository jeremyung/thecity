import * as THREE from 'three/webgpu';
import { Kit, disposeGroup } from './kit';
import { buildVehicleGeometry, vehicleSpec } from './vehicle-kit';
import { vehicleMaterial } from './vehicle-materials';
import { VehicleMotion } from './vehicles';
import { ArtMaterials, signTexture } from './materials';
import type { World } from '../types';

/** Kerb-side (+x) doorways on the bus elevation (metres, body space). */
const BUS_DOORS = [
  { z0: 3.25, z1: 4.15, y0: 0.29, y1: 2.06 },
  { z0: -0.86, z1: 0.22, y0: 0.29, y1: 2.06 },
];

/** Two shared city buses; simulation owns routes, passengers and stop placement. */
export class CityBuses {
  readonly root = new THREE.Group();
  readonly meshes: THREE.InstancedMesh[] = [];
  readonly ids: number[] = [];
  private stops = new THREE.Group();
  private stopSignature = '';
  private doorLight: THREE.InstancedMesh;
  private display: THREE.Mesh[] = [];
  private displayText = ['', ''];
  private matrix = new THREE.Matrix4();
  private pose = new THREE.Object3D();
  private dwellCount = 0;
  private state: THREE.InstancedBufferAttribute;
  private motion = new VehicleMotion();
  private wheelbase: number;
  private length: number;
  private dt = 1 / 60;
  private lastBegin = 0;
  constructor(
    private art: ArtMaterials,
    private retire: (release: () => void) => void,
  ) {
    // Body traced from the generated elevations: kerb-side doors on -x only.
    const geometry = buildVehicleGeometry('bus'),
      spec = vehicleSpec('bus');
    this.length = spec.length;
    this.wheelbase = Math.abs(spec.left.wheels[0].z - spec.left.wheels[1].z);
    this.state = new THREE.InstancedBufferAttribute(new Float32Array(8), 4);
    this.state.setUsage(THREE.DynamicDrawUsage);
    geometry.setAttribute('vstate', this.state);
    // The livery is fixed: tint white leaves the elevation's cream and green as painted.
    const tint = new THREE.InstancedBufferAttribute(
      new Float32Array([0.8, 0.8, 0.8, 0.8, 0.8, 0.8]),
      3,
    );
    geometry.setAttribute('vtint', tint);
    const mesh = new THREE.InstancedMesh(geometry, vehicleMaterial(art), 2);
    mesh.count = 0;
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    this.meshes.push(mesh);
    this.root.add(mesh);
    // Lit doorways replace the closed doors while dwelling: one per kerb-side door.
    this.doorLight = new THREE.InstancedMesh(
      new THREE.BoxGeometry(0.028, 1, 1),
      art.get('#ebc384', 0.5, true),
      4,
    );
    this.doorLight.count = 0;
    this.root.add(this.doorLight, this.stops);
    for (let i = 0; i < 2; i++) {
      const display = new THREE.Mesh(
        new THREE.PlaneGeometry(1.72, 0.38),
        new THREE.MeshBasicNodeMaterial({ color: '#d9d8ae' }),
      );
      display.visible = false;
      this.display.push(display);
      this.root.add(display);
    }
  }
  setWorld(world: World) {
    const seen = new Set<string>();
    const markers = (world.bus_lines ?? [])
      .filter((line) => line.active)
      .flatMap((line) => line.stop_positions ?? [])
      .filter((stop) => {
        // Terminal directions can share one physical pole; avoid coincident faces.
        const key = `${stop.node_id}:${stop.x.toFixed(3)}:${stop.z.toFixed(3)}`;
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      })
      .slice(0, 10);
    const signature = JSON.stringify(markers);
    if (signature !== this.stopSignature) {
      this.stopSignature = signature;
      const previous = this.stops;
      previous.removeFromParent();
      this.retire(() => disposeGroup(previous));
      const kit = new Kit();
      for (const stop of markers) {
        kit.cylinder(stop.x, 1.5, stop.z, 0.065, 3, this.art.get('#60746b'), 8);
        kit.box(
          stop.x,
          2.8,
          stop.z,
          0.62,
          0.66,
          0.12,
          this.art.get('#e4debf'),
          Math.PI / 2 - stop.heading,
        );
        kit.box(
          stop.x,
          2.82,
          stop.z,
          0.42,
          0.15,
          0.14,
          this.art.get('#456f65'),
          Math.PI / 2 - stop.heading,
        );
      }
      this.stops = kit.finish();
      this.root.add(this.stops);
    }
  }
  begin(now = performance.now()) {
    this.dt = this.lastBegin ? Math.min(0.1, (now - this.lastBegin) / 1000) : 1 / 60;
    this.lastBegin = now;
    this.motion.step();
    this.ids.length = 0;
    this.dwellCount = 0;
  }
  set(
    index: number,
    id: number,
    x: number,
    z: number,
    yaw: number,
    world: World | null,
    lift = 0,
    pitch = 0,
  ) {
    if (index >= 2) return;
    this.ids[index] = id;
    this.pose.position.set(x, lift, z);
    this.pose.rotation.set(-pitch, yaw, 0, 'YXZ');
    this.pose.scale.setScalar(1);
    this.pose.updateMatrix();
    this.matrix.copy(this.pose.matrix);
    for (const mesh of this.meshes) mesh.setMatrixAt(index, this.matrix);
    const m = this.motion.update(id, x, z, yaw, this.wheelbase, this.dt);
    this.state.setXYZW(index, m.odo, 0, yaw, m.brake);
    const bus = world?.buses?.find((b) => b.id === id);
    if (bus?.dwelling)
      for (const door of BUS_DOORS) {
        if (this.dwellCount >= 4) break;
        this.pose.position.set(x, lift, z);
        this.pose.rotation.set(-pitch, yaw, 0, 'YXZ');
        this.pose.updateMatrix();
        this.pose.translateX(1.165);
        this.pose.translateY((door.y0 + door.y1) / 2);
        this.pose.translateZ((door.z0 + door.z1) / 2);
        this.pose.scale.set(1, door.y1 - door.y0, door.z1 - door.z0);
        this.pose.updateMatrix();
        this.doorLight.setMatrixAt(this.dwellCount++, this.pose.matrix);
        this.pose.scale.setScalar(1);
      }
    const destination = world?.nodes.find((node) => node.id === bus?.next_stop);
    const label =
      destination?.kind === 'station'
        ? '駅前'
        : destination?.kind === 'market'
          ? '青葉通り'
          : destination
            ? '住宅街'
            : 'まちバス';
    const display = this.display[index];
    if (this.displayText[index] !== label) {
      this.displayText[index] = label;
      const mat = display.material as THREE.MeshBasicNodeMaterial,
        old = mat.map;
      mat.map = signTexture(label, '#263e39', '#eadba2');
      mat.needsUpdate = true;
      if (old) this.retire(() => old.dispose());
    }
    display.visible = true;
    display.position.set(0, 2.18, this.length / 2 + 0.03).applyMatrix4(this.matrix);
    display.quaternion.setFromRotationMatrix(this.matrix);
  }
  finish(count: number) {
    for (const mesh of this.meshes) {
      mesh.count = Math.min(count, 2);
      mesh.instanceMatrix.needsUpdate = true;
      mesh.computeBoundingSphere();
    }
    this.state.needsUpdate = true;
    this.doorLight.count = this.dwellCount;
    this.doorLight.instanceMatrix.needsUpdate = true;
    this.doorLight.computeBoundingSphere();
    for (let i = count; i < 2; i++) this.display[i].visible = false;
  }
  reset() {
    this.begin();
    this.finish(0);
    this.motion.clear();
    this.stopSignature = '';
    const old = this.stops;
    old.removeFromParent();
    this.retire(() => disposeGroup(old));
    this.stops = new THREE.Group();
    this.root.add(this.stops);
  }
  dispose() {
    this.root.removeFromParent();
    this.reset();
    for (const mesh of [...this.meshes, this.doorLight]) {
      mesh.geometry.dispose();
      mesh.dispose();
    }
    for (const display of this.display) {
      display.geometry.dispose();
      const mat = display.material as THREE.MeshBasicNodeMaterial;
      mat.map?.dispose();
      mat.dispose();
    }
  }
}
