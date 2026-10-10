// @ts-expect-error Node built-in types are not included in the app TypeScript config.
import { readFileSync } from 'node:fs';
import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

type TransferState = 'home' | 'walking_to_station' | 'waiting_station' | 'riding_outbound' | 'working_external' | 'waiting_gateway' | 'riding_inbound' | 'walking_home';
type EngineSnapshot = {
  seed: number;
  tick: number;
  stage: string;
  residents: Array<{ id: number; name: string; activity: string; transfer?: TransferState; train_id?: number | null; location_node?: number | null }>;
  trains: Array<{ id: number; route: number[]; progress: number; direction: number; passengers?: number; capacity?: number; station_waiting?: number; gateway_waiting?: number }>;
};
type EngineSave = {
  tick: number;
  next_id: number;
  residents: Array<{ id: number; transfer?: TransferState; train_id?: number | null; queue_slot?: number | null }>;
  travelers: Array<{
    id: number;
    resident: number;
    route: number[];
    edge_index: number;
    edge_progress: number;
    access_phase: number;
    access_progress: number;
    origin_access: { points: Array<{ x: number; z: number }> } | null;
    destination_access: { points: Array<{ x: number; z: number }> } | null;
  }>;
  nodes: Array<{ id: number; x: number; z: number; kind: string }>;
  trains: Array<{ id: number; route: number[]; passengers?: number[] }>;
};

const engines: CityEngine[] = [];
const makeEngine = (seed = 87_231) => {
  const engine = new CityEngine(seed);
  engines.push(engine);
  return engine;
};

function snapshot(engine: CityEngine): EngineSnapshot {
  return JSON.parse(engine.snapshot()) as EngineSnapshot;
}

function saveState(engine: CityEngine): EngineSave {
  return JSON.parse(engine.save()) as EngineSave;
}

function advanceTo(engine: CityEngine, tick: number) {
  for (let advanced = 0; advanced < tick; advanced += 2_000) {
    engine.step(Math.min(2_000, tick - advanced));
  }
}

function findOnboard(engine: CityEngine, maxBatches = 48): EngineSnapshot | null {
  for (let batch = 0; batch < maxBatches; batch++) {
    engine.step(2_000);
    const world = snapshot(engine);
    if (world.residents.some((resident) => resident.train_id !== null && resident.train_id !== undefined)) return world;
  }
  return null;
}

function findMovement(movements: Float32Array, id: number) {
  for (let offset = 0; offset < movements.length; offset += 5) {
    if (movements[offset] === id) return [movements[offset + 1], movements[offset + 2], movements[offset + 3], movements[offset + 4]];
  }
  return null;
}

function sampleAccessPath(points: Array<{ x: number; z: number }>, distance: number) {
  let remaining = distance;
  for (let i = 0; i < points.length - 1; i++) {
    const from = points[i];
    const to = points[i + 1];
    const dx = to.x - from.x;
    const dz = to.z - from.z;
    const length = Math.hypot(dx, dz);
    if (remaining <= length || i === points.length - 2) {
      const t = length > 0 ? Math.min(1, remaining / length) : 0;
      return { x: from.x + dx * t, z: from.z + dz * t };
    }
    remaining -= length;
  }
  return points[points.length - 1];
}

function expectQueuePlatform(movements: Float32Array, resident: EngineSave['residents'][number], state: EngineSave) {
  const atStation = resident.transfer === 'waiting_station';
  const train = state.trains[0];
  const stationNode = state.nodes.find((node) => node.kind === 'station' && train.route.includes(node.id));
  expect(stationNode).toBeDefined();
  const terminalId = atStation ? stationNode!.id : train.route.find((nodeId) => nodeId !== stationNode!.id)!;
  const terminal = state.nodes.find((node) => node.id === terminalId);
  const axisEnd = state.nodes.find((node) => node.id === train.route.find((nodeId) => nodeId !== stationNode!.id));
  expect(terminal).toBeDefined();
  expect(axisEnd).toBeDefined();

  const axisLength = Math.hypot(axisEnd!.x - stationNode!.x, axisEnd!.z - stationNode!.z);
  const axisX = (axisEnd!.x - stationNode!.x) / axisLength;
  const axisZ = (axisEnd!.z - stationNode!.z) / axisLength;
  const direction = atStation ? 1 : -1;
  const along = 12 + Math.floor(resident.queue_slot! / 8) * 2.4;
  const across = 5.2 + (resident.queue_slot! % 8) * 1.3;
  const expectedX = terminal!.x + axisX * direction * along - axisZ * across;
  const expectedZ = terminal!.z + axisZ * direction * along + axisX * across;
  const movement = findMovement(movements, resident.id);
  expect(movement).not.toBeNull();
  expect(Math.hypot(movement![0] - expectedX, movement![1] - expectedZ)).toBeLessThan(0.03);
  expect(movement![3]).toBe(0);

  const queuedAtSameTerminal = state.residents.filter((item) => item.transfer === resident.transfer && item.queue_slot !== null && item.queue_slot !== undefined);
  const positions = queuedAtSameTerminal.map((item) => findMovement(movements, item.id)).filter(Boolean) as number[][];
  expect(new Set(queuedAtSameTerminal.map((item) => item.queue_slot)).size).toBe(queuedAtSameTerminal.length);
  expect(positions.length).toBe(queuedAtSameTerminal.length);
  const positionKeys = positions.map(([x, z]) => `${x.toFixed(3)}:${z.toFixed(3)}`);
  expect(new Set(positionKeys).size).toBe(positionKeys.length);
}

function expectConsistentBoarding(world: EngineSnapshot, state: EngineSave, movements: Float32Array) {
  const residents = new Map(state.residents.map((resident) => [resident.id, resident]));
  const travelerIds = new Set(state.travelers.map((traveler) => traveler.id));
  const trainIds = new Set(state.trains.map((train) => train.id));
  const passengerIds: number[] = [];

  expect(new Set(state.travelers.map((traveler) => traveler.id)).size).toBe(state.travelers.length);
  expect(new Set(state.trains.map((train) => train.id)).size).toBe(state.trains.length);
  for (const train of state.trains) {
    const passengerList = train.passengers ?? [];
    passengerIds.push(...passengerList);
    expect(new Set(passengerList).size).toBe(passengerList.length);
  }
  expect(new Set(passengerIds).size).toBe(passengerIds.length);

  const worldTrains = new Map(world.trains.map((train) => [train.id, train]));
  for (const train of state.trains) {
    const passengerList = train.passengers ?? [];
    const view = worldTrains.get(train.id);
    expect(view).toBeDefined();
    expect(view!.passengers).toBe(passengerList.length);
    expect(view!.capacity).toBeGreaterThan(0);
    expect(passengerList.length).toBeLessThanOrEqual(view!.capacity!);
    expect(view!.station_waiting).toBeGreaterThanOrEqual(0);
    expect(view!.station_waiting).toBeLessThanOrEqual(96);
    expect(view!.gateway_waiting).toBeGreaterThanOrEqual(0);
    expect(view!.gateway_waiting).toBeLessThanOrEqual(96);
    expect(Number.isFinite(view!.progress)).toBe(true);
    expect(view!.progress).toBeGreaterThanOrEqual(0);
    for (const passengerId of passengerList) {
      expect(residents.get(passengerId)?.train_id).toBe(train.id);
      expect(travelerIds.has(passengerId)).toBe(false);
      expect(trainIds.has(passengerId)).toBe(false);
    }
  }
  for (const resident of state.residents) {
    const isRiding = resident.transfer === 'riding_inbound' || resident.transfer === 'riding_outbound';
    expect(isRiding).toBe(resident.train_id !== null && resident.train_id !== undefined);
    if (resident.train_id !== null && resident.train_id !== undefined) {
      expect(passengerIds.filter((id) => id === resident.id)).toHaveLength(1);
    }
  }

  expect(movements.length % 5).toBe(0);
  const movementIds: number[] = [];
  for (let offset = 0; offset < movements.length; offset += 5) movementIds.push(movements[offset]);
  expect(new Set(movementIds).size).toBe(movementIds.length);
  for (const residentId of passengerIds) expect(movementIds).not.toContain(residentId);
  for (const trainId of trainIds) expect(movementIds.filter((id) => id === trainId)).toHaveLength(1);
}

beforeAll(() => {
  const wasm = readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url));
  initSync({ module: wasm });
});

afterEach(() => {
  for (const engine of engines.splice(0)) engine.free();
});

describe('compiled WASM rail transfers', () => {
  it('publishes bounded train capacity and a consistent resident transfer roster', () => {
    const engine = makeEngine();
    engine.set_development_rate(16);
    advanceTo(engine, 72_000);

    const world = snapshot(engine);
    const state = saveState(engine);
    expect(world.seed).toBe(87_231);
    expect(world.tick).toBe(72_000);
    expect(world.residents.length).toBeGreaterThan(0);
    expect(world.residents.some((resident) => Boolean(resident.transfer))).toBe(true);
    expect(world.trains.length).toBeGreaterThan(0);
    expectConsistentBoarding(world, state, engine.movements());
  }, 60_000);

  it('moves through building door connectors and resumes mid-connector deterministically', () => {
    const engine = makeEngine();
    engine.set_development_rate(16);
    let traveler: EngineSave['travelers'][number] | undefined;
    let world: EngineSnapshot | undefined;
    for (let tick = 0; tick < 24_000 && !traveler; tick += 1_000) {
      engine.step(1_000);
      const state = saveState(engine);
      const candidate = state.travelers.find((item) =>
        (item.access_phase === 0 || item.access_phase === 2)
        && item.access_progress > 0
        && ((item.access_phase === 0 && item.origin_access?.points.length)
          || (item.access_phase === 2 && item.destination_access?.points.length)));
      if (candidate) {
        traveler = candidate;
        world = snapshot(engine);
      }
    }
    expect(traveler).toBeDefined();
    const connector = traveler!.access_phase === 0 ? traveler!.origin_access! : traveler!.destination_access!;
    const connectorPosition = sampleAccessPath(connector.points, traveler!.access_progress);
    const movement = findMovement(engine.movements(), traveler!.id);
    expect(movement).not.toBeNull();
    expect(Math.hypot(movement![0] - connectorPosition.x, movement![1] - connectorPosition.z)).toBeLessThan(0.02);
    expect(connector.points.length).toBeGreaterThanOrEqual(2);

    const serialized = engine.save();
    const resumed = CityEngine.restore(serialized);
    engines.push(resumed);
    expect(resumed.save()).toBe(serialized);
    expect(resumed.snapshot()).toBe(engine.snapshot());
    expect(Array.from(resumed.movements())).toEqual(Array.from(engine.movements()));
    for (let batch = 0; batch < 24; batch++) {
      engine.step(100);
      resumed.step(100);
      expect(resumed.save()).toBe(engine.save());
      expect(Array.from(resumed.movements())).toEqual(Array.from(engine.movements()));
    }
    expect(world).toBeDefined();
  }, 60_000);

  it('places waiting rail residents at distinct station or gateway platform queue slots', () => {
    const engine = makeEngine();
    engine.set_development_rate(16);
    const found = new Map<'waiting_station' | 'waiting_gateway', { resident: EngineSave['residents'][number]; state: EngineSave; movements: Float32Array }>();
    for (let tick = 0; tick < 45_000 && found.size < 2; tick += 1_000) {
      engine.step(1_000);
      const state = saveState(engine);
      for (const transfer of ['waiting_station', 'waiting_gateway'] as const) {
        if (found.has(transfer)) continue;
        const resident = state.residents.find((item) => item.transfer === transfer && item.queue_slot !== null && item.queue_slot !== undefined);
        if (resident) found.set(transfer, { resident, state, movements: engine.movements() });
      }
    }
    expect(Array.from(found.keys())).toEqual(expect.arrayContaining(['waiting_station', 'waiting_gateway']));
    for (const { resident, state, movements } of found.values()) expectQueuePlatform(movements, resident, state);

    const restored = CityEngine.restore(engine.save());
    engines.push(restored);
    expect(restored.save()).toBe(engine.save());
    expect(restored.snapshot()).toBe(engine.snapshot());
    expect(Array.from(restored.movements())).toEqual(Array.from(engine.movements()));
  }, 60_000);

  it('restores an onboard save and deterministically replays the complete transfer', () => {
    const engine = makeEngine();
    engine.set_development_rate(16);
    advanceTo(engine, 72_000);
    const onboardWorld = findOnboard(engine);
    expect(onboardWorld).not.toBeNull();
    const onboardState = saveState(engine);
    const onboardResidents = onboardState.residents.filter((resident) => resident.train_id !== null && resident.train_id !== undefined);
    expect(onboardResidents.length).toBeGreaterThan(0);
    expectConsistentBoarding(onboardWorld!, onboardState, engine.movements());

    const serialized = engine.save();
    const resumed = CityEngine.restore(serialized);
    engines.push(resumed);
    expect(resumed.save()).toBe(serialized);
    expect(resumed.snapshot()).toBe(engine.snapshot());
    expect(Array.from(resumed.movements())).toEqual(Array.from(engine.movements()));

    for (let batch = 0; batch < 16; batch++) {
      engine.step(250);
      resumed.step(250);
      expect(resumed.save()).toBe(engine.save());
      expect(resumed.snapshot()).toBe(engine.snapshot());
    }
  }, 60_000);
});
