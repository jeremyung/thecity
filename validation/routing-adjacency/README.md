# Per-query routing adjacency — 2026-10-11

`Engine::route_avoiding` previously scanned all roads for every popped search
node. It now gathers eligible edge endpoints once, sorts `(node ID, source road
index)` pairs, and binary-searches each node's contiguous adjacency range.
This preserves source order, parallel edges, rail filtering, connected/open
requirements, blocked-node behavior, floating-point cost arithmetic, and queue
tie-breaking. Self-loops appear once, as in the old filter. The temporary vector
is at most two entries per road; nothing is serialized or cached across calls.
Road openings, splits, closures and restoration therefore need no cache
invalidation. No world rules, resource limits, or save versions changed.

## Measurements

Same Node version, seed 1204, development rate 60, 300,000 ticks, no concurrent
test suite during the timing run:

| Implementation | Simulation time |
| --- | ---: |
| Previous full scan | 56.05 s |
| Rejected per-query BTreeMap of vectors | 58.80 s |
| Compact sorted vector | 54.24 s |

The compact run is 3.22% below the recorded unprofiled baseline; single-run
timings do not establish statistical significance or browser gains. The tree
version was replaced because its allocation/index cost did not demonstrate a
benefit. Geometry and other systems still dominate total endurance time.

All three final saves hash to
`e803f8f16b23d28d8dcf8c4cb002367bc1f1263c523108cc9f14ff33fe535ffb`.
The production candidate hash is in `compact-index.json`. Both candidate runs
pass resource caps, active-traveler accounting, and exact save restoration.
Reproduce with `npm run wasm` followed by
`node scripts/profile-endurance-wasm.mjs /tmp/routing.json`.

## Equivalence coverage

`engine/src/route_equivalence_tests.rs` retains the previous full-scan search
as an independent oracle. Cases cover equal-cost diamonds, source reversal,
disconnected graphs, filters, parallel edges, self-loops, blocked endpoints,
80 deterministic generated graphs, and a 200-node sparse-ID graph with
fractional costs and mutations between queries. This optimization originally
preserved reconstruction truncation at `MAX_ROUTE_NODES`. A subsequent
[safety fix](../route-cap-safety.md) explicitly rejects oversized suffixes that
omit the requested origin, with separate boundary and delivery regression tests.

Browser worker tail latency, maximum-cap graph performance and visual fidelity
remain unverified by this checkpoint. No test timeout has been increased.

## Verification

- `node scripts/test-engine.mjs --release`: 359 passed, 43 ignored, 44.56 s.
- Production `npm run wasm` build succeeds (existing warnings).
- `npx vitest run tests/wasm.test.ts`: all five tests pass, 82.51 s for the
  file; the 300,000-tick case retains its original 60 s timeout. The file's total
  duration includes the other four tests and is not the endurance duration.
- `git diff --check` passes. The full frontend suite was not rerun because no
  frontend source changed; this is not a claim that its prior timeouts are fixed.
