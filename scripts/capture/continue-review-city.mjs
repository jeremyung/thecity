import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { CityEngine, initSync } from '../../public/wasm/city_engine.js';

const [input, output, tickText = '64000'] = process.argv.slice(2);
const ticks = Number(tickText);
if (
  !input ||
  !output ||
  resolve(input) === resolve(output) ||
  !Number.isSafeInteger(ticks) ||
  ticks < 0 ||
  ticks > 2000000
) {
  throw new Error(
    'Usage: node scripts/capture/continue-review-city.mjs input.json distinct-output.json [ticks 0..2000000]',
  );
}
const source = await readFile(input, 'utf8');
const wasm = await readFile(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url));
const hash = (data) => createHash('sha256').update(data).digest('hex');
initSync({ module: wasm });
const engine = CityEngine.restore(source);
let twin;
try {
  const before = JSON.parse(engine.snapshot());
  for (let remaining = ticks; remaining > 0; remaining -= 2000)
    engine.step(Math.min(remaining, 2000));
  const after = JSON.parse(engine.snapshot());
  if (after.tick - before.tick !== ticks)
    throw new Error('Continuation horizon differs from requested ticks');
  const save = engine.save();
  twin = CityEngine.restore(save);
  if (twin.save() !== save) throw new Error('Noncanonical restore');
  engine.step(2000);
  twin.step(2000);
  if (engine.save() !== twin.save()) throw new Error('Continuation replay mismatch');
  const summarize = (world) => ({
    tick: world.tick,
    buildings: world.buildings.length,
    roads: world.roads.length,
    occupiedHomes: world.buildings.filter(
      (b) => ['home', 'apartment'].includes(b.kind) && b.stage === 'open' && b.occupancy > 0,
    ).length,
    activeCommerce: world.buildings.filter((b) => b.stage === 'open' && b.business_active).length,
  });
  const report = {
    input,
    output,
    ticks,
    wasmSha256: hash(wasm),
    sourceSha256: hash(source.trim()),
    saveSha256: hash(save),
    before: summarize(before),
    after: summarize(after),
    canonicalRestore: true,
    continuationTicksVerified: 2000,
  };
  await writeFile(output, save + '\n');
  await writeFile(output + '.provenance.json', JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report, null, 2));
} finally {
  twin?.free();
  engine.free();
}
