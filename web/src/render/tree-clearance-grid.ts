import type { Building, RailAlignment, Point, RearExtensionView, Road, WorldNode } from '../types';

const TREE_CELL = 24;

/** Sparse uniform grid. Every item's conservative bounds are registered in all
 * overlapped cells; insertion order is retained in each cell. */
class SpatialCells<T> {
  private readonly cells = new Map<string, T[]>();
  private static readonly empty: readonly never[] = [];

  insertBox(item: T, minX: number, minZ: number, maxX: number, maxZ: number) {
    for (let cx = Math.floor(minX / TREE_CELL); cx <= Math.floor(maxX / TREE_CELL); cx++) {
      for (let cz = Math.floor(minZ / TREE_CELL); cz <= Math.floor(maxZ / TREE_CELL); cz++) {
        const key = `${cx}:${cz}`;
        const cell = this.cells.get(key);
        if (cell) cell.push(item);
        else this.cells.set(key, [item]);
      }
    }
  }

  at(x: number, z: number): readonly T[] {
    return (
      this.cells.get(`${Math.floor(x / TREE_CELL)}:${Math.floor(z / TREE_CELL)}`) ??
      SpatialCells.empty
    );
  }
}

export interface TreeClearanceCandidates {
  buildings: readonly Building[];
  roads: readonly Road[];
  rearExtensions: readonly RearExtensionView[];
}

/** Build broad-phase candidates for the existing exact tree clearance checks. */
export function createTreeClearanceGrid(
  buildings: readonly Building[],
  roads: readonly Road[],
  rearExtensions: readonly RearExtensionView[],
  nodes: readonly WorldNode[],
  maxCanopy: number,
  roadReach: (kind: string) => number,
  railAlignments: readonly RailAlignment[] = [],
): (point: Point) => TreeClearanceCandidates {
  const railById = new Map(railAlignments.map((path) => [path.road_id, path]));
  const nodeById = new Map(nodes.map((node) => [node.id, node]));
  const buildingCells = new SpatialCells<Building>();
  const roadCells = new SpatialCells<Road>();
  const extensionCells = new SpatialCells<RearExtensionView>();

  for (const building of buildings) {
    // Triangle inequality covers rotated corners, asymmetric anchor rear depth,
    // and the widest tree canopy used by the exact predicate.
    const radius =
      Math.hypot(building.width / 2, building.depth / 2 + (building.anchor ? 4 : 0)) +
      maxCanopy * Math.SQRT2;
    buildingCells.insertBox(
      building,
      building.x - radius,
      building.z - radius,
      building.x + radius,
      building.z + radius,
    );
  }
  for (const road of roads) {
    if (road.kind === 'expressway') continue;
    const a = nodeById.get(road.from),
      b = nodeById.get(road.to);
    if (!a || !b) continue;
    const reach = roadReach(road.kind);
    const samples = railById.get(road.id)?.samples;
    const xs = samples ? samples.map((s) => s[1]) : [a.x, b.x];
    const zs = samples ? samples.map((s) => s[2]) : [a.z, b.z];
    roadCells.insertBox(
      road,
      Math.min(...xs) - reach,
      Math.min(...zs) - reach,
      Math.max(...xs) + reach,
      Math.max(...zs) + reach,
    );
  }
  for (const extension of rearExtensions) {
    if (extension.stage === 'survey') continue;
    const radius = Math.hypot(extension.width / 2, extension.depth / 2) + maxCanopy * Math.SQRT2;
    extensionCells.insertBox(
      extension,
      extension.x - radius,
      extension.z - radius,
      extension.x + radius,
      extension.z + radius,
    );
  }

  return (point) => ({
    buildings: buildingCells.at(point.x, point.z),
    roads: roadCells.at(point.x, point.z),
    rearExtensions: extensionCells.at(point.x, point.z),
  });
}
