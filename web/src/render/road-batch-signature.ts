import type { World } from '../types';

/** Completed infrastructure depends on every crossing/reservation, but not on
 * its construction progress. Keep attached batches available to style traversal.
 * Input ordering is retained: lighting has a placement cap and kit order matters.
 */
export function roadBatchSignatures(world: World) {
  const completed = JSON.stringify([
    world.nodes.map(n => [n.id, n.x, n.z, n.kind]),
    world.roads.map(r => [r.id, r.from, r.to, r.kind, r.stage === 'open', r.connected, r.lanes]),
    world.expressway_profile ?? null, world.road_decks ?? [], world.rail_platforms ?? [], world.rail_alignments ?? [],
    world.terrain.river, world.terrain.river_half_width ?? 32,
  ]);
  // Retain the existing twelve-step construction display cadence. Completed
  // roads ignore progress; open status is captured in the completed key above.
  const construction = JSON.stringify([
    world.roads.filter(r => r.stage !== 'open').map(r => [r.id, r.stage, Math.floor(r.progress * 12)]),
    world.projects.filter(p => p.kind === 'road' && p.progress < 1)
      .map(p => [p.id, p.from, p.to, p.stage, Math.floor(p.progress * 12)]),
  ]);
  return { completed, construction };
}
