// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import type { World } from '../web/src/types';

it('constructs shallow infill on connected existing streets and replays the resulting city', () => {
  initSync({ module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)) });
  const engine = CityEngine.restore(readFileSync(new URL('../validation/spine-ranking-seed-87231-by-sites-false.json', import.meta.url), 'utf8'));
  let resumed: CityEngine | undefined;
  try {
    const before: World = JSON.parse(engine.snapshot());
    const previousIds = new Set(before.parcels.map(p => p.id));
    engine.set_development_rate(16);
    for (let batch = 0; batch < 100; batch++) engine.step(200);
    const after: World = JSON.parse(engine.snapshot());
    expect(after.tick - before.tick).toBe(20000);
    const added = after.parcels.filter(p => !previousIds.has(p.id) && p.depth === 8);
    expect(added.length).toBeGreaterThan(0);
    expect(after.roads.length).toBe(before.roads.length);
    for (const parcel of added) {
      expect(after.buildings.some(b => b.parcel_id === parcel.id && b.stage === 'open')).toBe(true);
      expect(after.roads.some(r => r.connected && r.stage === 'open' &&
        (r.from === parcel.access_node || r.to === parcel.access_node))).toBe(true);
    }
    const checkpoint = engine.save();
    resumed = CityEngine.restore(checkpoint);
    expect(resumed.save()).toBe(checkpoint);
    engine.step(400); resumed.step(400);
    expect(resumed.save()).toBe(engine.save());
  } finally { resumed?.free(); engine.free(); }
}, 60000);
