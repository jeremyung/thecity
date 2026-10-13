# Road rebuild staging — 2026-10-12

Status: candidate, not performance or growth-continuity acceptance.

The renderer retains the committed road/construction/station groups while a
replacement is built. `StagedKit` constructs up to 128 primitives per call,
then merges at most 12,000 vertices and 36,000 indices per call. Candidate groups
receive the current material style immediately before their atomic scene swap.
One active build and one latest queued snapshot avoid repeated cancellation.
This retains all road detail; it does not lower geometry quality.

## Evidence

`node scripts/audit-road-staging.mjs` compares the current renderer with the
local pre-change `.tools/road-batch-review/before-staging.mjs` bundle. That
baseline is an ignored local capture, not a portable committed dependency.
The diagnostic intentionally drains jobs offline to compare completed output;
it does not establish production scheduling or browser performance.

The initial measurements (superseded below) record Apple M2 Max, arm64 macOS,
the fixture path/hash, and eight WASM snapshots starting at tick 18918, 80 ticks
apart. Every snapshot matches the baseline's sorted triangle attribute/material
hash, instanced transforms, mesh count and triangle count. The four full rebuilds
use 49 step calls each; construction-only updates use no staged steps.

Full-build preparation is 18.9–30.2 ms; maximum individual step is 2.1–4.1 ms
per build. The synchronous baseline full calls are 38.9–74.1 ms. This is a
sequential diagnostic with cold-start and allocation effects, not a paired
browser speedup claim. Total CPU cost is not consistently reduced. At one step
per animation frame, 49 calls occupy approximately 0.82 seconds at 60 fps,
before queued snapshot delay or slow frames. Active geometry can lag the current
simulation. A coherent batch swap alone does not establish coherent visible
traffic/building growth.

Browser startup smoke: existing localhost tab, WebGPU, Anime, High detail,
50 mm lens, DOF off, Watch camera, paused day 0 07:18 with 21 buildings.
All four geometry queues reached zero through ordinary scheduling. The rolling
120-frame ready window reported p95 9.2 ms and longest 12.2 ms. Seed, exact tick
and weather were not exported, so this is only a startup smoke check, not a
reproducible visual comparison or growth-performance gate.
[Screenshot](screenshots/ground-depth/road-staging-founder.png).

27 focused tests pass (staged kit, road batches, ground surfaces, paved-ground
clearance). TypeScript and production build pass with the existing bundle-size
warning. A read-only Luna audit found the queued preparation exception bypassed
signature reset; root confirmed and fixed it, with a regression proving committed
roads survive and the request signature permits retry. Material ordering and
geometry parity were independently checked by root.

## Next acceptance work

Preparation still builds explicit custom geometry, lighting, station geometry
and construction synchronously. Split that work into resumable preparation
before tuning cadence; merely increasing step quotas will not fix its measured
19–30 ms cost. Measure input-to-visible latency during actual developing-city
growth, verify actors and dependent buildings against committed infrastructure,
and check style/reset/fallback transitions. Reject or revise the candidate if
it trades hitches for visibly stale infrastructure. No full-city fidelity or
original ground-artifact resolution is claimed.

## Resumable preparation and shared frame budget

The candidate now records a generator immediately and prepares bank segments,
individual road geometry and station platforms over subsequent animation frames.
Submitted topology is cloned before both immediate and queued jobs, so later
snapshot mutation cannot affect a partially prepared build. Cancelling an active
generator runs its cleanup and releases unfinished construction geometry.
Construction-only updates retain the existing synchronous path.

Preparation advances at most 128 yield points within a soft 2 ms budget. Primitive
and merge work advances at most eight existing bounded slices within its own soft
2 ms budget on later frames. These phases do not run together in the same call.
A single indivisible operation can overrun the budget; this is not a hard 2 ms
bound. Lighting, an individual station, construction merging and schema allocation
remain indivisible and must be profiled on larger/mature inputs.

The first generator trial with only 16 preparation units per frame retained
geometry parity but needed 78–81 calls (1.30–1.35 seconds at 60 fps):
[intermediate diagnostic](road-staging-preparation-cost.json). That unnecessary
waiting motivated allowing cheap operations to share the frame budget.

[Current measurements](road-staging-cost.json) retain parity in all eight
snapshots. Full jobs start in 0.44–0.77 ms, finish in 19–25 update calls, and have
maximum individual calls of 3.98–5.65 ms. The synchronous baseline's full calls
are 38.57–71.40 ms in this run. Equivalent call counts at 60 fps are 0.32–0.42
seconds; actual browser latency is unmeasured and includes queueing/render load.
Total CPU is 47–58 ms, so the claim is redistribution of work, not reduced total
cost. Timings are sequential CPU diagnostics, not a paired browser benchmark.

30 focused tests and production build pass (existing bundle warning). Root
strengthened the delegated cancellation test: it now actually advances into
construction, observes created geometry disposal exactly once on cancellation,
and checks committed groups remain. The previous version only cancelled an
unstarted generator despite its test title. The strengthened road suite passes
14/14 independently; TypeScript passes. The startup screenshot earlier in this
report predates this generator revision and must not be cited as its browser
validation. Next: developing/mature ordinary scheduling, visible infrastructure
lag, camera motion, style/reset and fallback checks. Candidate remains unaccepted.

## Ordinary browser growth — 2026-10-12

Imported `.tools/fidelity-review/sidewalk-live-growth.json` through the UI and
used the production scheduler throughout. Seed 87231, High detail, WebGPU,
Overview, 50 mm, DOF off, Anime, clear weather, 1280×720. Import displayed
442 buildings (the renderer/session had advanced beyond the input checkpoint).
Requested 8× simulation and retained 16× development. No queues were drained
manually. At 578 buildings/day 1 06:21 the rolling 120-frame window reported
33 fps, frame p95 95.0 ms, CPU p95 41.3 ms and longest frame 203.1 ms, with five
neighborhood batches and one each sidewalk/road build pending.
[Growth capture](screenshots/ground-depth/road-staging-growth.png).

Pause was requested at the displayed 663 buildings/day 1 09:45. All queues
subsequently reached zero normally. The ready Anime window reported p95 15.5 ms,
longest 31.9 ms; settled Miniature p95 16.9 ms, longest 20.9 ms. These windows do
not include every transition and are not matched before/after performance
measurements. Accelerated growth remains visibly/perceptually unaccepted.

Miniature then Anime both applied successfully without losing the scene:
[Miniature](screenshots/ground-depth/road-staging-miniature.png),
[Anime](screenshots/ground-depth/road-staging-anime.png). Miniature preparation
reported 295 shader samples, 2,559 ms submission and 6,406 ms await; style
latency remains unresolved. Final console query returned no warnings/errors.
An initial graphics-start delay message recovered before Retry could be clicked;
that startup failure mode remains open independently.

The UI-exported final checkpoint is retained at
`.tools/fidelity-review/road-staging-live-growth.json`, SHA-256
`01d1b6d6d239feb2e284adcac2588215eb0e55ea9553dd8a33aa95da39503ef1`.
Restoring it in WASM confirms tick 27769, 664 buildings, day 1/minute 586.14,
clear weather/intensity 0. The final tick progressed while pause was applied;
the export, rather than the earlier displayed 663 count, is authoritative.

## Proven dependency gap and next intervention

Root independently checked the Luna code audit: `setWorld` publishes new
bridge spans, transit state, access paths and building work immediately, while
`updateRoads` retains the previous committed road group. `setMovement` separately
replaces actor positions; `updateActors` does not consult road commit state.
The six-float movement contract carries position and surface height but no road
dependency. Existing actors can enter a newly opened road, so gating only newly
created actor IDs would be insufficient. `building_access` already carries a
road ID, but its absence must not silently authorize a building on unavailable
infrastructure.

This proves a possible visible-state mismatch by code; the sampled browser
captures do not isolate a particular actor crossing missing geometry. Do not
claim an observed floating-vehicle incident or an exact latency measurement.
Next bounded intervention: establish explicit renderer-visible infrastructure
dependencies, keep unrelated movement running, and admit affected building and
actor presentation only when its supporting geometry is present. Include
existing travelers entering new roads, bridge surface poses, worker snapshot/
movement ordering, world reset, queued/coalesced commits, and both styles in
tests. Consider small full-detail road additions if dependency holding causes
noticeable popping; do not conceal the problem with blanket actor suppression
or freeze all world state. No architectural rewrite or quality reduction is
justified by the current evidence.

## Building admission implemented — partial dependency repair

`VisibleInfrastructure` tracks requested road geometry identities and the batch
actually committed by the renderer. Each road candidate captures its identities
with a world-reset epoch; stale tokens cannot unlock reused IDs after reset.
`updateBuildingBatches` skips buildings whose explicit `building_access.road_id`
is missing, closed, disconnected, or differs from the committed geometry. It
continues scanning so unrelated eligible buildings can still be built. Blocked
entries do not hold otherwise valid neighborhood chunk work. Existing committed
building geometry remains until its replacement can be admitted.

The exported tick27769 city contains 664 buildings and 664 access entries, with
no missing building dependencies. This is a fixture coverage check, not proof
that every engine stage always publishes access. Missing access deliberately
fails closed and remains visible in the pending-building count; future coverage
must include founding, demolition/renewal and malformed snapshot behavior.

40 focused tests and production build pass (existing bundle-size warning).
The module tests cover unchanged unrelated roads, moved endpoints/decks, closed
and disconnected roads, missing access, obsolete intermediate geometry and reset
epochs. A renderer integration test runs the actual road commit and building
queue methods: geometry creation is withheld before commit and occurs afterward.
Root corrected the delegated reset test's initially exposed stale-token hole by
adding capture epochs, then independently ran the suites. Browser appearance,
queue completion and added snapshot-processing cost are not yet verified for
this revision; prior screenshots predate this building gate.

This does not close the dependency gap. Actors, bridge poses, ground, access
paths, rear extensions and transit visuals still consume current world state.
A bounded engine audit located movement dependencies: routed travelers and buses
need the current edge; building access needs its frontage road; rail queues and
alighting need platform provenance; train and expressway footprints can span
multiple supporting segments. Do not approximate this by gating only new actor
IDs. The six-float movement tuple currently lacks these dependencies; a typed
same-tick integer sidecar or equivalent explicit contract is needed, with worker
epoch/order tests. No simulation, save schema or movement protocol was changed
in this partial repair.

## Building gate browser and engine coverage

Root reviewed and independently reran `node scripts/audit-building-access-gate.mjs`.
The [corpus](building-access-gate-corpus.json) samples seeds 0, 1 and 87231 from
founding through 12,000 ticks at development16, every200 ticks, plus the saved
664-building fixture. All32,604 building observations have exactly one access
entry to an open connected road with existing endpoints (65,208 endpoint checks,
zero failures). Repeated observations are not unique buildings. This does not
cover mature renewal, every intermediate tick or malformed snapshots.

Browser: normal UI import of tick27769, seed87231, High detail/WebGPU, Anime,
Overview/50mm, DOF off, clear weather,1280×720. The building/road/sidewalk/chunk
queues reached zero with664 buildings. Requested4× simulation and16× development;
the city grew to834 buildings. At782 buildings the rolling window reported frame
p95 29.8ms and longest95.8ms; near pause at834 it reported p95 92.5ms and
longest154.6ms. These differing windows demonstrate ongoing hitches, not an A/B
speedup. Five building entries visible near pause subsequently cleared along
with all other geometry queues through normal scheduling.
[Growth](screenshots/ground-depth/building-gate-growth.png).

Settled Anime p95 was15.1ms/longest15.9ms; Miniature successfully applied with
all queues still zero and p95 16.8ms/longest25.7ms.
[Miniature diagnostics](screenshots/ground-depth/building-gate-miniature.png),
[final scene](screenshots/ground-depth/building-gate-final.png).
Style preparation still took seconds (351samples,2635ms submission,7926ms await).
An initial graphics-delay warning again disappeared without intervention; final
console query returned no warnings/errors. These are scoped browser checks,
not continuous-motion, fallback or full artistic acceptance.

The final UI export is `.tools/fidelity-review/building-gate-live-growth.json`,
SHA-256 `02229ace29f754d251e9019cdc727c12b83ec944ec111bf88d0da0f3507319ae`.
WASM restore independently confirms834 buildings, tick34591, day1/minute995.46,
clear/intensity0. The gate did not stall this run's valid development.

### Movement contract implementation constraints

The next implementation should emit position and supporting-infrastructure data
from the same engine traversal, rather than duplicate movement logic in a second
pass that can diverge in row omission/order. Keep dependency IDs as integers in
a same-tick sidecar, version the worker contract, and preserve buffer recycling,
world epochs and backpressure. AccessPath currently stores points/heights but no
road provenance, so access and alighting dependencies require explicit treatment.
Resolve current road edges and full train/expressway footprints authoritatively;
do not infer the nearest road from rendered coordinates. The renderer must retain
the last valid presentation of affected existing actors until support is visible,
while unrelated actors continue. Camera following and bridge surface metadata
must use that same presented pose. Test road-open transitions for existing actors,
unknown/missing dependencies, coalesced snapshots, reset/reused IDs, platform
waiting/alighting, and save/replay. No movement protocol change is implemented yet.
