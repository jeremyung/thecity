import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { CityEngine, initSync } from '../../public/wasm/city_engine.js';

const wasm = readFileSync(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url));
initSync({ module: wasm });
const input = 'validation/spine-ranking-seed-87231-by-sites-false.json';
const engine = CityEngine.restore(readFileSync(input, 'utf8'));
const hash64 = (value) => {
  let hash = 0xcbf29ce484222325n;
  for (const byte of Buffer.from(value))
    hash = BigInt.asUintN(64, (hash ^ BigInt(byte)) * 0x100000001b3n);
  return hash.toString(16).padStart(16, '0');
};
const oldSave = JSON.parse(engine.save());
engine.set_development_rate(16);
const initialIds = new Set(oldSave.parcels.map(({ id }) => id));
const rows = [];
const summarize = () => {
  const raw = engine.save();
  const save = JSON.parse(raw);
  const newShallow = save.parcels.filter((p) => !initialIds.has(p.id) && p.depth === 8);
  const latest = newShallow.slice(-3).map(({ id, x, z, width, depth, road_id, access_node }) => ({
    id,
    x,
    z,
    width,
    depth,
    road_id,
    access_node,
  }));
  return {
    tick: save.tick,
    parcel_count: save.parcels.length,
    shallow_count: newShallow.length,
    added_ids: newShallow.map(({ id }) => id),
    latest,
    // Preserve the full 64-bit state; JavaScript Number parsing loses low bits.
    rng_growth: raw.match(/"rng_growth":(\d+)/)?.[1],
    frontage_road_cursor: save.frontage_road_cursor,
    frontage_slot_cursor: save.frontage_slot_cursor,
    save_hash: hash64(raw),
  };
};
rows.push({
  event: 'start',
  ...summarize(),
  wasm_sha256: createHash('sha256').update(wasm).digest('hex'),
});
for (let i = 0; i < 100; i++) {
  engine.step(200);
  const row = summarize();
  rows.push(row);
  if (row.tick === 243200 || row.tick === 251400) {
    writeFileSync(`/tmp/shallow-wasm-${row.tick}.json`, engine.save());
  }
}
writeFileSync(
  'validation/shallow-wasm-trace.jsonl',
  rows.map((row) => JSON.stringify(row)).join('\n') + '\n',
);
console.log(
  JSON.stringify({ wasmSha256: rows[0].wasm_sha256, checkpoints: rows.length, final: rows.at(-1) }),
);
engine.free();
