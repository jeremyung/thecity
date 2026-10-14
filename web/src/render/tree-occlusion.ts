import { roadWidth } from './road-elevation';
import type { Building, Parcel, Point, RearExtensionView as RearExtension, Road, World } from '../types';

export type TreePoint = { x: number; z: number; s: number };

const TREE_CELL = 24;

export function treeCanopy(p: TreePoint) { return p.s * 1.7 + 2; }
export function roadTreeClearance(kind: Road['kind']) { return roadWidth(kind) / 2 + (kind === 'lane' ? 1.5 : 3.5); }

export function buildingHidesTree(b: Building, p: TreePoint) {
  const dx = p.x - b.x, dz = p.z - b.z, c = Math.cos(b.rotation), s = Math.sin(b.rotation), localX = c * dx - s * dz, localZ = s * dx + c * dz, canopy = treeCanopy(p);
  return Math.abs(localX) < b.width / 2 + canopy && localZ > -b.depth / 2 - canopy && localZ < b.depth / 2 + canopy + (b.anchor ? 4 : 0);
}
/**
 * Farthest a hidden tree's trunk can be from the building center. The test
 * region is a box (width/2+canopy, depth/2+canopy+anchor rear reach); by the
 * triangle inequality this radius covers its corners at any rotation.
 */
export function buildingTreeReach(b: Building, maxCanopy: number) { return Math.hypot(b.width / 2, b.depth / 2 + (b.anchor ? 4 : 0)) + maxCanopy * Math.SQRT2; }
export function roadHidesTree(r: Road, nodes: Map<number, Point>, p: TreePoint) {
  const a = nodes.get(r.from), b = nodes.get(r.to);
  return !!a && !!b && r.kind !== 'expressway' && distanceToSegment(p, a, b) < roadTreeClearance(r.kind);
}
export function parcelHidesTree(parcel: Parcel, p: TreePoint) {
  const dx = p.x - parcel.x, dz = p.z - parcel.z, rotation = parcel.rotation ?? 0, c = Math.cos(rotation), s = Math.sin(rotation), canopy = treeCanopy(p);
  return Math.abs(c * dx - s * dz) < parcel.width / 2 + canopy && Math.abs(s * dx + c * dz) < parcel.depth / 2 + canopy;
}
export function extensionHidesTree(e: RearExtension, p: TreePoint) {
  if (e.stage === 'survey') return false;
  const dx = p.x - e.x, dz = p.z - e.z, c = Math.cos(e.rotation), s = Math.sin(e.rotation), canopy = treeCanopy(p);
  return Math.abs(c * dx - s * dz) < e.width / 2 + canopy && Math.abs(s * dx + c * dz) < e.depth / 2 + canopy;
}

/**
 * Tree visibility for one world snapshot. Each occluder is registered in every
 * grid cell its widest test can reach, so a tree tests only nearby candidates;
 * the per-pair tests are the same as a scan of every building and road.
 */
export function treeOcclusion(world: World, points: readonly TreePoint[]): (p: TreePoint) => boolean {
  const nodes = new Map<number, Point>(world.nodes.map(n => [n.id, n]));
  const maxCanopy = Math.max(0, ...points.map(treeCanopy));
  const buildingCells = new SpatialCells<Building>(TREE_CELL), roadCells = new SpatialCells<Road>(TREE_CELL);
  for (const b of world.buildings) buildingCells.insert(b, b.x, b.z, buildingTreeReach(b, maxCanopy));
  for (const r of world.roads) {
    const a = nodes.get(r.from), b = nodes.get(r.to);
    if (!a || !b || r.kind === 'expressway') continue;
    const reach = roadTreeClearance(r.kind);
    roadCells.insertBox(r, Math.min(a.x, b.x) - reach, Math.min(a.z, b.z) - reach, Math.max(a.x, b.x) + reach, Math.max(a.z, b.z) + reach);
  }
  // Pocket parks and the shrine grove plant their own trees around furniture.
  const built = new Set(world.buildings.map(b => b.parcel_id));
  const parks = (world.parcels ?? []).filter(p => p.protected && !built.has(p.id));
  const reserves = (world.terrain?.protected ?? []).filter(polygon => polygon.length >= 3).map(polygon => ({
    minX: Math.min(...polygon.map(p => p.x)), maxX: Math.max(...polygon.map(p => p.x)),
    minZ: Math.min(...polygon.map(p => p.z)), maxZ: Math.max(...polygon.map(p => p.z)),
  }));
  const extensions = (world.rear_extensions ?? []).slice(0, 2);
  return p => buildingCells.at(p.x, p.z).some(b => buildingHidesTree(b, p))
    || parks.some(park => parcelHidesTree(park, p))
    || reserves.some(r => p.x > r.minX && p.x < r.maxX && p.z > r.minZ && p.z < r.maxZ)
    || roadCells.at(p.x, p.z).some(r => roadHidesTree(r, nodes, p))
    || extensions.some(e => extensionHidesTree(e, p));
}

/** Uniform grid of items registered in every cell their bounds overlap. */
export class SpatialCells<T> {
  private cells = new Map<string, T[]>();
  private static readonly empty: never[] = [];
  constructor(private size: number) {}
  insert(item: T, x: number, z: number, radius: number) { this.insertBox(item, x - radius, z - radius, x + radius, z + radius); }
  insertBox(item: T, minX: number, minZ: number, maxX: number, maxZ: number) {
    for (let cx = Math.floor(minX / this.size); cx <= Math.floor(maxX / this.size); cx++) for (let cz = Math.floor(minZ / this.size); cz <= Math.floor(maxZ / this.size); cz++) {
      const key = `${cx}:${cz}`, cell = this.cells.get(key);
      if (cell) cell.push(item); else this.cells.set(key, [item]);
    }
  }
  at(x: number, z: number): readonly T[] { return this.cells.get(`${Math.floor(x / this.size)}:${Math.floor(z / this.size)}`) ?? SpatialCells.empty; }
}

export function distanceToSegment(p: Point, a: Point, b: Point) { const dx = b.x - a.x, dz = b.z - a.z; const t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.z - a.z) * dz) / (dx * dx + dz * dz || 1))); return Math.hypot(p.x - a.x - dx * t, p.z - a.z - dz * t); }
