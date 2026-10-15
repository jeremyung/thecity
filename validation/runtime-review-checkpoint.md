# Runtime review checkpoint

This source-only replacement for PR12 starts from main `fca58ca` and preserves main's parks, signage and textured four-car fleet. It does not merge the evidence-heavy branch or rewrite history. The overall living-city goal and visual acceptance remain incomplete.

## Review order

1. Engine movement/access, causal errands and expressway traffic: `engine/src/lib.rs`, `engine/src/expressway_traffic.rs`, `engine/src/neighborhood_errands.rs`, and the access modules.
2. Worker/presentation ordering, committed infrastructure and bounded renderer work: `web/src/worker`, `web/src/render/movement-presentation.ts`, `visible-infrastructure.ts`, `building-chunks.ts`, `road-batches.ts`.
3. Curved rail and platform integration with main's four-car fleet: `rail-path.ts`, `train-placement.ts`, `trains.ts`, and `city.ts`.
4. Building fitting, camera, atmosphere, water, planting and their targeted tests. Latest narrow-site correction keeps 5 m planner-valid parcels within their geometry envelope.
5. Specs and the living-city goal; these describe intent separately from implementation evidence.

PR17 overhead wiring is now included through `7ea0e22`. Main `710406f` brings PR18 cladding and room windows, including its apartment-occupancy invalidation fix. The initial separate assessment in `pr17-pr18-review.md` is historical; the integration result is recorded below.

## Fresh checkout

Install Node.js 22+, Python 3.9+, Rust, the wasm32-unknown-unknown target and wasm-bindgen-cli 0.2.108.

```sh
npm ci
npm run setup:fixtures
npm run test:engine -- --release
npm run wasm
npm test
npm run build
```

The fixture setup uses a committed SHA-256 manifest and a 3.6 MB archive containing 23 regression witnesses. It refuses conflicting local files, verifies every member, and only writes ignored fixture paths. Offline: `npm run setup:fixtures -- --archive /path/to/test-fixtures.tar.gz`. The archive has no screenshots or recordings. The approved archive and manifest are now [published](https://github.com/jeremyung/thecity/releases/tag/runtime-review-fixtures-2026-10-14). A fresh temporary checkout of the setup scripts successfully downloaded, verified and wrote all 23 fixtures on 2026-10-15; none were already present.

## Main integration and review feedback — 2026-10-15

Merged main `710406f` into the PR17-inclusive review branch. Resolved all seven conflicting files explicitly. The new room-window material and cladding attributes supersede the old window boolean; cloud shading, optimized box geometry, balcony doors and office side ribbons remain. Pane tagging occurs on the optimized box path with nested offsets, and binary occupancy invalidates geometry consistently for homes, apartments and businesses. A new rotated/nested-offset regression verifies every pane vertex's seed, schedule, occupancy and dimensions. Existing construction checks now test baked vacancy rather than the removed emissive-overlay material.

Validation of the combined code:

- Native release: 504 passed, 0 failed, 59 ignored (51.83 s).
- Web: 688 passed across 124 suites (117.89 s), including the integration regressions. No timeouts or geometry assertions relaxed.
- Fresh default WASM and TypeScript/production build pass; existing bundle-size warning remains.
- Fixture integrity: all 7 tests pass.
- Four focused browser captures: home, terrace apartment and office side ribbons, Anime/Miniature at 13:00 and 21:00. ANGLE Metal on Apple M2 Max; no page or console errors. Pane interiors and night variation remain visible; balcony and office glazing geometry remain intact. Captures are local under `.tools/pr19-merge/`, not committed. This is an isolated authored-kit scene-pass check in the app lighting environment, not a full post-processing, mature-city performance or WebGPU acceptance run. The UI clock still shows simulation time, while lighting uses the preview hour.
- Full remote CI must run on the pushed merge; the earlier green `74d1fa8` result is not evidence for this integration.

The ignore rules explicitly retain main's curated PR18 surface captures and benchmark JSON (about 16 MB of existing-main images), while new generated validation stays ignored. The texture-budget JSON is documented as a local generated report, not a committed baseline. Build-mode selection uses a lookup table rather than nested ternaries. Experimental features remain opt-in with outputs isolated under `.tools`: they preserve reproducible comparison work and are not promoted into the default application by this merge.

## Earlier verification — 2026-10-15

Hardware: Apple M2 Max, macOS, Node 26.7.0. Tested a clean source copy with fixture hydration, preserving the dirty development checkout.

- Native release: 504 passed, 0 failed, 59 ignored; 51.86 seconds. Ignored exploratory diagnostics are not acceptance coverage.
- Web: 667 passed, 0 failed across 122 suites. Bounded two-worker run: 125.02 seconds. An earlier run concurrent with native compilation timed out the tower geometry test at 20 seconds; the unconstrained repeat also passed. No assertions or timeouts were relaxed.
- Production TypeScript/Vite build passes; existing bundle-size warning remains.
- Fresh default WASM release build passes; existing Rust warnings remain. Freshly rebuilt WASM: 79 tests pass across 15 dependent suites.
- Browser smoke: Chromium 145.0.7632.6, WebGLBackend with ANGLE Metal requested; fresh-world startup and Anime/Miniature switching complete without page errors. The earlier SwiftShader attempt was stopped after five minutes while switching styles. This is not a performance measurement, WebGPU check, mature-city review or matched visual comparison (the Anime frame preceded full scene population).
- Fixture integrity/overwrite tests: 7 passed; local archive hydration verifies all 23 files and is idempotent.

CI runs fixture hydration, native tests, WASM build, web tests and production build on Ubuntu / Node 22. [Run 38728614698](https://github.com/jeremyung/thecity/actions/runs/38728614698) failed at fixture download with HTTP 404 before the assets were published. Native, WASM, web and build checks were not reached in that run. Public fixture delivery is now verified; a successful full CI run remains a merge gate.

A complete baseline/candidate comparison now resolves the earlier mature-loading timeouts: the baseline completes in 339.5 s and the bounded contour-scheduling change in 164.4 s, with all 1,007 building buffers identical. Both captured the same numerical pose and world. Loading frame p95 was 36.3/36.7 ms; building-update p95 increased 3.4→5.2 ms. See [mature-building-loading.md](mature-building-loading.md) for exact scope, hashes and limitations. This is one WebGL renderer-only pair, not production-save, WebGPU or whole-city acceptance.

Latest source verification after that change:668 web tests pass across 122 suites (118.89 s), TypeScript passes, including 34 targeted contour/chunk/construction tests. No engine code changed, so the previously recorded native and WASM results remain the engine checkpoint evidence.

## Merge gates and scope freeze

- Current main remains `fca58ca`; GitHub reports no merge conflicts. The source checkpoint is 286 changed paths (Git name-only count) and contains no newly added recordings or bulk visual evidence.
- Approved fixture publication and the fresh public download/hash check are complete. Obtain a successful Ubuntu / Node 22 CI run; keep this PR draft until it passes.
- The mature-loading concern is now scoped by a complete WebGL comparison and a geometry-preserving improvement. General browser performance, WebGPU and production lifecycle acceptance remain follow-up gates for the full living-city goal.
- Keep PR #17/#18 and the later experimental street-layout work separate. The authored neighborhood study was invalidated by a future expressway reservation; the reservation-aware residual route screen was rejected for insufficient benefit. Neither is included here.
- The dirty development checkout is not the review branch. Do not bulk-stage it or merge its historical evidence commits into this PR.

## Latest visual review

The completed [neighborhood stage review](neighborhood-stage-review.md) compares 72 captures across three seeds, two stages, two styles and six fixed poses. Current WASM restoration matches the captured geometry. Mature witnesses have no unreleased lots or unfinished construction, but repeated strips and major-corridor dominance remain. This adds review findings only; no experimental layout changes or capture binaries are included.

## Evidence boundaries

Both [locked concepts](https://github.com/jeremyung/thecity/releases/tag/locked-concepts-2026-10-14) remain publicly accessible with hashes. All 42 local references are preserved. Historical reports may link to local captures not distributed with this PR; those links are supporting history, not a fresh-checkout test dependency. No recordings, bulk historical archive or additional reference images are included in this update.

This checkpoint does not claim full browser/hardware performance, audio acceptance, mature multi-seed visual fidelity or completion of spec08. Geometry fitting and test passes alone do not establish those outcomes.

## Final readiness recheck — 2026-10-15

Fetched origin: main is still `fca58ca`; the review branch has no merge conflicts.
The fixture archive SHA-256 remains
`8c229253b8edc0979b86aebbfebce972ef95c3a9fa7d6720743566198c1aa83c`.
All 23 fixture members verify against the committed manifest; all seven hydration
integrity tests pass again. Diff whitespace checks pass. This recheck changes
only documentation; the source validation above remains applicable and was not
rerun or represented as new evidence.

The user approved publication of only `test-fixtures.tar.gz` (3,612,617 bytes)
and its manifest (5,423 bytes). Both are uploaded at the pinned fixture release.
A fresh default `npm run setup:fixtures` completed successfully: 23 verified,
23 written, zero previously present. No recordings, screenshots or historical
archive were uploaded. Keep PR19 draft until the full CI run succeeds.
PR19 replaces PR12; do not merge both. The separate dirty development checkout
and unaccepted corridor experiments are not included in this checkpoint.

## CI geometry-test overhead correction — 2026-10-15

Run38753307912 passed fixture setup/integrity, native release tests and the WASM
build, then failed two web tests on their existing timeouts (low-rise lifecycle
5s, tower bounds20s);666 other tests passed. Production build was not reached.
The two checks repeatedly created matcher objects per vertex. The correction
scans the same coordinates and bounds but reports the first invalid vertex once
per mesh/group. No sample, assertion condition, timeout or test case is removed.
On AppleM2Max/macOS Node26.7, the same two suites pass19 tests before and after:
13.84s before,2.75s after. This is test-harness overhead, not a runtime performance
claim. Full Ubuntu/Node22 CI must pass on the new commit before leaving draft.
