import { readFileSync, writeFileSync } from 'node:fs';
import { AccessDiscovery, initSync } from '../../public/wasm/city_engine.js';

initSync({
  module: readFileSync(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url)),
});
const save = readFileSync(
  new URL('../../validation/early-reservation-wasm-87231-save.json', import.meta.url),
  'utf8',
);
const faceRef = JSON.parse(
  readFileSync(new URL('../../validation/face-target-reference.json', import.meta.url), 'utf8'),
);
const inputRef = JSON.parse(
  readFileSync(
    new URL('../../validation/access-descriptor-reference.json', import.meta.url),
    'utf8',
  ),
);
const fbits = (x) => {
  const f = new Float32Array(1);
  const u = new Uint32Array(f.buffer);
  f[0] = x;
  return u[0];
};
const pointBits = (xs) => xs.map(([x, z]) => [fbits(x), fbits(z)]);
const stage = (cp) => Object.keys(cp.stage)[0];
const cmpF32 = (a, b) => {
  if (a < b) return -1;
  if (a > b) return 1;
  if (a === 0 && b === 0) {
    const sa = Object.is(a, -0),
      sb = Object.is(b, -0);
    return sa === sb ? 0 : sa ? -1 : 1;
  }
  return 0;
};
const rectOrder = (a, b) =>
  cmpF32(a.x, b.x) ||
  cmpF32(a.z, b.z) ||
  cmpF32(a.width, b.width) ||
  cmpF32(a.depth, b.depth) ||
  cmpF32(a.rotation, b.rotation);
const compare = (a, b) => {
  if (a.length !== b.length)
    return {
      exact: false,
      actual_count: a.length,
      reference_count: b.length,
      first_mismatch: null,
    };
  for (let i = 0; i < a.length; i++)
    if (JSON.stringify(a[i]) !== JSON.stringify(b[i]))
      return {
        exact: false,
        actual_count: a.length,
        reference_count: b.length,
        first_mismatch: { index: i, actual: a[i], reference: b[i] },
      };
  return { exact: true, actual_count: a.length, reference_count: b.length };
};
const job = new AccessDiscovery(save);
const stageSummary = [];
let descriptorInput = null,
  descriptorCount = null,
  wasmBits = null,
  work = 0,
  calls = 0,
  prior = '';
for (;;) {
  const cp = JSON.parse(job.save());
  const key = stage(cp);
  if (key !== prior) {
    stageSummary.push({ stage: key, at_work: work });
    prior = key;
  }
  if (key === 'CopyGraph' && !stageSummary.at(-1).counts)
    Object.assign(stageSummary.at(-1), {
      counts: [cp.stage.CopyGraph.graph_nodes.length, cp.stage.CopyGraph.graph_edges.length],
    });
  if (key === 'Trace' && !stageSummary.at(-1).counts)
    stageSummary.at(-1).counts = [cp.stage.Trace.retained_edges.length];
  if (key === 'Selection' && !stageSummary.at(-1).counts)
    stageSummary.at(-1).counts = [cp.stage.Selection.selection.faces.length];
  if (key === 'Descriptors') {
    const d = cp.stage.Descriptors;
    descriptorInput = {
      phase: d.phase,
      face: d.face,
      hosts: d.hosts,
      nodes: d.nodes,
      parcels: d.parcels,
      targets: d.targets,
    };
  }
  const step = JSON.parse(job.advance(65_536));
  calls++;
  work += step.work_used;
  if (step.error) throw new Error(step.error);
  if (step.done) {
    descriptorCount = step.descriptor_count;
    wasmBits = JSON.parse(job.descriptor_bits());
    break;
  }
  if (calls > 1000) throw new Error('stage scan cap');
}
job.free();
if (!descriptorInput) throw new Error('DescriptorSearch stage not observed');
const face = descriptorInput.face;
const hostBits = descriptorInput.hosts.map((h) => [
  h.road_id,
  h.from,
  h.to,
  fbits(h.ax),
  fbits(h.az),
  fbits(h.bx),
  fbits(h.bz),
]);
const expectedHosts = inputRef.hosts
  .map((h) => [h.road_id, h.from, h.to, fbits(h.ax), fbits(h.az), fbits(h.bx), fbits(h.bz)])
  .sort((a, b) => a[0] - b[0]);
const nodeBits = descriptorInput.nodes.map((n) => [n.id, fbits(n.x), fbits(n.z)]);
const expectedNodes = inputRef.nodes
  .map((n) => [n.id, fbits(n.x), fbits(n.z)])
  .sort((a, b) => a[0] - b[0]);
const parcelBits = descriptorInput.parcels.map((r) => [
  fbits(r.x),
  fbits(r.z),
  fbits(r.width),
  fbits(r.depth),
  fbits(r.rotation),
]);
const sortedRef = inputRef.parcels
  .slice()
  .sort(rectOrder)
  .filter((r, i, a) => i === 0 || rectOrder(a[i - 1], r) !== 0)
  .map((r) => [fbits(r.x), fbits(r.z), fbits(r.width), fbits(r.depth), fbits(r.rotation)]);
const targets = pointBits(descriptorInput.targets);
const expectedTargets = faceRef.target_bits.slice().sort((a, b) => {
  const ux = new Uint32Array(1),
    fx = new Float32Array(ux.buffer);
  ux[0] = a[0];
  const ax = fx[0];
  ux[0] = b[0];
  const bx = fx[0];
  const x = cmpF32(ax, bx);
  if (x) return x;
  ux[0] = a[1];
  const az = fx[0];
  ux[0] = b[1];
  return cmpF32(az, fx[0]);
});
const facePolygon = pointBits(face.polygon);
const expectedPolygon = pointBits(inputRef.face.polygon);
const expectedBits = inputRef.ordered_descriptor_bits;
const expectedKeys = new Set(expectedBits.map((x) => JSON.stringify(x)));
const wasmKeys = new Set(wasmBits.map((x) => JSON.stringify(x)));
const exactSetMatches = wasmBits.filter((x) => expectedKeys.has(JSON.stringify(x))).length;
const nativeByHost = new Map();
for (const row of expectedBits) {
  const k = String(row[0]);
  if (!nativeByHost.has(k)) nativeByHost.set(k, []);
  nativeByHost.get(k).push(row);
}
const sameHostTMatches = wasmBits.filter((row) =>
  (nativeByHost.get(String(row[0])) || []).some((n) => n[1] === row[1]),
).length;
const firstDifferences = [];
for (
  let i = 0;
  i < Math.min(wasmBits.length, expectedBits.length) && firstDifferences.length < 8;
  i++
)
  if (JSON.stringify(wasmBits[i]) !== JSON.stringify(expectedBits[i]))
    firstDifferences.push({
      index: i,
      wasm: wasmBits[i],
      native: expectedBits[i],
      wasm_near_host_t: (nativeByHost.get(String(wasmBits[i][0])) || [])
        .filter((n) => n[1] === wasmBits[i][1])
        .slice(0, 3),
    });
const nearestByHostT = wasmBits.map((w) => {
  const matches = (nativeByHost.get(String(w[0])) || []).filter((n) => n[1] === w[1]);
  let best = Infinity,
    bestRow = null;
  for (const n of matches) {
    const dx = Math.abs(
      new Float32Array(new Uint32Array([w[2]]).buffer)[0] -
        new Float32Array(new Uint32Array([n[2]]).buffer)[0],
    );
    const dz = Math.abs(
      new Float32Array(new Uint32Array([w[3]]).buffer)[0] -
        new Float32Array(new Uint32Array([n[3]]).buffer)[0],
    );
    const d = Math.hypot(dx, dz);
    if (d < best) {
      best = d;
      bestRow = n;
    }
  }
  return { distance: best, wasm: w, native: bestRow };
});
const nearestDistances = nearestByHostT.map((x) => x.distance).sort((a, b) => a - b);
const groupCounts = (rows) => {
  const m = new Map();
  for (const r of rows) {
    const k = `${r[0]}:${r[1]}`;
    m.set(k, (m.get(k) || 0) + 1);
  }
  return m;
};
const wc = groupCounts(wasmBits),
  nc = groupCounts(expectedBits);
let changedHostTGroups = 0;
for (const k of new Set([...wc.keys(), ...nc.keys()]))
  if ((wc.get(k) || 0) !== (nc.get(k) || 0)) changedHostTGroups++;
const report = {
  scope:
    'WASM frozen checkpoint stage comparison against native reference inputs/output; no reference or engine source changed.',
  source_save: 'validation/early-reservation-wasm-87231-save.json',
  source_save_sha256: 'f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e',
  wasm_work: work,
  advance_calls: calls,
  descriptor_count: descriptorCount,
  stages: stageSummary,
  descriptor_stage: {
    phase: descriptorInput.phase,
    input_counts: {
      hosts: hostBits.length,
      nodes: nodeBits.length,
      parcels: parcelBits.length,
      targets: targets.length,
    },
  },
  descriptor_comparison: {
    changed_host_t_groups: changedHostTGroups,
    nearest_same_host_t_endpoint_distance: {
      zero: nearestDistances.filter((x) => x === 0).length,
      le_0_001: nearestDistances.filter((x) => x <= 0.001).length,
      le_0_1: nearestDistances.filter((x) => x <= 0.1).length,
      median: nearestDistances[Math.floor(nearestDistances.length / 2)],
      max: nearestDistances.at(-1),
    },
    mismatch_count: wasmBits.filter((x, i) => JSON.stringify(x) !== JSON.stringify(expectedBits[i]))
      .length,
    exact_tuple_set_intersection: exactSetMatches,
    wasm_unique_tuples: wasmKeys.size,
    native_unique_tuples: expectedKeys.size,
    same_host_and_host_t_match_count: sameHostTMatches,
    first_differences: firstDifferences,
  },
  comparisons: {
    face_identity_matches: JSON.stringify(face.identity) === JSON.stringify(inputRef.face.identity),
    face_polygon_bits: compare(facePolygon, expectedPolygon),
    targets_bits: compare(targets, expectedTargets),
    hosts_sorted_by_road_id_bits: compare(hostBits, expectedHosts),
    nodes_sorted_by_id_bits: compare(nodeBits, expectedNodes),
    parcels_sorted_deduplicated_bits: compare(parcelBits, sortedRef),
  },
};
writeFileSync(
  new URL('../../validation/discovery-parity-diagnosis.json', import.meta.url),
  JSON.stringify(report, null, 2) + '\n',
);
console.log(JSON.stringify(report, null, 2));
