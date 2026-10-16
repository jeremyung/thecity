import { describe, expect, it } from 'vitest';
import { VisibleInfrastructure } from '../web/src/render/visible-infrastructure';
import type { World } from '../web/src/types';

function minimalWorld(overrides: Partial<World> = {}): World {
  return {
    schema_version: 1,
    engine_version: 'test',
    seed: 1,
    tick: 0,
    day: 0,
    minute_of_day: 360,
    development_rate: 1,
    revision: 1,
    stage: 'Founding',
    bounds: { min_x: -100, max_x: 100, min_z: -100, max_z: 100 },
    terrain: { river: [], protected: [], gateways: [] },
    weather: { kind: 'clear', intensity: 0 },
    nodes: [
      { id: 1, x: 0, z: 0, kind: 'street' },
      { id: 2, x: 20, z: 0, kind: 'street' },
      { id: 3, x: 40, z: 0, kind: 'street' },
    ],
    roads: [
      {
        id: 10,
        from: 1,
        to: 2,
        kind: 'local',
        stage: 'open',
        progress: 1,
        connected: true,
        length: 20,
        lanes: 2,
      },
      {
        id: 20,
        from: 2,
        to: 3,
        kind: 'local',
        stage: 'open',
        progress: 1,
        connected: true,
        length: 20,
        lanes: 2,
      },
    ],
    parcels: [],
    buildings: [],
    projects: [],
    trains: [],
    events: [],
    building_access: [
      {
        building_id: 100,
        road_id: 10,
        points: [
          { x: 5, z: 0 },
          { x: 5, z: 5 },
        ],
      },
      {
        building_id: 200,
        road_id: 20,
        points: [
          { x: 25, z: 0 },
          { x: 25, z: 5 },
        ],
      },
    ],
    stats: { population: 0, active_travelers: 0, buildings: 0, active_projects: 0 },
    ...overrides,
  };
}

describe('VisibleInfrastructure', () => {
  it('holds a building until its exact supporting road geometry is committed', () => {
    const world = minimalWorld();
    const gate = new VisibleInfrastructure();
    gate.request(world);
    expect(gate.canPresentBuilding(100)).toBe(false);

    gate.commit(gate.capture(world));
    expect(gate.canPresentBuilding(100)).toBe(true);
  });

  it('waits for every road and platform dependency before presenting movement', () => {
    const world = minimalWorld({
      rail_platforms: [
        {
          node_id: 30,
          x: 40,
          z: 5,
          ux: 1,
          uz: 0,
          surface_y: 4,
          stair_x: 44,
          stair_bottom_z: 5,
          stair_top_z: 5,
          ground_y: 0,
          stair_steps: 8,
          ground_path: [
            { x: 0, z: 0 },
            { x: 7.5, z: -1.5 },
            { x: 7.5, z: -21.5 },
            { x: 4.05, z: -21.5 },
            { x: 4.05, z: -20 },
          ],
          ground_width: 1.6,
          construction_phase: 4,
        },
      ],
    });
    const gate = new VisibleInfrastructure();
    gate.request(world);
    const support = { resolved: true, roads: [10, 20], platforms: [30] };
    expect(gate.canPresentMovement(support)).toBe(false);

    gate.commit(gate.capture(world));
    expect(gate.canPresentMovement(support)).toBe(true);

    const changedRoad = minimalWorld({
      rail_platforms: world.rail_platforms,
      nodes: world.nodes.map((node) => (node.id === 3 ? { ...node, z: 4 } : node)),
    });
    gate.request(changedRoad);
    expect(gate.canPresentMovement(support)).toBe(false);
    gate.commit(gate.capture(changedRoad));
    expect(gate.canPresentMovement(support)).toBe(true);
  });

  it('allows open expressways outside the connected ground graph', () => {
    const world = minimalWorld({
      roads: [
        {
          id: 10,
          from: 1,
          to: 2,
          kind: 'expressway',
          stage: 'open',
          progress: 1,
          connected: false,
          length: 20,
          lanes: 4,
        },
      ],
      road_decks: [
        {
          road_id: 10,
          deck_height: 8,
          controls: [
            [4, 0],
            [16, 0],
          ],
        },
      ],
    });
    const gate = new VisibleInfrastructure();
    gate.request(world);
    gate.commit(gate.capture(world));
    expect(gate.canPresentMovement({ resolved: true, roads: [10], platforms: [] })).toBe(true);

    const closed = minimalWorld({
      roads: world.roads.map((road) => ({ ...road, stage: 'construction' })),
    });
    gate.request(closed);
    gate.commit(gate.capture(closed));
    expect(gate.canPresentMovement({ resolved: true, roads: [10], platforms: [] })).toBe(false);
  });

  it('fails closed when a requested movement road is missing or closed', () => {
    const support = { resolved: true, roads: [10], platforms: [] };
    const gate = new VisibleInfrastructure();
    const missing = minimalWorld({ roads: [] });
    gate.request(missing);
    gate.commit(gate.capture(missing));
    expect(gate.canPresentMovement(support)).toBe(false);

    const closed = minimalWorld({
      roads: minimalWorld().roads.map((road) =>
        road.id === 10 ? { ...road, stage: 'construction' } : road,
      ),
    });
    gate.request(closed);
    gate.commit(gate.capture(closed));
    expect(gate.canPresentMovement(support)).toBe(false);
  });

  it('waits for changed platform geometry even when its node ID is reused', () => {
    const platform = {
      node_id: 30,
      x: 40,
      z: 5,
      ux: 1,
      uz: 0,
      surface_y: 4,
      stair_x: 44,
      stair_bottom_z: 5,
      stair_top_z: 5,
      ground_y: 0,
      stair_steps: 8,
      ground_path: [
        { x: 0, z: 0 },
        { x: 7.5, z: -1.5 },
        { x: 7.5, z: -21.5 },
        { x: 4.05, z: -21.5 },
        { x: 4.05, z: -20 },
      ],
      ground_width: 1.6,
      construction_phase: 4,
    };
    const initial = minimalWorld({ rail_platforms: [platform] });
    const gate = new VisibleInfrastructure();
    gate.request(initial);
    gate.commit(gate.capture(initial));
    const support = { resolved: true, roads: [], platforms: [30] };
    expect(gate.canPresentMovement(support)).toBe(true);

    const identity = gate.captureMovementIdentity(support);
    const changed = minimalWorld({ rail_platforms: [{ ...platform, surface_y: 5 }] });
    gate.request(changed);
    expect(gate.canPresentMovement(support)).toBe(false);
    expect(gate.matchesMovementIdentity(support, identity)).toBe(true);
    gate.commit(gate.capture(changed));
    expect(gate.matchesMovementIdentity(support, identity)).toBe(false);
    expect(gate.canPresentMovement(support)).toBe(true);
  });

  it('requires a committed complete platform, including after a phase regression', () => {
    const platform = {
      node_id: 30,
      x: 40,
      z: 5,
      ux: 1,
      uz: 0,
      surface_y: 8,
      stair_x: 4.05,
      stair_bottom_z: -20,
      stair_top_z: -1,
      ground_y: 0.27,
      stair_steps: 44,
      ground_path: [
        { x: 0, z: 0 },
        { x: 7.5, z: -1.5 },
        { x: 7.5, z: -21.5 },
        { x: 4.05, z: -21.5 },
        { x: 4.05, z: -20 },
      ],
      ground_width: 1.6,
      construction_phase: 4,
    };
    const gate = new VisibleInfrastructure();
    const support = { resolved: true, roads: [], platforms: [30] };
    for (const construction_phase of [0, 1, 2, 3, 4, 2, 4]) {
      const next = minimalWorld({ rail_platforms: [{ ...platform, construction_phase }] });
      gate.request(next);
      expect(gate.canPresentMovement(support)).toBe(false);
      gate.commit(gate.capture(next));
      expect(gate.canPresentMovement(support)).toBe(construction_phase === 4);
    }
  });

  it('rejects unresolved and empty support and ignores unrelated infrastructure changes', () => {
    const initial = minimalWorld();
    const gate = new VisibleInfrastructure();
    gate.request(initial);
    gate.commit(gate.capture(initial));
    expect(gate.canPresentMovement({ resolved: false, roads: [10], platforms: [] })).toBe(false);
    expect(gate.canPresentMovement({ resolved: true, roads: [], platforms: [] })).toBe(false);

    const unrelated = minimalWorld({
      nodes: initial.nodes.map((node) => (node.id === 3 ? { ...node, z: 7 } : node)),
    });
    gate.request(unrelated);
    gate.commit(gate.capture(initial));
    expect(gate.canPresentMovement({ resolved: true, roads: [10], platforms: [] })).toBe(true);
  });

  it('keeps buildings on committed roads ready while another road is staging', () => {
    const initial = minimalWorld();
    const gate = new VisibleInfrastructure();
    gate.request(initial);
    gate.commit(gate.capture(initial));

    const changedSecondRoad = minimalWorld({
      nodes: initial.nodes.map((node) => (node.id === 3 ? { ...node, z: 8 } : node)),
    });
    gate.request(changedSecondRoad);
    gate.commit(gate.capture(initial));

    expect(gate.canPresentBuilding(100)).toBe(true);
    expect(gate.canPresentBuilding(200)).toBe(false);
  });

  it('does not reuse a road identity when endpoints or bridge deck geometry change', () => {
    const initial = minimalWorld({
      roads: [minimalWorld().roads[0]],
      building_access: [minimalWorld().building_access![0]],
      road_decks: [
        {
          road_id: 10,
          deck_height: 7,
          controls: [
            [4, 0],
            [16, 0],
          ],
        },
      ],
    });
    const gate = new VisibleInfrastructure();
    gate.request(initial);
    gate.commit(gate.capture(initial));

    const movedEndpoint = minimalWorld({
      roads: [minimalWorld().roads[0]],
      building_access: [minimalWorld().building_access![0]],
      road_decks: initial.road_decks,
      nodes: initial.nodes.map((node) => (node.id === 2 ? { ...node, x: 22 } : node)),
    });
    gate.request(movedEndpoint);
    expect(gate.canPresentBuilding(100)).toBe(false);
    gate.commit(gate.capture(movedEndpoint));
    expect(gate.canPresentBuilding(100)).toBe(true);

    const changedDeck = minimalWorld({
      roads: movedEndpoint.roads,
      building_access: movedEndpoint.building_access,
      nodes: movedEndpoint.nodes,
      road_decks: [
        {
          road_id: 10,
          deck_height: 9,
          controls: [
            [4, 0],
            [18, 0],
          ],
        },
      ],
    });
    gate.request(changedDeck);
    expect(gate.canPresentBuilding(100)).toBe(false);
  });

  it('includes lanes in road geometry identity for movement dependencies', () => {
    const initial = minimalWorld();
    const gate = new VisibleInfrastructure();
    gate.request(initial);
    gate.commit(gate.capture(initial));
    const support = { resolved: true, roads: [10], platforms: [] };
    expect(gate.canPresentMovement(support)).toBe(true);

    const changed = minimalWorld({
      roads: initial.roads.map((road) => (road.id === 10 ? { ...road, lanes: 3 } : road)),
    });
    gate.request(changed);
    expect(gate.canPresentMovement(support)).toBe(false);
  });

  it.each([
    ['missing road', (world: World) => ({ ...world, roads: [] })],
    [
      'closed road',
      (world: World) => ({
        ...world,
        roads: world.roads.map((road) => ({ ...road, stage: 'construction' })),
      }),
    ],
    [
      'disconnected road',
      (world: World) => ({
        ...world,
        roads: world.roads.map((road) => ({ ...road, connected: false })),
      }),
    ],
    ['missing access dependency', (world: World) => ({ ...world, building_access: [] })],
  ])('fails closed for a %s dependency', (_label, transform) => {
    const gate = new VisibleInfrastructure();
    const world = transform(minimalWorld());
    gate.request(world);
    gate.commit(gate.capture(world));
    expect(gate.canPresentBuilding(100)).toBe(false);
  });

  it('does not let an obsolete intermediate road commit unlock the newest geometry', () => {
    const first = minimalWorld();
    const second = minimalWorld({
      nodes: first.nodes.map((node) => (node.id === 2 ? { ...node, x: 22 } : node)),
    });
    const newest = minimalWorld({
      nodes: first.nodes.map((node) => (node.id === 2 ? { ...node, x: 24 } : node)),
    });
    const gate = new VisibleInfrastructure();
    gate.request(first);
    gate.commit(gate.capture(first));
    gate.request(second);
    gate.request(newest);

    gate.commit(gate.capture(second));
    expect(gate.canPresentBuilding(100)).toBe(false);
    gate.commit(gate.capture(newest));
    expect(gate.canPresentBuilding(100)).toBe(true);
  });

  it('requires a fresh commit after reset even when the new world reuses road IDs', () => {
    const oldWorld = minimalWorld();
    const gate = new VisibleInfrastructure();
    const oldCommit = gate.capture(oldWorld);
    gate.request(oldWorld);
    gate.commit(oldCommit);
    expect(gate.canPresentBuilding(100)).toBe(true);

    gate.reset();
    const replacement = minimalWorld({ revision: 2 });
    gate.request(replacement);
    gate.commit(oldCommit);
    expect(gate.canPresentBuilding(100)).toBe(false);
    expect(gate.canPresentMovement({ resolved: true, roads: [10], platforms: [] })).toBe(false);
    gate.commit(gate.capture(replacement));
    expect(gate.canPresentBuilding(100)).toBe(true);
  });
});
