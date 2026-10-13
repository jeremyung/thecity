import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { build } from 'esbuild';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

mkdirSync('.tools/movement-support', { recursive: true });
await build({ entryPoints: ['web/src/movement-support.ts'], outfile: '.tools/movement-support/decoder.mjs', bundle: true, platform: 'node', format: 'esm' });
const { decodeMovementPacket } = await import('../.tools/movement-support/decoder.mjs');
const wasm = readFileSync('public/wasm/city_engine_bg.wasm');
initSync({ module: wasm });
const report = { seed: 87231, developmentRate: 16, steps: 40000, sampleEvery: 800,
  observations: 0, unresolved: 0, counts: {}, witnesses: [],
  wasmSha256: createHash('sha256').update(wasm).digest('hex') };
const engine = new CityEngine(report.seed);
engine.set_development_rate(report.developmentRate);
try {
  for (let tick = report.sampleEvery; tick <= report.steps; tick += report.sampleEvery) {
    engine.step(report.sampleEvery);
    const saved = engine.save();
    const restored = CityEngine.restore(saved);
    try {
      assert.equal(restored.save(), saved, 'Fresh checkpoint must restore without repair');
      assert.deepEqual(restored.movement_packet(), engine.movement_packet());
      assert.equal(restored.presentation_revision(), engine.presentation_revision());
    } finally { restored.free(); }
    const { movement, support } = decodeMovementPacket(engine.movement_packet());
    assert.deepEqual(movement, engine.movements());
    const world = JSON.parse(engine.snapshot());
    assert.equal(world.presentation_revision, engine.presentation_revision());
    assert.equal(world.tick, tick, 'Audit must sample the advertised tick');
    const roads = new Map(world.roads.map(road => [road.id, road]));
    const nodes = new Set(world.nodes.map(node => node.id));
    const platforms = new Set((world.rail_platforms ?? []).map(platform => platform.node_id));
    for (const [id, deps] of support) {
      report.observations++;
      if (!deps.resolved) {
        report.unresolved++;
        const activity = JSON.parse(engine.inspect(id)).activity;
        report.counts[activity] = (report.counts[activity] ?? 0) + 1;
        if (report.witnesses.length < 8) report.witnesses.push({ tick, id, activity });
        continue;
      }
      for (const roadId of deps.roads) {
        const road = roads.get(roadId);
        assert.ok(road?.stage === 'open' && (road.kind === 'expressway' || road.connected));
        assert.ok(nodes.has(road.from) && nodes.has(road.to));
      }
      for (const platform of deps.platforms) assert.ok(platforms.has(platform));
    }
  }
} finally { engine.free(); }
writeFileSync('validation/movement-support-fresh.json', JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ observations: report.observations, unresolved: report.unresolved, counts: report.counts }));
