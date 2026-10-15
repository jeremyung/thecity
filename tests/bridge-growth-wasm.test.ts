// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import type { World } from '../web/src/types';

it.each([87231, 0, 5])('grows a connected far-bank neighborhood through current construction, seed %i', seed => {
  initSync({ module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)) });
  const engine = new CityEngine(seed);
  engine.set_development_rate(16);
  let replay: CityEngine | undefined;
  try {
    let bridgeId: number | undefined;
    let bridgeConstruction = false, roadConstruction = false, buildingConstruction = false;
    let settled = false;
    for (let tick = 20; tick <= 64000; tick += 20) {
      engine.step(20);
      const world: World = JSON.parse(engine.snapshot());
      expect(world.tick).toBe(tick);
      bridgeId ??= world.roads.find(r => r.kind === 'bridge')?.id;
      const bridge = world.roads.find(r => r.id === bridgeId);
      if (!bridge) continue;
      bridgeConstruction ||= bridge.stage !== 'open';
      const nodes = new Map(world.nodes.map(n => [n.id, n]));
      const a = nodes.get(bridge.from)!, b = nodes.get(bridge.to)!;
      const west = a.x < b.x ? a : b;
      // Classify against the curved river at each position, not a global x cutoff.
      const westSide = (x: number, z: number) => {
        const river = world.terrain.river;
        const pair = river.slice(1).findIndex((p, i) => river[i].z <= z && p.z >= z);
        if (pair < 0) return false;
        const a = river[pair], b = river[pair + 1];
        const riverX = a.x + (b.x - a.x) * (z - a.z) / (b.z - a.z);
        return x < riverX - (world.terrain.river_half_width ?? 32);
      };
      roadConstruction ||= world.projects.some(p => {
        const a = p.from === null ? undefined : nodes.get(p.from);
        const b = p.to === null ? undefined : nodes.get(p.to);
        return p.kind === 'road' && p.progress < 1 && a && b && westSide(a.x, a.z) && westSide(b.x, b.z);
      });
      buildingConstruction ||= world.buildings.some(b => westSide(b.x, b.z) && b.stage !== 'open');
      const bankConnected = world.roads.some(r => r.stage === 'open' && r.connected
        && !['rail', 'bridge'].includes(r.kind) && (r.from === west.id || r.to === west.id));
      const house = world.buildings.find(b => westSide(b.x, b.z) && b.stage === 'open'
        && ['home', 'apartment'].includes(b.kind) && b.occupancy > 0);
      if (bridge.stage === 'open' && bridge.connected && bankConnected && house) {
        expect(world.building_access?.some(access => access.building_id === house.id)).toBe(true);
        settled = true;
        break;
      }
    }
    expect(bridgeConstruction).toBe(true);
    expect(roadConstruction).toBe(true);
    expect(buildingConstruction).toBe(true);
    expect(settled).toBe(true);
    replay = CityEngine.restore(engine.save());
    engine.step(2000); replay.step(2000);
    expect(replay.save()).toBe(engine.save());
  } finally { replay?.free(); engine.free(); }
}, 60000);
