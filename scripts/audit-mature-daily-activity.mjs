import { readFileSync, writeFileSync } from 'node:fs';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
initSync({ module: readFileSync('public/wasm/city_engine_bg.wasm') });
const engine = CityEngine.restore(readFileSync(process.argv[2] ?? '.tools/fidelity-review/tick-80000.json', 'utf8'));
const initial = JSON.parse(engine.save());
const countBy = (values, key) => values.reduce((counts, value) => {
  const name = String(value[key]); counts[name] = (counts[name] ?? 0) + 1; return counts;
}, {});
const samples = [];
try {
  // One full authoritative day, sampled every 30 simulated minutes.
  for (let offset = 0; offset <= 24000; offset += 500) {
    if (offset) engine.step(500);
    const state = JSON.parse(engine.save());
    samples.push({ tick: state.tick, hour: (6 + state.tick / 1000) % 24,
      population: state.population, residents: state.residents.length,
      buildings: state.buildings.length, weather: state.weather_kind,
      localTravelers: state.travelers.length,
      errands: countBy(state.neighborhood_errands?.outings ?? [], 'phase'),
      byActivity: countBy(state.travelers, 'activity'),
      byKind: countBy(state.travelers, 'kind'),
      residentActivity: countBy(state.residents, 'activity'),
      expresswayTrips: state.expressway_traffic.trips?.length ?? null,
    });
  }
} finally { engine.free(); }
writeFileSync(process.argv[3] ?? 'validation/mature-daily-activity.json', JSON.stringify({
  seed: initial.seed, developmentRate: initial.development_rate,
  scope: 'Fresh current-format mature city, ordinary growth and travel continue for one day. Local travelers exclude train/bus passenger and external expressway counts. Instantaneous samples are not unique-trip totals or route-success rates.',
  samples,
}, null, 2) + '\n');
console.log(JSON.stringify({ samples: samples.length,
  min: Math.min(...samples.map(s => s.localTravelers)),
  max: Math.max(...samples.map(s => s.localTravelers)),
  daytime: samples.filter(s => s.hour >= 11 && s.hour <= 15).map(s => ({ hour: s.hour, localTravelers: s.localTravelers, byActivity: s.byActivity })),
}));
