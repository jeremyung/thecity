import * as THREE from 'three/webgpu';
import { float, smoothstep, uv, vec2 } from 'three/tsl';
import type { ArtMaterials } from './materials';
import {
  buildLiveryGeometry,
  buildVehicleGeometry,
  vehicleSpec,
  type LiveryPanel,
  type VehicleBody,
} from './vehicle-kit';
import { LIVERY_INDEX, liveryMaterial, vehicleMaterial, type Livery } from './vehicle-materials';

/**
 * Movement-buffer kinds for road vehicles. Kind 1 stays the private compact so
 * older traces and tools keep working; the engine emits the rest by trip type
 * and spaces queues by the class length. Each class has several bodies of
 * that length so traffic does not repeat one silhouette.
 */
export const VEHICLE_KINDS = { compact: 1, kei: 4, taxi: 5, van: 6, truck: 7, mail: 8 } as const;
const KIND_VARIANTS: Record<number, { body: VehicleBody; weight: number }[]> = {
  1: [
    { body: 'compact', weight: 45 },
    { body: 'compact-crossover', weight: 30 },
    { body: 'compact-minivan', weight: 25 },
  ],
  4: [
    { body: 'kei', weight: 50 },
    { body: 'kei-hatch', weight: 35 },
    { body: 'kei-truck', weight: 15 },
  ],
  5: [
    { body: 'taxi', weight: 65 },
    { body: 'taxi-classic', weight: 35 },
  ],
  6: [{ body: 'van', weight: 1 }],
  7: [{ body: 'truck', weight: 1 }],
  // Mail runs in the tall kei wagon: the livery panel is drawn for that body.
  8: [{ body: 'kei', weight: 1 }],
};
export function isRoadVehicle(kind: number) {
  return kind in KIND_VARIANTS;
}
/** The class's primary body. */
export function bodyForKind(kind: number): VehicleBody | null {
  return KIND_VARIANTS[kind]?.[0].body ?? null;
}

/** Stable per-vehicle random stream: the same car looks the same all trip. */
function vehicleRandom(id: number, salt: number) {
  // Weyl counter through the murmur3 finaliser: consecutive ids spread evenly.
  let state = (Math.imul(id, 0x9e3779b1) + Math.imul(salt, 0x7f4a7c15)) >>> 0;
  return () => {
    state = (state + 0x9e3779b9) >>> 0;
    let h = state;
    h ^= h >>> 16;
    h = Math.imul(h, 0x85ebca6b);
    h ^= h >>> 13;
    h = Math.imul(h, 0xc2b2ae35);
    h ^= h >>> 16;
    return (h >>> 0) / 4294967296;
  };
}
function weighted<T extends { weight: number }>(list: T[], r: number) {
  let t = r * list.reduce((sum, item) => sum + item.weight, 0);
  for (const item of list) {
    t -= item.weight;
    if (t < 0) return item;
  }
  return list[list.length - 1];
}
export function vehicleBody(kind: number, id: number): VehicleBody | null {
  const variants = KIND_VARIANTS[kind];
  return variants ? weighted(variants, vehicleRandom(id, 1)()).body : null;
}

/** Road surface top: local road slabs are 0.3 m deep centred at 0.15. */
export const ROAD_SURFACE_Y = 0.3;

/**
 * Private paint, weighted roughly like Tokyo streets: mostly pearl white,
 * black and silver, a minority of colour. Kei cars run softer pastels.
 */
const PRIVATE_PAINT = {
  car: [
    ['#f1eee6', 26],
    ['#2a2e31', 18],
    ['#c7cacd', 16],
    ['#7d8287', 9],
    ['#3b5874', 7],
    ['#8d3c37', 6],
    ['#d8cdb5', 6],
    ['#5c7a72', 5],
    ['#4a3f38', 4],
    ['#b9a77e', 3],
  ],
  kei: [
    ['#f1ede2', 22],
    ['#e6dcc0', 13],
    ['#b8d1c3', 12],
    ['#efc8b4', 10],
    ['#8ea8bd', 9],
    ['#c7cacd', 9],
    ['#2d3134', 8],
    ['#f3e3a1', 6],
    ['#a8574c', 6],
    ['#6e8a5c', 5],
  ],
  work: [
    ['#f2f2ec', 70],
    ['#c7cacd', 20],
    ['#9fb7c6', 10],
  ],
} as const;
const PAINT_SET: Partial<Record<VehicleBody, keyof typeof PRIVATE_PAINT>> = {
  kei: 'kei',
  'kei-hatch': 'kei',
  'kei-truck': 'work',
};
const FLEET_LIVERY: Partial<Record<number, { livery: Livery; paint: string; weight: number }[]>> = {
  5: [
    { livery: 'まちタクシー', paint: '#1c2023', weight: 35 },
    { livery: '青葉交通', paint: '#1f2c52', weight: 30 },
    { livery: 'みどり交通', paint: '#2f6a4e', weight: 20 },
    { livery: 'さくら無線', paint: '#efe9dc', weight: 15 },
  ],
  6: [
    { livery: 'ひだまり便', paint: '#f3f3ef', weight: 30 },
    { livery: 'つばめ急便', paint: '#f3f3ef', weight: 30 },
    { livery: '白鳥クリーニング', paint: '#eef3f5', weight: 20 },
    { livery: 'まちの八百屋', paint: '#f1efe4', weight: 20 },
  ],
  7: [
    { livery: 'あおば運輸', paint: '#f2f2ec', weight: 30 },
    { livery: 'やまびこ引越センター', paint: '#f2f2ec', weight: 25 },
    { livery: 'ふじ冷凍便', paint: '#a7c8dc', weight: 25 },
    { livery: 'こばやし建材', paint: '#e2b93b', weight: 20 },
  ],
  8: [{ livery: 'まち郵便', paint: '#c3372c', weight: 1 }],
};
/** Livery panels in body space, all 2:1 to match the livery tiles. */
const PANELS: Partial<Record<VehicleBody, LiveryPanel>> = {
  taxi: { z0: 0.15, z1: 1.05, y0: 0.6, y1: 1.05 },
  'taxi-classic': { z0: 0.1, z1: 1.0, y0: 0.5, y1: 0.95 },
  van: { z0: -2.05, z1: 0.35, y0: 0.5, y1: 1.7 },
  truck: { z0: -2.2, z1: 0.6, y0: 0.95, y1: 2.35 },
  kei: { z0: -1.35, z1: 0.85, y0: 0.42, y1: 1.52 },
};

export interface VehicleLook {
  body: VehicleBody;
  paint: THREE.Color;
  livery: number;
  width: number;
}
/** Deterministic per-vehicle appearance: body, paint, livery and a little build variation. */
export function vehicleLook(kind: number, id: number): VehicleLook {
  const body = vehicleBody(kind, id) ?? 'compact',
    random = vehicleRandom(id, 2);
  const fleet = FLEET_LIVERY[kind];
  let paint: THREE.Color,
    livery = -1;
  if (fleet) {
    const f = weighted(fleet, random());
    paint = new THREE.Color(f.paint);
    livery = LIVERY_INDEX[f.livery];
  } else {
    const set = PRIVATE_PAINT[PAINT_SET[body] ?? 'car'].map(([hex, weight]) => ({ hex, weight }));
    paint = new THREE.Color(weighted(set, random()).hex);
  }
  // Age, wax and batch: a few percent of shade and saturation, never a new colour.
  const hsl = paint.getHSL({ h: 0, s: 0, l: 0 });
  paint.setHSL(
    hsl.h + (random() - 0.5) * 0.02,
    THREE.MathUtils.clamp(hsl.s * (0.85 + random() * 0.3), 0, 1),
    THREE.MathUtils.clamp(hsl.l * (0.93 + random() * 0.12), 0, 1),
  );
  return { body, paint, livery, width: 0.97 + random() * 0.06 };
}

/** Odometer, steer and brake tracking from interpolated poses. */
export class VehicleMotion {
  private state = new Map<
    number,
    {
      x: number;
      z: number;
      yaw: number;
      odo: number;
      speed: number;
      steer: number;
      brake: number;
      frame: number;
    }
  >();
  private frame = 0;
  step() {
    this.frame++;
    if (this.frame % 240 === 0)
      for (const [id, s] of this.state) if (this.frame - s.frame > 120) this.state.delete(id);
  }
  /** Returns (odometer, steer, brake) after advancing to this pose. */
  update(id: number, x: number, z: number, yaw: number, wheelbase: number, dt: number) {
    let s = this.state.get(id);
    if (!s) {
      s = { x, z, yaw, odo: (id % 97) * 0.37, speed: 0, steer: 0, brake: 0, frame: this.frame };
      this.state.set(id, s);
    }
    const travel = Math.hypot(x - s.x, z - s.z);
    // Reverse is never simulated; large jumps are teleports (loads, reroutes).
    const step = travel < 6 ? travel : 0,
      safe = Math.max(dt, 1e-3);
    const speed = step / safe,
      accel = (speed - s.speed) / safe;
    const turn = Math.atan2(Math.sin(yaw - s.yaw), Math.cos(yaw - s.yaw)) / safe;
    // Bicycle model: tan(steer) = wheelbase * yaw rate / speed.
    const steer =
      speed > 0.3
        ? THREE.MathUtils.clamp(Math.atan((wheelbase * turn) / speed), -0.55, 0.55)
        : s.steer;
    const k = 1 - Math.exp(-safe * 6);
    s.steer += (steer - s.steer) * k;
    s.speed += (speed - s.speed) * k;
    const braking = accel < -1.2 || s.speed < 0.25 ? 1 : 0;
    s.brake += (braking - s.brake) * (1 - Math.exp(-safe * 8));
    s.odo += step;
    s.x = x;
    s.z = z;
    s.yaw = yaw;
    s.frame = this.frame;
    return s;
  }
  clear() {
    this.state.clear();
  }
}

interface Slot {
  mesh: THREE.InstancedMesh;
  livery: THREE.InstancedMesh | null;
  state: THREE.InstancedBufferAttribute;
  tint: THREE.InstancedBufferAttribute;
  liveryIndex: THREE.InstancedBufferAttribute | null;
  ids: number[];
  count: number;
  wheelbase: number;
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
function attribute(geometry: THREE.BufferGeometry, name: string, size: number, capacity: number) {
  const a = new THREE.InstancedBufferAttribute(new Float32Array(capacity * size), size);
  a.setUsage(THREE.DynamicDrawUsage);
  geometry.setAttribute(name, a);
  return a;
}

/** Instanced road traffic: one body mesh per vehicle type, liveries and contact shadows. */
export class VehicleFleet {
  readonly root = new THREE.Group();
  readonly motion = new VehicleMotion();
  private slots = new Map<VehicleBody, Slot>();
  private shadows: THREE.InstancedMesh;
  private shadowCount = 0;
  private matrix = new THREE.Matrix4();
  private pose = new THREE.Object3D();
  constructor(
    private art: ArtMaterials,
    readonly capacity = 1000,
  ) {
    const body = vehicleMaterial(art),
      livery = liveryMaterial(art);
    const bodies = [
      ...new Set(Object.values(KIND_VARIANTS).flatMap((list) => list.map((v) => v.body))),
    ];
    for (const name of bodies) {
      const geometry = buildVehicleGeometry(name);
      const mesh = instanced(geometry, body, capacity);
      const state = attribute(geometry, 'vstate', 4, capacity),
        tint = attribute(geometry, 'vtint', 3, capacity);
      const panel = PANELS[name];
      let liveryMesh: THREE.InstancedMesh | null = null,
        liveryIndex: THREE.InstancedBufferAttribute | null = null;
      if (panel) {
        const g = buildLiveryGeometry(name, panel);
        liveryMesh = instanced(g, livery, capacity);
        liveryMesh.castShadow = false;
        liveryIndex = attribute(g, 'vlivery', 1, capacity);
        this.root.add(liveryMesh);
      }
      const wheels = vehicleSpec(name).left.wheels;
      this.slots.set(name, {
        mesh,
        livery: liveryMesh,
        state,
        tint,
        liveryIndex,
        ids: [],
        count: 0,
        wheelbase: Math.abs(wheels[0].z - wheels[1].z),
      });
      this.root.add(mesh);
    }
    // Soft contact shadow: the miniature's strongest grounding cue, one draw for all.
    const plane = new THREE.PlaneGeometry(1, 1);
    plane.rotateX(-Math.PI / 2);
    const shade = new THREE.MeshBasicNodeMaterial({
      color: '#1d2526',
      transparent: true,
      depthWrite: false,
    });
    const d = uv().sub(vec2(0.5, 0.5)).mul(vec2(2, 2)).length();
    shade.opacityNode = smoothstep(float(1), float(0.35), d).mul(0.42).sub(art.night.mul(0.12));
    this.shadows = new THREE.InstancedMesh(plane, shade, capacity);
    this.shadows.count = 0;
    this.shadows.frustumCulled = false;
    this.shadows.renderOrder = 1;
    this.shadows.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    this.root.add(this.shadows);
  }
  /** Picking: each body mesh with the actor ids of its instances. */
  get pickable() {
    return [...this.slots.values()].flatMap((s) => [
      { mesh: s.mesh, ids: s.ids },
      ...(s.livery ? [{ mesh: s.livery, ids: s.ids }] : []),
    ]);
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
  /** Place one vehicle; yaw is the renderer heading (rotation.y), lift the bridge/road offset. */
  add(
    id: number,
    kind: number,
    x: number,
    z: number,
    yaw: number,
    lift: number,
    pitch: number,
    dt: number,
  ) {
    if (!isRoadVehicle(kind)) return false;
    const look = vehicleLook(kind, id),
      body = look.body,
      slot = this.slots.get(body)!;
    if (slot.count >= this.capacity) return false;
    const i = slot.count++,
      spec = vehicleSpec(body);
    const m = this.motion.update(id, x, z, yaw, slot.wheelbase, dt);
    // Width varies a few percent per car; length stays the engine's queue length.
    this.pose.position.set(x, ROAD_SURFACE_Y + lift, z);
    this.pose.rotation.set(-pitch, yaw, 0, 'YXZ');
    this.pose.scale.set(look.width, 1, 1);
    this.pose.updateMatrix();
    slot.mesh.setMatrixAt(i, this.pose.matrix);
    slot.state.setXYZW(i, m.odo, m.steer, yaw, m.brake);
    slot.tint.setXYZ(i, look.paint.r, look.paint.g, look.paint.b);
    if (slot.livery && slot.liveryIndex) {
      slot.livery.setMatrixAt(i, this.pose.matrix);
      slot.liveryIndex.setX(i, look.livery);
    }
    slot.ids[i] = id;
    if (this.shadowCount < this.capacity) {
      this.pose.position.y += 0.02;
      this.pose.scale.set(spec.width + 0.5, 1, spec.length + 0.7);
      this.pose.updateMatrix();
      this.shadows.setMatrixAt(this.shadowCount++, this.pose.matrix);
    }
    return true;
  }
  finish() {
    for (const s of this.slots.values()) {
      s.mesh.count = s.count;
      s.mesh.instanceMatrix.needsUpdate = true;
      s.state.needsUpdate = true;
      s.tint.needsUpdate = true;
      if (s.livery && s.liveryIndex) {
        s.livery.count = s.count;
        s.livery.instanceMatrix.needsUpdate = true;
        s.liveryIndex.needsUpdate = true;
      }
    }
    this.shadows.count = this.shadowCount;
    this.shadows.instanceMatrix.needsUpdate = true;
  }
  reset() {
    this.begin();
    this.finish();
    this.motion.clear();
  }
  dispose() {
    this.root.removeFromParent();
    for (const s of this.slots.values()) {
      s.mesh.geometry.dispose();
      s.mesh.dispose();
      if (s.livery) {
        s.livery.geometry.dispose();
        s.livery.dispose();
      }
    }
    this.shadows.geometry.dispose();
    (this.shadows.material as THREE.Material).dispose();
    this.shadows.dispose();
  }
}
