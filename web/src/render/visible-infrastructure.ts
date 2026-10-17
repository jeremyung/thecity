import type { World } from '../types';
import type { MovementSupport } from '../movement-support';

/** Geometry identities, not merely simulation-open flags, determine visibility. */
export function roadGeometryIdentities(world: World): Map<number, string> {
  const nodes = new Map(world.nodes.map((node) => [node.id, node]));
  const decks = new Map((world.road_decks ?? []).map((deck) => [deck.road_id, deck]));
  const result = new Map<number, string>();
  for (const road of world.roads) {
    const a = nodes.get(road.from),
      b = nodes.get(road.to);
    if (road.stage !== 'open' || (!road.connected && road.kind !== 'expressway') || !a || !b)
      continue;
    result.set(
      road.id,
      JSON.stringify([road.kind, a.x, a.z, b.x, b.z, road.lanes, decks.get(road.id) ?? null]),
    );
  }
  return result;
}

/** Rail platform IDs in movement packets are the supporting platform node IDs. */
export function platformGeometryIdentities(world: World): Map<number, string> {
  // Only complete platforms can support service, even if unfinished deck exists.
  return new Map(
    (world.rail_platforms ?? [])
      .filter((platform) => platform.construction_phase === 4)
      .map((platform) => [
        platform.node_id,
        JSON.stringify([
          platform.x,
          platform.z,
          platform.ux,
          platform.uz,
          platform.surface_y,
          platform.stair_x,
          platform.stair_bottom_z,
          platform.stair_top_z,
          platform.ground_y,
          platform.stair_steps,
          platform.construction_phase,
          platform.ground_path,
          platform.ground_width,
        ]),
      ]),
  );
}

export interface MovementGeometryIdentity {
  roads: number[];
  platforms: number[];
}

export interface InfrastructureCommit {
  epoch: number;
  roads: ReadonlyMap<number, string>;
  platforms: ReadonlyMap<number, string>;
}

/** Dependencies are checked against the actually committed batch. Movement
 * callers must first establish that request() describes the packet's geometry;
 * matching IDs alone cannot establish freshness across worker messages.
 */
export class VisibleInfrastructure {
  private epoch = 0;
  private nextGeometryRevision = 1;
  private roadRevisions = new Map<number, number>();
  private platformRevisions = new Map<number, number>();
  private requested = new Map<number, string>();
  private committed = new Map<number, string>();
  private requestedPlatforms = new Map<number, string>();
  private committedPlatforms = new Map<number, string>();
  private buildingRoads = new Map<number, number>();

  request(world: World): void {
    this.requested = roadGeometryIdentities(world);
    this.requestedPlatforms = platformGeometryIdentities(world);
    this.buildingRoads = new Map(
      (world.building_access ?? []).map((access) => [access.building_id, access.road_id]),
    );
  }

  capture(world: World): InfrastructureCommit {
    return {
      epoch: this.epoch,
      roads: roadGeometryIdentities(world),
      platforms: platformGeometryIdentities(world),
    };
  }

  commit(candidate: InfrastructureCommit): void {
    if (candidate.epoch !== this.epoch) return;
    this.roadRevisions = this.revisions(candidate.roads, this.committed, this.roadRevisions);
    this.platformRevisions = this.revisions(
      candidate.platforms,
      this.committedPlatforms,
      this.platformRevisions,
    );
    this.committed = new Map(candidate.roads);
    this.committedPlatforms = new Map(candidate.platforms);
  }

  canPresentBuilding(id: number): boolean {
    const road = this.buildingRoads.get(id);
    if (road === undefined) return false;
    const requested = this.requested.get(road);
    return requested !== undefined && this.committed.get(road) === requested;
  }

  canPresentMovement(support: MovementSupport): boolean {
    if (!support.resolved || support.roads.length + support.platforms.length === 0) return false;
    return (
      support.roads.every((id) => {
        const requested = this.requested.get(id);
        return requested !== undefined && this.committed.get(id) === requested;
      }) &&
      support.platforms.every((id) => {
        const requested = this.requestedPlatforms.get(id);
        return requested !== undefined && this.committedPlatforms.get(id) === requested;
      })
    );
  }

  private revisions(
    next: ReadonlyMap<number, string>,
    previous: ReadonlyMap<number, string>,
    revisions: ReadonlyMap<number, number>,
  ): Map<number, number> {
    const result = new Map<number, number>();
    for (const [id, geometry] of next) {
      result.set(
        id,
        previous.get(id) === geometry ? revisions.get(id)! : this.nextGeometryRevision++,
      );
    }
    return result;
  }

  /** Capture only when accepting a target or retiring interpolation support.
   * Frame checks use numeric revisions; unchanged geometry keeps its revision. */
  captureMovementIdentity(support: MovementSupport): MovementGeometryIdentity {
    return {
      roads: support.roads.map((id) => this.roadRevisions.get(id)!),
      platforms: support.platforms.map((id) => this.platformRevisions.get(id)!),
    };
  }

  matchesMovementIdentity(support: MovementSupport, identity: MovementGeometryIdentity): boolean {
    if (!support.resolved || support.roads.length + support.platforms.length === 0) return false;
    for (let i = 0; i < support.roads.length; i++) {
      const revision = this.roadRevisions.get(support.roads[i]);
      if (revision === undefined || revision !== identity.roads[i]) return false;
    }
    for (let i = 0; i < support.platforms.length; i++) {
      const revision = this.platformRevisions.get(support.platforms[i]);
      if (revision === undefined || revision !== identity.platforms[i]) return false;
    }
    return true;
  }

  reset(): void {
    this.epoch++;
    this.roadRevisions.clear();
    this.platformRevisions.clear();
    this.requested.clear();
    this.committed.clear();
    this.requestedPlatforms.clear();
    this.committedPlatforms.clear();
    this.buildingRoads.clear();
  }
}
