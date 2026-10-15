# Current review handoff — 2026-10-15

This section supersedes the historical notes below, including their goal-status and save-migration statements. The living-city goal remains incomplete; obsolete-save compatibility is not required.

The review branch is `codex/city-runtime-review`, draft [PR19](https://github.com/jeremyung/thecity/pull/19), based on main `fca58ca`. Start with [runtime-review-checkpoint.md](runtime-review-checkpoint.md) for review order, reproduction instructions and merge gates. Do not merge the dirty development branch or its bulk evidence history into this checkpoint.

Source verification: 504 native tests pass, 668 web tests pass, fresh WASM and 79 dependent tests pass, TypeScript/production build and seven fixture-integrity tests pass. The contour scheduling improvement preserves all 1,007 compared building buffers and reduces the measured mature renderer load from 339.5 to 164.4 seconds; see [mature-building-loading.md](mature-building-loading.md) for the CPU tradeoff and measurement limits.

The latest remote [CI run](https://github.com/jeremyung/thecity/actions/runs/38711243317) fails at fixture hydration with HTTP 404, before runtime tests. The prepared 3.6 MB regression fixture archive and hash manifest require explicit publication approval. Verify the default download and obtain passing Ubuntu/Node22 CI before marking this PR ready to merge.

The [three-seed stage review](neighborhood-stage-review.md) covers 72 matched captures, both styles, developing and mature stages, with depth of field off. Developing gaps fill; mature repeated frontage strips, dominant elevated corridors, sparse banks and the visible city boundary remain fidelity gaps. Mature station views are partly tower-obstructed. Captures stay local; this report does not claim user visual acceptance or full production lifecycle/WebGPU acceptance.

Scope is frozen for this checkpoint. PR17/18 and subsequent street-layout experiments remain separate. Keep all local reference images; recordings and bulk evidence are excluded. The next whole-block layout study belongs to follow-up work and has no implementation in this PR.

---

# Historical engineering notes

# Engineering checkpoint — 2026-10-10

## Current review corrections — 2026-10-11

PR #3 now validates saved master-plan entity mappings and tests continuation of
the original live engine against a restored copy during construction. See
[review corrections and validation](pr3-review-corrections.md).

The immediate milestone is the existing 800 m city: visual quality in both
styles, normal startup/growth/loading, and browser measurements on named hardware.
Larger maps and imports remain subsequent work. See the updated
[delivery priorities](../specs/08-delivery.md#immediate-milestone-after-the-master-plan-review-2026-10-11).
Backward save compatibility and migrations are not required; current-format
validation and replay remain required. Older migration notes below are historical.
The external goal objective has been updated and the goal remains paused for review.


This is a working checkpoint, not completed visual or live-growth acceptance.

## Inhabited rear extensions — implementation in progress, 2026-10-11

The production scheduler now admits at most two rear-extension projects, after
usable infill opportunities. It scans at most eight buildings per 160-tick pass,
reserves the enlarged parcel immediately, and keeps the original building open
at its original geometry and capacity until completion. Pending hosts are excluded
from turnover/renewal. Schema 4 persists pending plans, the scan cursor and the
earliest original footprint of each extended building; older saves migrate with
empty corresponding state. Renderer staging, vegetation
clearance and construction audio are integrated. Footprint changes now invalidate
the building render cache.

The normal-step native diagnostic restores mature seed 87231 at tick 272000 and
reaches two structure-stage extensions at tick 275520. Root then restored that
fixture in the rebuilt WASM engine and verified exact same-target replay every
20 ticks through both completions at tick 276200: capacities 35→70 and 8→14,
depths 12→24 m and 12→20 m. Pending capacity stays unchanged and nominal front
edges stay fixed. See [the machine-readable check](rear-extension-lifecycle.json)
and `scripts/verify-rear-extension.mjs`. This cross-target fixture test does not
claim native and WASM growth histories are identical.

Validation: full native release suite 381 passed / 0 failed / 48 ignored;
rebuilt WASM; five focused frontend/WASM files 17/17; TypeScript and production
build pass (existing large-chunk warning). The ignored scheduler export was
run separately by the engine agent; root verified its saved state through WASM.
Large generated fixtures remain in `.tools/rear-extension-review/`.

**Original form is now retained.** Completed extensions render the original kit
at its unchanged world transform and add a permanent rear volume. Repeated
extensions keep the earliest original descriptor through save/restore. Tests
compare actual retained mesh geometry and world positions across rotations,
check the rear wall join independently, and verify foundation contact with the
ground. Construction uses the same final envelope and joins a previous annex
when a building extends again. Apartment annex glazing now invalidates on
empty/occupied transitions. Ordinary buildings keep their existing render path.

Both-style asset inspections are in [apartment/anime](screenshots/rear-extension/1343-anime.png),
[apartment/miniature](screenshots/rear-extension/1343-miniature.png),
[shop/anime](screenshots/rear-extension/1369-anime.png), and
[shop/miniature](screenshots/rear-extension/1369-miniature.png).
They use real pending/completed snapshots and production kits, isolated studio
lighting, the same camera, and software WebGL. They show original/construction/
completed forms; they do not exercise the full city pipeline, water, DOF or
hardware performance. The first capture harness used duplicate Three modules
and produced white silhouettes; those images were replaced after fixing the
harness to use Vite's shared module. **Full-city visual acceptance remains open:**
review the shape in context and the incremental chunk swap at completion;
the new upper annex is still a simple mass and needs art-direction review.

Completion handoff now retains the last construction pose until the exact
permanent building descriptor has committed to a visible neighborhood batch.
Queued or partially merged geometry does not count as visible. The two-site
temporary geometry cap remains: under rendering backlog, a newer construction
site waits for an older handoff to free a slot. Chunk replacement, cross-chunk
moves, clear/reset and delayed handoff tests pass. Four focused render suites
pass 31/31, and the production build passes.

Full-scene anime captures now show [construction](screenshots/rear-extension/city-construction-anime.png)
and the [completed addition](screenshots/rear-extension/city-completed-anime.png)
at the same camera. Software WebGL, noon preview, DOF disabled, balanced quality;
the UI clock still displays simulation time rather than the forced preview hour.
The renderer pipeline was detached while manually draining bounded geometry
queues, then restored for capture; this is not normal startup/performance
evidence. The region still has large green gaps and the new annex is visually
plain. The full-city miniature attempt returned from the style-switch await but
timed out after 30 seconds during screenshot readback. Asset-level miniature
checks remain available; full-city miniature acceptance is unproven. Use
`scripts/capture-rear-extension-city.mjs` for the anime capture and handoff report.
The [successful handoff report](rear-extension-render-handoff.json) records
1,832 initial geometry updates to drain the mature scene. The completion update
keeps both old construction groups until update 139, when their replacement
batch commits; the temporary group count then becomes one, matching the new
survey-stage project (4076) already present in the completed snapshot. Both
building and chunk queues finish empty. These are manual update counts, not
elapsed frame-time measurements.

[A 32,000-tick normal WASM continuation](rear-extension-growth.json) from the
mature checkpoint reaches tick 304000 with the same 871 buildings and 210 roads,
40 durable original-footprint records, and two pending extensions. Nominal
building area increases from 162,545.24 to 166,441.37 m² (+2.397%) and total
capacity from 12,346 to 12,835. Root recomputed those metrics from the saved
states. The continuation script verifies canonical restore and another paired
2,000 ticks; its hashes are included. These sums are not rendered area union,
district density, or visual acceptance, and 40 is the number of extended hosts,
not necessarily the total number of completed projects.

The coverage audit now uses the production `buildExtendedBuilding` adapter and
persisted original footprints, rather than stretching the original kit over the
expanded parcel. Five focused raster tests pass, including identical legacy
coverage without history and a retained-form case that differs from a stretched
kit. The matched [before](rear-extension-rendered-before.json) and
[after](rear-extension-rendered-after.json) reports use the same compiled kit
hash and 0.5 m cell-center raster. Over the above continuation, projected kit
surface union above 1 m rises from 124,413.25 to 127,631.50 m² (+3,218.25 m²,
about 2.59%). Full-domain coverage rises from 19.440% to 19.942%, a gain of
0.503 percentage points. The denominator includes water and protected land;
surfaces include roofs, eaves and props. This is a before/after continuation,
not a disabled-extension control, and does not establish district density or
artistic acceptance. It confirms visible growth is measurable but still modest.
Other kit benchmark scripts retain their existing `buildingKit` export and do
not automatically measure persisted annexes.

The engine also retains a failed completion at .999 for retry; reservation tests
cover conflicts, but a terminal recovery policy still needs review.

Reproduce the scheduler fixture (absolute paths are required by the native test):

```sh
REAR_EXTENSION_FIXTURE_OUT="$PWD/.tools/rear-extension-review/scheduler-save.json" \
REAR_EXTENSION_SNAPSHOT_OUT="$PWD/.tools/rear-extension-review/scheduler-snapshot.json" \
node scripts/test-engine.mjs --release export_first_mature_rear_extension_fixture -- --ignored --nocapture
npm run wasm
node scripts/verify-rear-extension.mjs .tools/rear-extension-review/scheduler-save.json validation/rear-extension-lifecycle.json .tools/rear-extension-review/completed-snapshot.json
# With the local Vite server running on 127.0.0.1:5173:
node scripts/capture-rear-extension-study.mjs
```

## Residual block diagnosis — 2026-10-11

The [saved-frontage census](mature-frontage-allocation.json), reproduced with
`scripts/audit-frontage-allocation.mjs`, follows explicit parcel frontage road
IDs rather than proximity. At seed 87231/tick 304000, 560 of 871 buildings front
neighborhood-tagged roads (116,709.93 m² of 166,441.37 m² nominal envelopes),
309 front other roads and two have no frontage record. All 139 tagged road IDs
are live. A tagged road does not prove a building came from the canonical block
subdivider: outside frontage and subsequent infill can share it. Nevertheless,
the evidence does not support treating neighborhood planning as absent.

Do not repeat the road-opening reservation experiment. A new source review
initially proposed that same rejected family; comparison with
`early-reservation-gap-diagnosis.md` corrected the proposal before implementation.
Shared-edge candidates already filter blocked canonical lots, whereas candidates
with four new roads require the complete canonical lot set. The next bounded
diagnostic examines this asymmetry without relaxing geometry or access checks;
no partial-block production policy is approved yet.

The test-only sweep now checks all 162 anchor indices in 25 bounded planner
calls on seed 87231/tick 272000. Baseline and partial-lot treatment both admit
zero plans. Of 623 partial full-block evaluations (not guaranteed unique),
599 retain fewer than four collision-clear lots, 13 fail the experiment's
additional usefulness floor of two lots on each long edge, eight fail world
bounds and three fail road/parcel corridor clearance. Only 24 retain at least
four lots; none passes all remaining checks. This rejects the tested partial
82×50 m four-road family for this checkpoint, not arbitrary irregular blocks,
other seeds or all residual land development. No fresh-seed growth trial is
justified by these results. The source state is unchanged, both post-scan states
validate and restore canonically; construction replay is correctly untested
because no plan was admitted. See [the diagnostic](partial-block-diagnostic.json).
The unchanged baseline reproduces all six rejection counts from the prior mature
trace exactly. Root verified the source/save hashes, reviewed the bounded
witnesses, and ran the native release suite: 384 passed, zero failed, 50 ignored.
Two subsequently added focused diagnostic tests pass separately. The ignored
25-call diagnostic was run separately by the agent and its report reviewed by
root. No browser/WASM or visual improvement is claimed by this test-only change.

Reproduce:

```sh
PARTIAL_BLOCK_REPORT="$PWD/validation/partial-block-diagnostic.json" \
node scripts/test-engine.mjs --release mature_full_block_partial_lot_diagnostic -- --ignored --nocapture
```

The renderer already paves 96% of each open/renovating building's parcel width
and depth and draws a separate entrance-to-road access path. Broad green gaps
in the recent full-city capture mostly expose land outside these developed
surfaces. Painting those voids as service yards would obscure the urban-form
problem. Keep undeveloped/protected ground distinct while improving actual
occupation and access.

## Initial station-core depth experiment — not promoted, 2026-10-11

`initial_depth_diagnostic.rs` tests a single 12→18 m depth increase at the
ordinary building reservation boundary, after use/floor selection. It retains
the road-facing edge, original capacity formula, collision checks and tower
reservations. Eligibility requires an explicitly oriented ordinary frontage,
an online station with an ordinary-network route, StationCore profile and at
least 40 connected ordinary roads. The module and admission hook are test-only;
normal native/WASM simulation behavior is unchanged.

The paired fresh-seed experiment runs baseline/treatment through tick 242000
at development rate 16. Root measured all four saved cities with the same
production retained-form kit bundle and 0.5 m raster:

| Seed | Baseline projected union | Depth-18 projected union | Change |
| --- | ---: | ---: | ---: |
| 0 | 126,103.25 m² | 127,145.50 m² | +1,042.25 m² (+0.827%) |
| 87231 | 128,443.00 m² | 127,524.25 m² | −918.75 m² (−0.715%) |

See `initial-depth-seed-{0,87231}-{baseline,depth18}-coverage.json` and
[the experiment report](initial-depth-experiment.json). The intervention
accepts 20 and 21 deeper parcels respectively, but changes subsequent growth
and does not reliably increase total coverage. Keep it experimental. This is
evidence against promoting this particular local rule, not against deeper
buildings as part of coherent block subdivision. The next urban-form work
should address interior access and block occupation together rather than
enlarging scattered selected lots. Capacity still depends on use/floors, so
nominal floor area is not an independently calibrated population measure.

Validation: three focused native tests and five rendered-coverage tests pass;
TypeScript passes. The corrected paired diagnostic validates all four target
states, canonical restores and exact 2,000-tick same-target replay with the
treatment guard active on both replay engines. Root checked target counts and
SHA-256s against the actual saves and coverage inputs. An earlier diagnostic
dropped the guard before replay and sampled counts after replay; both defects
were fixed and the paired experiment rerun. No full-suite, cross-target or
browser visual acceptance is claimed for this test-only experiment.

Reproduce with:

```sh
node scripts/test-engine.mjs --release paired_station_core_initial_depth_experiment -- --ignored --nocapture
node scripts/audit-rendered-coverage.mjs --input .tools/initial-depth-review/seed-0-baseline-tick242000-save.json --output validation/initial-depth-seed-0-baseline-coverage.json
# Repeat the audit for depth18 and seed 87231 using their matching filenames.
```

## Mixed-row subdivision follow-up — 2026-10-11

[One narrow row plus one wider row](block-subdivision-study/mixed-rows.md)
preserves narrow-side lot positions and gains area in all 73 sampled blocks.
The 11/14 option adds 2.5–2.9% nominal area with about 10% fewer lots; 11/18 adds
4.2–4.3% with about 20% fewer lots. All previous baseline observations/variants
remain exact. Four new mixed-layout tests and existing block/spine checks pass
(14 total), plus the long diagnostic. This remains test-only: no district/use
assignment, footprint-aware capacity, lifecycle or visual acceptance is claimed.

## Within-block subdivision study — 2026-10-11

[Test-only comparison over 73 live block footprints](block-subdivision-study/README.md)
holds roads, inset/gaps, outside parcels and exact clearance checks fixed.
Target frontage 14 m adds 4.80–5.49% feasible nominal area but loses about 19%
of individual lots; 18 m adds 8.13–8.67% area but loses about 40% of lots.
No runtime change is promoted. Capacity currently depends on use/floors rather
than footprint, so wider lots alone could reduce activity. Next candidate is a
mixed subdivision coupled to physically justified building capacity, assessed
through complete growth and rendered coverage. Both diagnostic tests pass and
root's repeated two-seed report is byte-identical. This is opportunity geometry,
not final city density or visual acceptance.

## Oversized-route safety — 2026-10-11

[Route reconstruction now rejects a capped suffix missing its origin](route-cap-safety.md).
Previously consumers could accept it and spawn an actor midway through a trip.
Exactly 128 nodes remain accepted; longer reconstructed paths return no route.
Six routing tests and the native release suite pass (361 passed, 43 ignored).
The rebuilt WASM 300,000-tick final save remains identical to the baseline.
Transit cancellation/recovery is still incomplete. Visual review was retried,
but screenshot capture remains unavailable while the Mac is locked; no renderer
study was promoted.

## Compact routing adjacency — 2026-10-11

[Routing now builds a source-ordered adjacency vector per query](routing-adjacency/README.md),
avoiding the previous full-road scan at every visited node. The 300,000-tick
standalone WASM run takes 54.24 s versus the prior 56.05 s measurement, with an
identical final-save hash and cap/restore checks passing. This single comparison
is not proof of browser speedup. Native release suite: 359 passed, 43 ignored.
The temporary index is rebuilt per query; no persistence or cache invalidation
changes. A BTreeMap variant was slower and is not used.
The production WASM rebuild and all five tests in `tests/wasm.test.ts` also pass
(82.51 s for the file, with the endurance case's 60 s timeout unchanged).

## Endurance timing baseline — 2026-10-11

[Current production WASM measurements](endurance-profile/README.md) reproduce
the 300,000-tick endurance workload in 56.05 s unprofiled / 56.61 s sampled,
with identical final-save hashes, collection caps and exact restore passing.
This is close to the 60 s test limit; previous Vitest timeouts remain unresolved.
Routing and collision geometry dominate samples, not final serialization.
Next candidate: eliminate per-route-node full road scans while preserving
source-order routing semantics. No runtime optimization or timeout change yet.

## Fair-budget control — 2026-10-11

The [independent-budget follow-up](shallow-block-experiment/fair-budget/README.md)
confirms that preserving baseline scan slots avoids the prior regression:
both treatment saves and snapshots exactly equal their controls. At most eight
extra candidates per evaluation produced zero feasible 42 m alternatives in
13,737/16,021 attempts. This rules out score tuning for those sampled candidates.
The test-only hooks were removed; the archive contains reproduction code and
hashes. No runtime density or visual improvement is claimed.

## Shallower block fallback rejected — 2026-10-11

A 42 m shared-edge fallback for rejected 50 m blocks produced zero winning
admissions in two matched seeds, despite 19,330/22,689 attempts. Charging those
attempts to the existing candidate budget changed baseline selection and reduced
rendered coverage by 1.55%/3.06%. Root reproduced all four saves and snapshots
byte-for-byte and independently audited kit coverage. The experiment is removed
from engine source; its [patch and evidence](shallow-block-experiment/README.md)
remain reproducible. Future adaptive block search must account for candidate
budget displacement rather than treating extra shapes as free improvements.

## Destination-first follow-up — 2026-10-11

The [destination-first diagnostic](destination-first-access.md) tests lanes and
standard lots before connecting them to a host street. On the same exhausted
face, all 8,976 layouts were scanned without truncation: 74 clear lanes remained,
34 with zero viable lots and 40 with one. None supported two, so no connection
was attempted. Root reproduced the report exactly and verified nine synthetic
tests. This is test-only evidence, not a production density gain. Investigate
lot packing/early block subdivision across faces and seeds before adding further
route variants for these rejected layouts. Do not weaken occupied-parcel or
new-node clearances based on this result.

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

## Exhausted-face geometry diagnosis

The new [rejection audit](exhausted-face-rejections.md) evaluates all 2,440
candidates for one exhausted face against fresh captured geometry. It rejects
2,401 on existing parcel footprints, 22 on roads, 10 on endpoints and 7 on lot
viability. All start points clear parcels; 1,597 conflict within 5 m and 2,329
within 20 m. The remaining 72 parcel-conflicting rays clear their first 20 m.
This supports investigating obstacle-aware connectors, not claiming the face
is geometrically inaccessible. No alternative road shape has been validated.

Do not repeat the rejected road-opening or spine-priority experiments; their
mixed or negative coverage results are recorded in early-reservation-gap-diagnosis.md.
The attempted water-material adjustment was reverted after visual review failed
to establish an improvement; screenshots/water-review records the rejected trial.

## Bounded bent-connector search — 2026-10-11

[Offline geometry search](bent-access-witness.md) checks 29,280 two-segment
routes on the exhausted face. No route qualifies with standard frontage,
closer frontage, or small 8 m-deep lots. Twelve routes pass the earlier geometry
stages; eight then fail node/split clearance and four fail lot viability.
An apparent small-lot witness was rejected during root review because its bend
used a 1 m split-point test instead of the 18 m new-node rule. The diagnostic
now preserves that rule and has a regression for it. No runtime planner change
is justified by this finite search; it does not prove that all bent routes are
impossible. See the report for the exact family and remaining scope.

## Evening rendering — 2026-10-11

Accepted a modest hemisphere/ground-bounce adjustment after matched WebGPU
Miniature and Anime comparisons. Facades and lanes read more clearly at 19:00;
noon retains the previous lighting. The temporary A/B control was removed.
See screenshots/night-fill/README.md. Build/typecheck passes. No added lights,
passes or geometry; no performance improvement is claimed. This does not resolve
the sparse layout, facade repetition or the overall art gate.

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

## Road-pair spatial filter checkpoint

An isolated 5,000-segment quota-stepped AABB pair iterator is implemented and independently checked, with checkpoint validation hardened by the primary agent. See [road-pair-broadphase.md](road-pair-broadphase.md). It is not connected to production preflight yet; the live 512-node/edge cap remains. Next: exact narrow-phase parity and coordinated graph-stage integration, including job versions and worker profiling.

The next compatibility gate is now covered natively: a concrete f32 endpoint-cancellation witness proves raw AABBs can omit an existing rejection. A projection-aware `Bounds::from_segment_f32` adapter and actual-predicate parity tests address this before integration; eight preflight tests pass. Details in `road-pair-broadphase.md`. Production preflight still uses all pairs.

## Live spatial preflight integration

Spatial filtering now runs in production preflight, using a 16 × 16 grid and rounding-aware bounds. The one-axis attempt regressed mature logical work and was replaced before acceptance. Mature preflight: 119,232 → 102,097 work units (same graph/49-face reference); full native release: 342 passed, 42 ignored; explicitly run mature preflight/composed-trace checks pass. Rebuilt WASM returns all 3,180 native-reference descriptors exactly across quotas and JSON resume. See `road-pair-broadphase.md` and `access-discovery-wasm-profile.json`.

Preflight and filter checkpoints are schema 2. Unsupported-coordinate and high-work cases use bounded exact fallback. Normal saves without obsolete active jobs load; old nested preflight jobs reject explicitly and still need a migration/restart path. Live capture remains capped at 512 nodes/edges. Next: indexed node/edge validation and coordinated graph-stage scaling, then browser measurements. Visual appearance is unchanged by this engine change.

## Legacy active preflight save compatibility

The previous old-job limitation is resolved for preflight schema1: restore restarts only its validation over the original frozen graph. Every non-job city field is preserved. Two native legacy tests and all twelve preflight tests pass; rebuilt Node WASM verifies exact migrated roundtrip and deterministic 24-tick continuation on the 159-node/206-edge mature graph. Corrupt current and unknown future formats remain rejected. See `road-pair-broadphase.md` for reproduction and `legacy-preflight-wasm.json` for hashes. Other older planner formats are not covered by this migration.

## Preflight identity indexing

Node IDs, endpoint lookup, edge IDs and undirected edge pairs now use transient deterministic tree indexes. Mature preflight falls from 102,097 to 18,095 logical units with the same indexed graph. Full native suite: 345 passed / 42 ignored; rebuilt WASM matches all 3,180 descriptors and legacy1/2 migration checks. Preflight schema3 automatically restarts those older computations from frozen inputs.

Two 6,000-tick Chromium worker runs do not show a clear overall timing gain (p95 both0.4ms; max25.6→25.8ms). See `identity-index-preflight.md` for scope and raw evidence. Live cap remains512; next remove node-coincidence/remaining graph-stage scaling bottlenecks before increasing caps. Visual appearance is unchanged.

## Node coincidence spatial indexing — 2026-10-11

A transient nine-cell index replaces preflight's full node-pair scan for supported coordinates, retaining the exact f32 predicate, source-order failures and schema3 cursor meaning. Mature preflight falls18,095→5,692 units with unchanged49faces; full native351pass/42ignored and rebuilt WASM3180descriptor parity pass. Total discovery decreases only0.17%; no browser timing or density gain is claimed. See [node-coincidence-index.md](node-coincidence-index.md). Coupled512limits remain; continue graph-stage scaling and visual/density work.

## Shared live capture — 2026-10-11

Live search startup now computes the shared viability/collision snapshot once, retaining independent adapter limits and source order. Native355pass/43ignored; paired-vs-independent startup and200tick saves match; rebuilt WASM live200tick save/resume passes, as does3180descriptor parity. Native capture medians improve by roughly0.1ms on the mature fixture, not enough to explain prior worker spikes. Capture remains synchronous;512limits and visual acceptance are unchanged. See [shared-live-capture.md](shared-live-capture.md) for measured scope and the remaining consistency requirements.

## Compact midrise development study — 2026-10-11

Development-only `?midrise=compact` tests wider 4–11-floor apartment/office bodies inside existing parcels; production ignores the flag. The 850-building fixture gains only 1,341 m² projected kit coverage (+1.09%), with +0.81% triangles and +1.93 MB counted CPU geometry arrays. This cannot solve the citywide empty interiors. Containment, entry and cross-style vertex checks pass; runtime visual approval is pending because both screenshot APIs failed while the Mac was locked. Default appearance remains selected. See [study and reproduction](compact-midrise-review/README.md); do not promote without matched style/time views and browser measurements.

The focused render suite passes 34 tests and the production build passes. Full web testing is not green: unrestricted run264pass/2timeouts; two-worker run265pass/1timeout; isolated300,000-tick WASM endurance also exceeded60seconds (62.15seconds). Timeout limits were not relaxed. See the study report for scope; do not present this checkpoint as full validation or visual acceptance.

## Bounded PLATEAU source pilot — 2026-10-11

[Source pilot](plateau-pilot/README.md) now contains three individually retrieved
Tokyo meshes, exact provenance/hashes/resource license statements, and provisional
250 m crop geometry/metrics. Sources plus metadata total 26,254,659 compressed
bytes; no browser dependency is added. Six extractor tests and three source runs
pass, but Taito direct Building count differs from catalog by one and two catalog
byte counts disagree. Roof-edge semantics, missingness and unresolved completeness
are explicit. Calibration is not accepted; source-member reconciliation, diagrams,
district/context review and synthetic visual/performance comparisons remain.

## PLATEAU source reconciliation and diagrams — 2026-10-11

[The pilot](plateau-pilot/README.md) now reconciles all three catalog feature
counts: Taito's extra item is a geometry-free CityObjectGroup, not a missing
building. Explicit raw/status fields exclude 9999 unknown markers from floor
statistics. Twelve focused tests pass; root reran all sources and reproduced
the five generated JSON/SVG artifacts byte-for-byte. Three same-scale diagrams
were rendered in Chromium and visually checked. Compact sample median roof-edge
area is57.62m², station114.67m²; this supports preserving small-building fabric
rather than promoting uniform wider lots. No density targets or production
changes are accepted. Next compare matched synthetic crops and resolve land-use,
frontage and roof-shape calibration before a multi-seed growth/render trial.

## Matched-size synthetic morphology — 2026-10-11

[Three250m synthetic crops](synthetic-morphology/README.md) separate exact nominal
envelopes and actual production-kit raster coverage. The most envelope-covered
inland crop among60 tested windows yields37.75% nominal versus28.87% rendered;
compact PLATEAU roof-edge coverage is48.47%. Methods/selection differ and are
explicit, but facade widening alone is insufficient. Compact synthetic count130
versus source452 supports testing finer varied frontage and connected blocks,
with capacity semantics and finite caps retained. Four coverage tests pass; three
real-kit measurements and independent polygon unions are recorded. No production
policy change. Next: a compact-block prototype assessed through multi-seed growth
and actual rendered views; do not turn the sample directly into a citywide target.

## Compact varied frontage prototype — 2026-10-11

[Test-only compact rows](block-subdivision-study/compact-rows.md) add20–21% feasible
lots across the same73 blocks, but reduce nominal area3–4%. They preserve required
clearances and retain occasional larger merged sites. All16 block/spine tests and
the long diagnostic pass; the prior five variants remain exact. Not promoted as
a density fix. Land-budget accounting of the strongest inland crop shows37.75%
envelopes,25.65% open-road5m buffers and36.60% unclassified remainder; the latter
is not proven buildable. Next investigate connected usable remainder and block
layout, then integrate smaller frontage with footprint-aware capacity and actual
multi-seed construction/render validation. Production appearance remains unchanged.

## Production narrow midpoint infill — 2026-10-11

[Concrete enumeration fix](synthetic-morphology/narrow-midpoint-infill.md): append
existing-minimum8×12m shapes at grid positions after old candidates. Road2985
provides a regression where wider/shifted production candidates fail but this
shape passes exact geometry/access. Normal admission, construction and save/resume
are covered. Full native369pass/45ignored; rebuilt WASM normal30,000tick mature
continuation850→871buildings, including18new narrow buildings open; exact resumed
saves match every200ticks after midpoint. Per-road list cap66→100;48-site and
24-road evaluation caps remain. No save schema change. Browser timing and visual
review remain open; this is incremental infill, not overall density/art acceptance.

## Narrow-building entrance correction — 2026-10-11

The new mature continuation is exported for review. A geometry audit found five
new low apartment/office instances with no visible generic-facade entrance. The
shared kit now supplies a centered door and clear window opening, constrains
balconies with parcel bounds, and bounds narrow gable trim. Root verifies 19
focused geometry tests and production build/typecheck; style geometry matches.
[Coverage and reproduction](synthetic-morphology/narrow-midpoint-infill.md#render-review-follow-up--2026-10-11)
record a modest gain, not density acceptance. Headless WebGPU loaded 871 buildings
but screenshot capture timed out without frame metrics. Usable captures remain
the 850-building views; live visual and browser-performance review are still open.

## First-frame readiness and recovery — 2026-10-11

Graphics startup now waits for a successful JS frame submission before clearing
its watchdog. Synchronous frame exceptions stop the loop and reach startup or
runtime recovery feedback; disposal safely settles pending startup. Root passes
11 focused lifecycle/recovery tests, production build/typecheck, and a local
browser fault-injection check with saving still available. A software WebGL
founder capture succeeds, but the mature WebGPU capture remains unresolved; no
GPU presentation/performance or artistic acceptance follows from this fix.
[Evidence and limitations](screenshots/render-startup/README.md).

## 871-building static visual review — 2026-10-11

[Latest capture](screenshots/narrow-infill/README.md) now renders the complete
871-building continuation on software WebGL, with building/chunk queues empty.
Intermediate GPU draws were skipped during preparation; this is not normal
loading, hardware WebGPU or performance evidence. Root rejects overall art
acceptance: broad green block interiors and detached development strips remain
the dominant morphology problem. A four-floor office crown audit finds only two
instances, so that geometry change was not promoted as a density intervention.
Next prioritize connected interior development and matched-view comparison.

## Transit recovery checkpoint — 2026-10-11

Passengers whose final walking route is unavailable can now leave a bus or
inbound train into a persisted waiting-at-stop state. The queue is capped at 256;
the scheduler examines at most eight entries and issues at most eight route
queries per tick. Failed paths are reconsidered on geometry changes, with a
bounded timer fallback if the revision token is exhausted. Destination reopening
without a geometry change is covered separately. Full waiting areas retain
vehicle ownership; full traveler capacity avoids route queries.

Waiting residents emit pedestrian movement tuples at their saved curb/platform
position. Reconnection creates a walk from that exact position. Activity schema
2 migrates old saves with empty recovery state; older binaries reject new saves.
Root review caught and corrected missing movement tuples and same-geometry
reopening. Independent checks: native **376 passed, 0 failed, 45 ignored**;
rebuilt WASM and four focused web suites **43 passed**; production build passed
with the existing large-bundle warning. The WASM recovery test uses the historical
seed5 bus fixture with a deliberately disconnected final walking leg, then a
synthetic road reconnection; it is not evidence of procedural road discovery.

Permanent route unavailability still requires an explicit cancellation/replanning
policy; a bounded waiting queue alone does not satisfy the complete travel
requirement. Queue-full passengers may retry at later terminal visits. No density,
art, hardware WebGPU, or browser performance acceptance follows from this change.

## Deeper center-street trial rejected — 2026-10-11

[Matched WASM evidence](deep-spine-review/README.md) tests adding an 82×110 m
center-street candidate before the existing square, retaining the fixed scan
budget and all clearances. Pure layouts and 10,000-tick candidate save/resume
pass, but actual projected coverage changes +2.46% in seed0 and −3.37% in seed87231
at tick242000. Candidate native tests also exposed three planning/scheduling
expectation failures. Production planner and WASM were restored; only rectangle
geometry/replay regressions and evidence are retained. Restored native371pass,
0fail,45ignored. Next isolate displaced projects/reservation timing before
promoting another local ranking or size preference; no art acceptance claimed.

## Spatial trial follow-up — 2026-10-11

[Matched layout map](deep-spine-review/layout-87231.png) locates the rejected
deep-spine trial's gains and losses. Seed87231 has fewer ordinary road metres
and stops new-building completions earlier, but final saves cannot prove which
reservation displaced an alternative project. Corrected the earlier scan-budget
explanation: spine exhaustion does not stop ordinary candidate scanning. Keep
production unchanged; trace planning/admission decisions before another policy
change. The SVG generator uses exact saved nominal envelopes, not rendered
coverage; root rendered and inspected the final PNG.

## Mature planner scheduling ruled out — 2026-10-11

The test-only `mature_planner_trace_tests` module restores the current 871-building
WASM save natively without modifying it. [Trace](mature-planner-trace.json): all
top-level neighborhood gates pass (demand 4, no active projects, no reserved plan,
160 locally qualified anchors). Twenty-five bounded calls cover all 162 anchor
indices; none accepts a neighborhood layout. A separate 2,000-tick normal
continuation makes 25 early reservation attempts and advances the anchor cursor
between samples, while buildings/parcels/roads stay at 871/871/210. The final
cursor equals its initial value after cycling; that alone is not evidence of a
stalled scheduler.

This rules out scheduling starvation for this checkpoint, not all geometry
opportunities. The per-call shared/full/spine budgets still apply; the trace does
not exhaust every hypothetical layout or classify the primitive rejection.
Next inspect or extend geometry that fits residual developed blocks, with exact
admission evidence, rather than increasing scheduler cadence or demand. The
previous larger-block trial remains rejected. This trace is native diagnostic
evidence, not cross-target growth equivalence or visual acceptance.

Reproduce with an existing exported mature save:
`CITY_TRACE_SAVE=/absolute/path/save.json MATURE_PLANNER_TRACE_REPORT="$PWD/validation/mature-planner-trace.json" node scripts/test-engine.mjs --release mature_planner_trace -- --ignored --nocapture`.
The default input is `.tools/narrow-frontage-review/final-save.json` (ignored,
generated by `scripts/verify-narrow-frontage-wasm.mjs`). Root ran the diagnostic:
1 passed; production code is unchanged.
Input SHA-256: `74db8e9f9ac2bef4a828bd877d6d31a124ba522d9365033a50c51bc963add01d`.

### Exact neighborhood rejection evidence

The admission checks now live in `engine/src/neighborhood_admission.rs` and
return named reasons through the original boolean wrapper. All checks, ordering,
clearances and candidate budgets are retained. Test-only scoped capture adds no
saved state or runtime telemetry. The trace retains at most eight witnesses per
reason for the entire sweep, plus per-call counts.

Across 1,736 candidate evaluations (not necessarily unique layouts), first
rejections are: proposed-lot geometry 1,107; junction degree 400; fewer than four
lots 214; boundary corner 7; road/parcel corridor conflict 7; protected crossing 1.
All eight retained lot-geometry witnesses fail existing-parcel clearance. These
are bounded first examples, not a census proving every lot failure has that
cause. The evidence favors parcel-aware residual-space development and access
planning over loosening clearance or merely accelerating the current rectangular
block proposals. No new development policy is promoted in this checkpoint.

Root compared every pre-existing JSON trace field against the previous report:
exact match, including all cursor samples and city counts. Full native suite:
376 passed, 0 failed, 46 ignored; two subsequently added capture cleanup tests
pass separately. WASM rebuild passes and three focused WASM suites pass 5/5.
The independently executed ignored mature trace passes 1/1. No new screenshot
or visual improvement is claimed by this behavior-preserving refactor.

## Rear extension opportunity — 2026-10-11

[Geometry experiment and map](rear-extension-diagnostic/README.md) finds 145
compatible rear parcel extensions in the mature871 checkpoint, adding 13,422.74
m² of nominal envelopes while preserving frontage and existing clearances. All
562 eligible unchanged footprints pass the baseline control; two focused tests
pass independently. The hypothetical map still has large gaps. No production
growth, construction, capacity, occupancy or rendering rule changed. Next build
a bounded phased extension lifecycle with reservations, preserved entrances,
completion-only capacity, save/replay and both-style visual checks. This is a
candidate for testing, not acceptance of city density or the art target.

## Master-plan generator (2026-10-11)

The incremental frontier/infill planner is replaced for new worlds by a seeded master plan and its demand-driven realization; see [the master-plan review](masterplan-review/README.md) and [world growth](../specs/02-world-growth.md#master-plan). Three mature seeds measure 49.6–51.6% compact and 39.9–42.2% station rendered coverage, against 28.9% and 20.1% for the legacy planner. Native suite: 392 passed, 1 pre-existing failure, 52 ignored. Web suite: 290 of 290. Open items are listed in the review.
