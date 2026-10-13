import { describe, expect, it } from 'vitest';
import { MovementPresentation } from '../web/src/render/movement-presentation';
import { VisibleInfrastructure } from '../web/src/render/visible-infrastructure';
import type { MovementSupport } from '../web/src/movement-support';
import type { World } from '../web/src/types';

function world(endpointX = 10, includeSecondRoad = false): World {
  const base = {
    schema_version: 1, engine_version: 'test', seed: 1, tick: 0, day: 0, minute_of_day: 360,
    development_rate: 1, revision: 1, stage: 'Founding',
    bounds: { min_x: -100, max_x: 100, min_z: -100, max_z: 100 },
    terrain: { river: [], protected: [], gateways: [] }, weather: { kind: 'clear' as const, intensity: 0 },
    nodes: [
      { id: 1, x: 0, z: 0, kind: 'street' },
      { id: 2, x: endpointX, z: 0, kind: 'street' },
      { id: 3, x: 20, z: 0, kind: 'street' },
    ],
    roads: [
      { id: 10, from: 1, to: 2, kind: 'local', stage: 'open', progress: 1, connected: true, length: endpointX, lanes: 2 },
    ],
    parcels: [], buildings: [], projects: [], trains: [], events: [],
    stats: { population: 0, active_travelers: 0, buildings: 0, active_projects: 0 },
  } satisfies World;
  return includeSecondRoad ? {
    ...base,
    roads: [...base.roads, { id: 20, from: 2, to: 3, kind: 'local', stage: 'open', progress: 1, connected: true, length: 20 - endpointX, lanes: 2 }],
  } : base;
}

function pose(id: number, x: number, heading = 0): Float32Array {
  return new Float32Array([id, x, 0, heading, 0, -1]);
}

const support = (...roads: number[]): MovementSupport => ({ resolved: true, roads, platforms: [] });

function committedGate(snapshot: World): VisibleInfrastructure {
  const gate = new VisibleInfrastructure();
  gate.request(snapshot);
  gate.commit(gate.capture(snapshot));
  return gate;
}

describe('MovementPresentation', () => {
  it('freezes the last rendered pose while changed road geometry stages', () => {
    const initial = world();
    const gate = committedGate(initial);
    const presentation = new MovementPresentation();
    presentation.receive(pose(42, 2), new Map([[42, support(10)]]), 50);
    presentation.sample(0, gate);
    expect(presentation.sample(50, gate).get(42)?.x).toBe(2);

    const changed = world(12);
    gate.request(changed);
    presentation.receive(pose(42, 30), new Map([[42, support(10)]]), 50);
    expect(presentation.sample(100, gate).get(42)?.x).toBe(2);
  });

  it('withholds a new actor until all supporting roads are committed', () => {
    const initial = world();
    const gate = committedGate(initial);
    const changed = world(10, true);
    gate.request(changed);
    const presentation = new MovementPresentation();
    presentation.receive(pose(70, 18), new Map([[70, support(20)]]), 50);
    expect(presentation.sample(0, gate).has(70)).toBe(false);
    gate.commit(gate.capture(changed));
    expect(presentation.sample(1, gate).get(70)?.x).toBe(18);
  });

  it('releases the latest paused packet after a later infrastructure commit', () => {
    const initial = world();
    const gate = committedGate(initial);
    const presentation = new MovementPresentation();
    presentation.receive(pose(42, 2), new Map([[42, support(10)]]), 50);
    presentation.sample(0, gate);
    expect(presentation.sample(50, gate).get(42)?.x).toBe(2);

    const changed = world(10, true);
    gate.request(changed);
    presentation.receive(pose(42, 12), new Map([[42, support(10, 20)]]), 50);
    expect(presentation.sample(60, gate).get(42)?.x).toBe(2);

    gate.commit(gate.capture(changed));
    expect(presentation.sample(70, gate).get(42)?.x).toBe(2);
    expect(presentation.sample(95, gate).get(42)?.x).toBe(7);
  });

  it('retires actors absent from the latest packet', () => {
    const gate = committedGate(world());
    const presentation = new MovementPresentation();
    presentation.receive(pose(42, 2), new Map([[42, support(10)]]), 50);
    expect(presentation.sample(0, gate).has(42)).toBe(true);
    presentation.receive(new Float32Array(), new Map(), 50);
    expect(presentation.sample(50, gate).has(42)).toBe(false);
  });

  it('requires a fresh commit after reset even when actor and road IDs are reused', () => {
    const snapshot = world();
    const gate = committedGate(snapshot);
    const presentation = new MovementPresentation();
    presentation.receive(pose(42, 2), new Map([[42, support(10)]]), 50);
    expect(presentation.sample(0, gate).has(42)).toBe(true);

    gate.reset();
    presentation.reset();
    gate.request(snapshot);
    presentation.receive(pose(42, 4), new Map([[42, support(10)]]), 50);
    expect(presentation.sample(10, gate).has(42)).toBe(false);
    gate.commit(gate.capture(snapshot));
    expect(presentation.sample(11, gate).get(42)?.x).toBe(4);
  });

  it('drops a pose whose committed supporting geometry was invalidated', () => {
    const initial = world();
    const gate = committedGate(initial);
    const presentation = new MovementPresentation();
    presentation.receive(pose(42, 2), new Map([[42, support(10)]]), 50);
    presentation.sample(0, gate);

    const changed = world(12);
    gate.request(changed);
    presentation.receive(pose(42, 20), new Map([[42, support(10)]]), 50);
    expect(presentation.sample(50, gate).get(42)?.x).toBe(2);
    gate.commit(gate.capture(changed));
    expect(presentation.sample(60, gate).get(42)?.x).toBe(20);
  });

  it('stops interpolation when source support disappears but destination support remains', () => {
    const initial = world(10, true);
    const gate = committedGate(initial);
    const presentation = new MovementPresentation();
    presentation.receive(pose(42, 0), new Map([[42, support(10)]]), 50);
    presentation.sample(0, gate);
    presentation.receive(pose(42, 10), new Map([[42, support(20)]]), 50);
    presentation.sample(10, gate);
    expect(presentation.sample(35, gate).get(42)?.x).toBe(5);

    const destinationOnly: World = { ...initial, roads: initial.roads.filter(road => road.id === 20) };
    gate.request(destinationOnly);
    gate.commit(gate.capture(destinationOnly));
    expect(presentation.sample(36, gate).get(42)?.x).toBe(10);
  });

  it('copies packet poses and dependency arrays before the caller can mutate them', () => {
    const gate = committedGate(world());
    const presentation = new MovementPresentation();
    const movement = pose(42, 3);
    const roads = [10];
    presentation.receive(movement, new Map([[42, { resolved: true, roads, platforms: [] }]]), 50);
    movement[1] = 99;
    roads[0] = 999;
    expect(presentation.sample(0, gate).get(42)?.x).toBe(3);
  });

  it('interpolates headings across the short arc at the wraparound', () => {
    const gate = committedGate(world());
    const presentation = new MovementPresentation();
    presentation.receive(pose(42, 0, 3.1), new Map([[42, support(10)]]), 50);
    presentation.sample(0, gate);
    presentation.receive(pose(42, 0, -3.1), new Map([[42, support(10)]]), 50);
    const halfway = presentation.sample(75, gate).get(42)!.h;
    expect(Math.abs(halfway)).toBeGreaterThan(3.0);
    expect(Math.abs(halfway)).toBeLessThan(Math.PI);
  });
});
