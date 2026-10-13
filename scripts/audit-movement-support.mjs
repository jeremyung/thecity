import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { build } from 'esbuild';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

// The baseline must be captured BEFORE rebuilding the candidate WASM. This
// checks old output, not merely two entry points sharing the candidate traversal.
const baselinePath = process.argv[2] ?? '.tools/movement-support/baseline.json';
const outputPath = process.argv[3] ?? 'validation/movement-support-corpus.json';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const baselineBytes = readFileSync(baselinePath);
const baseline = JSON.parse(baselineBytes);
const wasm = readFileSync('public/wasm/city_engine_bg.wasm');
assert.notEqual(hash(wasm), baseline.wasmSha256, 'Oracle must precede candidate build');
mkdirSync('.tools/movement-support', { recursive: true });
const decoderPath = resolve('.tools/movement-support/decoder.mjs');
await build({ entryPoints: ['web/src/movement-support.ts'], outfile: decoderPath,
  bundle: true, platform: 'node', format: 'esm' });
const { decodeMovementPacket } = await import(pathToFileURL(decoderPath).href);
initSync({ module: wasm });
const report = { baselineSha256: hash(baselineBytes), previousWasmSha256: baseline.wasmSha256,
  wasmSha256: hash(wasm), samples: [], observations: 0, unresolved: 0, witnesses: [] };
for (const sample of baseline.samples) {
  const fixture = readFileSync(sample.path);
  const engine = CityEngine.restore(fixture.toString());
  try {
    engine.step(sample.offset);
    const world = JSON.parse(engine.snapshot());
    const legacy = engine.movements();
    const { movement, support } = decodeMovementPacket(engine.movement_packet());
    // JSON oracle cannot distinguish signed zero; candidate packet/legacy can.
    assert.equal(JSON.stringify(Array.from(legacy)), JSON.stringify(sample.movement), 'Pre-change pose mismatch');
    assert.deepEqual(movement, legacy, 'Packet pose mismatch');
    const roads = new Map(world.roads.map(road => [road.id, road]));
    const nodes = new Set(world.nodes.map(node => node.id));
    const platforms = new Set((world.rail_platforms ?? []).map(platform => platform.node_id));
    const counts = {};
    for (let i = 0; i < movement.length; i += 6) {
      const id = movement[i], kind = movement[i + 4], deps = support.get(id);
      const count = counts[kind] ??= { resolved: 0, unresolved: 0 };
      report.observations++;
      if (!deps.resolved) {
        count.unresolved++; report.unresolved++;
        if (report.witnesses.length < 8) report.witnesses.push({ path: sample.path,
          tick: world.tick, id, kind, roads: deps.roads, platforms: deps.platforms,
          activity: JSON.parse(engine.inspect(id)).activity });
        continue;
      }
      count.resolved++;
      for (const id of deps.roads) {
        const road = roads.get(id);
        // Expressway continuity is checked by the engine's route_is_open;
        // connected describes the separate ground-access graph.
        assert.ok(road?.stage === 'open' && (road.kind === 'expressway' || road.connected), `Invalid support road ${id}`);
        assert.ok(nodes.has(road.from) && nodes.has(road.to));
      }
      for (const id of deps.platforms) assert.ok(platforms.has(id), `Missing platform ${id}`);
    }
    report.samples.push({ path: sample.path, fixtureSha256: hash(fixture), offset: sample.offset,
      tick: world.tick, actors: movement.length / 6, counts });
  } finally { engine.free(); }
}
writeFileSync(outputPath, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ samples: report.samples.length, observations: report.observations,
  unresolved: report.unresolved, outputPath }));
