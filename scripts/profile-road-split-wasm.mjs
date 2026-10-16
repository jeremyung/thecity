import assert from 'node:assert/strict';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';

const wasmDir = resolve(process.env.ROAD_SPLIT_WASM_DIR ?? '.tools/wasm-profiling');
const fixtureSpecs = [
  [
    'detached_geometry',
    process.env.ROAD_SPLIT_DETACHED_FIXTURE ?? '/tmp/road-split-profile-stress.json',
  ],
  [
    'connected_geometry',
    process.env.ROAD_SPLIT_CONNECTED_FIXTURE ?? '/tmp/road-split-profile-connected.json',
  ],
  [
    'all_affected_connected',
    process.env.ROAD_SPLIT_ALL_AFFECTED_FIXTURE ?? '/tmp/road-split-profile-all-affected.json',
  ],
];
for (const [name, path] of fixtureSpecs) {
  if (!existsSync(path))
    throw new Error(
      `Missing ${name} fixture at ${path}. Export both road-split profile fixtures with the opt-in Rust profile and copy them to /tmp.`,
    );
}
const wasmPath = pathToFileURL(resolve(wasmDir, 'city_engine_bg.wasm'));
const outputPath = new URL('../validation/road-split-wasm-profile.json', import.meta.url);
const wasmBytes = readFileSync(wasmPath);
const { CityEngine, initSync } = await import(
  pathToFileURL(resolve(wasmDir, 'city_engine.js')).href
);
initSync({ module: wasmBytes });

const samples = Number(process.env.ROAD_SPLIT_WASM_SAMPLES ?? 25);
assert.ok(
  Number.isInteger(samples) && samples >= 5 && samples <= 200,
  'sample count must be 5..200',
);

if (typeof CityEngine.prototype.split_open_road !== 'function') {
  throw new Error(
    'WASM API unavailable: CityEngine.split_open_road(road_id, fraction) is required to time the actual atomic split. Restore/save-only timing is not an atomic split measurement.',
  );
}

function indexById(items) {
  return new Map(items.map((item) => [item.id, item]));
}
function position(save, traveler) {
  const nodes = indexById(save.nodes);
  if (traveler.edge_index >= traveler.route_lengths.length) {
    const node = nodes.get(traveler.route.at(-1));
    return [node.x, node.z];
  }
  const a = nodes.get(traveler.route[traveler.edge_index]);
  const b = nodes.get(traveler.route[traveler.edge_index + 1]);
  const t = Math.max(
    0,
    Math.min(1, traveler.edge_progress / traveler.route_lengths[traveler.edge_index]),
  );
  return [a.x + (b.x - a.x) * t, a.z + (b.z - a.z) * t];
}
function assertSamePosition(a, b) {
  assert.ok(
    Math.abs(a[0] - b[0]) < 1e-4 && Math.abs(a[1] - b[1]) < 1e-4,
    `traveler position changed: ${a} -> ${b}`,
  );
}
function hasHostEdge(route, host) {
  for (let i = 0; i + 1 < route.length; i++) {
    if (
      (route[i] === host.from && route[i + 1] === host.to) ||
      (route[i] === host.to && route[i + 1] === host.from)
    )
      return true;
  }
  return false;
}
function routeAfterSplit(route, host, splitNodeId) {
  const output = [];
  for (let i = 0; i < route.length; i++) {
    output.push(route[i]);
    if (
      i + 1 < route.length &&
      ((route[i] === host.from && route[i + 1] === host.to) ||
        (route[i] === host.to && route[i + 1] === host.from))
    )
      output.push(splitNodeId);
  }
  return output;
}
function rawObjectById(json, arrayName, id) {
  const marker = `"${arrayName}":[`;
  const arrayStart = json.indexOf(marker);
  assert.notEqual(arrayStart, -1, `missing ${arrayName} array`);
  let i = arrayStart + marker.length;
  while (i < json.length) {
    while (json[i] === ',' || /\s/.test(json[i] ?? '')) i++;
    if (json[i] === ']') break;
    assert.equal(json[i], '{', `expected object in ${arrayName}`);
    const start = i;
    let depth = 0;
    let inString = false;
    let escaped = false;
    for (; i < json.length; i++) {
      const ch = json[i];
      if (inString) {
        if (escaped) escaped = false;
        else if (ch === '\\') escaped = true;
        else if (ch === '"') inString = false;
      } else if (ch === '"') inString = true;
      else if (ch === '{') depth++;
      else if (ch === '}' && --depth === 0) {
        const raw = json.slice(start, i + 1);
        if (JSON.parse(raw).id === id) return raw;
        i++;
        break;
      }
    }
  }
  throw new Error(`could not find ${arrayName} entity ${id}`);
}
function appendRawArrayElement(objectText, property, rawValue) {
  const pattern = new RegExp(`("${property}":\\[)([^\\]]*)(\\])`);
  assert.ok(pattern.test(objectText), `missing ${property} array in entity`);
  return objectText.replace(
    pattern,
    (_all, before, contents, after) =>
      `${before}${contents}${contents ? ',' : ''}${rawValue}${after}`,
  );
}
function percentile(values, p) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.ceil((sorted.length - 1) * p)];
}

function runFixture(name, fixturePath) {
  const fixture = JSON.parse(readFileSync(fixturePath, 'utf8'));
  const sourceSave = fixture.save_json;
  const source = JSON.parse(sourceSave);
  const hostId = fixture.host_road_id;
  const fraction = fixture.split_fraction;
  if (name === 'all_affected_connected') {
    assert.equal(fixture.all_affected, true, 'many-affected fixture must be explicitly marked');
    assert.equal(
      fixture.connected_geometry,
      true,
      'many-affected case must exercise connected geometry',
    );
  }
  assert.equal(source.roads.length, 4999, `${name} must start at the road cap minus one`);
  assert.equal(source.travelers.length, 2000, `${name} must contain the traveler cap`);

  // Check the complete split before collecting timings.
  const oldHost = source.roads.find((road) => road.id === hostId);
  assert.ok(oldHost, 'fixture host road must exist');
  const affectedIndices = source.travelers.flatMap((traveler, index) =>
    hasHostEdge(traveler.route, oldHost) ? [index] : [],
  );
  assert.ok(affectedIndices.length > 0, `${name} must include affected routes`);
  const correctnessEngine = CityEngine.restore(sourceSave);
  try {
    const rawResult = correctnessEngine.split_open_road(hostId, fraction);
    const after = JSON.parse(correctnessEngine.save());
    const result = typeof rawResult === 'string' ? JSON.parse(rawResult) : rawResult;
    assert.equal(after.roads.length, source.roads.length + 1);
    assert.equal(after.travelers.length, source.travelers.length);
    const a = source.nodes.find((node) => node.id === oldHost.from);
    const b = source.nodes.find((node) => node.id === oldHost.to);
    const splitNode = after.nodes.find((node) => node.id === result.split_node_id);
    assert.ok(splitNode, 'split result must identify the inserted node');
    assert.ok(
      Math.hypot(
        splitNode.x - (a.x + (b.x - a.x) * fraction),
        splitNode.z - (a.z + (b.z - a.z) * fraction),
      ) < 0.01,
    );
    for (let i = 0; i < source.travelers.length; i++) {
      assertSamePosition(
        position(source, source.travelers[i]),
        position(after, after.travelers[i]),
      );
      const expectedRoute = hasHostEdge(source.travelers[i].route, oldHost)
        ? routeAfterSplit(source.travelers[i].route, oldHost, result.split_node_id)
        : source.travelers[i].route;
      assert.deepEqual(
        after.travelers[i].route,
        expectedRoute,
        `route topology ${i} changed incorrectly`,
      );
    }
    assert.equal(
      after.travelers[affectedIndices[0]].route.length,
      source.travelers[affectedIndices[0]].route.length + 1,
    );
    const afterSave = correctnessEngine.save();
    const restored = CityEngine.restore(afterSave);
    try {
      assert.equal(restored.save(), afterSave, 'post-split city did not restore canonically');
    } finally {
      restored.free();
    }
  } finally {
    correctnessEngine.free();
  }

  // A rejected endpoint-adjacent split must leave all serialized state unchanged.
  const rejectedEngine = CityEngine.restore(sourceSave);
  let rejectionAtomic = false;
  try {
    const before = rejectedEngine.save();
    let threw = false;
    try {
      rejectedEngine.split_open_road(hostId, 0.001);
    } catch {
      threw = true;
    }
    assert.ok(threw, 'near-endpoint split should be rejected');
    assert.equal(rejectedEngine.save(), before, 'rejected split mutated saved state');
    rejectionAtomic = true;
  } finally {
    rejectedEngine.free();
  }

  // Promote one affected 127-node route to the valid 128-node cap. Splitting
  // its host edge would expand it to 129 nodes and must reject atomically.
  const lateIndex = affectedIndices[0];
  const lateActor = source.travelers[lateIndex];
  let lateRejection = {
    supported: false,
    limitation: 'Could not extend the affected route through a valid final edge.',
  };
  const lastNodeId = lateActor.route.at(-1);
  const finalEdge = source.roads.find(
    (road) =>
      road.stage === 'open' &&
      ((road.from === lastNodeId && road.to === lateActor.destination) ||
        (road.to === lastNodeId && road.from === lateActor.destination)),
  );
  if (finalEdge) {
    let rawActor = rawObjectById(sourceSave, 'travelers', lateActor.id);
    rawActor = appendRawArrayElement(rawActor, 'route', String(lateActor.destination));
    const rawRoad = rawObjectById(sourceSave, 'roads', finalEdge.id);
    const rawLength = rawRoad.match(/"length":([^,}]+)/)?.[1];
    assert.ok(rawLength, 'final road must have a serialized length');
    rawActor = appendRawArrayElement(rawActor, 'route_lengths', rawLength);
    const originalActor = rawObjectById(sourceSave, 'travelers', lateActor.id);
    const lateSave = sourceSave.replace(originalActor, rawActor);
    let lateEngine;
    try {
      lateEngine = CityEngine.restore(lateSave);
    } catch (error) {
      lateRejection = {
        supported: false,
        limitation: `Engine does not restore the constructed max-length route: ${error.message}`,
      };
    }
    if (lateEngine) {
      try {
        const before = lateEngine.save();
        const lateRestoresCanonically = before === lateSave;
        let threw = false;
        try {
          lateEngine.split_open_road(hostId, fraction);
        } catch {
          threw = true;
        }
        assert.ok(threw, 'max-route split should reject after route expansion exceeds the cap');
        assert.equal(lateEngine.save(), before, 'late rejected split mutated saved state');
        lateRejection = {
          supported: lateRestoresCanonically,
          atomic: true,
          actor_index: lateIndex,
          route_nodes_before: lateActor.route.length + 1,
          expected_expanded_route_nodes: lateActor.route.length + 2,
          limitation: lateRestoresCanonically
            ? undefined
            : 'Restore canonicalized the JSON representation; restored engine accepted the max-length fixture.',
        };
      } finally {
        lateEngine.free();
      }
    }
  }

  // Restore each engine before timing, excluding fixture restore and serialization.
  const warmupSamples = 5;
  for (let i = 0; i < warmupSamples; i++) {
    const engine = CityEngine.restore(sourceSave);
    try {
      engine.split_open_road(hostId, fraction);
    } finally {
      engine.free();
    }
  }
  const durationsMs = [];
  for (let i = 0; i < samples; i++) {
    const engine = CityEngine.restore(sourceSave);
    try {
      const start = performance.now();
      engine.split_open_road(hostId, fraction);
      durationsMs.push(performance.now() - start);
    } finally {
      engine.free();
    }
  }
  return {
    name,
    fixture_path: fixturePath,
    host_road_id: hostId,
    split_fraction: fraction,
    roads_before: source.roads.length,
    nodes_before: source.nodes.length,
    travelers: source.travelers.length,
    affected_route_nodes_before: source.travelers[affectedIndices[0]].route.length,
    affected_travelers: affectedIndices.length,
    samples,
    warmup_samples: warmupSamples,
    timing_ms: {
      p50: percentile(durationsMs, 0.5),
      p95: percentile(durationsMs, 0.95),
      max: Math.max(...durationsMs),
    },
    correctness: {
      road_count_increased_by_one: true,
      split_node_at_requested_fraction: true,
      all_traveler_positions_preserved: true,
      affected_and_unaffected_route_topology_correct: true,
      post_split_save_restores_canonically: true,
      rejected_split_atomic: rejectionAtomic,
      late_max_route_rejection: lateRejection,
    },
  };
}

const report = {
  scope:
    'Node.js WASM split-call timing with profiling-only WASM build; includes JavaScript/WASM call boundary and small result JSON serialization, excludes fixture restore, state parsing, and post-split save; not browser, worker, or frame timing',
  runtime: { node: process.version, platform: process.platform, arch: process.arch },
  wasm_sha256: createHash('sha256').update(wasmBytes).digest('hex'),
  cases: fixtureSpecs.map(([name, path]) => runFixture(name, path)),
};
writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
