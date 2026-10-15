import type { Point } from '../types';

type Bounds = { minX: number; maxX: number; minZ: number; maxZ: number };
type Owner = { polygon: readonly Point[]; bounds: Bounds };

const CELL_SIZE = 32;
const EPS = 1e-9;
const MIN_AREA = 1e-8;

const signedArea = (polygon: readonly Point[]): number => {
  let sum = 0;
  for (let i = 0; i < polygon.length; i++) {
    const a = polygon[i], b = polygon[(i + 1) % polygon.length];
    sum += a.x * b.z - b.x * a.z;
  }
  return sum / 2;
};

function boundsOf(polygon: readonly Point[]): Bounds {
  return polygon.reduce((b, p) => ({
    minX: Math.min(b.minX, p.x), maxX: Math.max(b.maxX, p.x),
    minZ: Math.min(b.minZ, p.z), maxZ: Math.max(b.maxZ, p.z),
  }), { minX: Infinity, maxX: -Infinity, minZ: Infinity, maxZ: -Infinity });
}

function clean(polygon: readonly Point[]): Point[] {
  const result: Point[] = [];
  for (const p of polygon) {
    const prior = result[result.length - 1];
    if (!prior || Math.hypot(p.x - prior.x, p.z - prior.z) > EPS) result.push({ x: p.x, z: p.z });
  }
  if (result.length > 1 && Math.hypot(result[0].x - result[result.length - 1].x, result[0].z - result[result.length - 1].z) <= EPS) result.pop();
  return result.length >= 3 && Math.abs(signedArea(result)) > MIN_AREA ? result : [];
}

const cross = (a: Point, b: Point, p: Point) => (b.x - a.x) * (p.z - a.z) - (b.z - a.z) * (p.x - a.x);
const boxesOverlap = (a: Bounds, b: Bounds) => a.minX <= b.maxX + EPS && a.maxX + EPS >= b.minX && a.minZ <= b.maxZ + EPS && a.maxZ + EPS >= b.minZ;

function clip(polygon: readonly Point[], a: Point, b: Point, inside: boolean): Point[] {
  const result: Point[] = [];
  for (let i = 0; i < polygon.length; i++) {
    const start = polygon[i], end = polygon[(i + 1) % polygon.length];
    const cs = cross(a, b, start), ce = cross(a, b, end);
    const startInside = inside ? cs >= -EPS : cs <= EPS;
    const endInside = inside ? ce >= -EPS : ce <= EPS;
    if (startInside) result.push(start);
    if (startInside !== endInside) {
      const t = cs / (cs - ce);
      result.push({ x: start.x + (end.x - start.x) * t, z: start.z + (end.z - start.z) * t });
    }
  }
  return clean(result);
}

/** Subtract one convex polygon, yielding a disjoint set of convex remainder pieces. */
function subtract(subject: readonly Point[], cutter: readonly Point[]): Point[][] {
  const ccw = signedArea(cutter) > 0;
  const edges = ccw ? cutter : [...cutter].reverse();
  let remainder = [...subject];
  const pieces: Point[][] = [];
  for (let i = 0; i < edges.length && remainder.length; i++) {
    const outside = clip(remainder, edges[i], edges[(i + 1) % edges.length], false);
    if (outside.length) pieces.push(outside);
    remainder = clip(remainder, edges[i], edges[(i + 1) % edges.length], true);
  }
  return pieces;
}

/**
 * Gives each point of overlapping ground surfaces to the first polygon that
 * claims it. Claims are expected to be convex; invalid/non-finite claims are
 * ignored and never enter the ownership index.
 */
export class RoadSurfacePartition {
  private readonly owners: Owner[] = [];
  private readonly cells = new Map<string, number[]>();

  /** Fork the current ownership state for an independent set of later claims. */
  fork(): RoadSurfacePartition {
    const fork = new RoadSurfacePartition();
    // Owner polygons and bounds are private immutable values after insertion,
    // so their records can be shared while the mutable indexes are copied.
    fork.owners.push(...this.owners);
    for (const [key, indices] of this.cells) fork.cells.set(key, [...indices]);
    return fork;
  }

  claim(input: readonly Point[]): Point[][] {
    if (input.length < 3 || input.some(p => !Number.isFinite(p.x) || !Number.isFinite(p.z))) return [];
    const original = clean(input);
    if (!original.length) return [];
    const box = boundsOf(original);
    const candidates = new Set<number>();
    for (const key of this.cellKeys(box)) for (const index of this.cells.get(key) ?? []) candidates.add(index);

    let pieces: Point[][] = [original];
    for (const index of [...candidates].sort((a, b) => a - b)) {
      const owner = this.owners[index];
      if (!boxesOverlap(box, owner.bounds)) continue;
      const next: Point[][] = [];
      for (const piece of pieces) next.push(...subtract(piece, owner.polygon));
      pieces = next;
      if (!pieces.length) break;
    }

    // Register a copy of the full submitted footprint, even when it was wholly
    // covered. That preserves insertion-order ownership for later claims.
    const ownerIndex = this.owners.length;
    this.owners.push({ polygon: original, bounds: box });
    for (const key of this.cellKeys(box)) {
      const entries = this.cells.get(key) ?? [];
      entries.push(ownerIndex);
      this.cells.set(key, entries);
    }
    return pieces;
  }

  private *cellKeys(box: Bounds): Generator<string> {
    const minX = Math.floor(box.minX / CELL_SIZE), maxX = Math.floor(box.maxX / CELL_SIZE);
    const minZ = Math.floor(box.minZ / CELL_SIZE), maxZ = Math.floor(box.maxZ / CELL_SIZE);
    for (let x = minX; x <= maxX; x++) for (let z = minZ; z <= maxZ; z++) yield `${x},${z}`;
  }
}
