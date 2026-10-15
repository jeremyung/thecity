import type { World } from '../types';

/** Inputs read by CityRenderer's tree clearance predicate, excluding tree points. */
export function treeClearanceSignature(world: World): string {
  const nodes = new Map(world.nodes.map(node => [node.id, node]));
  const roads = world.roads.flatMap(road => {
    if (road.kind === 'expressway') return [];
    const from = nodes.get(road.from), to = nodes.get(road.to);
    return from && to ? [[road.kind, from.x, from.z, to.x, to.z]] : [];
  });
  const buildings = world.buildings.map(building => [
    building.x, building.z, building.rotation, building.width, building.depth, !!building.anchor,
  ]);
  const ground = (world.ground ?? []).map(patch => patch.points);
  const rearExtensions = (world.rear_extensions ?? []).slice(0, 2).filter(extension => extension.stage !== 'survey').map(extension => [
    extension.x, extension.z, extension.rotation, extension.width, extension.depth,
  ]);
  const platforms = (world.rail_platforms ?? []).map(p => [
    p.x, p.z, p.ux, p.uz, p.stair_x, p.stair_bottom_z, p.stair_top_z, p.ground_path, p.ground_width,
  ]);
  return JSON.stringify([ground, buildings, roads, rearExtensions, platforms, world.rail_alignments ?? []]);
}
