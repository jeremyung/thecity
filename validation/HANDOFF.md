# Engineering checkpoint — 2026-10-10

This is a working checkpoint, not completed visual or live-growth acceptance.

## Follow-up: sweep exhausted block interiors

Remote main is `5aaa0dd`; local checkpoint `9348104` is a separate, unpushed
follow-up. Current working changes add a saved, bounded history of exhausted
face identities. Subsequent searches skip those exact faces before sampling,
so an inaccessible winning block does not permanently hide runner-up interiors.
Successful admission does not mark a face exhausted. A completed search with
no remaining viable face resets the sweep; failures preserve history. History
is capped at 512 faces and 32,768 node IDs, with FIFO eviction.

The native integration test exhausts a mature search, saves/restores the city,
and verifies that the next search selects a different face without changing
roads or allocating IDs. Selector tests exercise quota-1/7 checkpoint replay;
discovery tests reject malformed identity prefixes and ownership. Production
WASM and the web build pass. Full native release suite: 316 passed, 39 ignored.
The [30,000-tick browser continuation](face-sweep-browser-profile.md) observes
11 selected faces, one connected road opening and both dependent frontages;
buildings increase only 850 → 853. General visual density improvement is not yet
demonstrated: the reviewed mature scene still has large undeveloped interiors.

Discovery checkpoint schema is now 4 and face-selection schema is 2. Old
diagnostic checkpoints, and active live-job saves using those older nested
schemas, are rejected; they are not silently restarted. City saves without an
active job remain compatible and default to an empty exhausted-face history.

## Fragmented frontage follow-up

Host sampling now retains midpoint candidates and appends at most two edge
samples for fragmented free spans, within the existing six-sample cap. Full
native suite: 321 passed, 39 ignored; mature golden descriptors and WASM build
pass. The matched 30,000-tick browser continuation is byte-identical to the
previous run: this fixes a candidate-coverage blind spot but provides **no density
gain on the mature fixture**. See [comparison](fragmented-frontage-browser-profile.md).
Host-preparation checkpoint schema is now 4. Stop speculative sample expansion;
investigate rejected geometry/upstream block layout and the documented render
quality gaps. Runtime screenshots are in screenshots/latest/.

## Verified work

- Persisted, quota-stepped face discovery, target selection, descriptor generation and proposal validation.
- Composed planning job reproduces 67 ranked pages and accepts the known legal mature-fixture route (road 3317, two lots), including JSON resume; see `pr-review-fixes.md`.
- Geometry revisions track relevant mutations and fail closed on counter exhaustion.
- Indexed atomic road splitting preserves routes, positions and rejection atomicity. Native capped connected benchmark: 5.90 ms mean, 8.31 ms p95; Node WASM split profiling now covers detached and connected cap fixtures; see `road-split-wasm-profile.md`. Includes a connected case with all 2,000 traveler routes affected, independently rerun. A short actual Chromium worker run now verifies active-job restore and reports sampled batch means; worst-case tick and frame timing remain unmeasured (see `live-worker-profile.md`).
- Live integration native release regression: 307 passed, 39 ignored; production WASM builds and excludes the profiling mutation API. Mature planning passes at both quotas. Web: 261 passed with one timeout in the full concurrent run; all 11 tests in the affected file pass in isolation. See `merge-review-verification.md`.
- Native/WASM discovery descriptor output and proposal results match on the tested mature fixture. This is fixture-specific output parity, not proof of general cross-target replay.

- Primary browser worker check: 2,789 ticks in 20 seconds at requested 8×; active 275 KB planner checkpoint and final save restore byte-identically in fresh epochs. Seven focused live tests pass, including road opening and real-step geometry churn. Web build passes. See `live-worker-profile.md`.

- Individual Chromium WASM tick check: 6,000 mature ticks, p95 0.5 ms, p99 10.3 ms, max 25.2 ms. Final save restores exactly. This excludes renderer/scheduler and is not a maximum-world guarantee. See `live-tick-profile.md` for measurements and coupled graph-cap audit.

## Next implementation steps

1. Replace quadratic graph-preflight scans with deterministic indexing before expanding the coupled 512 graph/face limits; reconcile node and legacy-footprint caps. Do not truncate the graph.
2. Diagnose why the remaining viable block interiors reject access candidates. Fragmented-span edge sampling is implemented, but its matched mature continuation shows no density gain; inspect rejection causes and upstream street/block layout before broadening samples again.
3. Measure browser worker tick cost, including snapshot capture and full simulation stepping, before accepting the provisional production admission budget.
4. Validate the new live hook under geometry churn and through actual worker construction. Frozen discovery now persists across ticks; fresh validation retries twice before skipping a stale candidate; actor remapping remains atomic.
5. Extend the short browser worker run to long-duration construction/opening and bounded-state acceptance; focused native admission-to-opening and real-step resume checks now pass (7 focused tests).
6. Continue visual work: dense continuous street fabric, less unexplained lawn, stronger style fidelity, camera composition and sound listening checks. Current appearance is below the supplied Tokyo references.

The composed planner is now attached to `Engine::step` in the follow-up branch. It uses a provisional 4,096-work-unit tick quota and 1,200-tick scan interval; synchronous captures are outside that counter and still need worker profiling. Complete graph capture currently caps at 512 nodes/edges and refuses oversized input. The controlled mature fixture admits road 3317 with moving actors and repeated save/resume, then opens its connected branch and creates both lots through ordinary simulation ticks. A separate real-step test preserves resumed saves through geometry churn. These checks do not establish long-duration worker construction or visual acceptance. JSON parsing uses `float_roundtrip` to preserve persisted f64 polygon areas exactly.

## Run from a checkout

```sh
npm ci
npm run wasm
npm run dev
```

WASM binaries, toolchains, dependencies, secrets and build outputs are ignored. `npm run wasm` requires the Rust wasm32 target and matching wasm-bindgen tooling (the build script also recognizes the local `.tools` runtime). The dev server uses http://127.0.0.1:5173/.

Specifications under `specs/` remain the product source of truth. `validation/spec-audit.md` is chronological; later entries supersede earlier failed or pending checks. Historical failure logs are retained as evidence, not current acceptance results.
