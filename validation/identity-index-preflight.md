# Deterministic preflight identity indexes

Road preflight now uses a BTreeMap for node IDs and BTreeSets for road IDs and undirected endpoint pairs. Node duplicate checks and road endpoint/duplicate lookups no longer repeatedly scan prefixes. Source-node indices and validation precedence are retained: finite node coordinates before node identity checks; road self-check, from/to existence, ID duplication, endpoint-pair duplication, then geometry.

Each bounded validation action consumes a work unit; tree operations cost O(log cap), so these units are not equal-cost CPU instructions. Successful validation inserts into transient indexes. Checkpoints contain no map copies. Restore reconstructs maps from validated prefixes, including partially completed node validation; that bounded reconstruction occurs outside advance accounting. At this checkpoint node coincidence testing was still quadratic. A subsequent [node-coincidence index](node-coincidence-index.md) addresses the sparse case; the full planner still caps captures at 512 nodes/edges.

## Verification and measured scope

- Full native release suite: **345 passed, 0 failed, 42 ignored**, 48.45 seconds. Focused preflight suite: 13 passed. The new empty-node/missing-endpoint regression covers initial and failed save restoration at quotas 1/7/48.
- Mature graph: 159 nodes, 206 edges, same 49-face reference. Preflight work is **18,095**, down from the prior spatial-filter version's **102,097** (82.28%), consistent at quotas 1/7/48. See `road-face-preflight-mature.json`.
- Production WASM rebuilt. Discovery remains bit-exact with **3,180** native-reference descriptors across quotas 48/4096/16384 and JSON resume; total discovery work **7,392,819**. See `access-discovery-wasm-profile.json`. This reduction is about 1.12% of the complete discovery job; the much larger preflight percentage must not be presented as total-planner improvement.
- Preflight schema is now **3**. Versions 1 and 2 restart validation from the original frozen graph. Both native integration cases preserve non-job world fields and deterministic migrated continuation. Node WASM additionally checks schema2 from an actual prior built binary, captured before rebuilding. Current corrupted cursors and schema4 reject. See `legacy-preflight-wasm.json` and `legacy-preflight-schema2-wasm.json`.

## Browser worker comparison

Same seed87231 mature fixture, 6,000 consecutive `CityEngine.step(1)` calls in a dedicated Chromium worker, tick242000→248000:

| Metric | Before identity indexes | Final indexed build |
| --- | ---: | ---: |
| p95 step duration | 0.4 ms | 0.4 ms |
| p99 step duration | 10.4 ms | 10.0 ms |
| Maximum step duration | 25.6 ms | 25.8 ms |

Raw reports are `identity-index-worker-before.json` and `identity-index-worker-after.json`, including WASM identity and scope. Both final saves round-trip byte-identically. Both observed summaries end with 853 buildings, 210 roads and branch4003 open/connected. Final saves were not retained for cross-build byte comparison, so matching summary counts are not a claim of identical complete worlds.

These individual runs do **not** demonstrate an overall timing improvement. Machine load and noise are uncontrolled; p95 is unchanged and the maximum is slightly higher. Long ticks still come from other work. Save/snapshot overhead is excluded from per-step timings; these are neither renderer frame measurements nor full-app throughput measurements.

## Remaining work

Scale node-coincidence and remaining topology/face stages before raising their coupled limits. Measure synchronous capture and restore costs. Continue density, architectural variety, style fidelity and water work; this change does not alter art assets or establish visual acceptance.
