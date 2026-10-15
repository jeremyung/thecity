# Runtime review checkpoint

This source-only replacement for PR12 starts from main `fca58ca` and preserves main's parks, signage and textured four-car fleet. It does not merge the evidence-heavy branch or rewrite history. The overall living-city goal and visual acceptance remain incomplete.

## Review order

1. Engine movement/access, causal errands and expressway traffic: `engine/src/lib.rs`, `engine/src/expressway_traffic.rs`, `engine/src/neighborhood_errands.rs`, and the access modules.
2. Worker/presentation ordering, committed infrastructure and bounded renderer work: `web/src/worker`, `web/src/render/movement-presentation.ts`, `visible-infrastructure.ts`, `building-chunks.ts`, `road-batches.ts`.
3. Curved rail and platform integration with main's four-car fleet: `rail-path.ts`, `train-placement.ts`, `trains.ts`, and `city.ts`.
4. Building fitting, camera, atmosphere, water, planting and their targeted tests. Latest narrow-site correction keeps 5 m planner-valid parcels within their geometry envelope.
5. Specs and the living-city goal; these describe intent separately from implementation evidence.

PR17 and PR18 are not included. Their separate review is in `pr17-pr18-review.md`: overhead wiring needs authoritative curved rail integration, and room windows need apartment-occupancy invalidation.

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

The fixture setup uses a committed SHA-256 manifest and a 3.6 MB archive containing 23 regression witnesses. It refuses conflicting local files, verifies every member, and only writes ignored fixture paths. Offline: `npm run setup:fixtures -- --archive /path/to/test-fixtures.tar.gz`. The archive has no screenshots or recordings. **Publication is awaiting explicit user approval; the default download is not yet available and remains a merge blocker.**

## Current verification — 2026-10-14

Hardware: Apple M2 Max, macOS, Node 26.7.0. Tested a clean source copy with fixture hydration, preserving the dirty development checkout.

- Native release: 504 passed, 0 failed, 59 ignored; 51.86 seconds. Ignored exploratory diagnostics are not acceptance coverage.
- Web: 667 passed, 0 failed across 122 suites. Bounded two-worker run: 125.02 seconds. An earlier run concurrent with native compilation timed out the tower geometry test at 20 seconds; the unconstrained repeat also passed. No assertions or timeouts were relaxed.
- Production TypeScript/Vite build passes; existing bundle-size warning remains.
- Fresh default WASM release build passes; existing Rust warnings remain. Freshly rebuilt WASM: 79 tests pass across 15 dependent suites.
- Browser smoke: Chromium 145.0.7632.6, WebGLBackend with ANGLE Metal requested; fresh-world startup and Anime/Miniature switching complete without page errors. The earlier SwiftShader attempt was stopped after five minutes while switching styles. This is not a performance measurement, WebGPU check, mature-city review or matched visual comparison (the Anime frame preceded full scene population).
- Fixture integrity/overwrite tests: 7 passed; local archive hydration verifies all 23 files and is idempotent.

CI runs fixture hydration, native tests, WASM build, web tests and production build on Ubuntu/Node22. That workflow is prepared but has not run remotely.

## Evidence boundaries

Both [locked concepts](https://github.com/jeremyung/thecity/releases/tag/locked-concepts-2026-10-14) remain publicly accessible with hashes. All 42 local references are preserved. Historical reports may link to local captures not distributed with this PR; those links are supporting history, not a fresh-checkout test dependency. No recordings, bulk historical archive or additional reference images are included in this update.

This checkpoint does not claim full browser/hardware performance, audio acceptance, mature multi-seed visual fidelity or completion of spec08. Geometry fitting and test passes alone do not establish those outcomes.
