import * as THREE from 'three/webgpu';
import manifest from './train-manifest.json';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { Builder } from './vehicle-kit';
import { RAIL_CENTER_OFFSET, PANTOGRAPH_CONTACT_HEIGHT } from './rail-profile';

/**
 * Train cars extruded from the traced side elevation (scripts/assets/build-train-kits.py),
 * with a flat front cap traced from the generated head-on view. Same local frame
 * and attribute contract as the road vehicles: +z forward, +y up, ground at
 * y=0, kerb side +x. Livery is baked into the atlas, so `tinted` is 0 and the
 * shared vehicle material leaves the colour alone.
 */
export type TrainClass = keyof typeof manifest.classes;
export type TrainCar = 'cab' | 'trailer' | 'motor';
type Side = {
  uv: number[];
  outline: number[][];
  belt: number;
  roofY: number;
  wheels: { z: number; y: number; r: number }[];
  paint: number[];
  roof?: string;
  front?: { uv: number[]; outline: number[][]; width: number; height: number };
};
type CarSpec = { left: Side };
type ClassSpec = {
  length: number;
  width: number;
  height: number;
  tumblehome: number;
  gap: number;
  consist: TrainCar[];
  cars: Record<TrainCar, CarSpec>;
};

export const TRAIN_CLASS: TrainClass = 'commuter';

export function trainClass(cls: TrainClass = TRAIN_CLASS): ClassSpec {
  return manifest.classes[cls] as ClassSpec;
}

function sideUV(spec: ClassSpec, side: Side, z: number, y: number): [number, number] {
  const [u0, v0, u1, v1] = side.uv;
  return [
    u0 + ((spec.length / 2 - z) / spec.length) * (u1 - u0),
    v0 + (y / spec.height) * (v1 - v0),
  ];
}

function clipRing(ring: THREE.Vector2[], belt: number, keepBelow: boolean) {
  const inside = (p: THREE.Vector2) => (keepBelow ? p.y <= belt : p.y >= belt);
  const out: THREE.Vector2[] = [];
  for (let i = 0; i < ring.length; i++) {
    const a = ring[i],
      b = ring[(i + 1) % ring.length];
    if (inside(a)) out.push(a);
    if (inside(a) !== inside(b)) {
      const t = (belt - a.y) / (b.y - a.y);
      out.push(new THREE.Vector2(a.x + (b.x - a.x) * t, belt));
    }
  }
  return out;
}

function bodyRoof(spec: ClassSpec) {
  return Math.max(...spec.cars.cab.left.outline.map(([, y]) => y));
}
function bodyBelt(spec: ClassSpec) {
  return Math.min(spec.cars.cab.left.belt, bodyRoof(spec) - 0.05);
}
function halfWidth(spec: ClassSpec) {
  const roofY = bodyRoof(spec),
    belt = bodyBelt(spec);
  const [front, rear] = [0.04, 0.04];
  return (y: number, z = 0) => {
    const lean = 1 - spec.tumblehome * Math.max(0, (y - belt) / Math.max(0.01, roofY - belt));
    const reach = Math.min(0.6, spec.length * 0.14);
    const end = THREE.MathUtils.smoothstep(Math.abs(z), spec.length / 2 - reach, spec.length / 2);
    return (spec.width / 2) * lean * (1 - (z > 0 ? front : rear) * end);
  };
}

/** Passenger door bounds traced in atlas pixels; cab access doors are excluded.
 * Convert through the same UV transform as the body so the opening and paint agree. */
export function trainDoorBounds(role: TrainCar, cls: TrainClass = TRAIN_CLASS) {
  const spec = trainClass(cls),
    side = spec.cars[role].left;
  const pixels: Record<TrainCar, number[][]> = {
    cab: [
      [200, 291, 56, 183],
      [524, 616, 56, 183],
      [850, 942, 56, 183],
      [1182, 1273, 56, 183],
    ],
    trailer: [
      [164, 248, 325, 450],
      [507, 590, 325, 450],
      [853, 939, 325, 450],
      [1198, 1275, 325, 450],
    ],
    motor: [
      [191, 283, 580, 711],
      [650, 745, 580, 711],
      [1117, 1207, 580, 711],
    ],
  };
  const [u0, v0, u1, v1] = side.uv;
  return pixels[role].map(([left, right, top, bottom]) => ({
    minZ: spec.length / 2 - ((right / manifest.atlas[0] - u0) / (u1 - u0)) * spec.length,
    maxZ: spec.length / 2 - ((left / manifest.atlas[0] - u0) / (u1 - u0)) * spec.length,
    minY: ((1 - bottom / manifest.atlas[1] - v0) / (v1 - v0)) * spec.height,
    maxY: ((1 - top / manifest.atlas[1] - v0) / (v1 - v0)) * spec.height,
  }));
}

function clipAxis(poly: THREE.Vector2[], axis: 'x' | 'y', value: number, below: boolean) {
  const out: THREE.Vector2[] = [];
  for (let i = 0; i < poly.length; i++) {
    const a = poly[i],
      b = poly[(i + 1) % poly.length];
    const ia = below ? a[axis] <= value : a[axis] >= value;
    const ib = below ? b[axis] <= value : b[axis] >= value;
    if (ia) out.push(a);
    if (ia !== ib) out.push(a.clone().lerp(b, (value - a[axis]) / (b[axis] - a[axis])));
  }
  return out;
}

/** Subtract rectangles without leaving triangles across an open doorway. */
function doorPanels(poly: THREE.Vector2[], doors: ReturnType<typeof trainDoorBounds>) {
  let panels = [poly];
  for (const d of doors)
    panels = panels.flatMap((p) => {
      const middle = clipAxis(clipAxis(p, 'x', d.minZ, false), 'x', d.maxZ, true);
      return [
        clipAxis(p, 'x', d.minZ, true),
        clipAxis(p, 'x', d.maxZ, false),
        clipAxis(middle, 'y', d.minY, true),
        clipAxis(middle, 'y', d.maxY, false),
      ].filter((part) => part.length >= 3 && Math.abs(THREE.ShapeUtils.area(part)) > 1e-8);
    });
  return panels;
}

/** One sliding leaf in car coordinates, with the original atlas UVs.
 * Animate by translating along z; the UVs remain attached to the leaf. */
export function buildTrainDoorLeaf(
  cls: TrainClass,
  role: TrainCar,
  index: number,
  sideSign: -1 | 1,
  leaf: -1 | 1,
) {
  const spec = trainClass(cls),
    side = spec.cars[role].left,
    door = trainDoorBounds(role, cls)[index];
  if (!door) throw new RangeError(`Unknown passenger door ${index} for ${role}`);
  const b = new Builder(),
    half = halfWidth(spec),
    belt = bodyBelt(spec);
  const mid = (door.minZ + door.maxZ) / 2;
  const lo = leaf < 0 ? door.minZ : mid,
    hi = leaf < 0 ? mid : door.maxZ;
  const levels = [door.minY, ...(belt > door.minY && belt < door.maxY ? [belt] : []), door.maxY];
  for (let i = 0; i < levels.length - 1; i++) {
    const points = [
      [lo, levels[i]],
      [hi, levels[i]],
      [hi, levels[i + 1]],
      [lo, levels[i + 1]],
    ];
    const lean =
      levels[i] >= belt
        ? ((spec.width / 2) * spec.tumblehome) / Math.max(0.01, bodyRoof(spec) - belt)
        : 0;
    const normal = new THREE.Vector3(sideSign, lean, 0).normalize();
    for (const k of sideSign < 0 ? [0, 1, 2, 0, 2, 3] : [0, 2, 1, 0, 3, 2]) {
      const [z, y] = points[k],
        [u, v] = sideUV(spec, side, z, y);
      b.vertex(new THREE.Vector3(sideSign * (half(y, z) + 0.003), y, z), normal, u, v, 0, sideSign);
    }
  }
  return b.finish(0, belt);
}

/** All leaves share one draw batch per car role; motion stays in the shader. */
export function buildTrainDoors(cls: TrainClass, role: TrainCar) {
  const parts: THREE.BufferGeometry[] = [];
  for (const [index, door] of trainDoorBounds(role, cls).entries())
    for (const side of [-1, 1] as const)
      for (const leaf of [-1, 1] as const) {
        const g = buildTrainDoorLeaf(cls, role, index, side, leaf);
        const count = g.getAttribute('position').count,
          travel = new Float32Array(count * 2);
        for (let i = 0; i < count; i++) {
          travel[i * 2] = side;
          travel[i * 2 + 1] = (leaf * (door.maxZ - door.minZ)) / 2;
        }
        g.setAttribute('doorTravel', new THREE.BufferAttribute(travel, 2));
        parts.push(g);
      }
  const result = mergeGeometries(parts)!;
  for (const part of parts) part.dispose();
  return result;
}

/** One car body. `role` picks the side elevation and, for the cab, the front cap. */
export function buildTrainCar(cls: TrainClass, role: TrainCar, doorApertures = false) {
  const spec = trainClass(cls),
    side = spec.cars[role].left;
  const b = new Builder();
  const ring = side.outline.map(([z, y]) => new THREE.Vector2(z, y));
  if (THREE.ShapeUtils.isClockWise(ring)) ring.reverse();
  const roofY = bodyRoof(spec),
    belt = bodyBelt(spec),
    half = halfWidth(spec);
  const lean = ((spec.width / 2) * spec.tumblehome) / Math.max(0.01, roofY - belt);

  for (const keepBelow of [true, false]) {
    const poly = clipRing(ring, belt, keepBelow);
    if (poly.length < 3) continue;
    const panels = doorApertures ? doorPanels(poly, trainDoorBounds(role, cls)) : [poly];
    for (const panel of panels) {
      const tris = THREE.ShapeUtils.triangulateShape(panel, []);
      for (const sign of [-1, 1]) {
        const n = new THREE.Vector3(sign, keepBelow ? 0 : lean, 0).normalize();
        for (const tri of tris) {
          const order = sign < 0 ? tri : [tri[0], tri[2], tri[1]];
          for (const index of order) {
            const p = panel[index];
            const [u, v] = sideUV(spec, side, p.x, p.y);
            b.vertex(new THREE.Vector3(sign * half(p.y, p.x), p.y, p.x), n, u, v, 0, sign);
          }
        }
      }
    }
  }

  const count = ring.length;
  const edgeNormal = (i: number) => {
    const a = ring[i],
      c = ring[(i + 1) % count];
    return new THREE.Vector2(c.y - a.y, -(c.x - a.x)).normalize();
  };
  const pointNormal = (i: number, edge: number) => {
    const own = edgeNormal(edge),
      other = edgeNormal(edge === i ? (i - 1 + count) % count : (i + 1) % count);
    return own.dot(other) > Math.cos(THREE.MathUtils.degToRad(38))
      ? own.clone().add(other).normalize()
      : own;
  };
  for (let i = 0; i < count; i++) {
    const j = (i + 1) % count,
      en = edgeNormal(i);
    if (ring[i].distanceTo(ring[j]) < 1e-4) continue;
    const corners = [i, j].map((k) => {
      const p = ring[k],
        n = pointNormal(k, i);
      const inset = p.clone().addScaledVector(en, Math.abs(en.x) > 0.7 ? -0.22 : -0.05);
      return { p, n, inset };
    });
    const quad = [
      [corners[0], -1],
      [corners[1], -1],
      [corners[1], 1],
      [corners[0], 1],
    ] as const;
    const upward = en.y > 0.7;
    const verts = quad.map(([c, s]) => {
      const [u, v] = upward
        ? sideUV(spec, side, c.inset.x, side.roofY)
        : sideUV(spec, side, c.inset.x, c.inset.y);
      return {
        pos: new THREE.Vector3(s * half(c.p.y, c.p.x), c.p.y, c.p.x),
        n: new THREE.Vector3(0, c.n.y, c.n.x),
        u,
        v,
        s,
      };
    });
    for (const k of [0, 2, 1, 0, 3, 2])
      b.vertex(verts[k].pos, verts[k].n, verts[k].u, verts[k].v, 1, verts[k].s);
  }

  // Head-on front cap, traced from the generated view. Sits on the nose plane.
  if (side.front) {
    const f = side.front,
      [u0, v0, u1, v1] = f.uv;
    const poly = f.outline.map(([x, y]) => new THREE.Vector2(x, y));
    if (THREE.ShapeUtils.isClockWise(poly)) poly.reverse();
    const zFront = spec.length / 2;
    const tris = THREE.ShapeUtils.triangulateShape(poly, []);
    for (const tri of tris) {
      for (const index of [tri[0], tri[2], tri[1]]) {
        const p = poly[index];
        const u = u0 + (p.x / f.width + 0.5) * (u1 - u0);
        const v = v0 + (p.y / f.height) * (v1 - v0);
        b.vertex(
          new THREE.Vector3(p.x, p.y, zFront),
          new THREE.Vector3(0, 0, 1),
          u,
          v,
          0,
          Math.sign(p.x) || 1,
        );
      }
    }
  }

  // Roof equipment: a low air-conditioner box, and a simple pantograph.
  if (side.roof === 'ac' || side.roof === 'pantograph') {
    const ac = new THREE.BoxGeometry(1.15, 0.2, spec.length * 0.42);
    ac.translate(0, roofY + 0.1, 0);
    const acPos = ac.getAttribute('position'),
      acNor = ac.getAttribute('normal'),
      acIdx = ac.getIndex()!;
    for (let t = 0; t < acIdx.count; t++) {
      const k = acIdx.getX(t);
      const p = new THREE.Vector3().fromBufferAttribute(acPos, k),
        n = new THREE.Vector3().fromBufferAttribute(acNor, k);
      const [u, v] = sideUV(spec, side, THREE.MathUtils.clamp(p.z, -9, 9), side.roofY);
      b.vertex(p, n, u, v, 3, Math.sign(p.x));
    }
    ac.dispose();
  }
  if (side.roof === 'pantograph') {
    // Articulated collector stays under the corridor wire; the shoe spans x,
    // across the track, while the diamond arms lie in the longitudinal plane.
    const bottom = roofY + 0.16,
      top = PANTOGRAPH_CONTACT_HEIGHT - 0.08;
    const mid = (bottom + top) / 2,
      centerZ = -3.4;
    const append = (g: THREE.BufferGeometry) => {
      const pos = g.getAttribute('position'),
        nor = g.getAttribute('normal'),
        idx = g.getIndex()!;
      for (let t = 0; t < idx.count; t++) {
        const k = idx.getX(t);
        b.vertex(
          new THREE.Vector3().fromBufferAttribute(pos, k),
          new THREE.Vector3().fromBufferAttribute(nor, k),
          side.uv[0],
          side.uv[1],
          4,
          0,
        );
      }
      g.dispose();
    };
    for (const sign of [-1, 1])
      for (const [y0, z0, y1, z1] of [
        [bottom, centerZ, mid, centerZ + sign * 0.65],
        [mid, centerZ + sign * 0.65, top, centerZ],
      ]) {
        const a = new THREE.Vector3(0, y0, z0),
          end = new THREE.Vector3(0, y1, z1),
          delta = end.clone().sub(a);
        const arm = new THREE.BoxGeometry(0.065, delta.length(), 0.065);
        arm.applyQuaternion(
          new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0, 1, 0), delta.normalize()),
        );
        arm.translate(0, (y0 + y1) / 2, (z0 + z1) / 2);
        append(arm);
      }
    const shoe = new THREE.BoxGeometry(1.8, 0.08, 0.12);
    shoe.translate(0, PANTOGRAPH_CONTACT_HEIGHT - 0.04, centerZ);
    append(shoe);
  }

  const tyreWidth = 0.26,
    frontZ = Math.max(...side.wheels.map((w) => w.z));
  for (const w of side.wheels) {
    for (const sign of [-1, 1]) {
      // Wheelset gauge follows the rails, independently of the wider car body.
      const x = sign * RAIL_CENTER_OFFSET;
      const cyl = new THREE.CylinderGeometry(w.r, w.r, tyreWidth, 14, 1);
      cyl.rotateZ(Math.PI / 2);
      const pos = cyl.getAttribute('position'),
        nor = cyl.getAttribute('normal'),
        index = cyl.getIndex()!;
      for (let t = 0; t < index.count; t += 3) {
        const ks = [index.getX(t), index.getX(t + 1), index.getX(t + 2)];
        const normals = ks.map((k) => new THREE.Vector3().fromBufferAttribute(nor, k));
        if (normals.every((n) => Math.abs(n.x) > 0.5 && Math.sign(n.x) !== sign)) continue;
        ks.forEach((k, i) => {
          const p = new THREE.Vector3().fromBufferAttribute(pos, k),
            n = normals[i];
          const [u, v] =
            Math.abs(n.x) > 0.5
              ? sideUV(spec, side, w.z + p.z, w.y + p.y)
              : sideUV(spec, side, w.z, w.y - w.r * 0.92);
          b.vertex(new THREE.Vector3(p.x + x, p.y + w.y, p.z + w.z), n, u, v, 2, sign, [
            w.y,
            w.z,
            w.r,
            w.z === frontZ ? 0 : 0,
          ]);
        });
      }
      cyl.dispose();
    }
  }
  return b.finish(0, belt);
}

/** Cars of one consist in travel order, front to rear, with centre offsets in metres.
 * Index 0 is the leading car and takes the +offset (ahead along the heading);
 * index n-1 is the trailing car at -offset. The front cap is local +z, so the
 * leading car keeps the heading and only the trailing car is turned by PI. */
export function consistCars(cls: TrainClass = TRAIN_CLASS) {
  const spec = trainClass(cls),
    n = spec.consist.length,
    pitch = spec.length + spec.gap;
  return spec.consist.map((role, i) => ({
    role,
    offset: ((n - 1) / 2 - i) * pitch,
    leading: i === 0,
    trailing: i === n - 1,
  }));
}

/** Half the total consist length; must equal the engine's TRAIN_HALF_LENGTH. */
export function consistHalfLength(cls: TrainClass = TRAIN_CLASS) {
  const spec = trainClass(cls),
    n = spec.consist.length;
  return ((n - 1) * (spec.length + spec.gap)) / 2 + spec.length / 2;
}
