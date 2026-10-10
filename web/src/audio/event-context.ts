import type { Building, Point, Road, World, WorldEvent, WorldNode } from "../types";

export interface PositionedAudioEvent extends WorldEvent {
  x: number;
  z: number;
}

type AudioSnapshot = Pick<World, "events" | "roads" | "nodes" | "buildings">;
type MovingPosition = Pick<Point, "x" | "z">;

const AUDIO_EVENT_KINDS = new Set([
  "TrainStationStop",
  "BusStop",
  "RoadOpened",
  "BuildingOpened",
  "BusinessOpened",
  "BusinessReopened",
]);

function finitePosition(position: MovingPosition | undefined): position is MovingPosition {
  return position !== undefined && Number.isFinite(position.x) && Number.isFinite(position.z);
}

function buildingPosition(buildings: readonly Building[], id: number): MovingPosition | undefined {
  const building = buildings.find((item) => item.id === id);
  return building && finitePosition(building) ? building : undefined;
}

function roadMidpoint(roads: readonly Road[], nodes: readonly WorldNode[], id: number): MovingPosition | undefined {
  const road = roads.find((item) => item.id === id);
  if (!road) return undefined;
  const from = nodes.find((node) => node.id === road.from);
  const to = nodes.find((node) => node.id === road.to);
  if (!finitePosition(from) || !finitePosition(to)) return undefined;
  const x = (from.x + to.x) * 0.5;
  const z = (from.z + to.z) * 0.5;
  return Number.isFinite(x) && Number.isFinite(z) ? { x, z } : undefined;
}

/**
 * Resolve the small recent-event window into real world positions for audio.
 * Events with stale ticks, unsupported kinds, missing entities, or invalid
 * coordinates are omitted rather than placed at an invented origin.
 */
export function resolveAudioEventContext(
  snapshot: AudioSnapshot | null,
  moving: ReadonlyMap<number, MovingPosition>,
  currentTick: number,
): PositionedAudioEvent[] {
  if (!snapshot || !Number.isFinite(currentTick)) return [];
  const positioned: PositionedAudioEvent[] = [];

  // Event history is chronological. Walk backward so the output is the latest
  // eight resolvable events while keeping the amount of work bounded by the
  // engine's capped event list and stopping once entries are stale.
  for (let index = snapshot.events.length - 1; index >= 0 && positioned.length < 8; index--) {
    const event = snapshot.events[index];
    const age = currentTick - event.tick;
    if (age >= 60) break;
    if (age < 0 || !AUDIO_EVENT_KINDS.has(event.kind)) continue;

    let position: MovingPosition | undefined;
    if (event.kind === "TrainStationStop" || event.kind === "BusStop") {
      const movingPosition = moving.get(event.entity_id);
      if (finitePosition(movingPosition)) position = movingPosition;
    } else if (event.kind === "RoadOpened") {
      position = roadMidpoint(snapshot.roads, snapshot.nodes, event.entity_id);
    } else {
      position = buildingPosition(snapshot.buildings, event.entity_id);
    }

    if (!finitePosition(position)) continue;
    positioned.push({ ...event, x: position.x, z: position.z });
  }

  positioned.reverse();
  return positioned;
}
