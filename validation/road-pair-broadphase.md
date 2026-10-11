# Road-pair spatial filter — isolated scaling component

The live planner still refuses graph captures above 512 nodes/edges. This component does not change that limit or the rendered city. It is the first replacement for quadratic preflight pair enumeration, pending integration.

`engine/src/road_pair_broadphase.rs` indexes up to 5,000 finite segment AABBs into 128 x-axis buckets and streams conservative overlapping pairs in source-index lexicographic order. Bounds use a 1e-5 tolerance. Min/max scans, bucket initialization, membership insertion, query visits, bucket transitions and emitted pairs consume logical quota. Candidate deduplication is a BTreeSet capped by input count; no complete pair graph is stored. Zero quota leaves state unchanged. JSON checkpoints use an optional emission cursor rather than a machine-width sentinel.

Restore validates stage-local fields, exact index membership prefixes and the complete current candidate set. It rejects deleted candidates, inconsistent scan cursors and premature stage changes covered by the tests. This is structural validation, not authentication against coordinated alteration of every field. Construction validation, checkpoint cloning/serialization and restore validation are synchronous outside the advance quota. Restore may scan 640,000 memberships with bounded set operations; it is not a constant-time operation.

## Verification

Primary rerun: `node scripts/test-engine.mjs --release road_pair_broadphase -- --nocapture`.

Five focused tests cover randomized oracle equality, stable ordering and JSON resume at quotas 1/7/48, zero-quota identity, initial/terminal checkpoints, epsilon-touching geometry, malformed checkpoints, a 5,000-segment street grid and coincident-bound streaming.

The grid emits 14,701 candidates versus 12,497,500 possible unordered pairs. Total search work is 1,370,078 logical units; these are not milliseconds and do not compare equal-cost operations. The test compares the full ordered output against an independently enumerated AABB oracle. This is native component evidence, not a browser performance measurement.

## Remaining work

- Integrate with the exact preflight intersection predicate, conservatively accounting for its f32 arithmetic; AABB parity alone does not prove live narrow-phase parity.
- Replace other quadratic duplicate/node/endpoint validation, and update topology, bridges, face walk, selection and capture together. Never truncate the graph to fit a limit.
- Version affected persisted planner jobs and validate native/WASM replay. Normal city saves must remain supported.
- Profile complete worker steps, synchronous capture and restore, not just this iterator.
- Address adversarial broad bounds: objects spanning all buckets can incur repeated visits, so the index does not guarantee subquadratic total work. Current membership storage is capped at 640,000 and pending candidates at 5,000.

No density, frame-rate or visual-quality improvement is claimed by this checkpoint.
