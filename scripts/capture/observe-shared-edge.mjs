import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { CityEngine, initSync } from '../../public/wasm/city_engine.js';
const output = process.argv[2];
if (!output) throw new Error('Usage: node scripts/capture/observe-shared-edge.mjs output.json');
const wasm = await readFile(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url));
initSync({ module: wasm });
const engine = new CityEngine(87231);
try {
  engine.set_development_rate(16);
  let prior = JSON.parse(engine.save()),
    evidence = null;
  for (let tick = 80; tick <= 72000; tick += 80) {
    engine.step(80);
    const state = JSON.parse(engine.save());
    const plan = state.neighborhood_plan;
    if (
      plan &&
      JSON.stringify(plan.segments) !== JSON.stringify(prior.neighborhood_plan?.segments)
    ) {
      const reused = state.roads.filter(
        (road) =>
          road.stage === 'open' &&
          road.connected &&
          prior.roads.some((old) => old.id === road.id && old.stage === 'open' && old.connected) &&
          plan.segments.some(
            ([a, b]) => (road.from === a && road.to === b) || (road.from === b && road.to === a),
          ),
      );
      if (reused.length) {
        evidence = {
          seed: state.seed,
          tick: state.tick,
          previousTick: prior.tick,
          plan,
          reusedRoads: reused.map((road) => ({
            id: road.id,
            from: road.from,
            to: road.to,
            length: road.length,
          })),
          method:
            'New plan observed at 80-tick intervals; reused roads were already open and connected in the preceding sample.',
        };
        break;
      }
    }
    prior = state;
  }
  const report = {
    wasmSha256: createHash('sha256').update(wasm).digest('hex'),
    found: !!evidence,
    evidence,
    scannedThroughTick: evidence?.tick ?? 72000,
  };
  await writeFile(output, JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report));
  if (!evidence) process.exitCode = 1;
} finally {
  engine.free();
}
