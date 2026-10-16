// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import type { World } from '../web/src/types';

it('publishes connected bus service, real riders and replayable movement through WASM', () => {
  initSync({
    module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)),
  });
  const engine = new CityEngine(87231);
  let restored: CityEngine | undefined;
  try {
    engine.set_development_rate(16);
    let world: World = JSON.parse(engine.snapshot());
    for (let batch = 0; batch < 180; batch++) {
      engine.step(2000);
      world = JSON.parse(engine.snapshot());
      if (world.buses?.some((bus) => bus.passengers > 0)) break;
    }
    const buses = world.buses ?? [],
      lines = world.bus_lines ?? [];
    expect(buses.length).toBeGreaterThan(0);
    expect(buses.length).toBeLessThanOrEqual(2);
    expect(buses.some((bus) => bus.passengers > 0)).toBe(true);
    for (const line of lines) {
      expect(line.route.length).toBeLessThanOrEqual(128);
      expect(line.stops.length).toBeLessThanOrEqual(5);
      for (let i = 1; i < line.route.length; i++) {
        const a = line.route[i - 1],
          b = line.route[i];
        expect(
          world.roads.some(
            (road) =>
              road.stage === 'open' &&
              road.kind !== 'rail' &&
              ((road.from === a && road.to === b) || (road.from === b && road.to === a)),
          ),
        ).toBe(true);
      }
      for (const stop of line.stop_positions ?? []) {
        expect(line.stops).toContain(stop.node_id);
        expect(Number.isFinite(stop.x + stop.z + stop.heading)).toBe(true);
      }
    }
    const movements = engine.movements();
    for (const bus of buses) {
      expect(bus.passengers).toBeLessThanOrEqual(bus.capacity);
      expect(bus.capacity).toBe(24);
      let found = false;
      for (let i = 0; i < movements.length; i += 6)
        if (movements[i] === bus.id) {
          expect(movements[i + 4]).toBe(3);
          found = true;
        }
      expect(found).toBe(true);
      expect(JSON.parse(engine.inspect(bus.id)).entityType).toBe('bus');
    }
    restored = CityEngine.restore(engine.save());
    engine.step(2000);
    restored.step(2000);
    expect(restored.save()).toBe(engine.save());
    expect(restored.movements()).toEqual(engine.movements());
  } finally {
    engine.free();
    restored?.free();
  }
}, 20000);

it('serves the historical reverse-stop queue rather than driving past its passengers', () => {
  initSync({
    module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)),
  });
  const engine = CityEngine.restore(
    readFileSync(new URL('./fixtures/bus-seed5-legacy-queue.json', import.meta.url), 'utf8'),
  );
  let replay: CityEngine | undefined;
  const resident = () =>
    JSON.parse(engine.inspect(46)) as {
      transfer: string;
      location_node: number;
      bus_origin_stop: number;
      bus_destination_stop: number;
    };
  try {
    expect(resident().transfer).toBe('waiting_bus');
    expect(resident().bus_origin_stop).toBe(5);
    expect(resident().bus_destination_stop).toBe(6);
    let boarded = false;
    for (let elapsed = 0; elapsed < 24000; elapsed += 20) {
      engine.step(20);
      if (resident().transfer === 'riding_bus') {
        boarded = true;
        break;
      }
    }
    expect(boarded, 'a reverse bus must stop at node 5 and board the waiting resident').toBe(true);
    replay = CityEngine.restore(engine.save());
    let arrived = false;
    for (let elapsed = 0; elapsed < 8000; elapsed += 20) {
      engine.step(20);
      replay.step(20);
      const current = resident();
      if (current.transfer === '' && current.location_node === 6) {
        arrived = true;
        break;
      }
    }
    expect(arrived, 'the resident must finish the bus trip and walk home').toBe(true);
    expect(replay.save()).toBe(engine.save());
  } finally {
    replay?.free();
    engine.free();
  }
}, 60000);

it('creates passenger-serving service for seed 5 after growth changes', () => {
  initSync({
    module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)),
  });
  const engine = new CityEngine(5);
  let replay: CityEngine | undefined;
  try {
    engine.set_development_rate(60);
    for (let tick = 0; tick < 72000; tick += 2000) engine.step(2000);
    let world: World = JSON.parse(engine.snapshot());
    expect(world.bus_lines?.length).toBeGreaterThan(0);
    let boarded = false;
    // Observe actual boarding rather than treating a moving bus as useful service.
    for (let tick = 0; tick < 24000; tick += 20) {
      engine.step(20);
      world = JSON.parse(engine.snapshot());
      if (world.buses?.some((bus) => bus.passengers > 0)) {
        boarded = true;
        break;
      }
    }
    expect(boarded, 'a naturally scheduled resident must use the generated line').toBe(true);
    replay = CityEngine.restore(engine.save());
    engine.step(2000);
    replay.step(2000);
    expect(replay.save()).toBe(engine.save());
  } finally {
    replay?.free();
    engine.free();
  }
}, 60000);
