// @ts-expect-error Node built-in types are not included in the app TypeScript config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import { bezierRoad } from '../web/src/render/road-curve';
import type { World } from '../web/src/types';

it('runs saved regional cars on open cubic decks and resumes their actual trips', () => {
  initSync({
    module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)),
  });
  const live = new CityEngine(87231);
  let restored: CityEngine | undefined;
  try {
    live.set_development_rate(16);
    expect(
      Array.from(live.movements())
        .filter((_, i) => i % 6 === 5)
        .every((y) => y < 10),
    ).toBe(true);
    let state = JSON.parse(live.save());
    for (let batch = 0; batch < 40 && !state.expressway_traffic.trips.length; batch++) {
      live.step(2000);
      state = JSON.parse(live.save());
    }
    expect(state.expressway_traffic.trips.length).toBeGreaterThan(0);
    const world = JSON.parse(live.snapshot()) as World,
      nodes = new Map(world.nodes.map((n) => [n.id, n]));
    const curves = (world.road_decks ?? []).map((deck) => {
      const road = world.roads.find((r) => r.id === deck.road_id)!;
      const [c, d] = deck.controls;
      return {
        road,
        height: deck.deck_height + 0.15,
        path: bezierRoad(
          nodes.get(road.from)!,
          { x: c[0], z: c[1] },
          { x: d[0], z: d[1] },
          nodes.get(road.to)!,
        ),
      };
    });
    const movements = live.movements();
    for (const trip of state.expressway_traffic.trips) {
      const index = Array.from(movements).findIndex((v, i) => i % 6 === 0 && v === trip.id);
      expect(index).toBeGreaterThanOrEqual(0);
      expect(movements[index + 4]).toBe(1);
      let nearest = Infinity;
      let heading = 0;
      for (const { road, height, path } of curves) {
        if (Math.abs(height - movements[index + 5]) > 0.001) continue;
        expect(road.stage).toBe('open');
        for (let s = 0; s < path.length; s += 1) {
          const a = path.at(s),
            b = path.at(Math.min(s + 1, path.length)),
            dx = b.x - a.x,
            dz = b.z - a.z;
          const t = Math.max(
            0,
            Math.min(
              1,
              ((movements[index + 1] - a.x) * dx + (movements[index + 2] - a.z) * dz) /
                (dx * dx + dz * dz),
            ),
          );
          const distance = Math.hypot(
            movements[index + 1] - a.x - dx * t,
            movements[index + 2] - a.z - dz * t,
          );
          if (distance < nearest) {
            nearest = distance;
            heading = Math.atan2(dz, dx);
          }
        }
      }
      expect(nearest).toBeCloseTo(3.25 + trip.lane * 3.5, 1);
      expect(Math.abs(Math.cos(movements[index + 3] - heading))).toBeGreaterThan(0.995);
      const info = JSON.parse(live.inspect(trip.id));
      expect(info.kind).toBe('car');
      expect(nodes.has(info.origin) && nodes.has(info.destination)).toBe(true);
    }
    const save = live.save();
    restored = CityEngine.restore(save);
    expect(restored.save()).toBe(save);
    for (let step = 0; step < 10; step++) {
      live.step(80);
      restored.step(80);
      expect(restored.save()).toBe(live.save());
      expect(Array.from(restored.movements())).toEqual(Array.from(live.movements()));
    }
  } finally {
    live.free();
    restored?.free();
  }
}, 30000);
