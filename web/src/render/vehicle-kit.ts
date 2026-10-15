import * as THREE from 'three/webgpu';
import manifest from './vehicle-manifest.json';

/**
 * Vehicle bodies extruded from traced side elevations
 * (scripts/build-vehicle-kits.py). Local frame: +z forward, +y up, ground at
 * y=0, kerb side at +x: the driver's left in this right-handed, y-up frame
 * (engine KERB_LANE; see buses.test). The manifest's `left` elevation is the
 * kerb side, so it reads unmirrored there.
 *
 * Extra attributes drive the shared vehicle shader:
 *  - vpart: 0 side, 1 perimeter strip, 2 wheel, 3 roof sign
 *  - lateral: -1..1 across the body, for lamp placement on end faces
 *  - wheel: (axle y, axle z, radius, steer pivot x or 0)
 *  - tinted: 1 where a front face may become grille trim: painted bodies
 *    (all but the bus), below the belt line (bumper and lamp zone only)
 */
export type VehicleBody = keyof typeof manifest.bodies;
export const CAR_BODIES = ['compact', 'compact-crossover', 'compact-minivan', 'kei', 'kei-hatch', 'kei-truck', 'taxi', 'taxi-classic', 'van', 'truck'] as const satisfies readonly VehicleBody[];
type Side = { uv: number[]; outline: number[][]; belt: number; roofY?: number; wheels: { z: number; y: number; r: number }[]; paint: number[]; roofSign?: { z0: number; z1: number; y0: number; y1: number } };
type BodySpec = { length: number; width: number; height: number; tumblehome: number; round?: number[]; left: Side; right: Side | null };

export function vehicleSpec(body: VehicleBody): BodySpec {
  return manifest.bodies[body] as BodySpec;
}

/** Geometric length used by renderer spacing and engine following gaps. */
export function vehicleLength(body: VehicleBody) { return vehicleSpec(body).length; }

export class Builder {
  position: number[] = []; normal: number[] = []; uv: number[] = [];
  part: number[] = []; lateral: number[] = []; wheel: number[] = [];
  vertex(p: THREE.Vector3, n: THREE.Vector3, u: number, v: number, part: number, lateral: number, wheel = [0, 0, 0, 0]) {
    this.position.push(p.x, p.y, p.z); this.normal.push(n.x, n.y, n.z); this.uv.push(u, v);
    this.part.push(part); this.lateral.push(lateral); this.wheel.push(...wheel);
  }
  finish(tinted = 1, belt = Infinity) {
    const g = new THREE.BufferGeometry();
    const trimmable = new Float32Array(this.part.length);
    for (let i = 0; i < trimmable.length; i++) trimmable[i] = tinted && this.position[i * 3 + 1] <= belt + .05 ? 1 : 0;
    // Keep the authored vertex data lossless while placing its seven static
    // attributes in one GPU buffer. WebGPU limits each draw to eight vertex
    // buffers; vehicle instances add separate vstate/vtint buffers and the
    // instance matrix, so separate static buffers exceed that limit.
    const attributes = [
      ['position', this.position, 3], ['normal', this.normal, 3], ['uv', this.uv, 2],
      ['vpart', this.part, 1], ['lateral', this.lateral, 1], ['wheel', this.wheel, 4], ['tinted', trimmable, 1],
    ] as const;
    const stride = attributes.reduce((sum, [, , size]) => sum + size, 0);
    const interleaved = new THREE.InterleavedBuffer(new Float32Array(this.part.length * stride), stride);
    interleaved.setUsage(THREE.StaticDrawUsage);
    let offset = 0;
    for (const [name, source, size] of attributes) {
      for (let vertex = 0; vertex < this.part.length; vertex++) {
        for (let component = 0; component < size; component++) {
          interleaved.array[vertex * stride + offset + component] = source[vertex * size + component];
        }
      }
      g.setAttribute(name, new THREE.InterleavedBufferAttribute(interleaved, size, offset));
      offset += size;
    }
    g.computeBoundingSphere();
    return g;
  }
}

/** Atlas UV for a point (z, y) on a side elevation; the front sits at u0. */
function sideUV(spec: BodySpec, side: Side, z: number, y: number): [number, number] {
  const [u0, v0, u1, v1] = side.uv;
  return [u0 + (spec.length / 2 - z) / spec.length * (u1 - u0), v0 + y / spec.height * (v1 - v0)];
}

/** Clip a ring against y=belt; returns the part below (keepBelow) or above. */
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

export function buildVehicleGeometry(body: VehicleBody) {
  const spec = vehicleSpec(body), left = spec.left, right = spec.right ?? spec.left;
  const b = new Builder();
  const ring = left.outline.map(([z, y]) => new THREE.Vector2(z, y));
  if (THREE.ShapeUtils.isClockWise(ring)) ring.reverse();
  const roofY = bodyRoof(spec), belt = bodyBelt(spec), half = bodyHalfWidth(spec);
  const lean = spec.width / 2 * spec.tumblehome / Math.max(.01, roofY - belt);

  // Side faces, lower and upper bands, both sides.
  for (const keepBelow of [true, false]) {
    const poly = clipRing(ring, belt, keepBelow);
    if (poly.length < 3) continue;
    const tris = THREE.ShapeUtils.triangulateShape(poly, []);
    for (const sign of [-1, 1]) {
      const side = sign > 0 ? left : right;
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

  // Perimeter strip: roof, bonnet, ends and underside. Colour comes from just
  // inside the silhouette on the side elevation, so a black bumper stays black.
  const count = ring.length;
  const edgeNormal = (i: number) => {
    const a = ring[i], c = ring[(i + 1) % count];
    // Ring is counter-clockwise in (z, y): outward normal is (dy, -dz).
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
      // Ends sample deeper: the elevation's outermost column is lamp lens and
      // chrome highlight, which smeared across the whole width reads as a band.
      const inset = p.clone().addScaledVector(en, Math.abs(en.x) > .7 ? -.22 : -.05);
      return { p, n, inset };
    });
    // Ends get the kerb-side elevation colour; both sides are symmetric enough.
    const quad = [[corners[0], -1], [corners[1], -1], [corners[1], 1], [corners[0], 1]] as const;
    // Roofs and bonnets take a uniform body row chosen by the kit builder
    // (roofY): the elevation's top edge is a thin, streaky rim.
    const upward = en.y > .7;
    const verts = quad.map(([c, s]) => {
      const [u, v] = upward ? sideUV(spec, left, c.inset.x, left.roofY ?? Math.max(.1, belt - .15)) : sideUV(spec, left, c.inset.x, c.inset.y);
      return { pos: new THREE.Vector3(s * half(c.p.y, c.p.x), c.p.y, c.p.x), n: new THREE.Vector3(0, c.n.y, c.n.x), u, v, s };
    });
    for (const k of [0, 2, 1, 0, 3, 2]) b.vertex(verts[k].pos, verts[k].n, verts[k].u, verts[k].v, 1, verts[k].s);
  }

  // Wheels: textured caps from the elevation's own tyre; rotation in shader.
  const tyreWidth = spec.length > 6 ? .3 : spec.length > 4.5 ? .24 : .2;
  const frontZ = Math.max(...left.wheels.map(w => w.z));
  for (const w of left.wheels) {
    for (const sign of [-1, 1]) {
      const x = sign * (spec.width / 2 - tyreWidth / 2 - .02), steer = w.z === frontZ && spec.length < 6 ? x : 0;
      const cyl = new THREE.CylinderGeometry(w.r, w.r, tyreWidth, 16, 1);
      cyl.rotateZ(Math.PI / 2);
      const pos = cyl.getAttribute('position'), nor = cyl.getAttribute('normal'), index = cyl.getIndex()!;
      const side = sign > 0 ? left : right;
      for (let t = 0; t < index.count; t += 3) {
        const ks = [index.getX(t), index.getX(t + 1), index.getX(t + 2)];
        const normals = ks.map(k => new THREE.Vector3().fromBufferAttribute(nor, k));
        // The inner cap faces the body and is never seen.
        if (normals.every(n => Math.abs(n.x) > .5 && Math.sign(n.x) !== sign)) continue;
        ks.forEach((k, i) => {
          const p = new THREE.Vector3().fromBufferAttribute(pos, k), n = normals[i];
          // Caps sample the elevation's own tyre; tread samples rubber at the contact patch.
          const [u, v] = Math.abs(n.x) > .5 ? sideUV(spec, side, w.z + p.z, w.y + p.y) : sideUV(spec, side, w.z, w.y - w.r * .92);
          b.vertex(new THREE.Vector3(p.x + x, p.y + w.y, p.z + w.z), n, u, v, 2, sign, [w.y, w.z, w.r, steer]);
        });
      }
      cyl.dispose();
    }
  }

  // Taxi roof sign as a small lit box.
  if (left.roofSign) {
    const s = left.roofSign, w = .38, cz = (s.z0 + s.z1) / 2, depth = THREE.MathUtils.clamp(s.z1 - s.z0, .18, .45);
    const box = new THREE.BoxGeometry(w, s.y1 - s.y0, depth);
    box.translate(0, (s.y0 + s.y1) / 2, cz);
    const pos = box.getAttribute('position'), nor = box.getAttribute('normal'), index = box.getIndex()!;
    for (let t = 0; t < index.count; t++) {
      const k = index.getX(t);
      const p = new THREE.Vector3().fromBufferAttribute(pos, k), n = new THREE.Vector3().fromBufferAttribute(nor, k);
      const [u, v] = sideUV(spec, left, p.z, Math.min(p.y, s.y1 - .01));
      b.vertex(p, n, u, v, 3, Math.sign(p.x));
    }
    box.dispose();
  }
  // The bus keeps its painted livery; every other body takes instance paint.
  return b.finish(body === 'bus' ? 0 : 1, belt);
}

/**
 * Two flat livery panels (kerb and off side) for liveried bodies. UVs run so
 * that lettering reads left to right from either side; the shader offsets
 * them into the instance's livery tile.
 */
export function buildLiveryGeometry(body: VehicleBody, panel: LiveryPanel) {
  const spec = vehicleSpec(body), b = new Builder(), half = bodyHalfWidth(spec);
  // Split at the belt so the panel follows the leaning glasshouse.
  const rows = [panel.y0, ...[bodyBelt(spec)].filter(y => y > panel.y0 && y < panel.y1), panel.y1];
  for (const sign of [-1, 1]) {
    for (let r = 0; r < rows.length - 1; r++) {
      const ya = rows[r], yb = rows[r + 1];
      // From the -x side the front (+z) is on the viewer's right; reversed on +x.
      const corners = [[panel.z0, ya], [panel.z1, ya], [panel.z1, yb], [panel.z0, yb]].map(([z, y]) => {
        const u = sign < 0 ? (z - panel.z0) / (panel.z1 - panel.z0) : (panel.z1 - z) / (panel.z1 - panel.z0);
        return { p: new THREE.Vector3(sign * (half(y, z) + .012), y, z), u, v: (y - panel.y0) / (panel.y1 - panel.y0) };
      });
      const n = new THREE.Vector3(sign, 0, 0);
      // Seen from -x, +z is to the viewer's right: the ring is counter-clockwise.
      const order = sign < 0 ? [0, 1, 2, 0, 2, 3] : [0, 2, 1, 0, 3, 2];
      for (const k of order) b.vertex(corners[k].p, n, corners[k].u, corners[k].v, 4, sign);
    }
  }
  return b.finish();
}

export type LiveryPanel = { z0: number; z1: number; y0: number; y1: number };

function bodyRoof(spec: BodySpec) { return Math.max(...spec.left.outline.map(([, y]) => y)); }
function bodyBelt(spec: BodySpec) { return Math.min(spec.left.belt, bodyRoof(spec) - .05); }
/**
 * Half width at height y and length position z. The glasshouse leans inward
 * above the belt line, and the plan narrows toward each end (`round`: front,
 * rear fraction) so noses and tails are not flat slabs across the width.
 */
function bodyHalfWidth(spec: BodySpec) {
  const roofY = bodyRoof(spec), belt = bodyBelt(spec);
  const [front, rear] = spec.round ?? [0, 0], reach = Math.min(.6, spec.length * .14);
  return (y: number, z = 0) => {
    const lean = 1 - spec.tumblehome * Math.max(0, (y - belt) / Math.max(.01, roofY - belt));
    const end = THREE.MathUtils.smoothstep(Math.abs(z), spec.length / 2 - reach, spec.length / 2);
    return spec.width / 2 * lean * (1 - (z > 0 ? front : rear) * end);
  };
}
