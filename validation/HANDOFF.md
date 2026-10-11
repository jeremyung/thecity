# Engineering checkpoint — 2026-10-10

This is a working checkpoint, not completed visual or live-growth acceptance.

## Verified work

- Persisted, quota-stepped face discovery, target selection, descriptor generation and proposal validation.
- Composed planning job reproduces 67 ranked pages and accepts the known legal mature-fixture route (road 3317, two lots), including JSON resume; see `pr-review-fixes.md`.
- Geometry revisions track relevant mutations and fail closed on counter exhaustion.
- Indexed atomic road splitting preserves routes, positions and rejection atomicity. Native capped connected benchmark: 5.90 ms mean, 8.31 ms p95; WASM split timing remains unmeasured.
- Full native release regression: 299 passed, 39 ignored. Ignored mature integration gates were run separately; see `pr-review-planning.txt` and `spec-audit.md`.
- Native/WASM discovery descriptor output and proposal results match on the tested mature fixture. This is fixture-specific output parity, not proof of general cross-target replay.

## Next implementation steps

1. Broaden split-sampling coverage beyond single-interval hosts: edge samples now recover the known mature-fixture route, but multi-interval hosts still sample only interval midpoints.
2. Measure the optimized atomic split in WASM using the opt-in stress fixture export in `road_split_profile.rs`.
3. Attach the composed planner to live growth with persisted scheduling, deterministic retries and fresh candidate revalidation when geometry changes. Keep actor remapping atomic: geometry revisions alone do not protect moving actor state across ticks.
4. Verify actual construction/opening, dependent development, save/resume and bounded long-running behavior through the worker.
5. Continue visual work: dense continuous street fabric, less unexplained lawn, stronger style fidelity, camera composition and sound listening checks. Current appearance is below the supplied Tokyo references.

The composed planner is currently test-registered. Separate WASM diagnostic APIs do not attach it to `Engine::step`. The existing browser simulation runs, but these new planning components are not yet driving its growth.

## Run from a checkout

```sh
npm ci
npm run wasm
npm run dev
```

WASM binaries, toolchains, dependencies, secrets and build outputs are ignored. `npm run wasm` requires the Rust wasm32 target and matching wasm-bindgen tooling (the build script also recognizes the local `.tools` runtime). The dev server uses http://127.0.0.1:5173/.

Specifications under `specs/` remain the product source of truth. `validation/spec-audit.md` is chronological; later entries supersede earlier failed or pending checks. Historical failure logs are retained as evidence, not current acceptance results.
