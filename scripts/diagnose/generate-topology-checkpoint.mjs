import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { CityEngine, initSync } from '../../public/wasm/city_engine.js';

// A current browser-engine checkpoint, not a native/WASM parity assertion.
const seed = Number(process.argv[2] ?? 87231);
const label = process.argv[3] ?? 'founder-topology';
if (!/^[a-z0-9-]+$/.test(label)) throw new Error('Expected a lowercase checkpoint label');
if (!Number.isInteger(seed) || seed < 0 || seed > 0xffffffff)
  throw new Error('Expected a u32 seed');
const bytes = await readFile(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url));
initSync({ module: bytes });
const engine = new CityEngine(seed);
const started = performance.now();
try {
  engine.set_development_rate(16);
  for (let tick = 0; tick < 242000; tick += 200) engine.step(200);
  const snapshot = engine.snapshot();
  const world = JSON.parse(snapshot);
  if (world.tick !== 242000) throw new Error(`Unexpected tick ${world.tick}`);
  const save = engine.save();
  const restored = CityEngine.restore(save);
  try {
    if (restored.save() !== save) throw new Error('Canonical restore mismatch');
    for (let i = 0; i < 10; i++) {
      engine.step(200);
      restored.step(200);
    }
    if (restored.save() !== engine.save()) throw new Error('Twin continuation mismatch');
  } finally {
    restored.free();
  }
  const prefix = fileURLToPath(new URL(`../../validation/${label}-wasm-${seed}`, import.meta.url));
  await writeFile(`${prefix}-save.json`, save);
  await writeFile(`${prefix}-snapshot.json`, snapshot);
  const report = {
    seed,
    tick: world.tick,
    developmentRate: 16,
    buildings: world.buildings.length,
    roads: world.roads.length,
    canonicalRestore: true,
    twinContinuationTicks: 2000,
    elapsedMs: Math.round(performance.now() - started),
    wasmSha256: createHash('sha256').update(bytes).digest('hex'),
    saveSha256: createHash('sha256').update(save).digest('hex'),
    snapshotSha256: createHash('sha256').update(snapshot).digest('hex'),
    limitation:
      'Save validation and same-target replay; geometric planarity requires a separate road-face audit.',
  };
  await writeFile(`${prefix}-report.json`, JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report));
} finally {
  engine.free();
}
