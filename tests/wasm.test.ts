// @ts-expect-error Node built-in types are not included in the app TypeScript config.
import { readFileSync } from 'node:fs';
import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import type { World } from '../web/src/types';

const engines: CityEngine[] = [];
const makeEngine = (seed: number) => {
  const engine = new CityEngine(seed);
  engines.push(engine);
  return engine;
};

function snapshot(engine: CityEngine) {
  return JSON.parse(engine.snapshot()) as World;
}

function savedState(engine: CityEngine) {
  return JSON.parse(engine.save()) as {
    next_id: number;
    nodes: Array<{ id: number; x: number; z: number; kind: string }>;
    roads: Array<{ id: number; from: number; to: number; kind: string; stage: string; connected: boolean; lanes: number }>;
    parcels: Array<{ id: number; access_node: number; protected: boolean; reservation: number | null }>;
    buildings: Array<{ id: number; parcel_id: number; x: number; z: number; kind: string; stage: string; occupancy: number; capacity: number; name: string; anchor: boolean; history: string[] }>;
    projects: Array<{ id: number; kind: string; parcel_id: number | null; progress: number; reason: string }>;
    travelers: Array<{
      id: number;
      kind: number;
      route: number[];
      edge_index: number;
      access_phase: number;
      access_progress: number;
      origin_access: { points: Array<{ x: number; z: number }> } | null;
      destination_access: { points: Array<{ x: number; z: number }> } | null;
    }>;
    residents: Array<{ id: number; name: string; home_building: number; work_building: number | null; history: string[]; transfer?: string; bus_origin_stop?: number|null; bus_destination_stop?: number|null; queue_slot?: number|null }>;
    trains: Array<{ id: number; route: number[]; distance: number; direction: number; dwell_ticks: number }>;
    events: Array<{ kind: string; reason: string }>;
    population: number;
  };
}

function sampleAccessPath(points: Array<{ x: number; z: number }>, distance: number) {
  let remaining = distance;
  for (let index = 0; index < points.length - 1; index++) {
    const from = points[index];
    const to = points[index + 1];
    const dx = to.x - from.x;
    const dz = to.z - from.z;
    const length = Math.hypot(dx, dz);
    if (remaining <= length || index === points.length - 2) {
      const amount = length > 0 ? Math.min(1, remaining / length) : 0;
      return { x: from.x + dx * amount, z: from.z + dz * amount };
    }
    remaining -= length;
  }
  return points[points.length - 1];
}

beforeAll(() => {
  const wasm = readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url));
  initSync({ module: wasm });
});

afterEach(() => {
  for (const engine of engines.splice(0)) engine.free();
});

describe('compiled city engine', () => {
  it('replays deterministically, restores pending work byte-for-byte, and separates seeds', () => {
    const first = makeEngine(8);
    const sameSeed = makeEngine(8);
    const otherSeed = makeEngine(9);

    first.step(1_337);
    sameSeed.step(1_337);
    expect(first.save()).toBe(sameSeed.save());
    expect(first.snapshot()).toBe(sameSeed.snapshot());
    expect(snapshot(first).terrain.river).not.toEqual(snapshot(otherSeed).terrain.river);

    const serialized = first.save();
    const initialSave = savedState(first);
    const pending = initialSave.projects.filter((project) => project.progress < 1);
    expect(pending.length).toBeGreaterThan(0);
    expect(pending.some((project) => project.reason.length > 0)).toBe(true);

    const resumed = CityEngine.restore(serialized);
    engines.push(resumed);
    first.step(2_000);
    resumed.step(2_000);
    expect(resumed.save()).toBe(first.save());
    expect(resumed.snapshot()).toBe(first.snapshot());
  });

  it('opens a useful station, place history, and visible construction through the WASM API', () => {
    const engine = makeEngine(5);
    engine.set_development_rate(60);
    // The event history is bounded. Observe service opening while advancing,
    // rather than requiring its early event to survive all later city history.
    let sawStationService = false;
    let sawVisibleConstruction = false;
    for (let batch = 0; batch < 36; batch++) {
      engine.step(2_000);
      const observed = snapshot(engine);
      sawStationService ||= observed.events.some(event => event.kind === 'StationServiceStarted');
      const pending = savedState(engine).projects.find(project => project.kind === 'building' && project.progress < 1);
      if (pending) {
        expect(pending.reason).toMatch(/built|renovat|demand/i);
        expect(observed.projects.some(project => project.id === pending.id && project.progress < 1)).toBe(true);
        sawVisibleConstruction = true;
      }
    }

    const world = snapshot(engine);
    const station = world.nodes.find((node) => node.kind === 'station');
    const rail = world.roads.find((road) => road.kind === 'rail' && road.stage === 'open' && road.connected);
    const shop = world.buildings.find((building) => building.name === 'Kissa Aoba');

    expect(world.stage).toBe('Maturity');
    expect(station).toBeDefined();
    expect(rail).toBeDefined();
    expect(world.trains.length).toBeGreaterThan(0);
    expect(shop?.anchor).toBe(true);
    expect(shop?.history.length).toBeGreaterThan(0);
    expect(sawStationService).toBe(true);
    expect(sawVisibleConstruction).toBe(true);
  }, 30_000);

  it('keeps movement IDs, routes, positions, and headings valid against the saved graph', () => {
    const engine = makeEngine(5);
    engine.set_development_rate(60);
    for (let batch = 0; batch < 36; batch++) engine.step(2_000);

    const world = snapshot(engine);
    const state = savedState(engine);
    const movement = engine.movements();
    const nodes = new Map(world.nodes.map((node) => [node.id, node]));
    const travelers = new Map(state.travelers.map((traveler) => [traveler.id, traveler]));
    const trains = new Map(state.trains.map((train) => [train.id, train]));
    const roads = state.roads;
    const seenIds = new Set<number>();
    let trainCount = 0;

    expect(movement.length % 5).toBe(0);
    for (let offset = 0; offset < movement.length; offset += 5) {
      const id = movement[offset];
      const x = movement[offset + 1];
      const z = movement[offset + 2];
      const heading = movement[offset + 3];
      const kind = movement[offset + 4];
      expect(Number.isInteger(id)).toBe(true);
      expect(id).toBeGreaterThan(0);
      expect(id).toBeLessThan(state.next_id);
      expect(seenIds.has(id)).toBe(false);
      seenIds.add(id);
      expect(Number.isFinite(x) && Number.isFinite(z) && Number.isFinite(heading)).toBe(true);
      expect(x).toBeGreaterThanOrEqual(world.bounds.min_x);
      expect(x).toBeLessThanOrEqual(world.bounds.max_x);
      expect(z).toBeGreaterThanOrEqual(world.bounds.min_z);
      expect(z).toBeLessThanOrEqual(world.bounds.max_z);

      if(kind===3){
        // Bus route, capacity and replay contracts are exercised in bus-wasm.test.
        const bus=world.buses?.find(bus=>bus.id===id);
        expect(bus).toBeDefined();
        expect(world.bus_lines?.some(line=>line.id===bus?.line_id&&line.active)).toBe(true);
        continue;
      }
      if(kind===0&&!travelers.has(id)){
        const resident=state.residents.find(resident=>resident.id===id);
        expect(resident?.transfer).toBe('waiting_bus');
        const line=world.bus_lines?.find(line=>line.stops.includes(resident!.bus_origin_stop!)&&line.stops.includes(resident!.bus_destination_stop!));
        expect(line).toBeDefined();
        const direction=line!.stops.indexOf(resident!.bus_destination_stop!)>line!.stops.indexOf(resident!.bus_origin_stop!)?1:-1;
        const marker=line!.stop_positions?.find(stop=>stop.node_id===resident!.bus_origin_stop&&stop.direction===direction);
        expect(marker).toBeDefined();
        const offset=3+(resident!.queue_slot??0)*1.25;
        expect(x).toBeCloseTo(marker!.x+Math.cos(marker!.heading)*offset,3);
        expect(z).toBeCloseTo(marker!.z+Math.sin(marker!.heading)*offset,3);
        continue;
      }
      const isTrain = kind === 2;
      const actor = isTrain ? trains.get(id) : travelers.get(id);
      expect(actor).toBeDefined();
      if (isTrain) trainCount++;
      if (!isTrain) {
        const traveler = actor as (typeof state.travelers)[number];
        const access = traveler.access_phase === 0 ? traveler.origin_access
          : traveler.access_phase === 2 ? traveler.destination_access
            : null;
        if (access && access.points.length >= 2) {
          const expected = sampleAccessPath(access.points, traveler.access_progress);
          expect(Math.hypot(x - expected.x, z - expected.z)).toBeLessThan(0.03);
          continue;
        }
      }
      const edgeIndex = isTrain ? 0 : (actor as (typeof state.travelers)[number]).edge_index;
      const route = actor?.route ?? [];
      expect(edgeIndex + 1).toBeLessThan(route.length);
      const from = nodes.get(route[edgeIndex]);
      const to = nodes.get(route[edgeIndex + 1]);
      expect(from).toBeDefined();
      expect(to).toBeDefined();

      const road = roads.find((candidate) =>
        (candidate.from === from?.id && candidate.to === to?.id)
        || (candidate.from === to?.id && candidate.to === from?.id));
      expect(road).toBeDefined();
      expect(road?.stage).toBe('open');
      expect(road?.connected).toBe(true);
      expect(isTrain ? road?.kind : road?.kind !== 'rail').toBe(isTrain ? 'rail' : true);

      const headingFrom = isTrain && (actor as (typeof state.trains)[number]).direction < 0 ? to : from;
      const headingTo = isTrain && (actor as (typeof state.trains)[number]).direction < 0 ? from : to;
      const expectedHeading = Math.atan2((headingTo as World['nodes'][number]).z - (headingFrom as World['nodes'][number]).z, (headingTo as World['nodes'][number]).x - (headingFrom as World['nodes'][number]).x);
      const headingDifference = Math.atan2(Math.sin(heading - expectedHeading), Math.cos(heading - expectedHeading));
      expect(Math.abs(headingDifference)).toBeLessThan(1e-5);
    }

    expect(seenIds.size).toBeGreaterThan(0);
    expect(trainCount).toBeGreaterThan(0);
  });

  it('round-trips named residents and corridor trains while keeping presentation style client-side', () => {
    const engine = makeEngine(87_231);
    engine.set_development_rate(16);
    for (let batch = 0; batch < 120; batch++) engine.step(2_000);

    const world = snapshot(engine);
    const state = savedState(engine);
    const buildingIds = new Set(world.buildings.map((building) => building.id));
    const nodes = new Map(world.nodes.map((node) => [node.id, node]));
    const connectedRail = new Set(world.roads
      .filter((road) => road.kind === 'rail' && road.stage === 'open' && road.connected)
      .map((road) => `${Math.min(road.from, road.to)}:${Math.max(road.from, road.to)}`));

    expect(world.stage).toBe('Maturity');
    expect(world.residents?.length).toBeGreaterThan(0);
    expect(world.trains.length).toBeGreaterThan(0);
    expect(state.residents.length).toBe(world.residents?.length);
    for (const resident of world.residents ?? []) {
      expect(resident.name.length).toBeGreaterThan(0);
      expect(buildingIds.has(resident.home_building)).toBe(true);
      if (resident.work_building !== null) expect(buildingIds.has(resident.work_building)).toBe(true);
    }
    // Resident IDs are reused for their active journey, so inspect() reports the
    // traveler view while that resident is on the road. Select a resident at home/work.
    const travelingResidentIds = new Set(state.travelers.map((traveler) => traveler.id));
    const idleResident = world.residents?.find((resident) => !travelingResidentIds.has(resident.id));
    expect(idleResident).toBeDefined();
    const residentInspection = JSON.parse(engine.inspect(idleResident!.id)) as { entityType: string; id: number; name: string; activity: string; history: string[] };
    expect(residentInspection).toMatchObject({ entityType: 'resident', id: idleResident!.id, name: idleResident!.name, activity: idleResident!.activity });
    expect(residentInspection.history.length).toBeGreaterThan(0);
    for (const train of world.trains) {
      expect(train.route.length).toBeGreaterThanOrEqual(2);
      expect(train.direction === -1 || train.direction === 1).toBe(true);
      for (let i = 0; i < train.route.length - 1; i++) {
        const from = nodes.get(train.route[i]);
        const to = nodes.get(train.route[i + 1]);
        expect(from).toBeDefined();
        expect(to).toBeDefined();
        expect(connectedRail.has(`${Math.min(from!.id, to!.id)}:${Math.max(from!.id, to!.id)}`)).toBe(true);
      }
      const inspection = JSON.parse(engine.inspect(train.id)) as { entityType: string; id: number; kind: string; direction: number; history: string[] };
      expect(inspection).toMatchObject({ entityType: 'train', id: train.id, kind: 'rail', direction: train.direction });
      expect(inspection.history.length).toBeGreaterThan(0);
    }

    // Style is renderer state; the authoritative engine save and snapshot contain no style field.
    expect(world).not.toHaveProperty('style');
    expect(JSON.parse(engine.save())).not.toHaveProperty('style');
    const save = engine.save();
    const restored = CityEngine.restore(save);
    engines.push(restored);
    expect(restored.save()).toBe(save);
    expect(restored.snapshot()).toBe(engine.snapshot());
    expect(Array.from(restored.movements())).toEqual(Array.from(engine.movements()));
    engine.step(2_000);
    restored.step(2_000);
    expect(restored.save()).toBe(engine.save());
    expect(restored.snapshot()).toBe(engine.snapshot());
  }, 60_000);

  it('stays within collection caps and restores a mature world through WASM validation', () => {
    const engine = makeEngine(1_204);
    engine.set_development_rate(60);
    for (let batch = 0; batch < 150; batch++) engine.step(2_000);

    const world = snapshot(engine);
    const state = savedState(engine);
    const counts = {
      buildings: world.buildings.length,
      roads: world.roads.length,
      parcels: world.parcels.length,
      projects: state.projects.length,
      pendingProjects: state.projects.filter((project) => project.progress < 1).length,
      travelers: state.travelers.length,
      events: state.events.length,
    };
    expect(counts.buildings).toBeLessThanOrEqual(2_000);
    expect(counts.roads).toBeLessThanOrEqual(5_000);
    expect(counts.parcels).toBeLessThanOrEqual(4_000);
    expect(counts.projects).toBeLessThanOrEqual(76);
    expect(counts.pendingProjects).toBeLessThanOrEqual(12);
    expect(counts.travelers).toBeLessThanOrEqual(2_000);
    expect(counts.events).toBeLessThanOrEqual(10_000);
    expect(state.population).toBeLessThanOrEqual(30_000);
    const queuedResidents = state.residents.filter((resident) => resident.transfer === 'waiting_station' || resident.transfer === 'waiting_gateway' || resident.transfer === 'waiting_bus').length;
    expect(world.stats.active_travelers).toBe(state.travelers.length + queuedResidents);

    const restored = CityEngine.restore(engine.save());
    engines.push(restored);
    expect(restored.save()).toBe(engine.save());
  }, 60_000);
});
