// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
// Legacy-planner world: needs `npm run wasm:legacy`. Remove with the legacy feature.
import { CityEngine, initSync } from '../.tools/wasm-legacy/city_engine.js';

it('alights a real bus rider at the curb, persists the wait, and resumes when connected', () => {
  initSync({
    module: readFileSync(new URL('../.tools/wasm-legacy/city_engine_bg.wasm', import.meta.url)),
  });
  const state = JSON.parse(
    readFileSync(new URL('./fixtures/bus-seed5-legacy-queue.json', import.meta.url), 'utf8'),
  );
  const resident = state.residents.find((r: { id: number }) => r.id === 46);
  const stop = state.nodes.find((n: { id: number }) => n.id === 6);
  // Deliberately disconnect the final walking leg; leave the existing bus line
  // and its scheduled boarding/alighting behavior intact.
  const destination = state.next_id++;
  state.nodes.push({ id: destination, x: stop.x + 18, z: stop.z - 18, kind: 'junction' });
  resident.home_node = destination;
  const engine = CityEngine.restore(JSON.stringify(state));
  let replay: CityEngine | undefined;
  let connected: CityEngine | undefined;
  let connectedReplay: CityEngine | undefined;
  const inspect = () => JSON.parse(engine.inspect(46));
  const position = (e: CityEngine) => {
    const data = e.movements();
    for (let i = 0; i < data.length; i += 6)
      if (data[i] === 46) return Array.from(data.slice(i, i + 6));
    return undefined;
  };
  try {
    engine.set_development_rate(0);
    let boarded = false;
    for (let tick = 0; tick < 32000; tick += 20) {
      engine.step(20);
      const person = inspect();
      boarded ||= person.transfer === 'riding_bus';
      if (person.transfer === 'waiting_alight') break;
    }
    expect(boarded).toBe(true);
    const waiting = inspect();
    expect(waiting.transfer).toBe('waiting_alight');
    expect(waiting.bus_id).toBeNull();
    expect(waiting.location_node).toBe(6);
    expect(waiting.pending_alight.destination_node).toBe(destination);
    const tuple = position(engine)!;
    expect(tuple).toBeDefined();
    expect(tuple[1]).toBeCloseTo(waiting.pending_alight.wait_position.x, 4);
    expect(tuple[2]).toBeCloseTo(waiting.pending_alight.wait_position.z, 4);
    expect(tuple[4]).toBe(0);
    const saved = JSON.parse(engine.save());
    expect(saved.buses.every((bus: { passengers: number[] }) => !bus.passengers.includes(46))).toBe(
      true,
    );
    replay = CityEngine.restore(engine.save());
    expect(position(replay)).toEqual(tuple);
    for (let i = 0; i < 5; i++) {
      engine.step(40);
      replay.step(40);
    }
    expect(replay.save()).toBe(engine.save());
    expect(position(engine)).toEqual(tuple);

    // Synthetic reconnection isolates recovery from procedural growth policy.
    const repaired = JSON.parse(engine.save());
    repaired.roads.push({
      id: repaired.next_id++,
      from: 6,
      to: destination,
      kind: 'local',
      stage: 'open',
      progress: 1,
      connected: true,
      length: Math.hypot(18, 18),
      lanes: 1,
    });
    repaired.access_geometry_revision++;
    connected = CityEngine.restore(JSON.stringify(repaired));
    connectedReplay = CityEngine.restore(connected.save());
    connected.step(1);
    connectedReplay.step(1);
    expect(connectedReplay.save()).toBe(connected.save());
    const resumedState = JSON.parse(connected.save());
    const resumed = resumedState.residents.find((r: { id: number }) => r.id === 46);
    expect(resumed.transfer).toBe('walking_from_bus');
    expect(resumed.pending_alight).toBeUndefined();
    const walker = resumedState.travelers.find((t: { resident: number }) => t.resident === 46);
    expect(walker.origin_access.points[0]).toEqual(
      saved.residents.find((r: { id: number }) => r.id === 46).pending_alight.wait_position,
    );
    // Heading may turn toward the new route; position must remain continuous.
    expect(position(connected)?.slice(0, 3)).toEqual(tuple.slice(0, 3));
  } finally {
    connectedReplay?.free();
    connected?.free();
    replay?.free();
    engine.free();
  }
}, 60000);
