# Compact neighborhood lane candidate — 2026-10-13

Primary deficiency: ordinary compact districts use local roads for every recursive
subdivision, while narrow lanes account for only about 3% of mature ground-street
length in the previous whole-city audit. The candidate keeps the first cut as a
6.5 m collector and makes subsequent compact cuts 4 m lanes. Lattice collectors,
station districts, arterials and elevated corridors are unchanged by this rule.
Lot admission uses the existing lane setback; this is an engine geometry change,
not a renderer-only narrowing. Later parcel partition and access-stub generation
can change, so road/lot IDs are not matched across newly generated cities.

An access prerequisite was found: bus service previously selected connectors on
the general non-rail graph, including lanes. New bus connectors now use only open,
connected local, arterial and bridge roads. Walking routes retain lane access.
Existing saved bus lines are not retroactively rerouted by this candidate.

## Initial evidence

Same-seed native plan exports, before and after the subdivision change:

| Seed | Lane chord length before → after | Lots before → after |
| --- | ---: | ---: |
| 87231 | 643 → 2,212 m | 1,839 → 1,886 |
| 0 | 649 → 3,174 m | 1,730 → 1,746 |
| 5 | 845 → 2,804 m | 2,062 → 2,066 |

See `compact-lanes-plan-comparison.json`. Exports use the existing ignored
`masterplan::export` test and `PLAN_SEED`/`PLAN_JSON` switches; files are retained
under `.tools/compact-lanes/{before,after}-SEED.json`.

Seven routing tests pass, including a concrete lane shortcut/wider bus detour,
reverse travel, blocked junction and closed-road cases. Twenty bus-focused tests
pass before the subdivision change. Eight master-plan tests pass after it,
including deterministic generation, disjoint lots, frontage clearance and elevated
curve clearance. WASM rebuild passes. These are bounded checks, not visual acceptance.

`scripts/audit-compact-lane-growth.mjs` generates ordinary three-seed cities,
captures tick 10000/83000 worlds, checks sampled bus road classes and compares
100 steps of original/restored continuation at each checkpoint. All six captures
pass: mature worlds have 65/87/75 open lanes and 1886/1746/2066 buildings in
seed order 87231/0/5. Every sampled bus edge remains local or arterial; each seed
has an operating route. Exact save equality holds for all 600 continuation steps.
See `compact-lanes-growth.json`. Early tick10000 worlds have no open lanes yet,
so this rule does not improve those earliest streets. Full native validation now passes: 471 passed, zero failed, 54 ignored; see
`compact-lanes-native-tests.txt`.

## Stop condition and remaining work

Keep this as a candidate until matched ordinary neighborhood/frontage/overview
views in both styles demonstrate a useful improvement. Hold camera, lens, time,
weather and renderer settings fixed; compare same seed/tick while explicitly
acknowledging that generator changes produce different parcel layouts. Include
weak and developing views. Reject collisions, inaccessible frontage, lost bus
service or changes whose only benefit is a higher lane count.

This does not fix straight river/rail corridors, oversized expressway composition,
all building gaps, city edges or overall artistic fidelity. No claim of improved
frame time or release readiness is made.


## Controlled baseline prepared

A source copy at `/tmp/thecity-compact-baseline-engine` restores only the old
compact subdivision rule; it retains the candidate bus eligibility fix. Its WASM
lives in `.tools/compact-lanes/baseline-wasm`, separately from the candidate public
WASM. The same audit script accepts optional WASM directory, output directory and
report path. `compact-lanes-baseline-growth.json` confirms all six baseline captures
and 600 original/restored steps pass. This isolates street classification from
bus eligibility in the intended visual comparison. Baseline snapshots/saves are
under `.tools/compact-lanes/baseline`.

An isolated Vite server on 5174 serves that WASM with the current renderer; 5173
continues to serve the candidate public WASM. No paired render is accepted yet.
Overview centroids depend on generated buildings, so the automatic overview pose
is not guaranteed identical across versions. Use an explicit common pose or report
any camera difference; do not call automatic overviews a matched comparison.

Preliminary browser capture `screenshots/compact-lanes/baseline-seed0-overview-preliminary.png`
is explicitly **not settled evidence**: subsequent renderer details showed seven
neighborhood batches queued, despite zero buildings queued. The missing distant
buildings must not be interpreted as sparse simulation or a baseline visual defect.
Wait for all production queues to finish before the paired captures. Baseline UI
is High detail, 50 mm, DOF off, Miniature, live 17:00 at tick83000.

## First settled matched render — seed 0, tick 83000

Both Miniature captures in `screenshots/compact-lanes/seed0-waterfront-miniature-*.png`
are 1280×720, High detail, 50 mm, DOF off, live 17:00, overcast intensity0.32354864.
All four production queue counts were zero and both backends reported ready.
The new dev-only Read camera pose control reads the actual director camera/target;
its captured JSON confirms identical numerical poses in both versions. See the
adjacent `seed0-waterfront-miniature-pose.json`. This control is read-only and does
not change the camera or simulation. TypeScript and56camera tests pass.

Observation: the broad expressway and river-bank roads remain dominant, and the
water still reads as a large repeating reflection surface. Narrower secondary
lanes do not resolve this view. Building realization differs because changing lot
partition changes downstream generation; the comparison is same seed/tick and
viewing conditions, not identical buildings. A centerline projection diagnostic
finds only one of64exact-endpoint local-to-lane changes in seed0 has its midpoint
inside this waterfront frame. It is a weak-view regression check, not a positive
acceptance test for interior narrow streets.

Two common-home framing attempts (candidate6486/baseline7246 and6336/6289) produced
different actual camera positions, despite identical subject position/dimensions.
The obstacle-aware building shot selector chooses different views for changed
surroundings. Those views are NOT accepted as matched evidence. Do not repeatedly
hunt for an advantageous subject; use an explicit shared safe pose for the interior
comparison. Candidate retention remains pending that neighborhood evidence and
additional seeds/states. Automatic overview framing has the same comparability
limitation noted above.

The Anime pair is also captured after both100/100shader preparations applied and
all production queues reached zero. `seed0-waterfront-anime-pose.json` records the
same actual camera as Miniature. Before/after metadata equality was checked for
both styles. Anime adds edge/material definition but the same broad-road and
water-surface composition problem remains. These are four settled renders of one
weak view; not three-seed/neighborhood acceptance or a performance benchmark.

## Shared interior pose prepared

The dev-only camera panel now accepts an explicit JSON position/target. The director
validates finite coordinates, target bounds, polar angle, distance, minimum height
and obstacle clearance before changing state; rejected poses are inert. Building
review uses the same validated application path. Added tests cover identical held
poses across differing surroundings and inert rejection of invalid/obstructed
requests. All58camera tests and TypeScript pass.

Interior pose is position[-120,180,120], target[-250,0,0], 50mm1280×720. Both browser
versions accepted it and reported identical actual coordinates (z differs from120
by normal floating-point rounding only). `compact-lanes-interior-framing.json`
records a descriptive projection check: changed local-to-lane segment midpoints
inside this frame are2/7/4 for seeds87231/0/5. This counts midpoints, excludes
occlusion, and is not visible street coverage. Initial interior review starts with
seed0 while production neighborhood batches finish. No new geometry intervention
is being layered over the lane candidate.

## Settled interior result — still unaccepted

Four settled `seed0-interior-{anime,miniature}-{before,after}.png` captures now use
the exact shared pose, with adjacent actual-pose JSON. Metadata equality was checked
for both pairs; all queues were zero, style preparation complete, seed/tick/weather/
lens/viewport/quality/DOF matched. Normal camera-update floating point rounding is
below1e-10m and identical between versions. Camera readout/application remains dev-only.

The candidate visibly narrows side streets but does not produce a clear overall
fidelity win in this view. A broad elevated road still dominates the center;
substantial grass wedges/gaps remain, with more conspicuous green space in parts
of the candidate. Different building heights and silhouettes are downstream effects
of new lot generation and cannot be credited solely to road width. The existing
8mature images (waterfront+interior,both styles,before/after) support this limited
finding, not candidate retention or full three-seed acceptance.

Next diagnose these visible gaps against admitted lots, rejected corridor lots,
lot-ground polygons and final building envelopes, while checking the same interior
pose in the other seeds/developing states. `masterplan_ground` emits admitted,
developed lot outlines only; grass is the natural terrain underlay. That source
contract alone does not identify which category explains a particular green wedge.
Do not fill everything with decorative paving/trees or add another density scalar.
Keep the lane rule a reversible candidate until evidence supports retaining it.

The follow-up [lot admission audit](lot-admission-gaps.md) identifies a concrete
roji-deleted lot at a visible grass gap. Whole-lot removal after corridor insertion
is the next structural issue to correct; another road-width/density scalar is not
the next action. Lane candidate remains unaccepted.
