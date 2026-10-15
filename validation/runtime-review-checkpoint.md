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

## Current verification — 2026-10-15

Hardware: Apple M2 Max, macOS, Node 26.7.0. Tested a clean source copy with fixture hydration, preserving the dirty development checkout.

- Native release: 504 passed, 0 failed, 59 ignored; 51.86 seconds. Ignored exploratory diagnostics are not acceptance coverage.
- Web: 667 passed, 0 failed across 122 suites. Bounded two-worker run: 125.02 seconds. An earlier run concurrent with native compilation timed out the tower geometry test at 20 seconds; the unconstrained repeat also passed. No assertions or timeouts were relaxed.
- Production TypeScript/Vite build passes; existing bundle-size warning remains.
- Fresh default WASM release build passes; existing Rust warnings remain. Freshly rebuilt WASM: 79 tests pass across 15 dependent suites.
- Browser smoke: Chromium 145.0.7632.6, WebGLBackend with ANGLE Metal requested; fresh-world startup and Anime/Miniature switching complete without page errors. The earlier SwiftShader attempt was stopped after five minutes while switching styles. This is not a performance measurement, WebGPU check, mature-city review or matched visual comparison (the Anime frame preceded full scene population).
- Fixture integrity/overwrite tests: 7 passed; local archive hydration verifies all 23 files and is idempotent.

CI runs fixture hydration, native tests, WASM build, web tests and production build on Ubuntu / Node 22. [Run 38693036212](https://github.com/jeremyung/thecity/actions/runs/38693036212) failed at fixture download with HTTP 404. Native, WASM, web and build checks were not reached remotely. Publishing the two prepared fixture files and rerunning CI remain merge gates.

A later seed 0 at tick 83000 renderer-only capture attempt did not finish loading within 120 or 300 seconds. The second attempt made progress to four pending building chunks before timing out; both browser processes closed. No complete mature comparison was produced. This is an unresolved loading concern, not proof of a deadlock or an accepted performance result. Diagnose it before treating the checkpoint as mature-browser validated.

A bounded 60.3-second diagnostic on the same checkpoint sources confirmed the actual GPU as ANGLE Metal / Apple M2 Max. It recorded 2,097 static-chunk updates and five commits, with no page errors. The object queue emptied by 40 seconds, while 16 static chunks remained pending at the end. One active chunk advanced from source 9/122 to 111/122 between 40 and 60 seconds. Static update calls consumed about 1.58 seconds cumulatively (30.4ms maximum); this shows incremental preparation progress, not a blocked commit. Instrumented render calls include nested work and cannot be summed as frame time. No scheduler quotas or production code were changed. Complete loading duration and steady-state performance remain unverified.

## Merge gates and scope freeze

- Current main remains `fca58ca`; GitHub reports no merge conflicts. The source checkpoint is 283 changed files and contains no newly added recordings or bulk visual evidence.
- Publish only the required fixture archive and manifest after explicit approval, verify the default download against the committed hashes, and obtain a successful Ubuntu / Node 22 CI run. Keep this PR draft until those gates pass.
- Resolve or explicitly scope the mature-scene loading concern before claiming general browser readiness. Existing local tests validate source behavior; they do not replace that check.
- Keep PR #17/#18 and the later experimental street-layout work separate. The authored neighborhood study was invalidated by a future expressway reservation; the reservation-aware residual route screen was rejected for insufficient benefit. Neither is included here.
- The dirty development checkout is not the review branch. Do not bulk-stage it or merge its historical evidence commits into this PR.

## Evidence boundaries

Both [locked concepts](https://github.com/jeremyung/thecity/releases/tag/locked-concepts-2026-10-14) remain publicly accessible with hashes. All 42 local references are preserved. Historical reports may link to local captures not distributed with this PR; those links are supporting history, not a fresh-checkout test dependency. No recordings, bulk historical archive or additional reference images are included in this update.

This checkpoint does not claim full browser/hardware performance, audio acceptance, mature multi-seed visual fidelity or completion of spec08. Geometry fitting and test passes alone do not establish those outcomes.
