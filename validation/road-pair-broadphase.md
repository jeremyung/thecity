# Road-pair spatial filter — live preflight integration

The live planner still refuses graph captures above 512 nodes/edges. Spatial pair filtering is now integrated into preflight, replacing unconditional pair enumeration for supported graphs. It does not change that cap or the rendered city.

`engine/src/road_pair_broadphase.rs` indexes up to 5,000 finite segment AABBs into a 16 × 16 grid and streams conservative overlapping pairs in source-index lexicographic order. Bounds use a 1e-5 tolerance. Min/max scans, bucket initialization, membership insertion, query visits, bucket transitions and emitted pairs consume logical quota. Candidate deduplication is a BTreeSet capped by input count; no complete pair graph is stored. Zero quota leaves state unchanged. JSON checkpoints use an optional emission cursor rather than a machine-width sentinel.

Restore validates stage-local fields, exact index membership prefixes and the complete current candidate set. It rejects deleted candidates, inconsistent scan cursors and premature stage changes covered by the tests. This is structural validation, not authentication against coordinated alteration of every field. Construction validation, checkpoint cloning/serialization and restore validation are synchronous outside the advance quota. Restore may scan 1,280,000 memberships with bounded set operations; it is not a constant-time operation.

## Verification

Primary rerun: `node scripts/test-engine.mjs --release road_pair_broadphase -- --nocapture`.

Six focused tests cover randomized oracle equality, stable ordering and JSON resume at quotas 1/7/48, zero-quota identity, initial/terminal checkpoints, epsilon-touching geometry, malformed checkpoints, a 5,000-segment street grid and coincident-bound streaming.

The grid emits 14,701 candidates versus 12,497,500 possible unordered pairs. Total search work is 213,704 logical units; these are not milliseconds and do not compare equal-cost operations. The test compares the full ordered output against an independently enumerated AABB oracle. This is native component evidence, not a browser performance measurement.

## Remaining work

- Continue replacing the remaining quadratic node/duplicate validation; spatial pair filtering is integrated, but those other scans dominate the remaining mature preflight work.
- Replace other quadratic duplicate/node/endpoint validation, and update topology, bridges, face walk, selection and capture together. Never truncate the graph to fit a limit.
- Version affected persisted planner jobs and validate native/WASM replay. Normal city saves must remain supported.
- Profile complete worker steps, synchronous capture and restore, not just this iterator.
- Address adversarial broad bounds: objects spanning all buckets can incur repeated visits, so the index does not guarantee subquadratic total work. Current membership storage is capped at 1,280,000 and pending candidates at 5,000.

No density, frame-rate or visual-quality improvement is claimed by this checkpoint.

## Actual f32 predicate compatibility

A native regression demonstrates why raw endpoint AABBs are insufficient: segment (-100000, 0) → (-0.001, 0) and segment (0, 0) → (0, 1) have a mathematical x gap of 0.001. In the current f32 distance routine, `-100000 + (-0.001 - -100000)` rounds to 0. The live preflight therefore rejects this pair, but raw AABB filtering omits it.

`Bounds::from_segment_f32` includes the rounded reconstructed endpoint in each axis, for both endpoint orders, and adds 2e-5 distance padding. For clamped finite t, each rounded multiply/add is monotone, so the computed coordinate lies between the start and reconstructed endpoint. This bounds the actual projection without guessing a fixed world-scale roundoff allowance. Inputs must be finite and within ±100000; unsupported inputs return an error and require exact fallback on integration. Bounds are clipped to that domain, which cannot increase distance to input endpoints within the domain.

The live pair predicate was extracted without changing arithmetic or branch order. Eight preflight tests pass, including the cancellation witness and filtered-versus-all-pairs rejection equality on 24 seeded 66-edge cases at quotas 1, 7 and 48, with periodic JSON resume. Existing malformed-input, shared-endpoint, crossing, cap, checkpoint and handoff checks pass. The predicate compatibility checks preceded live integration. The 512-edge live limit remains; the preflight and broad-phase checkpoint schemas are now version 2.

## Live integration and rejected one-axis version

Bounds construction is charged one edge at a time. Runtime filters stay outside the checkpoint DTO; explicit checkpoint calls copy them, ordinary advances do not. Pair discovery and exact rejection tests have separate charges, including a persisted pending pair tied to the filter's emission cursor. Restore rejects incomplete indexes, removed pending checks, invalid graph indexes and premature completion.

The original one-axis buckets exhausted their spatial budget on the 206-edge mature graph and fell back, increasing preflight work from 119,232 to 140,936. That version was not accepted. The 16 × 16 replacement skips empty rectangle columns in constant time and finishes the same graph at **102,097 work units** for quotas 1/7/48, approximately **14.37% below the original all-pairs preflight**. This is logical work, not measured browser latency. The 5,000-segment component grid retains 14,701 candidate pairs with 213,704 work units and a 301,120-byte terminal JSON checkpoint.

Unsupported coordinates use the original exact scan. Spatial scanning is capped at one all-pairs count, excluding the separately bounded input conversion; exceeding it discards the filter and restarts exact enumeration. This preserves validation semantics after already-passed candidate tests while bounding pathological bucket duplication. A 1,700,000-unit overall preflight ceiling is enforced at runtime and restore. No graph is truncated.

Preflight schema 1 jobs now restart derived validation automatically from their saved nodes, edges and proposed edge. The surrounding planner and city are retained. New fields default during legacy deserialization; mixed-format downgrades, malformed current checkpoints and unsupported future schemas still reject.

## Final integration validation

- Full native release suite: 342 passed, 0 failed, 42 ignored (51.39 seconds in this run). This includes live admission and geometry-churn save/resume checks.
- Mature preflight and composed face-trace ignored checks explicitly run and passed; 49 reference faces are retained.
- Production WASM rebuilt successfully. `scripts/benchmark/profile-access-discovery.mjs` reports exact equality with all 3,180 native-reference descriptors, identical quota/save-resume output, and 7,476,821 logical discovery work units at quotas 48/4096/16384. See `access-discovery-wasm-profile.json` for Node measurements; these are not browser renderer or worker-frame measurements.
- Six focused spatial-filter tests and eleven preflight tests pass. The old one-axis integration was tested but superseded by the two-axis implementation before checkpointing.

Remaining scaling work includes node lookup/duplicate scans, other graph stages and their coupled limits, capture/restore profiling, representative browser worker measurements. No new density or visual-fidelity acceptance is claimed.

## Legacy active-job migration

`GraphPreflight::restore` recognizes schema 1 and creates a fresh current preflight over its frozen graph. It discards only obsolete scan cursors and derived validation progress. It does not regenerate the city, alter RNG/time, reset other planning stages, or admit geometry without validation. The new computation may complete at a different tick than the obsolete algorithm; exact continuation is tested between two migrated restores, not claimed against the old implementation.

Native verification: two legacy tests pass, including the mature active-job save and proposed-edge preservation; all twelve preflight tests pass. Current malformed cursors and future schema3 still reject. The fixture is generated by removing the fields introduced by schema2 from a real nested mature preflight checkpoint, not by exporting from an old binary.

Reproduce native export and WASM verification:

```sh
LEGACY_PREFLIGHT_SAVE_PATH=/tmp/thecity-legacy-preflight-save.json node scripts/test-engine.mjs --release legacy_preflight
npm run wasm
node scripts/diagnose/test-legacy-preflight-wasm.mjs /tmp/thecity-legacy-preflight-save.json
```

`legacy-preflight-wasm.json` records input and WASM hashes. Node WASM verifies all non-job city fields, the 159-node/206-edge frozen graph, exact migrated save roundtrip, 24-tick deterministic twin continuation, and rejection of corrupt current/future checkpoints. This covers preflight schema1 specifically, not every older planner schema.

## Subsequent identity-index integration

Preflight is now schema3 with automatic schema1/2 restart; broadphase remains schema2. Identity indexing further reduces mature preflight to18,095 work units. See `identity-index-preflight.md` for current native/WASM and browser-worker results.
