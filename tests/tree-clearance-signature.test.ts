import { describe, expect, it } from 'vitest';
import { treeClearanceSignature } from '../web/src/render/tree-clearance-signature';
import type { Building, RearExtensionView, World } from '../web/src/types';

function building(overrides: Partial<Building> = {}): Building {
  return {
    id: 100,
    parcel_id: 200,
    x: 3,
    z: 4,
    rotation: 0.2,
    width: 12,
    depth: 8,
    floors: 2,
    kind: 'home',
    stage: 'open',
    progress: 1,
    capacity: 4,
    occupancy: 2,
    built_day: 0,
    name: 'Home',
    appearance_seed: 5,
    history: [],
    anchor: false,
    vacant_days: 0,
    ...overrides,
  };
}

function extension(overrides: Partial<RearExtensionView> = {}): RearExtensionView {
  return {
    project_id: 1,
    building_id: 100,
    x: 20,
    z: 30,
    width: 6,
    depth: 4,
    rotation: 0.1,
    floors: 2,
    stage: 'building',
    progress: 0.4,
    ...overrides,
  };
}

function world(overrides: Partial<World> = {}): World {
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
    ],
    parcels: [],
    buildings: [building()],
    projects: [],
    trains: [],
    events: [],
    ground: [
      {
        block: 1,
        points: [
          [-5, -5],
          [5, -5],
          [5, 5],
          [-5, 5],
        ],
      },
    ],
    stats: { population: 0, active_travelers: 0, buildings: 1, active_projects: 0 },
    ...overrides,
  };
}

describe('treeClearanceSignature', () => {
  it('invalidates same-count ground, building, and road geometry changes', () => {
    const base = world();
    const signature = treeClearanceSignature(base);
    expect(
      treeClearanceSignature(
        world({
          ground: [
            {
              ...base.ground![0],
              points: [
                [-5, -5],
                [6, -5],
                [5, 5],
                [-5, 5],
              ],
            },
          ],
        }),
      ),
    ).not.toBe(signature);

    for (const change of [
      { x: 4 },
      { z: 5 },
      { rotation: 0.3 },
      { width: 13 },
      { depth: 9 },
      { anchor: true },
    ]) {
      expect(treeClearanceSignature(world({ buildings: [building(change)] }))).not.toBe(signature);
    }
    expect(
      treeClearanceSignature(
        world({ nodes: base.nodes.map((node) => (node.id === 2 ? { ...node, x: 21 } : node)) }),
      ),
    ).not.toBe(signature);
    expect(
      treeClearanceSignature(world({ roads: [{ ...base.roads[0], kind: 'arterial' }] })),
    ).not.toBe(signature);
  });

  it('ignores fields outside the clearance predicate', () => {
    const base = world();
    const signature = treeClearanceSignature(base);
    const irrelevant = world({
      tick: 99,
      revision: 42,
      weather: { kind: 'rain', intensity: 0.8 },
      buildings: [building({ progress: 0.3, floors: 12, stage: 'construction' })],
      roads: [{ ...base.roads[0], stage: 'survey', progress: 0.2, connected: false, lanes: 8 }],
      ground: [{ ...base.ground![0], block: 999 }],
    });
    expect(treeClearanceSignature(irrelevant)).toBe(signature);
  });

  it('includes only endpoint nodes used by non-expressway roads', () => {
    const base = world({
      roads: [
        {
          id: 10,
          from: 1,
          to: 2,
          kind: 'local',
          stage: 'survey',
          progress: 0,
          connected: false,
          length: 20,
          lanes: 1,
        },
        {
          id: 11,
          from: 2,
          to: 3,
          kind: 'expressway',
          stage: 'open',
          progress: 1,
          connected: true,
          length: 20,
          lanes: 4,
        },
        {
          id: 12,
          from: 1,
          to: 404,
          kind: 'bridge',
          stage: 'open',
          progress: 1,
          connected: true,
          length: 1,
          lanes: 1,
        },
      ],
    });
    const signature = treeClearanceSignature(base);
    expect(
      treeClearanceSignature(
        world({
          ...base,
          nodes: base.nodes.map((node) => (node.id === 3 ? { ...node, x: 99 } : node)),
        }),
      ),
    ).toBe(signature);
    expect(
      treeClearanceSignature(
        world({
          ...base,
          roads: base.roads.map((road) => (road.id === 11 ? { ...road, kind: 'local' } : road)),
        }),
      ),
    ).not.toBe(signature);
    expect(
      treeClearanceSignature(
        world({
          ...base,
          nodes: base.nodes.map((node) => (node.id === 1 ? { ...node, z: 2 } : node)),
        }),
      ),
    ).not.toBe(signature);
  });

  it('tracks clearance geometry for the first two extensions and survey visibility', () => {
    const first = extension({ project_id: 1 });
    const second = extension({ project_id: 2, x: 40 });
    const third = extension({ project_id: 3, x: 60 });
    const base = world({ rear_extensions: [first, second, third] });
    const signature = treeClearanceSignature(base);
    expect(
      treeClearanceSignature(world({ rear_extensions: [first, second, { ...third, x: 61 }] })),
    ).toBe(signature);
    expect(
      treeClearanceSignature(world({ rear_extensions: [first, { ...second, x: 41 }, third] })),
    ).not.toBe(signature);

    const survey = world({ rear_extensions: [extension({ stage: 'survey' })] });
    const surveySignature = treeClearanceSignature(survey);
    expect(
      treeClearanceSignature(
        world({ rear_extensions: [extension({ stage: 'survey', x: 99, depth: 12 })] }),
      ),
    ).toBe(surveySignature);
    expect(
      treeClearanceSignature(world({ rear_extensions: [extension({ stage: 'building' })] })),
    ).not.toBe(surveySignature);
    expect(
      treeClearanceSignature(world({ rear_extensions: [extension({ stage: 'finishing' })] })),
    ).toBe(treeClearanceSignature(world({ rear_extensions: [extension({ stage: 'building' })] })));
  });

  it('includes each tested rear-extension geometry field', () => {
    const base = world({ rear_extensions: [extension()] });
    const signature = treeClearanceSignature(base);
    for (const change of [{ x: 21 }, { z: 31 }, { rotation: 0.2 }, { width: 7 }, { depth: 5 }]) {
      expect(treeClearanceSignature(world({ rear_extensions: [extension(change)] }))).not.toBe(
        signature,
      );
    }
  });
});
