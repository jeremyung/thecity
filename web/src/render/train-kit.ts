import * as THREE from 'three/webgpu';
import manifest from './train-manifest.json';
import { Builder } from './vehicle-kit';

/**
 * Train cars extruded from the traced side elevation (scripts/build-train-kits.py),
 * with a flat front cap traced from the generated head-on view. Same local frame
 * and attribute contract as the road vehicles: +z forward, +y up, ground at
 * y=0, kerb side +x. Livery is baked into the atlas, so `tinted` is 0 and the
 * shared vehicle material leaves the colour alone.
 */
export type TrainClass = keyof typeof manifest.classes;
export type TrainCar = 'cab' | 'trailer' | 'motor';
type Side = {
  uv: number[]; outline: number[][]; belt: number; roofY: number;
  wheels: { z: number; y: number; r: number }[]; paint: number[]; roof?: string;
  front?: { uv: number[]; outline: number[][]; width: number; height: number };
};
type CarSpec = { left: Side };
type ClassSpec = { length: number; width: number; height: number; tumblehome: number; gap: number; consist: TrainCar[]; cars: Record<TrainCar, CarSpec> };

export const TRAIN_CLASS: TrainClass = 'commuter';

export function trainClass(cls: TrainClass = TRAIN_CLASS): ClassSpec {
  return manifest.classes[cls] as ClassSpec;
}

function sideUV(spec: ClassSpec, side: Side, z: number, y: number): [number, number] {
  const [u0, v0, u1, v1] = side.uv;
  return [u0 + (spec.length / 2 - z) / spec.length * (u1 - u0), v0 + y / spec.height * (v1 - v0)];
}

function clipRing(ring: THREE.Vector2[], belt: number, keepBelow: boolean) {
  const inside = (p: THREE.Vector2) => keepBelow ? p.y <= belt : p.y >= belt;
  const out: THREE.Vector2[] = [];
  for (let i = 0; i < ring.length; i++) {
    const a = ring[i], b = ring[(i + 1) % ring.length];
    if (inside(a)) out.push(a);
    if (inside(a) !== inside(b)) {
      const t = (belt - a.y) / (b.y - a.y);
      out.push(new THREE.Vector2(a.x + (b.x - a.x) * t, belt));
    }
  }
  return out;
}

function bodyRoof(spec: ClassSpec) { return Math.max(...spec.cars.cab.left.outline.map(([, y]) => y)); }
function bodyBelt(spec: ClassSpec) { return Math.min(spec.cars.cab.left.belt, bodyRoof(spec) - .05); }
function halfWidth(spec: ClassSpec) {
  const roofY = bodyRoof(spec), belt = bodyBelt(spec);
  const [front, rear] = [0.04, 0.04];
  return (y: number, z = 0) => {
    const lean = 1 - spec.tumblehome * Math.max(0, (y - belt) / Math.max(.01, roofY - belt));
    const reach = Math.min(.6, spec.length * .14);
    const end = THREE.MathUtils.smoothstep(Math.abs(z), spec.length / 2 - reach, spec.length / 2);
    return spec.width / 2 * lean * (1 - (z > 0 ? front : rear) * end);
  };
}

/** One car body. `role` picks the side elevation and, for the cab, the front cap. */
export function buildTrainCar(cls: TrainClass, role: TrainCar) {
  const spec = trainClass(cls), side = spec.cars[role].left;
  const b = new Builder();
  const ring = side.outline.map(([z, y]) => new THREE.Vector2(z, y));
  if (THREE.ShapeUtils.isClockWise(ring)) ring.reverse();
  const roofY = bodyRoof(spec), belt = bodyBelt(spec), half = halfWidth(spec);
  const lean = spec.width / 2 * spec.tumblehome / Math.max(.01, roofY - belt);

  for (const keepBelow of [true, false]) {
    const poly = clipRing(ring, belt, keepBelow);
    if (poly.length < 3) continue;
    const tris = THREE.ShapeUtils.triangulateShape(poly, []);
    for (const sign of [-1, 1]) {
      const n = new THREE.Vector3(sign, keepBelow ? 0 : lean, 0).normalize();
      for (const tri of tris) {
        const order = sign < 0 ? tri : [tri[0], tri[2], tri[1]];
        for (const index of order) {
          const p = poly[index];
          const [u, v] = sideUV(spec, side, p.x, p.y);
          b.vertex(new THREE.Vector3(sign * half(p.y, p.x), p.y, p.x), n, u, v, 0, sign);
        }
      }
    }
  }

  const count = ring.length;
  const edgeNormal = (i: number) => {
    const a = ring[i], c = ring[(i + 1) % count];
    return new THREE.Vector2(c.y - a.y, -(c.x - a.x)).normalize();
  };
  const pointNormal = (i: number, edge: number) => {
    const own = edgeNormal(edge), other = edgeNormal(edge === i ? (i - 1 + count) % count : (i + 1) % count);
    return own.dot(other) > Math.cos(THREE.MathUtils.degToRad(38)) ? own.clone().add(other).normalize() : own;
  };
  for (let i = 0; i < count; i++) {
    const j = (i + 1) % count, en = edgeNormal(i);
    if (ring[i].distanceTo(ring[j]) < 1e-4) continue;
    const corners = [i, j].map(k => {
      const p = ring[k], n = pointNormal(k, i);
      const inset = p.clone().addScaledVector(en, Math.abs(en.x) > .7 ? -.22 : -.05);
      return { p, n, inset };
    });
    const quad = [[corners[0], -1], [corners[1], -1], [corners[1], 1], [corners[0], 1]] as const;
    const upward = en.y > .7;
    const verts = quad.map(([c, s]) => {
      const [u, v] = upward ? sideUV(spec, side, c.inset.x, side.roofY) : sideUV(spec, side, c.inset.x, c.inset.y);
      return { pos: new THREE.Vector3(s * half(c.p.y, c.p.x), c.p.y, c.p.x), n: new THREE.Vector3(0, c.n.y, c.n.x), u, v, s };
    });
    for (const k of [0, 2, 1, 0, 3, 2]) b.vertex(verts[k].pos, verts[k].n, verts[k].u, verts[k].v, 1, verts[k].s);
  }

  // Head-on front cap, traced from the generated view. Sits on the nose plane.
  if (side.front) {
    const f = side.front, [u0, v0, u1, v1] = f.uv;
    const poly = f.outline.map(([x, y]) => new THREE.Vector2(x, y));
    if (THREE.ShapeUtils.isClockWise(poly)) poly.reverse();
    const zFront = spec.length / 2;
    const tris = THREE.ShapeUtils.triangulateShape(poly, []);
    for (const tri of tris) {
      for (const index of [tri[0], tri[2], tri[1]]) {
        const p = poly[index];
        const u = u0 + (p.x / f.width + .5) * (u1 - u0);
        const v = v0 + (p.y / f.height) * (v1 - v0);
        b.vertex(new THREE.Vector3(p.x, p.y, zFront), new THREE.Vector3(0, 0, 1), u, v, 0, Math.sign(p.x) || 1);
      }
    }
  }

  // Roof equipment: a low air-conditioner box, and a simple pantograph.
  if (side.roof === 'ac' || side.roof === 'pantograph') {
    const ac = new THREE.BoxGeometry(1.15, .2, spec.length * .42);
    ac.translate(0, roofY + .1, 0);
    const acPos = ac.getAttribute('position'), acNor = ac.getAttribute('normal'), acIdx = ac.getIndex()!;
    for (let t = 0; t < acIdx.count; t++) {
      const k = acIdx.getX(t);
      const p = new THREE.Vector3().fromBufferAttribute(acPos, k), n = new THREE.Vector3().fromBufferAttribute(acNor, k);
      const [u, v] = sideUV(spec, side, THREE.MathUtils.clamp(p.z, -9, 9), side.roofY);
      b.vertex(p, n, u, v, 3, Math.sign(p.x));
    }
    ac.dispose();
  }
  if (side.roof === 'pantograph') {
    for (const [tilt, y] of [[.5, 1.0], [-.5, 1.0]] as const) {
      const arm = new THREE.BoxGeometry(.1, .1, 2.4);
      arm.rotateX(tilt); arm.translate(0, roofY + y, -3.4);
      const pos = arm.getAttribute('position'), nor = arm.getAttribute('normal'), idx = arm.getIndex()!;
      for (let t = 0; t < idx.count; t++) {
        const k = idx.getX(t);
        b.vertex(new THREE.Vector3().fromBufferAttribute(pos, k), new THREE.Vector3().fromBufferAttribute(nor, k),
          side.uv[0], side.uv[1], 4, 0);
      }
      arm.dispose();
    }
    const bar = new THREE.BoxGeometry(.12, .08, 1.8); bar.translate(0, roofY + 2.0, -3.4);
    const bp = bar.getAttribute('position'), bn = bar.getAttribute('normal'), bi = bar.getIndex()!;
    for (let t = 0; t < bi.count; t++) { const k = bi.getX(t); b.vertex(new THREE.Vector3().fromBufferAttribute(bp, k), new THREE.Vector3().fromBufferAttribute(bn, k), side.uv[0], side.uv[1], 4, 0); }
    bar.dispose();
  }

  const tyreWidth = .26, frontZ = Math.max(...side.wheels.map(w => w.z));
  for (const w of side.wheels) {
    for (const sign of [-1, 1]) {
      const x = sign * (spec.width / 2 - tyreWidth / 2 - .04);
      const cyl = new THREE.CylinderGeometry(w.r, w.r, tyreWidth, 14, 1);
      cyl.rotateZ(Math.PI / 2);
      const pos = cyl.getAttribute('position'), nor = cyl.getAttribute('normal'), index = cyl.getIndex()!;
      for (let t = 0; t < index.count; t += 3) {
        const ks = [index.getX(t), index.getX(t + 1), index.getX(t + 2)];
        const normals = ks.map(k => new THREE.Vector3().fromBufferAttribute(nor, k));
        if (normals.every(n => Math.abs(n.x) > .5 && Math.sign(n.x) !== sign)) continue;
        ks.forEach((k, i) => {
          const p = new THREE.Vector3().fromBufferAttribute(pos, k), n = normals[i];
          const [u, v] = Math.abs(n.x) > .5 ? sideUV(spec, side, w.z + p.z, w.y + p.y) : sideUV(spec, side, w.z, w.y - w.r * .92);
          b.vertex(new THREE.Vector3(p.x + x, p.y + w.y, p.z + w.z), n, u, v, 2, sign, [w.y, w.z, w.r, w.z === frontZ ? 0 : 0]);
        });
      }
      cyl.dispose();
    }
  }
  return b.finish(0, belt);
}

/** Cars of one consist in travel order, front to rear, with centre offsets in metres. */
export function consistCars(cls: TrainClass = TRAIN_CLASS) {
  const spec = trainClass(cls), n = spec.consist.length, pitch = spec.length + spec.gap;
  return spec.consist.map((role, i) => ({ role, offset: (i - (n - 1) / 2) * pitch, leading: i === 0, trailing: i === n - 1 }));
}

/** Half the total consist length; must equal the engine's TRAIN_HALF_LENGTH. */
export function consistHalfLength(cls: TrainClass = TRAIN_CLASS) {
  const spec = trainClass(cls), n = spec.consist.length;
  return (n - 1) * (spec.length + spec.gap) / 2 + spec.length / 2;
}
