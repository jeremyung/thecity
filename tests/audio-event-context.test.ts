import { describe, expect, it } from 'vitest';
import { resolveAudioEventContext } from '../web/src/audio/event-context';
import type { Building, Road, World, WorldEvent, WorldNode, EventKind } from '../web/src/types';

type Snapshot = Pick<World, 'events' | 'roads' | 'nodes' | 'buildings'>;

function snapshot(overrides: Partial<Snapshot> = {}): Snapshot {
  return {
    events: [],
    roads: [],
    nodes: [],
    buildings: [],
    ...overrides,
  };
}

function event(id: number, tick: number, kind: EventKind, entity_id: number): WorldEvent {
  return { id, tick, kind, entity_id, reason: 'engine event' };
}

describe('resolveAudioEventContext', () => {
  it('resolves road openings from actual endpoint nodes and building events from the snapshot', () => {
    const roads: Road[] = [
      {
        id: 30,
        from: 4,
        to: 8,
        kind: 'local' as const,
        stage: 'open' as const,
        progress: 1,
        connected: true,
        length: 20,
        lanes: 2,
      },
    ];
    const nodes: WorldNode[] = [
      { id: 4, x: -12, z: 8, kind: 'junction' as const },
      { id: 8, x: 24, z: -4, kind: 'junction' as const },
    ];
    const buildings: Building[] = [
      {
        business_active: false,
        id: 51,
        parcel_id: 7,
        x: 6,
        z: 17,
        rotation: 0,
        width: 12,
        depth: 16,
        floors: 2,
        kind: 'shop' as const,
        stage: 'open' as const,
        progress: 1,
        capacity: 0,
        occupancy: 0,
        built_day: 1,
        name: 'Corner Shop',
        appearance_seed: 11,
        history: [],
        anchor: false,
        vacant_days: 0,
      },
    ];
    const events = [event(1, 100, 'RoadOpened', 30), event(2, 101, 'BuildingOpened', 51)];

    expect(
      resolveAudioEventContext(snapshot({ events, roads, nodes, buildings }), new Map(), 101),
    ).toEqual([
      { ...events[0], x: 6, z: 2 },
      { ...events[1], x: 6, z: 17 },
    ]);
  });

  it("uses interpolated movement for train/bus arrivals and resolves the engine's BusinessReopened event", () => {
    const buildings: Building[] = [
      {
        business_active: false,
        id: 90,
        parcel_id: 7,
        x: -8,
        z: 15,
        rotation: 0,
        width: 12,
        depth: 16,
        floors: 1,
        kind: 'cafe' as const,
        stage: 'open' as const,
        progress: 1,
        capacity: 0,
        occupancy: 0,
        built_day: 1,
        name: 'Cafe',
        appearance_seed: 11,
        history: [],
        anchor: false,
        vacant_days: 0,
      },
    ];
    const events = [
      event(1, 200, 'TrainStationStop', 4),
      event(2, 200, 'BusStop', 5),
      event(3, 201, 'BusinessReopened', 90),
    ];
    const movement = new Map([
      [4, { x: 35.5, z: 12.25, h: 0, k: 2 }],
      [5, { x: -3, z: 19, h: 0, k: 3 }],
    ]);

    expect(resolveAudioEventContext(snapshot({ events, buildings }), movement, 201)).toEqual([
      { ...events[0], x: 35.5, z: 12.25 },
      { ...events[1], x: -3, z: 19 },
      { ...events[2], x: -8, z: 15 },
    ]);
  });

  it('omits stale/future/unsupported events and never invents positions for missing references', () => {
    const events = [
      event(1, 1, 'RoadOpened', 90),
      event(2, 101, 'TrainStationStop', 404),
      event(3, 101, 'BusStop', 405),
      event(4, 101, 'BuildingOpened', 999),
      event(5, 101, 'UnknownCue' as unknown as EventKind, 1),
      event(6, 103, 'RoadOpened', 90),
      event(7, 101, 'RoadOpened', 91),
    ];
    const movement = new Map([[404, { x: Number.NaN, z: 4 }]]);
    const roads: Road[] = [
      {
        id: 91,
        from: 1,
        to: 2,
        kind: 'local' as const,
        stage: 'open' as const,
        progress: 1,
        connected: true,
        length: 12,
        lanes: 2,
      },
    ];
    const nodes: WorldNode[] = [{ id: 1, x: 4, z: 5, kind: 'junction' as const }]; // The destination node is absent.
    expect(resolveAudioEventContext(snapshot({ events, roads, nodes }), movement, 102)).toEqual([]);
  });

  it('returns only the latest eight resolvable events in chronological order', () => {
    const buildings: Building[] = Array.from({ length: 10 }, (_, index) => ({
      id: index + 1,
      parcel_id: index + 1,
      x: index * 3,
      z: -index,
      rotation: 0,
      width: 10,
      depth: 12,
      floors: 1,
      kind: 'home' as const,
      stage: 'open' as const,
      progress: 1,
      capacity: 4,
      occupancy: 2,
      business_active: false,
      built_day: 1,
      name: `Home ${index}`,
      appearance_seed: index,
      history: [],
      anchor: false,
      vacant_days: 0,
    }));
    const events = buildings.map((building, index) =>
      event(index + 1, 100 + index, 'BuildingOpened', building.id),
    );
    const result = resolveAudioEventContext(snapshot({ events, buildings }), new Map(), 109);
    expect(result.map((item) => item.id)).toEqual([3, 4, 5, 6, 7, 8, 9, 10]);
  });
});
