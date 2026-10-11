# Engineering checkpoint — 2026-10-10

This is a working checkpoint, not completed visual or live-growth acceptance.

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
2. Broaden split-sampling coverage beyond single-interval hosts: edge samples now recover the known mature-fixture route, but multi-interval hosts still sample only interval midpoints.
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
