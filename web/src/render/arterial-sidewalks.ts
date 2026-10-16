import { junctionFootprint } from './road-junction';
import type { Point, Road, World } from '../types';
import { ARTERIAL_SIDEWALK, roadWidth } from './road-elevation';

type Poly = Point[];
const EPS = 1e-7;
const MIN_AREA = 1e-4;

const area = (p: Poly) =>
  p.reduce((s, a, i) => {
    const b = p[(i + 1) % p.length];
    return s + a.x * b.z - b.x * a.z;
  }, 0) / 2;
const cross = (a: Point, b: Point, p: Point) =>
  (b.x - a.x) * (p.z - a.z) - (b.z - a.z) * (p.x - a.x);
function clean(p: Poly): Poly {
  const out: Poly = [];
  for (const v of p)
    if (!out.length || Math.hypot(v.x - out[out.length - 1].x, v.z - out[out.length - 1].z) > EPS)
      out.push(v);
  if (
    out.length > 1 &&
    Math.hypot(out[0].x - out[out.length - 1].x, out[0].z - out[out.length - 1].z) < EPS
  )
    out.pop();
  return out.length >= 3 && Math.abs(area(out)) > MIN_AREA ? out : [];
}
function clip(p: Poly, a: Point, b: Point, inside: boolean): Poly {
  const out: Poly = [];
  if (!p.length) return out;
  for (let i = 0; i < p.length; i++) {
    const s = p[i],
      e = p[(i + 1) % p.length],
      cs = cross(a, b, s),
      ce = cross(a, b, e);
    const si = inside ? cs >= -EPS : cs <= EPS,
      ei = inside ? ce >= -EPS : ce <= EPS;
    if (si) out.push(s);
    if (si !== ei) {
      const t = cs / (cs - ce);
      out.push({ x: s.x + (e.x - s.x) * t, z: s.z + (e.z - s.z) * t });
    }
  }
  return clean(out);
}
/** Subtract a convex polygon, returning the convex outside pieces. */
function subtract(subject: Poly, cutter: Poly): Poly[] {
  if (subject.length < 3 || cutter.length < 3) return subject.length ? [subject] : [];
  if (!hasPositiveIntersection(subject, cutter)) return [subject];
  const ccw = area(cutter) > 0,
    edges = ccw ? cutter : [...cutter].reverse();
  let remainder = subject;
  const pieces: Poly[] = [];
  for (let i = 0; i < edges.length && remainder.length; i++) {
    const a = edges[i],
      b = edges[(i + 1) % edges.length];
    const outside = clip(remainder, a, b, false);
    if (outside.length) pieces.push(outside);
    remainder = clip(remainder, a, b, true);
  }
  return pieces;
}
function rect(a: Point, b: Point, left: number, right: number): Poly {
  const dx = b.x - a.x,
    dz = b.z - a.z,
    l = Math.hypot(dx, dz),
    ux = dx / l,
    uz = dz / l;
  // left/right are signed offsets along the left normal (-uz, ux).
  const at = (t: number, o: number): Point => ({
    x: a.x + ux * t - uz * o,
    z: a.z + uz * t + ux * o,
  });
  return [at(0, left), at(l, left), at(l, right), at(0, right)];
}
function roadRect(a: Point, b: Point, width: number): Poly {
  return rect(a, b, -width / 2, width / 2);
}
function reservationWidth(kind: string) {
  return roadWidth(kind) + (kind === 'bridge' ? 2.8 : 0);
}
const isGround = (r: Road) => r.kind !== 'rail' && r.kind !== 'expressway';
const bounds = (p: Poly) => ({
  minX: Math.min(...p.map((v) => v.x)),
  maxX: Math.max(...p.map((v) => v.x)),
  minZ: Math.min(...p.map((v) => v.z)),
  maxZ: Math.max(...p.map((v) => v.z)),
});
const overlaps = (a: ReturnType<typeof bounds>, b: ReturnType<typeof bounds>) =>
  a.minX <= b.maxX + EPS &&
  a.maxX + EPS >= b.minX &&
  a.minZ <= b.maxZ + EPS &&
  a.maxZ + EPS >= b.minZ;
/** Convex SAT rejects separated or edge-touching polygons before clipping. */
function hasPositiveIntersection(a: Poly, b: Poly): boolean {
  if (a.length < 3 || b.length < 3 || !overlaps(bounds(a), bounds(b))) return false;
  for (const poly of [a, b])
    for (let i = 0; i < poly.length; i++) {
      const p = poly[i],
        q = poly[(i + 1) % poly.length],
        ax = -(q.z - p.z),
        az = q.x - p.x;
      let amin = Infinity,
        amax = -Infinity,
        bmin = Infinity,
        bmax = -Infinity;
      for (const v of a) {
        const d = v.x * ax + v.z * az;
        amin = Math.min(amin, d);
        amax = Math.max(amax, d);
      }
      for (const v of b) {
        const d = v.x * ax + v.z * az;
        bmin = Math.min(bmin, d);
        bmax = Math.max(bmax, d);
      }
      if (Math.min(amax, bmax) - Math.max(amin, bmin) <= EPS) return false;
    }
  return true;
}

/** Build full-length sidewalk geometry incrementally for frame-budgeted callers. */
export function* arterialSidewalkSteps(
  world: Pick<World, 'nodes' | 'roads'>,
): Generator<void, Point[][], void> {
  const nodes = new Map(world.nodes.map((n) => [n.id, n]));
  const roads = [...world.roads].sort((a, b) => a.id - b.id),
    emitted: { poly: Poly; box: ReturnType<typeof bounds> }[] = [];
  const ground: { r: Road; a: Point; b: Point; p: Poly; box: ReturnType<typeof bounds> | null }[] =
    [];
  const incidence = new Map<number, typeof ground>();
  for (const r of roads) {
    const a = nodes.get(r.from),
      b = nodes.get(r.to);
    if (a && b && isGround(r) && Math.hypot(a.x - b.x, a.z - b.z) > EPS) {
      const g = {
        r,
        a,
        b,
        p: roadRect(a, b, reservationWidth(r.kind)),
        box: null as ReturnType<typeof bounds> | null,
      };
      ground.push(g);
      for (const id of [r.from, r.to]) {
        const list = incidence.get(id) ?? [];
        list.push(g);
        incidence.set(id, list);
      }
    }
    yield;
  }
  for (const road of roads) {
    if (road.kind !== 'arterial' || road.stage !== 'open') continue;
    const a = nodes.get(road.from),
      b = nodes.get(road.to);
    if (!a || !b) continue;
    const length = Math.hypot(b.x - a.x, b.z - a.z);
    if (!Number.isFinite(length) || length < EPS) continue;
    const half = roadWidth(road.kind) / 2,
      extent = half + ARTERIAL_SIDEWALK;
    const strips = [
      { poly: rect(a, b, half, extent), corner: false },
      { poly: rect(a, b, -extent, -half), corner: false },
    ];
    // Join the outer sidewalk edges around bends. Only one incident arterial
    // owns each patch; clip it against the same road footprints as the strips.
    for (const node of [a, b]) {
      const arms = (incidence.get(node.id) ?? []).filter(
        (g) => g.r.stage === 'open' && g.r.kind === 'arterial',
      );
      if (arms.length < 2 || arms[0].r.id !== road.id) continue;
      const poly = junctionFootprint(
        node,
        arms.map((g) => ({
          end: g.r.from === node.id ? g.b : g.a,
          width: roadWidth(g.r.kind) + 2 * ARTERIAL_SIDEWALK,
        })),
      );
      if (poly.length) strips.push({ poly, corner: true });
    }
    for (const { poly: strip, corner } of strips) {
      let pieces: Poly[] = [strip];
      const bb = bounds(strip);
      const reservations: typeof ground = [];
      for (const g of ground) {
        if (corner || g.r.id !== road.id) {
          g.box ??= bounds(g.p);
          const incident =
            g.r.from === road.from ||
            g.r.to === road.from ||
            g.r.from === road.to ||
            g.r.to === road.to;
          const nearConstruction =
            g.r.stage !== 'open' &&
            [a, b].some((n) =>
              [g.a, g.b].some(
                (v) =>
                  Math.hypot(n.x - v.x, n.z - v.z) <= extent + reservationWidth(g.r.kind) / 2 + 1,
              ),
            );
          if (overlaps(bb, g.box) && (corner || incident || nearConstruction)) reservations.push(g);
        }
        yield;
      }
      for (const g of reservations) {
        const next: Poly[] = [];
        for (const p of pieces) {
          if (hasPositiveIntersection(p, g.p)) next.push(...subtract(p, g.p));
          else next.push(p);
          yield;
        }
        pieces = next;
        if (!pieces.length) break;
      }
      // Reserve exactly the same rounded footprint used by the carriageway builder.
      for (const node of [a, b]) {
        const incident = (incidence.get(node.id) ?? []).filter(
          (g) => g.r.stage === 'open' && g.r.kind !== 'bridge',
        );
        if (incident.length > 1) {
          const pad = junctionFootprint(
              node,
              incident.map((g) => ({
                end: g.r.from === node.id ? g.b : g.a,
                width: roadWidth(g.r.kind),
              })),
            ),
            next: Poly[] = [];
          for (const p of pieces) {
            if (hasPositiveIntersection(p, pad)) next.push(...subtract(p, pad));
            else next.push(p);
            yield;
          }
          pieces = next;
        }
      }
      for (const prior of emitted) {
        yield;
        if (overlaps(bb, prior.box)) {
          const next: Poly[] = [];
          for (const p of pieces) {
            if (hasPositiveIntersection(p, prior.poly)) next.push(...subtract(p, prior.poly));
            else next.push(p);
            yield;
          }
          pieces = next;
        }
      }
      for (const p of pieces) {
        const cleaned = clean(p);
        if (cleaned.length) emitted.push({ poly: cleaned, box: bounds(cleaned) });
        yield;
      }
    }
  }
  return emitted.map((entry) => entry.poly);
}

/** Synchronous convenience wrapper for tests and non-frame-budgeted callers. */
export function arterialSidewalkPolygons(world: Pick<World, 'nodes' | 'roads'>): Point[][] {
  const steps = arterialSidewalkSteps(world);
  let result = steps.next();
  while (!result.done) result = steps.next();
  return result.value;
}
