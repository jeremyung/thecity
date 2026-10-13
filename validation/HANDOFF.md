# Engineering checkpoint — 2026-10-10

## Motion retry — user requested

Repeated warmed paused1814 orbit: observed UI max11.1ms DOFoff/no recording,
19.9ms DOFoff/recording and14.0ms DOFon/no recording.143.1ms spike did not recur
in observed120-frame windows; full-run attribution remains unproven. No production
edit. Recording and metric screenshot retained; see `roof-services.md`. Tab32
remains paused Miniature with DOFon. Proceed to broader living-neighborhood gaps
rather than infer a performance fix or continue micro-detail tuning.

## Roof detail verification follow-up — 2026-10-12

Final seeded layout inspected in both styles at held1814,DOFoff,High/WebGPU,
1280x720. Miniature orbit recorded and0.5/1.5s decoded samples show attached roof
services from different angles. Full temporal/growth/night acceptance remains open;
an earlier orbit window had an unisolated143.1ms spike. Production build passes
with existing chunk warning. See `roof-services.md`. Close further roof expansion
for now; next review the complete living neighborhood's repetitive facades and
empty ground/roads against concepts. Tab32 city, tab33 saved orbit review.

## Roof services candidate — 2026-10-12

Flat-roof services now have physical supports, recognizable HVAC and a human-scale
access headhouse. Existing materials/batching reused. Root corrected gaps/buried
pipes and undersized door in delegated draft.25 geometry tests and TypeScript pass.
Matched Anime study supports roof silhouette improvement; final seed mirroring,
Miniature/motion/night still need browser validation. See `roof-services.md`.
Source backup `/tmp/city-kit-before-roof-services.ts` predates this pass only;
never reset the larger dirty worktree. No commit or full fidelity claim.

## Daylight fill spot checks completed — 2026-10-12

Candidate-only dawn06:00/dusk18:00/night22:00 captures now exist in both styles,
fixed building1814, DOFoff, High/WebGPU,1280x720. Root reviewed all six: no obvious
washout in this view; roof/wall separation and local night lights remain legible.
These are not matched before/after twilight pairs or motion acceptance. Keep fill
scalar provisionally and stop tuning it without new evidence. See `daylight-fill.md`.
Next primary action: examine a broader living neighborhood against locked04/01;
address frontage/architectural richness or activity from that evidence. Camera
and full-cycle acceptance still open. Tab32 holds Anime22:00 paused review scene.

## Daylight fill retained after capture recovery — 2026-10-12

Supersedes the reverted trial below: fresh browser tab32 captured matched Anime
and Miniature pairs at1280x720 with deterministic building1814 framing. The
one-parameter daytime hemisphere change0.5→1.0 improves shaded facade readability
while retaining cast shadows. Candidate retained provisionally; night endpoint
unchanged, dawn/dusk and motion still pending. See `daylight-fill.md` and its four
captures. TypeScript passes. No concept acceptance, performance claim or commit.

## Camera conclusion and daylight trial — 2026-10-12

Watch retains the lower-angle alternate-street candidate; the earlier steep-view
approach was rejected. Follow now separates subject focus from look-ahead. See
`watch-visibility-experiment.md` and `camera-dof-baseline.md` for evidence and limits.
Both-style held DOF-off captures and three sampled frames of the Anime Watch
transition support better readability, not full motion or concept acceptance.
The remaining visual gap includes overly dark shaded facades. An isolated daytime
hemisphere-fill trial (0.5 to1.0) was reverted because the browser screenshot API
returned zero width; there is no validated lighting change. Current production
formula remains `1.05 - daylight * .55`. Next: recover capture output and compare
daylight fill at fixed scene/camera/time before any further lens tuning. Do not
interpret the capture problem as a rendering or simulation failure: normal import
completed and all four render queues were zero. All work remains uncommitted.


## Persistent street lighting candidate — 2026-10-12

Cache eviction hypothesis rejected:80 misses had80 distinct keys, no same-key
prior deletion. Rebuilt street-lamp InstancedMeshes create UUID-dependent shader
keys. Root integrated Luna's fixed-capacity lighting pool: candidate matrices stay
off-scene until atomic road commit; four instance identities and owned materials
persist.40 focused tests pass; build passes. No trace wrappers remain.
One live-growth window showed26.5ms longest frame with2 batches pending, encouraging
but not full/matched performance acceptance (viewport changed). See latest section
of `backend-growth-attribution.md`. Next: repeat node-build attribution at fixed
viewport, verify both-style growth and lifecycle. Candidate remains uncommitted.


## Shader-node preparation attribution — 2026-10-12

Two ordinary-growth traces isolate synchronous node-state preparation as the main
render stall in sampled frames:228ms of245.5ms at tick9480. Backend pipeline creation
and attribute upload do not account for that cost. See `backend-growth-attribution.md`
for raw traces, exclusions and limits. Temporary wrappers removed from production.
Root and Luna inspected a plausible zero-reference builder-cache eviction during
batch replacement; same-key deletion/rebuild correlation is the next gate before
trying bounded post-render resource retirement. No quality reduction or cache-key
hack introduced; no broad performance acceptance. Lens candidate remains provisional.


## Night orbit evidence — 2026-10-12

Saved both-style canvas recordings and decoded-frame samples in
`screenshots/camera-dof-baseline/`; see `camera-dof-baseline.md` for exact setup.
Miniature visibly separates foreground/background; bright near windows broaden.
Sampled frames do not prove temporal acceptance. Camera recording also exposed a
120.3ms longest frame with queues zero; cause unisolated. Keep DOF candidate
provisional, avoid stronger blur tuning, investigate first-use/capture costs and
composition next. Tab29 is the live paused city; tab30 is the local clip review.


## Night lens comparison — 2026-10-12

Matched 1280×720, seed0/tick9250, review building1814, 50mm, High, paused22:00
captures confirm visible Miniature background defocus and much subtler Anime
separation. Central facade remains readable. No obvious broad halo in this still;
motion/occlusion and concept fidelity are unaccepted. No further shader tuning was
stacked on the existing cap-independent coverage candidate. All queues settled
normally; no captured browser errors. See `camera-dof-baseline.md` for exact files,
limitations and next gate. Current review tab29 is held for continuation.

## Contour-generation bottleneck and road motion review — 2026-10-12

Progress: reviewed the19.93s Miniature road recording through playback and decoded
frames1,6,19s. The join is clean in those samples, with completed pavement visible
at19s (`screenshots/road-surface-partition/motion-19s.png`). This recording predates
the ownership-cache change; it supports geometry partition behavior, not final
cache/both-style performance. No all-frame or full visual acceptance claim.

New CPU diagnostic `scripts/profile-growth-buildings.mjs` uses fresh public
seed0/dev16/tick9250 (260 buildings) and production factories/batches. Root reran
and inspected it. `BuildingChunks.set` calls synchronous `outlines(group)`:
THREE.EdgesGeometry for each material mesh, world transform, length filtering,
and array assembly. Isolated registrations (5 per contour mode/source) retain
matching geometry fingerprints; apartment1602 (32 floors,111168vertices) spends
~88ms median in outlines, versus ~.019ms total registration with contours absent.
Apartment1611 shows similar cost. This establishes contour generation as a major
synchronous cost for these sources. It does not prove causality for the observed
browser spike: initial population is profiled, not an exact growth delta.
Report: `growth-building-profile.json`, with raw repetitions and source hashes.

Next bounded performance change: incremental contour generation preserving
THREE.EdgesGeometry42-degree selection, Float32 rounding, world-space transform,
minimum3.5m edge length, deterministic output order and visibility at atomic batch
commit. Compare exact output against existing routine on real tall sources, and
measure maximum slice cost. Do not remove contours or lower architectural detail.
Mesh/source cancellation and lifetime must remain correct. Synchronous factory
costs also exceed20ms for some sources, but isolate contour work first. No production
change made in this diagnosis. Final-cache/both-style road motion and matched
night-apartment opening remain outstanding.

## Road ownership cache and growth recording — 2026-10-12

Progress: measured ownership replay before optimizing. Mature seed0/tick100000
(1730buildings,509 road claims,329 pads) spent roughly9.5–9.8ms on completed
ownership alone in warm Node CPU runs. Progress-only refresh now forks the
completed ownership index committed with the visible road batch; construction
claims never mutate the cache. Topology opening replaces it and world reset
clears it. No geometry, surface height or simulation changes from caching.

Root reviewed helper fork semantics and added independent polygon-intersection
assertions (replacing an indirect self-reclaim test). Actual renderer regression
now checks cached partition identity, zero original-cache claim calls on progress,
and replacement on opening, alongside the single surface raycast.26 focused tests
pass; TypeScript and production build pass (existing bundle warning).
Root reran scripts/profile-road-surface-partition.mjs. Current report records
15 samples per snapshot and exact output equality to full replay: tick9250 fork+
3construction-road claims median0.078917ms; tick100000 fork+empty construction
pass median0.0815ms. Mature active construction remains unmeasured. CPU only,
not complete renderer/browser/GPU performance.

Recorded normal growth from9250 in Miniature,Explore,High,50mm,DOFoff,noon preview:
`screenshots/road-surface-partition/miniature-growth.webm`. Recording is preserved
but not yet decoded/reviewed. During subsequent growth, UI measured frame p95
159.8ms / CPU p95146.2ms / longest227.1ms with neighborhood batches pending;
no causal attribution to road ownership is established. Before/after settle metrics
must not hide these spikes. HMR after cache change invalidates the live scene;
next reload fixture and verify final-cache motion/both styles, then investigate
remaining growth spikes. Night apartment matched gate remains pending.

## Ground-road surface ownership candidate — 2026-10-12

Progress: implemented RoadSurfacePartition (Luna helper, root reviewed/integrated).
A spatial index limits convex polygon subtraction candidates. CityRenderer claims
open roads by stable ID, then uncovered junction pads, then construction prefixes.
Ground pavement extrudes only remaining polygons at the same0.3m surface height;
no depth offset, blur, or simulated route changes. Bridges/elevated decks retain
their existing geometry. Existing pavement is not rebuilt for progress-only work.

Root added an actual CityRenderer raycast regression at the captured overlap:
exactly one .3m pavement hit at progress .7151829,.8582195 and1, with open-road
material retained. Road batches + partition:25 passed. Earlier road batches,
junction and markings:30 passed before adding that integration regression.
TypeScript and production build pass; existing bundle-size warning remains.

Browser: normal UI import seed0/tick9250,260buildings, Miniature WebGPU High,
50mm,DOFoff,noon preview,frame office1814; all queues settle normally. Screenshot
`screenshots/road-surface-partition/miniature-tick9250.png` shows the gray/tan
boundary clean, without the earlier striped triangle. Captured warnings/errors
empty. This is a static candidate check, not motion/all-style acceptance.
Next record growth through this join in both styles and measure partition/rebuild
cost at mature scale. Construction-only refresh currently replays completed
footprint claims to rebuild ownership; assess that cost before accepting it.
Night-apartment matched opening gate remains outstanding. Changes uncommitted.

## Quantified road edge overlap — 2026-10-12

Progress: root reran and reviewed `scripts/audit-road-surface-overlap.mjs` and
independently recomputed its triangle area. Fresh seed0, dev16, 201 snapshots
at ticks9200–9400: local1787 and arterial1807 have no common graph node and
nonintersecting centerlines, minimum distance9.200251m. Their road surfaces
(6.5m and16m wide) overlap by1.619904m² at9200,5.490180 at9250,6.163717 at9300,
and still6.163717 after arterial1807 opens at9350. No duplicate project draws.
Report `road-surface-overlap.json` contains exact polygons and source hash.

This corrects the earlier shared-node hypothesis for the captured triangle:
the implicated roads have edge-only envelope overlap. Root required actual
roadWidth import, exclusion of sloped bridges, and renderer-equivalent omission
rules before accepting the diagnostic. A surface-ownership/union fix must
handle separate nearby roads as well as junctions; clipping only shared nodes
would miss this case. No renderer correction is implemented yet.

Apartment capture now also accepts optional minimum floors (4–11). Tall witness:
seed0 apartment1350,10floors,tick4200/.868262 to4250/open,50-tick live/restore
parity. At22:00 Miniature/High/50mm/DOFoff its visible installed facade is dark
before opening (`screenshots/apartment-tall-cladding/miniature-before.png`).
After resuming, Watch changes the camera; the saved after image is not matched
and cannot support a continuity claim. Need hold Explore before replay and
capture exact after save/camera. Do not count this as a completed night gate.

## Night apartment fixture and road diagnostic — 2026-10-12

Progress: capture-midrise-cladding.mjs now accepts an optional third argument,
`apartment` (office remains default). It still uses only public seeded simulation
and compares the original live save against restored continuation every tick.
Generated `.tools/apartment-cladding-postmerge`: seed0, apartment1156, 4 floors,
progress .8692354 at2400, opens2450; all50 ticks agree. Current WASM hash matches
the post-merge office witness. Normal UI import works with zero queued geometry.

Night Miniature/High/50mm/DOFoff at22:00 preview was attempted. Neighboring tall
buildings obscure the target in the default review framing; exploratory camera
adjustments did not yield a reliable matched facade view. Therefore this is NOT
an emission/continuity pass. Evidence retained in
`screenshots/apartment-cladding-postmerge/occluded-night-review.png`; next use
an unobstructed normal-growth apartment witness rather than infer from neighbors.

Read-only road diagnosis localized a candidate to arterial1807/project1808,
node1777 (104.16684,19.800463) to1806 (123.55672,25.20845), attached to open1778.
Root confirmed city.ts emits road/construction box tops at .3 and extruded
open-road junction tops at .3, with junction arms excluding construction roads.
That allows coplanar overlap, but exact offending triangles remain unproven.
Next establish polygon overlap with the captured fixture before implementing
shared junction ownership/clipping. Do not mask with height offsets or blur.
No production renderer change made during this diagnostic.

## Post-merge Miniature opening review — 2026-10-12

Progress: regenerated the natural office fixture with current WASM. Seed 0,
office 1814, tick 9200/progress 0.8543333 to first open tick 9250; original live
and restored saves match on all 50 intervening ticks. Metadata including WASM
and save hashes: `screenshots/midrise-cladding-postmerge/metadata.json`.

Normal UI import and production scheduling, Miniature/WebGPU, High, 50 mm,
DOF off, noon preview, simulation 1× and development 16×. Recorded 19.928 seconds
at 2048×1152 in `screenshots/midrise-cladding-postmerge/miniature-opening.webm`.
Root played the clip through and inspected decoded frames at 1 and 3 seconds.
Installed lower window bands retain positions while the top floor and roof finish.
Before/after images are beside the clip. Browser warning/error logs were empty.
This supports this office's daytime facade continuity, not complete motion or
artistic acceptance. Final paused city has 282 buildings; final tick not exported.

A separate visible defect is now captured: striped/triangular road-surface overlap
at the growing road end left of the office (visible at 3 seconds and in the after
image). Cause not yet established; do not call this scene visually accepted.
Next finish the bounded night apartment emission check, then prioritize this
road/construction surface artifact. No production code changed in this review.

## Main integration — 2026-10-12

Progress: merged origin/main `2c443a9` (PRs #10/#11) into the active branch with
merge commit `76530ae`. Restored and reconciled the existing uncommitted work;
no unresolved Git conflicts remain. Safety stash
`92b096cf741da52b6111e8dae07773ebc9440777` is retained. All 562 previously untracked
files were present after restoration. Subsequent integration fixes remain
uncommitted and nothing was pushed.

Preserved movement protocol v5, presentation support gating, explicit surface
heights, current trains/doors, road curves, construction batches, camera and
material work while adopting the new vehicle fleet and left-hand traffic.
Decoder accepts all six road-vehicle classes. Actual fleet integration tests
verify each class waits for committed infrastructure and keeps its surface
height in both its instance matrix and camera tuple.

Updated existing offline fixture refresh to use the merged lane direction and
refreshed derived connector endpoints, including four endpoints in the bus
fixture. No runtime compatibility migration or relaxed save validation was added.
Native release: 454 passed, 54 ignored. Full web: 555 passed, two 5-second
timeouts (lowrise-kit continuity and WASM movement validity); both files then
passed unchanged with one worker (16 tests). That is not a green default suite.
Fresh 40,000-tick WASM movement audit: 12,197 observations, zero unresolved
supports, with current-save canonical restore and packet equality at checkpoints.

WebGPU smoke exposed an upstream vehicle buffer-layout limit (9 buffers on an
8-buffer device). Seven static attributes now share one interleaved buffer;
dynamic instance state stays separate. Root compared all attribute components
of all 11 bodies against HEAD: exact equality. Four focused vehicle/presentation
suites pass 30 tests. Build passes, existing large-bundle warning remains.
Normal UI import seed87231/tick30000/775 buildings, High, 50mm, DOF off, live
noon: Anime and Miniature prepare successfully, queues settle, captured error/
warning logs empty. Evidence: `screenshots/main-merge/miniature.png` and diagnostics.
This is a compilation/loading/style smoke check, not close vehicle motion,
device performance or artistic acceptance. Old local review saves containing
opposite-side driveway endpoints may reject; regenerate from current simulation.

Next return to the bounded midrise continuity review (Little Tokyo motion and
night apartment), using freshly generated fixtures after this engine merge.

## Midrise browser opening witness — 2026-10-12

Progress: normal UI import of seed 0/tick 9200, office 1814; WebGPU Anime,
High, 50 mm, DOF off, noon preview, 1× simulation and 16× development. Geometry
queues were zero before recording and after pausing. The 19.967-second canvas
recording is `screenshots/midrise-cladding/anime-opening.webm` (1355×1222).
Before/after screenshots, diagnostic text and a local video viewer sit beside it.
Root played the clip to its end, then inspected decoded frames at 1 and 3 seconds.
The lower window bands keep their positions; the top floor changes from exposed
structure to its authored facade/roof. No whole-facade replacement is visible in
these sampled frames. Recording ends in a 286-building city; the exact ending
tick was not exported. Captured browser warnings/errors are empty.

This is one office, one view and daytime Anime evidence. It does not establish
frame-by-frame absence of artifacts, apartment/night behavior, Little Tokyo
motion, fallback rendering, hardware performance or artistic acceptance. Next
review the same opening in Little Tokyo, then a naturally growing apartment at
night to check the corrected unlit construction panes. Keep the work bounded to
construction continuity before returning to the ranked broader visual gaps.

## Midrise cladding continuity — 2026-10-12

Progress: root inspected the delegated construction facade implementation and
corrected premature apartment window emission. Unopened midrises now substitute
unlit glass for lit panes while retaining their eventual positions. The shared
authored facade installs floor by floor; finished glazing matches opening.
Root strengthened coverage to include slab offices, narrow parcels, both styles,
all glazing materials, and occupied apartment opening. The structural slab test
now measures progress 0.69, before installed cladding replaces exposed plates;
its original footprint assertions remain. Four focused suites pass 23 tests;
TypeScript/production build pass with the existing bundle-size warning.

Root reran `scripts/capture-midrise-cladding.mjs`: seed 0, development rate 16,
natural office 1814, seven floors, progress 0.8543333 at tick 9200, opens at 9250.
Original live and restored engine saves agree on every intervening tick. Import
`.tools/midrise-cladding/before-opening.save.json` for browser review (the adjacent
snapshot JSON is not an importable save). Frame review building 1814 and observe
opening through normal scheduling. This browser/motion gate is still outstanding;
tests do not establish visible facade continuity or artistic acceptance. Keep this
as the primary workstream before any new visual intervention.

The preceding PR #11 review was progress: its latest head fixes the two earlier
findings, but conflicts with main and one default-suite timeout remain. No PR
merge or integration changed this branch during that review.

## Office cost and construction follow-through — 2026-10-12

Root reran pinned-baseline production-kit audit:126offices,+8952triangles,
+626640geometry-array bytes,unchanged1455meshes.58changed kits; no GPU/
frame-time claim.17massing/midrise tests pass including new slab construction
plates in both styles. See office-slab-variation.md and office-slab-cost.json.
Next primary bounded correction: generic late midrise facade panels still swap
to authored windows at opening. Reuse authored facade during late construction
before expanding variants; inspect normal playback. No production edit this turn.

## Office silhouette candidate — 2026-10-12

Retained bounded seeded straight-slab/top-setback variation for some midrise
offices, breaking the universal stepped profile. Root17focused tests/build pass.
Normal full1839 WebGPU import, low preset, Anime night/noon and Miniature noon
captures saved under screenshots/waterfront-growth/office-slab-*.png.
See office-slab-variation.md. No art/motion/performance completion claim.
Next: measure candidate kit cost and inspect close construction transition;
fine-edge noise in Miniature remains conspicuous and needs AA/resolution review.

## Isolated water trial rejected — 2026-10-12

Coefficient-preserving relocation of reflected radiance improves night
visibility but produces a pale mirror at noon/dusk. Reverted water.ts exactly.
See water-radiance-experiment.md and the isolated-radiance captures under
screenshots/waterfront-growth. No second shader tweak. Keep the two camera
presets; stop full-city water tuning until a distinct rough-reflection benchmark
exists. Next primary stream: mature architectural/material fidelity, grounded
in the blank facades and repetitive massing visible in these captures.
Water remains an open release gate. Candidate tsc/lifecycle test pass, no
captured browser errors; no cross-style/backend/motion acceptance claimed.

## Repeatable low waterfront fixture — 2026-10-12

Added DEV Hold low waterfront view, offset(130,55,215) from the same river
midpoint, preserving the overhead preset and collision/lens contracts.
Root48camera tests/build pass. Full1839 normal import,WebGPU Anime,22:00,
50mmDOFoff,High,1280×720: low camera screenshot is byte-identical after an
Overview detour. See waterfront-review-camera.md. No shader change yet.
Next: compare one isolated material correction against both fixed angles;
do not repeat rejected blind gain/mip trials or add lamps to hide attenuation.

## Waterfront reflection diagnosis — 2026-10-12

Full1839 save reimported normally, WebGPU Anime, paused22:00,50mmDOFoff,
High,1280×720; queues zero. High-angle raw reflection is sky/bridge dominated.
A lower orbit contains clear lit building reflections in raw mode, but normal
water heavily suppresses them. Do not add lights to compensate before isolating
surface shading. See new matched pairs and limits in waterfront-growth.md.
Next bounded step: a repeatable lower-angle review camera, retaining the
overhead gate, before any radiance candidate. Prior pale-mirror/banding trials
remain rejected. No production changes in this diagnostic pass.

## Bounded handoff correction: fallback evidence — 2026-10-12

Progress: verified normal pre-renewal71950 import and1× playback in WebGLBackend,
Anime,High,DOFoff,50mm,12:00preview,growth16×,target1243. All geometry queues
zero before recording and after pause/drain. Clip19.863s,1058×955:
screenshots/height-redevelopment/renewal-entry-webgl-anime.webm. Root played
through and inspected5s: stripped demolition structure already visible, neighbors
intact, no obvious old facade/roof contours. Saved entry-webgl-anime-5s.png;
viewer motion-review-entry-webgl.html. Captured warnings/errors empty.
Fallback backing resolution differs from WebGPU1355×1222; this is correctness
evidence, not a matched backend performance comparison. UI reported roughly
40fps ready and33fps during the run, with a209.6ms longest sampled frame;
full hardware/performance gate is not passed.

Close the primary bounded handoff correction at this checkpoint: measured entry
231→3updates and opening463→1, focused30tests/build pass, targeted WebGPU and
WebGL entry recordings plus WebGPU opening evidence. This is site-specific
verification, not all-seed/artistic acceptance. Oversized retirement still uses
normal slower rebuilding. Do not extend batching without new regressions.

Next primary workstream: re-evaluate current WebGPU waterfront/night preset
against locked04/01, then address the dominant composition/light/activity gap.
Existing ranked gaps are sparse riverfront composition/reflected city light,
repetitive street/facade character, and weak night style differentiation. Avoid
more synthetic batching optimization. Browser tab22 remains paused WebGL/Anime
at renewal site; return primary rendering to WebGPU for artistic work. Goal active;
no commit/push, no claim that full living-city requirements are complete.


## Fast entry browser comparison — 2026-10-12

Progress: normal pre-renewal71950import, DEV Frame review building1243,Miniature,
50mm,DOFoff,12:00preview,High/WebGPU,requested1×,growth16×. All geometry queues
zero before Record then Resume. New clip19.947s,1355×1222:
screenshots/height-redevelopment/renewal-entry-fast-day-1x.webm, with viewer
motion-review-entry-fast.html. Root played through, then inspected5s. Candidate
already shows stripped demolition structure at5s; baseline entry-day-5s retains
intact office. Saved entry-fast-day-5s.png. Neighbor facades remain visible and
no obvious old roof outline remains in that sampled frame. This supports the
synthetic231→3update improvement, not a frame-exact latency or hardware claim.

After normal queues drained, switched the paused current site to Anime. Saved
entry-retirement-anime.png shows cleared site without floating old facade/roof
contours; neighbors remain. Captured warnings/errors empty. This post-drain
capture does not alone test Anime during the exact index-retirement frame.
WebGL retirement path remains to verify. Original geometry fidelity remains
below concepts; do not broaden batching beyond this bounded correction without
new evidence. Next: narrow fallback check, then return to ranked city-wide art
and activity gaps. No production edit, commit or push this turn; goal active.


## Bounded committed-source retirement candidate — 2026-10-12

Progress: BuildingChunks can preflight and retire one committed building's
triangle/index and contour ranges atomically. Target triangles/lines become
degenerate; peer arrays remain unchanged and retired picking spans/membership
are removed. Existing buffers and owned resources remain until ordinary batch
retirement. No shared mesh is hidden. This consumes a scheduler slot instead
of extra build work; sources above36000indices or24000contourfloats retain the
ordinary rebuild fallback unchanged. Stale detached static commits cannot
resurrect a source already handed to the isolated pool.

Root integrated Luna's building-chunks change, independently inspected it, and
removed clearUpdateRanges calls that could lose earlier pending uploads before
a draw. Root regressions cover peer indices/contours/picking, retained upload
ranges, and over-quota no-mutation.30focused tests pass; updated retirement
suite2pass after the extra upload-range assertion. Production build passes with
existing bundle warning.

Actual seed87231 target1243 open71950→demolition72000 audit now takes3updates,
1peer regeneration before exact visibility, versus231/141before. Background
work drains229further updates with max1factory/update and zero pending. Report:
validation/renewal-entry-fast-retirement.json. CPU-only, not browser timing.
Next: repeat targeted entry-day1× recording via DEV Frame review building1243,
and inspect WebGPU/Anime contours and fallback before accepting this path.
Larger sources still use the slower fallback; broader goal remains incomplete.
No commit/push.


## Targeted entry recording confirms visible delay — 2026-10-12

Progress: new DEV camera target successfully frames1243 while still open at71950.
Normal import, pause, camera follow,50mm,Miniature,DOFoff,12:00preview,High/WebGPU;
all queues zero before Record then Resume1×. Growth remains16× from the save.
Captured19.936s,1355×1222 in screenshots/height-redevelopment/renewal-entry-day-1x.webm;
viewer motion-review-entry.html. Root played through and inspected0,5,10s.
At5s the original roof/facade remains intact; at10s a substantially stripped,
shorter structure is visible. Saved entry-day-5s.png and entry-day-10s.png.
Sparse samples establish the large change across this interval, not its exact
frame or engine tick. Earlier synthetic231update entry measurement isolates the
shared-neighborhood rebuild dependency; recording alone does not isolate all
worker/renderer latency. Broader hardware performance remains unproven.

Decision: entry latency merits a bounded correction before closing construction
batching. Investigate removing only the retiring building's index/contour ranges
from committed shared batches once the isolated replacement is ready, preserving
neighbors and picking. Avoid hiding a whole shared mesh, duplicating the city,
or weakening geometry quotas. Require one-owner tests, owned-resource lifetime,
and repeated matched entry recording; abandon if complexity exceeds measured
benefit. No implementation of that approach yet. No commit/push; goal active.


## Entry capture setup correction — 2026-10-12

Progress: normal1× recording from71950 could not establish targeted admission
latency because the renewal camera action appears only after admission, and the
close framing was reached after the20second capture. Do not treat that attempt
as visual acceptance or a timing measurement. No further batching optimization.

Added a DEV-only Review building ID / Frame review building control alongside
existing renderer diagnostics. It validates against the current world and calls
the existing follow handler, changing camera only. This enables framing1243
while still open before recording natural renewal. Production UI excludes it.
Next: reimport before-renewal.json, pause, frame1243, wait normal queues, record
at1×. This is a capture-setup correction after two inadequately targeted attempts.
The larger goal and ranked artistic gaps are unchanged. No commit/push.


## Renewal entry audit — 2026-10-12

Progress: added a before-renewal capture to the ordinary WASM growth audit.
No fixture edits: seed87231 tick71950 has1793buildings and target1243 open;
renewal is admitted at72000. Regeneration completes74450 and49live/restore
comparisons pass. Files are under .tools/height-redevelopment-entry.

Production-factory entry audit (validation/renewal-entry-batch-latency.json)
finds231updates/141regeneratedneighbors before exact demolition visibility,
with at most1factory/update. This is distinct from the corrected opening path
(1update/0neighbors). The old static batch must currently rebuild to remove its
target before the isolated replacement can appear. No missing-owner failure
was observed, but entry latency remains unresolved.

Normal UI import of71950, live2× playback, then pause and Visit renewal site
shows the active replacement structure in daylight. Screenshot:
screenshots/height-redevelopment/live-renewal-entry-day.png. Miniature,50mm,
DOFoff,12:00preview,High/WebGPU. Capture occurred after admission; it does not
prove early demolition motion or measure browser entry latency. No cinematic
acceptance claimed. The surrounding scene still has broad plain surfaces and
repetitive architecture compared with locked04.

Next: obtain a targeted entry recording at normal speed before deciding whether
the measured entry delay warrants more batching work. Do not expand batching
scope based only on synthetic timing; broader ranked art gaps remain important.
No commit/push. Goal remains active.


## Reopening style review — 2026-10-12

Progress: paused completed renewal scene survives Miniature→Anime→Miniature
through ordinary UI, with no captured warning/error logs. Saved matched initial
Miniature/Anime frames in screenshots/height-redevelopment/opening-*-style.png.
Same 1839-building paused world,22:00,50mm,DOFoff,High/WebGPU. This checks the
post-handoff scene, not a browser switch during an active migration. The return
click also changed camera mode to Explore; do not use its framing as matched
comparison evidence. The first two captures retain matching framing.

Root re-read locked concept04. This close night scene remains overly uniform
and dark, with weak style differentiation. Existing ranked city-wide gaps remain:
1. Waterfront composition and reflected city light (requires wider preset check).
2. Street/facade richness and a more connected pattern of warm activity.
3. Distinct material/atmospheric styles with DOF disabled.
Do not add more batching optimization without new evidence. Remaining bounded
construction check is live static→renewal entry; use the existing pre-renewal save.

A Luna regression originally checked initial source disposal rather than staged
migration. Root rejected that coverage; corrected test waits for the actual second
source group and checks cancellation/disposal, visible ownership and drain.
Full artistic, motion and hardware acceptance remain open. No commit/push.


## Live reopening recording — 2026-10-12

Progress: verified the fast-opening candidate through ordinary UI import and
production scheduling. Imported seed87231/tick73950 surface save, paused, visited
renewal site1243, Miniature,50mm,DOFoff,22:00 lighting preview,High/WebGPU;
requested2× playback. All geometry queues were zero before recording and after
pause. No manual queue drain. New recording is19.946s,1355×1222 (previous capture
was1355×1230, so viewport differs slightly). Live UI reached1839buildings.
Weather was not separately captured; do not claim a fully matched weather audit.

Recording: screenshots/height-redevelopment/miniature-night-renewal-fast-opening.webm.
Viewer: screenshots/height-redevelopment/motion-review-fast-opening.html.
Root played the clip through and inspected start,12s,andfinal frames: initially
partly clad,12s complete dark facade with crane,final completed facade with warm
windows and no crane. Prior repeat still showed finishing equipment at19s.
This supports improved timely reopening; sparse screenshot inspection does not
prove every frame free of artifacts. Captured console warnings/errors empty.
Night architectural/material richness remains below locked-concept acceptance.

Next: verify style changes and live entry into renewal; then reassess the ranked
visual gaps rather than continuing isolated batching optimization. Hardware
performance, broader motion and artistic acceptance remain incomplete. Goal
active; no commit/push.


## Prompt reopening candidate — 2026-10-12

The completed descriptor now first replaces the isolated construction batch.
Its identical static source is generated in a later scheduler slot, replacing
(rather than adding to) normal chunk work. The isolated batch remains visible
until the exact static descriptor commits, then retires. Superseding construction
updates cancel the pending migration through the existing immutable source path.

Production-factory audit on seed 87231, target 1243, surface tick 73950 to open
record tick 74450: first exact visibility fell from 463 updates / 141 regenerated
peers to 1 update / 0 peers (~1.41 ms for that synthetic CPU update). Background
handoff drained in 234 further updates, at most one factory per update, with
zero remaining work and continuous exact-source visibility. See
`validation/renewal-opening-fast-handoff.json`; this is not browser timing.

27 focused construction/chunk/tower tests pass, including a new regression
checking prompt opening, unchanged peers, per-update factory quota, continuous
triangle ownership, and eventual empty isolated pool. Production build passes
with the existing bundle-size warning. Browser motion/style verification remains
pending; no artistic or full performance acceptance claimed. Next: repeat normal
UI import and the Miniature night renewal recording through the live scheduler.
No commit or push. Goal remains active. Previous goal turn provided review
findings; this turn makes implementation and measured progress.


## Reopening handoff remains slow — 2026-10-12

Normal night recordings show intermediate cladding, but not timely reopening.
Actual open-descriptor synthetic handoff takes463updates/141peer rebuilds; isolated
construction phase remains1update/0peers. Next: promptly render final open state in
the isolated batch while static replacement builds, then retire it atomically;
avoid alternating slots wasted on blocked removal. See height-redevelopment.md.
26focusedtests pass. Browser tab22 is paused mature Miniature/night, tab23 shows
repeat video19s. Recordings and captures retained. No productionedit/commit/push.


## Construction batching candidate integrated — 2026-10-12

Non-open buildings now use isolated batches with atomic static handoff. Root found
and corrected shared-peer hiding and stale-removal disappearance in the initial
candidate. Twenty focused tests and production build pass (existing bundle warning).
Synthetic normal-save replacement takes one update and zero regenerated peers,
versus baseline231/141. See height-redevelopment.md for limitations. Next: matched
normal UI import and Miniature/night video; verify real live static→renewal first
handoff and style switching. Browser acceptance is pending; no commit/push.


## Renewal batch latency measured — 2026-10-12

Production-factory synthetic audit: one tower replacement rebuilds all 141 peers
in its 142-building neighborhood, taking 231 update calls. Isolated target requires
one call with unchanged geometry quotas. See `renewal-chunk-latency.json`,
`renewal-isolated-chunk-latency.json` and `height-redevelopment.md`. CPU drain is not
browser timing or motion acceptance. Next: bounded active-construction batching
with atomic static handoff, then matched normal-scheduler recording. No production
change, commit or push in this audit; goal remains active.


## Recorded night renewal exposes update jump — 2026-10-12

Progress: added DEV-only bounded canvas recorder; real Miniature night clip
miniature-night-renewal.webm is19.967s,1355×1230,~3.5MB. Root reviewed0/6/12/16/19s.
Same normal73950site,requested2×,development16,22preview,50mm,DOFoff,HighWebGPU.
Facade stays unlit through finishing, opens with samegeometry/light accents.
Large cladding jump between12and16s means progressive motion remains unaccepted.
Next: diagnose worker publication/building queue/chunkcommit latency at thissite,
not more facade redesign. No claim about causal layer yet. See height-redevelopment.md.
Buildpass,UI18pass,consoleempty. Tab22paused matureMiniature/night after run;
Tab23 evidence viewer paused16s. Closing City details cancels/releases recorder;
download is already saved in validation. No commit/push;goalactive.


## Tower finishing facade continuity — 2026-10-12

Progress: installed tower floors now use the authored open facade, replacing
underlying frame slabs; roof waits for full cladding, emission waits for opening.
Signature tracks erected/clad floor counts. Root corrected test material selection
and type issue; five focused files25pass, final tower suite4pass, buildpass.
Normal Anime74300→75789 run preserves facade/roof through reopening; queues0.
Evidence: height-redevelopment.md, finishing-authored-anime.png and
opened-authored-anime.png. Export15 confirms office1243 open12floors.
Tab22 paused75789,Anime,noon,50mm,DOFoff,HighWebGPU,847×769,building follow.
Next bounded check: partial cladding + motion recording, Miniature/night; do not
claim complete lifecycle/style acceptance from stills. No commit/push;goalactive.


## Tower construction silhouette corrected — 2026-10-12

Shared towerMassing now drives staged construction as well as open/demolition
geometry. Ordinary tick73550 UI import and matched both-style captures show the
shaft setback instead of the former full-width slab stack. Root inspected Luna's
actual-slab tests, added exact count and realistic stage/state; four focused
files17pass, updated test1pass, TypeScript/buildpass. See height-redevelopment.md.
Current tab22: paused seed87231 tick74300, Anime/noon/50mm/DOFoff/HighWebGPU,
following1243. Normal queues0; console warning/error query empty. Finishing crown
visible, temporary panels still differ from open facade.
Next: finishing facade/open transition and recorded lifecycle motion; no overall
art or skyline acceptance. No engine change, commit or push. Goal remains active.


## Renewal follow and long-run evidence — 2026-10-12

Previous goal implementation turn made progress: added the ordinary renewal-site
follow action and volume/occlusion-aware building framing; completed four fresh
30-day WASM runs. Current continuation verifies those results, captures the actual
construction baseline and strengthens the lower-worksite visibility regression.
Camera/follow/UI69pass; pending production build confirmed successful (bundle
warning only). Four seeds complete13renewals total, remain below3000buildings,
and pass final immediate restore plus200tick original/live replay. See
[height-redevelopment.md](height-redevelopment.md) for exact scope and limits.

Concrete next defect: construction is a full-width rectangular frame but its
finished tower has podium/shaft/crown setbacks. Reuse towerMassing for the staged
construction geometry; verify the same ordinary checkpoint before changing other
art systems. Demolition also remains visually bare. No completed motion recording
or artistic acceptance; do not treat this as a finished skyline workstream.
Tab22 remains paused on normal seed87231 tick73550 construction import, Miniature,
noon preview,50mm,DOFoff,HighWebGPU,847×769, following office1243. Current console
warning/error query empty. Goalactive, no commit/push. Older entries below are
historical checkpoints and may describe superseded browser state.


## Height redevelopment candidate — 2026-10-12

Engine office replacement and top-down demolition renderer are now candidates.
Root reran native7pass and web13pass; rebuilt WASM; fresh87231 ordinarily admits
6→12floor replacement at72000 and completes74450, live/restore2450tick equality.
Normal tick72400 UI import settles queues0. Close-up artistic/motion review,
later skyline distribution and multiseed behavior remain open. See
height-redevelopment.md/json and scripts/audit-height-redevelopment.mjs.
Tab22 paused day3 06:24, Anime/50mm/DOFoff, station review preset; image is only
import smoke. Engine source stable; Luna contribution inspected against exact
pre-edit lib.rs backup. No commit/push; goalactive. Next inspect actual office1243.

## Skyline allocation trial rejected — 2026-10-12

Plan audit finds ample tower sites: 19 for seed87231, all realized, but only one
18-floor tower. Other seeds have 28–39 suitable sites. Tested a first-three tower
preference through fresh WASM growth and normal browser import: mature87231 has
four towers, but seeds0/5/77 produce3–4 towers with only74–79 buildings. Rejected
and reverted the allocator candidate; original20%rule/cap8 restored. Keep the
diagnostic and evidence in skyline-allocation.md, skyline-allocation.json and
skyline-growth.json (candidate runs explicitly rejected). Native11pass/1ignored,
web51pass andbuildpass apply to the trial. Next primary workstream is causal
station-area height redevelopment, an existing spec02gap; do not solve it with
first-build probability/quota tuning. Rebuilt restored WASM and regenerated seed87231
to76000: saved state is byte-identical to pre-trial baseline, replay200passes.
Tab22 now holds baseline76000, paused10:00day3, Anime, Overview, DOFoff; temporary
viewport override reset. Goalactive, no commit/push.


## Full-density live interaction — 2026-10-12

Ran the 1,839-building city normally from day 3 09:55 to 14:02; followed the
train and switched Anime to Miniature while running. Journey and transit events
advanced, sampled queues stayed at zero, and the warning/error query was empty.
Rolling frame p95 samples were 11–14 ms; isolated longest frames reached 46 ms.
Style preparation took several seconds. See full-city-live.md for exact evidence
and limits: no motion recording, construction, audio or fallback acceptance.
Tab 22 is paused at 14:02, Miniature, 50 mm, DOF off, station follow stopped.
Next primary workstream is full-density skyline/height allocation versus camera
composition; repetitive masses and empty hills are now the clearest wide-view gap.
No source change, commit or push. Goal active.


## Shallow manual orbit enabled — 2026-10-12

Old1.32rad polar limit prevented horizon at50mmgroundtarget. Expanded to1.44rad;
height/obstacle recovery retained, Watch and heldpreset unchanged.45camera tests
+buildpass. Full1839normalimport queues0, actualmanualdrag now shows sky/elevations.
See shallow-orbit.md for limits; do not treat manualshot as automaticcomposition
acceptance. Tab22pausedAnime/noon/fullcity shallowpose. Goalactive,nocommit/push.


## Walk-up core service windows — 2026-10-12

Full-density baseline exposed blank outward core walls. Added narrow upper service
windows on outerXflank only; shared existingmaterials,36triangles/bay/floor.
Rootreviewed Luna geometrytest and corrected negative-side mirrored assertion.
Core+midrise9pass,buildpass. Full1839normalimport queues0, matchedMiniature noon
shows visiblewindowrhythm; see core-facade.md. Larger fidelitygaps remain.
No commit/push,goalactive.


## Full-density waterfront baseline corrected — 2026-10-12

Normal currentWASM growth723→1839buildings completes1839lots/564roads at75925.
West waterfront0→193buildings; prior723Maturitylabel was not plan completion.
New reproducibleaudit+JSON+normal-import fixture inwaterfront-growth.md. Full
Miniature noon/night captures fill westbank but still show darkwater/plainfacades.
Allqueues0 normally, no captured errors. No production behavior change. Use both
723development and1839full-density fixtures; don't tune growth from mislabeled
mature capture. Tab22pausedfullcity22preview. Goalactive,no commit/push.


## Continuous engineered bank retained — 2026-10-12

Removed repeating ico shoreline and persegment wall/path boxes. New riverbank.ts
sweeps shared X-offset boundary stations with outward normals and endpoint caps;
root reviewed Luna work, fixed cap scope and added manifold test.10focusedchecks
and build pass. Matched723cityAnime noon/dusk/night andMiniature noon remove
sawtooth edge; no water shader change. See riverbank.md. Motion/close bends/fallback
remain open; black nightwater/sparsefrontage still dominate. Tab22pausedMiniature
noonwaterfront. No commit/push. Goalactive.


## Rough reflection trial rejected — 2026-10-12

Tested mip-filtered reflected radiance with existing irregular normals at the
fixed723citywaterfront. Night still black; dusk flat. Reverted water.ts exactly;
TypeScript and lifecycle test passed before rejection. See water-rough-reflection.md.
Next primary pass is continuous engineered-bank geometry: current ico stones every
4.5m cause the obvious sawtooth shoreline. Stop shader gain tuning; preserve river
boundary/railings and compare fixed views. No commit/push. Goal active.


## Night water reflection isolated — 2026-10-12

Added DEV-only water normal/raw-reflection diagnostic (normal shader unchanged),
material ownership test;19checks+build pass. Exact723citywaterfront night pair
shows reflection pass contains sky/bridges/lamp points; final surface darkens it.
Raw output is a flat mirror, not a valid improvement. See water-night-reflection-probe.md.
Returned normal mode, tab22pausedAnime22previewwaterfront. No commit/push.
Next experiment needs rough-surface reflection with radiance preserved, not
more gain tuning or repeated rejected sinebands. Goalactive.


## Mature Anime lighting comparison — 2026-10-12

Previous turn progressed PR10 grid parity/browser checks. New held-camera
Anime night on/off captures show little city-wide contribution from4lights.
Restored on and captured deterministic waterfront: river nearly black, sparse
riverfront and transport decks dominate. Updated ranked fidelity gaps; do not
extend lightcount or repeat rejected water tuning without distinguishing
missing reflected content from shader failure. See nearby-street-lights.md.
Tab22pausedAnime22preview,723buildings,waterfront preset. Goalactive. No code
change/commit/push; added comparison evidence and explicit scope decision.


## Integrated PR10 larger-city check — 2026-10-12

Previous turn progressed local integration. New repeatable grid probe confirms
exact tree/trunk matrices and spheres at723→753buildings. Forced structural
rebuild CPU median25.482→8.038ms onM2Max; not browser perf. Normal723import
settled WebGPU Miniature09:55High50mmDOFoff with zeroqueues,p9516.5ms,824draws,
37textures,no captured errors. See pr10-integration.md and screenshots. Startup
delay notice resolved without retry. Tab22handed off paused. No source edits
this turn beyond diagnostic script/docs; no commit/push. Goalactive. Return to
fidelity/local-light acceptance rather than extending optimization workstream.


## PR10 useful changes integrated — 2026-10-12

User authorized integration and PR feedback. Added cache around existing skyline
search and broad-phase tree grid retaining current paved-ground/signature logic.
Corrected sign atlas was already ported. Kept per-frame shadows. Root-reviewed
Luna tree changes;61focused tests+build pass. Node cluster probe5.624→.165ms
p50cold/warm, exact values; not a browser/frame claim. See pr10-integration.md
and pr10-cache-cost.json. Feedback posted on PR10; its branch unchanged. No
commit/push/merge. Large pre-existing dirty tree preserved.


## Corrected shared shop-sign atlas port — 2026-10-12

PR10 review was progress. Ported only shared shop-sign material/UV mapping,
with derived528px atlas height fixing third-cell overflow. Kept current poster
atlas/WebGPU fix. Luna tests root-reviewed/strengthened;36focusedchecks and
build pass. Source/chunk resource ownership and actual3→1samechunk sign meshes
verified. Live10building seed87231tick570 noon Anime/Miniature captures show
anchor sign intact; no captured errors/warnings. Details:shop-sign-atlas.md.
PR10 unchanged/unmerged; no camera/tree/shadow port, commit or push. Tab22
handed off at pausedMiniature noon; exported shop-sign-city.json retained.
Goal active; broader lighting/fidelity and mature performance acceptance open.


## Lighting follow-up observations — 2026-10-12

Previous artifact reply was no progress (status restatement). Revalidated current
worktree and recovered pending test process: UI/selector27pass. Fresh startup
Miniature/Anime matched view and Anime orbit captured in nearby-street-lights.md.
Night style distinction remains weak; angular pavement lighting near the shop
needs on/off attribution before retaining/extending the prototype. Tab21 became
about:blank before export; no seed/tick recovered, so captures are observations
only. Original-camera terrain/motion acceptance still open. No production edit,
commit or push. Goal remains active.


## Matched street-light contribution — 2026-10-12

Previous turn progressed4light prototype. Added DEV-only Renderer details
on/off contribution button for exact held-camera review. Normal723building
import seed87231tick27925,clear,paused09:55,22:00preview,MiniatureHighWebGPU
50mmDOFoff1280×720. On/off captures show localized curb/lower-facade wash,
not overall night acceptance. Allqueues0; on framep9515.4–15.7ms. Off retains
shaderlights, so its15.0ms is not a true feature-cost baseline. Details in
nearby-street-lights.md. No more light features before remaining acceptance:
actor crossings, moving selection, occlusion, fallback/Anime and actualcost.
Tab20 retained at723city heldnightcamera. Goalactive,no commit/push.

## Four real street lights prototype — 2026-10-12

Previous turn progressed: rejected enlarged additive pools after playback.
This turn implements NearbyStreetLights (4fixed SpotLights,1000source bound,
0.5s selection,retention bias,fade-before-relocation). Sources exported from
actual lamp-head kit and admitted on staged road commit; reset extinguishes.
Root reviewed Luna tests and fixed duplicate physical position assignment.
32focused checks+build pass. Small21building city shows sidewalk/material
response in both styles. See nearby-street-lights.md for provisional status.
No shadows: inspect leakage, actor crossing and723building cost before retaining.
Startup notice/input timeouts occurred, eventually rendered; no causal cost claim.
Tab20 pausedMiniature22:00 preview, seed87231tick1246,50mmDOFoffHighWebGPU.
Save .tools/fidelity-review/nearby-lights-city21.json. Goal active, no commit/push.

## Ground light enlargement rejected in playback — 2026-10-12

Previous turn was progress (placement corrections and coverage candidate).
This turn verifies noon fade in both styles and normal723→766building growth
at1× simulation/16× development under22:00 lighting preview. A closer orbit
revealed enlarged pools look like repeated oval decals. Reverted only radii
enlargement; retained actual roadWidth, terrain-level local poles and removal
of duplicate legacy fixtures. Updated vertex-containment tests for original
4m longitudinal/min(1.8,width*.17) transverse footprint. See street-light-coverage.md.
No continuous-motion acceptance. HMR resets tab20 after source revert; use
growth-723.json if restoring. Next investigate bounded nearby real fixture lights,
not more additive-pool tuning or copying the rejected treatment to expressways.
Goal active, no commit/push.

## Ground lamp coverage — 2026-10-12

Previous turn progressed dusk/night evidence. Current production change fixes
local pool width (9→actual6.5), floating local bases (.475→terrain0), and removes
legacy duplicate local/arterial fixtures from roadKit (bridge path retained).
Existing four-mesh instanced lighting gets broader bounded pools, no new lights.
Root reviewed Luna test changes;24focused checks/build pass. Live723building
night preview shows warm patches on ground roads; expressways still unlit.
See street-light-coverage.md for captures, exact settings and limitations.
Tab20 retained at close night candidate. Goal active, no commit/push. Next
daylight/motion check before extending illumination onto curved elevated decks.

## Roof dusk/night review — 2026-10-12

Previous turn was progress (roof candidate plus browser evidence). This turn
held tab20 camera/seed87231/tick27925 and previewed18:00/22:00 in both styles.
Four captures in roof-surface.md show parapets/equipment remain readable;
no console warnings/errors. Keep candidate, stop adding rooftop ornament.
Original baseline pose remains unmatched; motion/fallback still open.
Night review identifies a bigger gap: roads lack readable local pools of light,
river is dark and style distinction is weak. Next bounded investigation should
address local light tied to existing lamps, not global exposure or bloom tuning.
Tab20 retained at paused22:00 Anime LIGHT PREVIEW. No source edits/tests this
turn (visual evidence and documentation only). Goal active, no commit/push.

## Roof material composition candidate — 2026-10-12

Previous screenshot response was no progress; this turn changed production
geometry/material composition and collected fresh browser evidence. Normal
Import of growth-723.json succeeded (seed87231/tick27925, paused09:55).
Flat roofs now expose their dark field instead of a full pale trim slab;
four thin coping boxes define parapets. Existing material palette, no textures.
Build and29 related tests pass. Both styles inspected; baseline framing differs,
so no matched whole-scene improvement claim. See roof-surface.md for limits.
Tab20 retained, candidate Anime, same held camera as candidate Miniature capture.
All production geometry queues settled. Night/fallback/motion checks still open.
Next: exact camera and dusk/night check before adding further architecture work.
Do not return to isolated road or shrub refinement. Goal remains active;
no commit/push.

## Intersection paint clearance — 2026-10-12

Previous turn progress: sidewalk corner fix with live77building evidence.
Added road-markings.ts finite neighbor-rectangle/paint-band clipping. Open locals
and arterials stop paint before intersecting carriageways; collinear, elevated
and unopened neighbors excluded. Root inspected Luna tests,62focused checks+
buildpass. Bothstyles WebGPU/WebGL oblique diagnostic clears paint insidejunction.
See ground-surface-artifacts.md; nofullcity/motion claim forlastpaintedit.
This bounded road-surface pass should now stop pending broader scene review.
Do not endlessly add road details: compare valid current mature/developing views
against concepts04/01 and choose the largestremaining experiencegap (plain
repeatedfacades/materials and growthhitches evidenced). Motion/streets realism
stillopen. Tab19 diagnosticretained. SourceHMR mayresetcitytab18; restorevalid
.tools/fidelity-review/junction-city77.json forfuturecityreview. Goalactive;
no commit/push.

## Sidewalk bend connection retained — 2026-10-12

Previous turn progress: normal24→77building growth exposed actual corner gaps.
Added outer corner paving at >=2open arterial nodes; clip all ground corridors,
shared asphalt pad and previously emitted strips. LowestID owns each patch.
Root reviewed Luna tests and ran52focused checks+build. City77 audit:49→104pieces,
+56.06m²; no new full-building-envelope overlaps (shop84 pre-existing.2698m²).
Live WebGPU bothstyles, sameheldcamera/time, allqueues0, gap visiblyclosed.
See ground-surface-artifacts.md and sidewalk-corner-city77.json. Script retained
scripts/audit-sidewalk-corners.mjs (requireslocalfixture). No broader perfclaim;
maturecost and motion remain open. Next boundedroadissue painttermination;
then reassess broader art rather than extending isolatedgeometry endlessly.
Tab18 pausedAnime77 retained. No commit/push;goalactive.

## Production growth witness for junctions — 2026-10-12

Previous turn was progress (shared footprint implementation/tests/captures).
This turn supplies ordinary startup/growth evidence. Obsolete developing save
rejected connector validation; no bypass. Fresh seed87231 grew24→77buildings
at4×/4× and settled all geometry queues. Retained valid local saves:
.tools/fidelity-review/junction-city24.json (tick1671), junction-city77.json
(tick7072). Both clear/intensity0. WebGPU1280×720, High,50mm,DOFoff. Same24building
camera/time compared Anime/Miniature. Growth had147.1ms sampled frame; no perf
acceptance. Latest ground-surface-artifacts.md records gestures and limits.
77building view exposes sidewalk endpoint gaps at bend; inspect actual node
and shared corner geometry next, without enlarging asphalt fill. No continuous
video, mature/fallback growth or full art acceptance. Tab18 paused Miniature77
retained. No production edits/tests needed this turn; docs/evidence updated.

## Rounded junction footprint candidate — 2026-10-12

Previous turn was progress (production surface/cone fixes and verified mesh
regressions). This turn replaces widest-road disks with width-aware rounded
convex footprint shared by road fill and sidewalk clipping. New helper
road-junction.ts; root reviewed Luna's five tests; 54 focused tests + build pass.
Matched WebGPU and inspected WebGL specimens remove the mixed-width bulb;
oblique T with sidewalks checked in both styles. Details/limits in latest
section of ground-surface-artifacts.md. Index-mode integration failure fixed.
Next use a saved developing city through normal scheduler and camera movement;
avoid endlessly polishing isolated specimens. Marking termination/inner-corner
rounding, growth stalls, broader art fidelity and audio acceptance remain open.
Goal active, no commit/push. Tab17 holds oblique diagnostic, marked handoff.

## Ground junction and open-local correction — 2026-10-12

Prior turn classified no progress (status restatement); this turn changes
production geometry and provides failing-before/passing-after evidence.
Ground junction/lane asphalt now meets the .3 m road top. Fixed open locals
incorrectly entering the construction-cone branch, restoring their edge lines.
Root reviewed Luna raycast regressions, reproduced all three failures with old
expressions and verified 34 focused tests plus build. Browser production-builder
specimen reviewed both styles in WebGPU and WebGL: continuous asphalt and
visible paint, no cones. See ground-surface-artifacts.md latest section.
This is isolated geometry evidence, not ordinary scheduler/motion acceptance.
Bulb-shaped mixed-width junctions remain conspicuous; next road intervention
should address footprint and marking termination together, with sidewalk/lot
clearances, rather than expanding or disguising the fill disk. Goal remains
active. No commit/push. Diagnostic tab17 retained; production city may have
reloaded during source edits, so restore a saved review world before comparing.

## Planting workstream stopped after composition check — 2026-10-12

Verge-mode specimen now compares connected planting with bare bank at identical
camera/material/light settings. Captured both100m states; contribution is small,
and near leaf speckling remains. No production integration warranted. Stop
individual-shrub and scatter iterations. Bank profile/material art review is
needed before reopening planting; river/road/building clearances must remain
authoritative. See shrub-specimen.md. Return to another evidenced, high-impact
defect rather than spending more turns on this marginal experiment. Known raised
road-junction disks and remaining growth stalls remain open. Goal active; no
production change/commit/push.

## Shrub approach reassessed after second failure — 2026-10-12

Second isolated hybrid specimen (196triangles, opaque interior + existing masked
leaf cards) reviewed25/250m in both styles. Pale speckling near and isolated dots
far still miss the concept; rejected for production. See shrub-specimen.md and
hybrid screenshots. Do not continue tuning individual procedural bushes. Next
reassess at bank-patch composition scale: connected irregular low cover with
intentional breaks and exposed stone/soil, inside actual bank cross-section.
Experimental validation sources only, no production changes. Tab16 retains
hybrid specimen, tab15 city waterfront. No goal status change.

## Isolated shrub specimen — 2026-10-12

Created validation/shrub-specimen.html/.ts, not production-imported. Current
materials, both styles, same opaque leaf-cluster geometry. Tested25/100/250m
in WebGPU. Mass survives minification but near view looks like rocks: rejected
for integration. See shrub-specimen.md and screenshots. Next one bounded hybrid
variant with opaque interior and finer leaf edges, fitted to the actual1.9m
bank strip; reassess after a second failure. Keep quality bar and no city-wide
scatter before specimen acceptance. Tab16 specimen retained, tab15 waterfront.
No production code change, commit/push, or goal status change.

## Camera browser follow-up and next visual work — 2026-10-12

`camera-anchor-browser.md` records ordinary575→723building growth. Stalls
persist (sampled242.5ms maximum); isolated CPU gain is retained without browser
performance acceptance. Saved tick27925,clear,day1 09:55. Settled Overview pair
has zero queues in both styles and aligned layout; prior riverbank mismatch is
not a demonstrated geometry defect. Tab15 retained paused Anime, Hold waterfront
view. Waterfront screenshot confirms flat-looking water, bare banks and repeated
stones. Next bounded visual work: dedicated low-shrub specimen per the rejected
bank-planting report, evaluated at real scale before integration. Do not repeat
scaled tree-card experiments. Growth timing, style latency and fidelity remain
open. No production code change or goal status change in this browser review.

## Camera anchor CPU improvement — 2026-10-12

Skyline neighborhoods now use temporary spatial bins with exact distance tests
and original ordering; losing candidates skip detailed framing. Root reviewed
Luna's A/B harness and reran it: five real growth worlds (575→775 buildings),
exact anchor parity, median buildAnchors 11.151→5.840 ms, p95 14.539→7.250 ms.
52 focused camera/follow/neighborhood tests and TypeScript/build pass. See
`camera-anchor-optimization.md` for scope, benchmark limitations and evidence.
No visual quality reduction or collision/selection semantics change intended.
Browser growth/motion verification remains next; tree cost and frame stalls
remain open. No commit/push or goal status change.

## Current growth bottleneck localized — 2026-10-12

See `growth-stage-v2.md` and two raw console traces. Ordinary growth world
updates: 79 samples, median 46.1 ms. Trees median 21.8 ms; the broad
transit/access bucket was narrowed in a second 40-sample trace to camera
director.world median 20.5 ms (max 22.5). Bus/train negligible at this scope.
Next profile director anchor/obstacle work and reduce it with behavior parity.
Frame trace quota was consumed by startup: no growth-frame attribution from
that trace. All temporary instrumentation removed; patches archived. Source
review finds no style-triggered riverbank geometry rebuild; pending road
railings/material contrast still need a settled capture comparison. No commit,
push, or goal status change. Vite reload reset the browser after trace removal.

## Browser growth recheck — 2026-10-12

See `movement-growth-browser.md`. Current save imported normally, then 8×
simulation / 16× growth advanced 385→575 buildings. At 500 buildings a sampled
loading window reached 259.1 ms longest frame; tree caching does not close the
growth-stall gate. Paused final Miniature has all queues zero, p95 14.8 ms,
longest 19.9 ms. Saved checkpoint tick 22026 and dawn captures in both styles.
Riverbank appearance differs across the captures; preparation/geometry parity
needs inspection before treating these as a fully matched comparison. Next
trace current growth stages, including work outside the render callback.
No code change, commit/push, or goal status change in this browser review.

## Movement cost and tree clearance cache — 2026-10-12

CPU diagnostic `movement-presentation-cost.json` measures the production packet,
decoder and presenter with immediate infrastructure commits, not browser/GPU
performance. On M2 Max / Node 26.7, seed 87231, ticks 12000–14400,
300 publications produced 93 structural snapshots. Presenter sampling p95 was
0.384 ms, snapshot/parse p95 4.880 ms. This does not explain or close the
previous browser growth frame maximum of 230.9 ms.

Tree clearance now caches the exact geometry inputs used by its predicate;
regenerating tree points invalidates the key. The A/B diagnostic
`tree-clearance-cache-cost.json` uses actual renderer methods and 1024 synthetic
tree sites over 46 fresh-world snapshots (ticks 14400–15600). It skips 15 passes,
preserves exact matrices and bounds, and reduces total measured CPU time from
717.43 to 501.62 ms. The slowest pass remains about 17 ms: aggregate improvement,
not a tail-latency fix or browser acceptance. Root reviewed the delegated cache
tests; four focused ground/tree suites pass 19 tests. Original-camera striping
and continuous-motion checks remain open in ground-surface-artifacts.md.
No commit/push or goal status change.

## Production movement presentation — 2026-10-12

Support packet is now wired through protocol v5, token/epoch validation and
bounded recycling. Renderer retains last valid displayed poses, gates new actors,
retries paused targets after commits, and feeds presented positions to camera
and audio. Withheld IDs suppress camera structural fallbacks. Bridge/train
platform geometry metadata now follows committed batches. See movement-support.md
production integration section for behavior and browser limitations.
57 focused camera/presentation tests and build/TypeScript pass. Full web run:
442pass/3timeouts; affected three suites pass22 on diagnostic rerun. Do not report
the default suite as green. No native changes this turn.
Browser ordinary startup grew323buildings; all geometry queues drained normally
after pause, no captured console errors. Loading longest230.9ms remains a failure.
Train follow sampled Miniature19:22; screenshot retained. Anime comparison was
interrupted by Vite reload after a comment edit. Tab15 now paused fresh Anime
settlement07:33. Next measure packet/publication and per-frame support cost,
verify matched saved motion/style scenes, and address measured growth stalls.
No commit/push; full goal remains active.

## Structural publication ordering — 2026-10-12

Corrected the previous caller-audit finding: expressway completion **does** bump
access generation in `complete_project`. Station construction threshold and
readiness are the independent missing phases. Engine snapshot/getter now share
an exact string presentation token including those phases; worker publishes a
matching world before movement when the token changes. Epoch reset clears the
remembered token. Platform commit identities also include the visual threshold.
Six native tests pass, including ordinary 12k-tick growth and restore coverage;
65 focused frontend tests and TypeScript pass. Initial native run had one faulty
test assumption about development-paused reservations, corrected without dropping
the real growth audit. Support packets and actor/camera filtering remain unwired.
WASM and web production builds pass (existing warnings). Fresh WASM audit:
50 checkpoints through 40k ticks, 7,283 actors, zero unresolved support; token
snapshot/getter and restore equivalence now asserted at every checkpoint.
Next complete that integration and measure structural publication frequency and
browser continuity. No claim of end-to-end visual or performance acceptance.

## Movement visibility dependencies — 2026-10-12

Gate now compares roads and rail-platform layouts against committed batches,
handles open expressways outside ground connectivity, and rejects unresolved or
empty support. Lane identity changes also invalidate the completed-road batch.
Root reviewed Luna changes and added actual staged-renderer commit checks;
44 focused gate/batch/decoder tests and TypeScript pass. Packet remains unwired.
The initial expressway ordering finding was corrected by the caller audit above;
station visual/readiness phases were the actual additional dependencies.
Next establish worker structural ordering, integrate retained actor poses and
camera presentation, then review ordinary browser growth. No visual acceptance,
commit, push, or goal status change.

## Historical regression inputs refreshed — 2026-10-12

17 static derived inputs under `tests/fixtures/access-joins/` retain historical
source files byte-for-byte. Manifest records6,648 route-join corrections and two
proportional active-progress adjustments; generator verifies no other state
changes. Runtime restore remains strict. No tests/assertions removed.
Native release: 437 passed, 0 failed, 53 ignored. First frontend run: 415 passed,
2 failed from remaining old dynamic fixture references; references corrected.
Final frontend: 417 passed across 77 files. Production build passes with the
existing large-chunk warning; diff check passes. Sessions33037/41845 finished.
Raw evidence: `validation/access-route-{native-release,web,build}-final.txt`.
This closes the native fixture regression only; movement packet remains unwired,
and browser motion/fidelity acceptance remains open. Next integrate coherent
revision/platform-aware presentation after measuring/removing row overhead.
Goal active; no commit/push.

## Saved movement joins — 2026-10-12

[Evidence](movement-support.md#saved-route-joins-and-delivery-returns). New
validator rejects origin/destination connectors that miss their route endpoint.
This exposed and fixed delivery returns reversing into the wrong traffic lane;
returns now regenerate the outgoing building connector. Two ordinary-growth
native validation/replay tests pass. Fresh WASM corpus restores50checkpoints,
7,283actor observations, zero unresolved support. All19bus-filter tests pass
after correcting malformed test geometry; no regression assertions removed.
Full native release run2993 finished:411passed,26failed,53ignored. All reported
failures hit historical fixture connector joins during restore. Raw output:
`validation/access-route-native-release.txt`. No process remains to poll.
Next refresh affected regression inputs preserving original scenario assertions
and historical evidence; do not bypass the validator or discard coverage.
Old783-pose fixtures can now reject and are historical evidence only.
Packet unwired; browser motion/art gates remain open. No commit/push.

## Transit connectors corrected — 2026-10-12

[Latest support evidence](movement-support.md#transit-support-and-endpoint-correction).
Fixed three reversed route-end arguments affecting bus boarding, bus alighting
and deferred alight recovery. Bus queue support now shares placement's edge;
rail support uses the platform's actual service axis despite extra branches.
Fresh seed87231: 40kticks, 7,283 sampled actors, zero unresolved support.
Old corpus: 783 poses unchanged, 27 unresolved samples including already-saved
incorrect connectors. Seven support + two endpoint native regressions pass;
21 focused frontend tests pass; WASM builds. Packet still unwired.
Next: validate/reject malformed current paths, measure allocation overhead,
then revision/platform-aware movement gating and real browser review.
Goal active; no commit/push or visual closure.

## Movement support audit — 2026-10-12

[Candidate and evidence](movement-support.md). Integer movement/support packet
remains unwired. Pre-change WASM oracle: 15 samples, 783 actor observations,
pose parity preserved; 55 unresolved pedestrian observations. Fixed bus support
edge selection by returning edge identity from the actual pose calculation.
Four focused native tests and 12 decoder tests pass; WASM/TypeScript pass.
Three duplicate full native runs were terminated, not passed. Next resolve
transit access/queue provenance, measure legacy row allocation, and define
world-revision/platform commit ordering before renderer integration.
No new browser/artistic acceptance. Goal active; no commit or push.

## Exact box geometry optimization — 2026-10-12

[Evidence and next action](box-geometry-cost.md). Full-road section measurement
located generation as the largest cost. First direct-transform prototype failed
normal parity and was discarded. Accepted helper preserves tested arrays exactly
and keeps standard transforms; Kit.box uses it. Root's alternating-order CPU
full-rebuild pairs:39.53→34.45ms median, faster15/16. Tail still~43ms; not smooth
growth acceptance.71tests/15suites, TypeScript/build/diffcheck pass.
Browser hot reload timed out; fresh reload recovered, normal680-building import
settled queues. Tab15 paused Anime,Overview,50mm,DOFoff,10:26,tick28434,clear0.
Next stage full road generation/merge, with bounded coalescing and coherent
construction/completed geometry; no further primitive micro-optimization detour.
Full goal active; no commit/push. Concept and original artifact acceptance open.


## Completed-road batch retention — 2026-10-12

[Implementation and evidence](road-batch-retention.md). Construction progress now
retains finished infrastructure, with topology/deck/platform/river dependencies.
Two batches preserve sampled triangle/attribute hashes; mesh count rises2–4 in
sampled scenes. CPU aggregate work fell42–44%, but topology-change tails remain
slow and a rerun p95 worsened: no tail-latency acceptance claim. Browser grew
441→680buildings and switched styles with no captured errors. Growth framep95
sample126.6ms remains a failure. Tab15 paused Anime,10:26,Overview,50mm,DOFoff.
Final regression:32 tests across6 suites pass; TypeScript, production build
(existing large-bundle warning) and diff check pass.
No commit/push. Full goal remains active; original scene/art acceptance open.


## Growth-stage measurement — 2026-10-12

[Profile](growth-stage-profile.md): 103 resumed-growth world updates, median
103.9 ms; road stage 61.9 ms, trees 22.7 ms, transit/access 18.3 ms.
Frame log cap was reached before growth, so render submission remains unranked.
Temporary profiling removed and archived. This is diagnosis, not a performance
fix or full closure of the user's ground artifact screenshot. Next evaluate
unchanged-road retention with complete dependency invalidation and batching.


## Live growth completes but hitches badly — 2026-10-12

Real WebGPU UI test: imported tick3066/38buildings, speed8/growth16, ordinary
loops advanced to tick18918/441buildings. Pausing let building/chunk/sidewalk
queues all reach zero. Export retained in .tools/fidelity-review/sidewalk-live-growth.json.
Growth sampled frame p95197–242ms vs settled20.6ms: **not performance accepted**.
[Evidence and exact settings](sidewalk-junctions.md#real-browser-growth-and-remaining-road-seams--2026-10-12).
Next profile named structural rebuild stages during growth before choosing a
performance fix. Road-junction disks separately explain visible raised arcs:
top.365 above asphalt.30 and marks~.3575; shared sidewalk trim must change with
road surface geometry. Don't blindly lower disks or remove coverage at bends.
No source geometry changed. Browser tab9 retained paused Miniature,09:00 preview,
50mm,DOFoff,High. Full goal remains active; no commit/push.


## Sidewalk coalescing implemented — 2026-10-12

The preceding turn supplied the fast-growth starvation witness. Same-world
updates now preserve one active job plus one latest queued snapshot; world reset
cancels both. Root reviewed Luna changes and checked committed geometry bounds.
34 focused tests pass after correcting the ground-test's incomplete renderer
stub (4 initial failures, no production guard added). Offline samefixture120s:
76 commits(vs13),0 cancellations(vs139),max committed-input age3.067s.
Final update remains pending; this is reduced lag, not instant synchronization.
[Evidence and tradeoffs](sidewalk-junctions.md#bounded-same-world-coalescing--2026-10-12).
Next verify in-browser fast growth and road transition geometry, then address
visible raised junction-disk seams rather than continuing scheduler work without
new evidence. No new visual captures or commit/push this turn.


## Continuous-growth scheduling defect found — 2026-10-12

WebGL developing-city captures show joined sidewalks in both styles and after
an orbit, all queues zero. High-detail fallback remains slow (sample p9543.5ms)
and Miniature shader preparation took tens of seconds. These are appearance
checks, not performance acceptance. More importantly, a production-owner/real
WASM offline fast-growth probe reveals repeated restart postponing sidewalk
commits for about22 simulated seconds. The paused-scene checks missed this.
[Evidence and next intervention](sidewalk-junctions.md#fallback-and-continuous-growth-review--2026-10-12).
Next bound topology-update coalescing while preserving world-reset cancellation,
current style at commit, resource safety and per-frame work. Compare commit gaps
on the same120s growth fixture; do not merely increase per-frame quotas.
Existing raised road-junction disk seams remain a separate visible defect.
No production code changed this review; goal remains active.


## Sidewalk repair integrated, browser checks begun — 2026-10-12

The prior turn made implementation/parity progress. CitySidewalks now drives the
staged builder once per frame, with copied topology invalidation, atomic mesh
replacement, style-at-commit and compile-aware retirement. Pending work appears
in diagnostics/loading gates. Root reviewed delegation and added failure
coverage;44 focused tests/build/diff-check pass. Developing WebGPU scene shows
joined corners in matched Anime/Miniature views. Mature850-building import
settles all queues through normal scheduling. Initial import CPU410ms remains
unresolved; settled-window timings do not cover that hitch.
[Implementation, captures and limits](sidewalk-junctions.md#live-staged-integration--2026-10-12).
Next check fast-growth updates and camera motion/fallback, including remaining
junction seams and doorway/bridge transitions. Keep the workstream on ground
continuity; do not claim the full original artifact or concept fidelity accepted.
Tab9 remains on mature paused Anime Overview; no commit/push.


## Detached sidewalk mesh job verified — 2026-10-12

Previous turn ended with a verified production-build wait; this turn adds the
staged clipping/extrusion/merge job and independent mature geometry comparison.
Root strengthened cancellation/ownership tests; 32 focused checks pass.
182 updates preserve exact merged vertex/index data. Node after-warmup maxima
.80–1.31 ms, cold maximum3.77 ms; not browser performance acceptance.
[Full evidence and remaining synchronous operations](sidewalk-junctions.md#detached-mesh-construction--2026-10-12).
Live CityRenderer integration remains pending. Next wire independent topology
invalidation, atomic replacement, stale-job cancellation, style-at-commit,
compile-aware retirement and pending-work diagnostics, then review in browser.
Do not restart on every construction-progress bucket; do not manually drain
this job during loading. No commit/push or claim that live sidewalks are fixed.


## Sidewalk clipping is resumable; runtime integration remains pending — 2026-10-11

Root stage profiling split clipping/extrusion/merge cost, then verified Luna's
resumable clipping against a pre-change frozen mature polygon baseline: exact
ordered equality for all1192 polygons. Nine geometry tests and related suites
pass22/22. At quota2048 the Node diagnostic took103 updates, max .606ms; this is
not browser acceptance and total work increased from generator overhead.
[Details and limits](sidewalk-junctions.md#resumable-clipping-checkpoint--2026-10-11).
Next implement detached staged extrusion and cancellation/atomic swap lifecycle,
then integrate and verify normally scheduled browser loading. Initial map/sort
and final collection remain synchronous. Candidate is still absent from live
CityRenderer; no sidewalk fix is claimed in the running city.

## Sidewalk candidate: geometry improves, synchronous integration rejected — 2026-10-11

Implemented pure clipped sidewalk polygons and a one-batch kit. Temporary live
integration visibly joins the developing-city corner in both styles; root ran
20 focused tests. Mature CPU construction measured ~96–110 ms; convex rejection
reduced it to ~22–31 ms but still runs in one synchronous rebuild. Live integration
was removed; `arterial-sidewalks.ts` and `sidewalk-kit.ts` remain unused candidates.
[Evidence, screenshots and next step](sidewalk-junctions.md). No claim the running
city has fixed sidewalks yet. Next investigate bounded incremental construction
or caching, retaining the complete geometry and lifecycle safety; don't stack
another synchronous geometry tweak without reassessing. No commit/push.

## Ground review moves to sidewalk continuity — 2026-10-11

The prior turn made progress with the matched GPU depth witness. Full-city
WebGPU samples now cover a restored 38-building developing checkpoint after
zoom/orbit/pan, with matched lower views in both styles, DOF off, zero queues,
and no visible ground striping. These are sampled poses, not continuous video
acceptance. [Settings and captures](ground-surface-artifacts.md#developing-city-camera-samples-and-next-continuity-defect--2026-10-11).

Next bounded intervention is arterial sidewalk corner continuity: existing
slabs stop 8 m short of nodes and the lower carriageway disk cannot join them.
Root checked the Luna audit in roadKit/rebuildRoads/building-access. Build and
validate corners against all incident carriageway corridors; do not fill road
space or adjacent lots to disguise the discontinuity. No source changed this
turn. Original screenshot camera and broader motion/style/fallback gates remain
open; avoid further depth-bias tuning without a new failing witness.

## Ground depth witness — 2026-10-11

The previous shadow-fix turn was verified progress. This turn adds a controlled
GPU reproduction of the striped/missing paving: actual ground builder/materials,
y=.035 over terrain y=0, city clipping range, 1000 m camera distance. Matched
panels with/without the current terrain-only polygon offset show that the offset
removes the artifact in WebGPU and WebGL, Anime and Miniature. No production
change or extra bias was introduced. [Probe and captures](ground-surface-artifacts.md#controlled-depth-witness--2026-10-11).

Next: verify the same policy in full-city camera motion and grazing views with
road/access detail retained. The original screenshot's exact camera is still
unknown; this synthetic witness is causal evidence for one defect class, not
full visual acceptance. Do not keep tuning offsets without a new failing witness.

## Shadow binding fix verified in WebGPU — 2026-10-11

Supersedes the next-step recommendations in the two historical shadow sections
below. Tracing found stale sampled GPU shadow bindings on unchanged shared
storefront atlas materials. Both atlas variants now explicitly use TSL
`materialColor`, preserving their default diffuse expression while refreshing
bindings. No global disposal, renderer monkeypatch or quality reduction remains.
The observer regression fails with the old material and passes with the fix;
25 focused tests and production build pass. Final uninstrumented mature-city
checks survive both style directions and High/Balanced resizing, with no new
console warnings/errors. [Evidence and limitations](ground-surface-artifacts.md#shadow-binding-diagnosis-and-narrow-material-fix--2026-10-11).

Original ground striping remains unreproduced at its exact camera and is still
open. WebGL verification of this narrow fix and long-session stability remain
open. Resume ground seam/access visual review; do not count this stability fix
as art-fidelity acceptance. No commit or push made.

## Shadow resize reproduction; experiments reverted — 2026-10-11

Reload recovers WebGPU, but High→Balanced independently reproduces the black
scene. Moving `sun.shadow.map` resize between frames and then adding explicit
depth-binding disposal each failed after subsequent style/quality transitions;
both experiments and their implementation-specific tests were reverted.
The second failure is recorded on the 850-building fixture after normal queues
drained. See [reproduction and rejected experiments](ground-surface-artifacts.md#shadow-failure-isolation-two-rejected-experiments--2026-10-11).
Next work must trace all shadow-target identities across main/reflection/warmup
passes before another fix. Do not repeat the single-target resize workaround or
accept passing lifecycle tests as GPU proof. Original ground striping stays open.

## WebGPU stability finding during ground review — 2026-10-11

Close-up ground checks cover Anime on WebGPU and both styles on WebGL, with no
striping visible in those sampled views; the original failing camera remains
unreproduced. A separate WebGPU black scene produced repeated destroyed
`ShadowDepthTexture` submission errors and persisted through camera/style changes.
Evidence and limitations are in [the ground report](ground-surface-artifacts.md#close-up-fallback-checks-and-separate-webgpu-failure--2026-10-11).
Next: verify reload recovery and isolate shadow resizing/style warmup lifetime
before further visual tuning. No causal fix or stability acceptance yet.

## Ground update cost isolated — 2026-10-11

Ground-only changes no longer rebuild roads, railings or street lighting. Ground
keeps a separate signature/group, four merged material batches and existing
resource retirement. Eleven focused tests and web build pass. CPU-only mature
ground diagnostic and ordinary growth from 26 to 38 buildings are recorded in
[the ground report](ground-surface-artifacts.md#ground-updates-isolated-from-roads--2026-10-11).
Zero rendering queues after growth; original striping, close-up seams/access,
fallback, motion and end-to-end performance acceptance remain open. Startup
briefly showed its graphics-delay warning before recovering without intervention.

## Progressive ground candidate implemented — 2026-10-11

Ground now follows retained lot partition polygons once a building site exists;
parcel release no longer paves entire blocks. Early-growth browser views in both
styles retain natural gaps; mature overview retains dense streets. Growth tests
7 passed/1 ignored, ground checks and six frontend clearance checks pass, WASM
and web builds pass; both checkpoints replay 200 WASM ticks identically. Mature
ground JSON is about 210 KB: measure snapshot/rebuild costs before accepting.
Close-up joints/access, motion, fallback and original depth-stripe reproduction
remain open. [Evidence](ground-surface-artifacts.md#progressive-lot-surfacing--2026-10-11).

## Mature comparison refreshed — 2026-10-11

Imported seed 87231/tick 168000 (1,839 buildings) through the normal UI and
captured both styles at dawn, noon and night with DOF off. Both geometry queues
reached zero normally. PR #3 tip remains an ancestor of this checkout. Mature
density is present; whole-block paving at the developing edge, dark night water,
repeated facades and weak night style distinction remain visible. Updated the
[current ranking and evidence](living-city-fidelity.md#current-mature-review-after-ground-clearance--2026-10-11).
Next primary work stays on causal ground surfacing; depth-striping acceptance
remains open. No runtime change or new performance acceptance in this review.

## Trees through block paving corrected — 2026-10-11

Vegetation now respects filled block polygons while retaining roots in gaps and
undeveloped land. Six focused checks and build pass. Restored seed 87231/tick
1804 shows the correction in both styles through normal rendering, with zero
queued geometry. Broad early paving and depth-striping acceptance remain open.
See [evidence](ground-surface-artifacts.md#paved-block-vegetation-clearance).


## User-observed ground artifacts — 2026-10-11

Recorded green striping/stepped patches over paving and trees through paved
areas. Depth precision is a hypothesis, not a confirmed diagnosis; the fresh
startup overview did not reproduce the user's exact view. A terrain-only depth-bias candidate now builds and renders in both styles;
matched original-view, motion and fallback verification remain open. See [artifact evidence and repair criteria](ground-surface-artifacts.md).


## Shared storefront notices — 2026-10-11

Added a bounded authored 512² paper-notice atlas to existing shop glazing, with
shared style materials and no emission or generation spend. Both-style specimen
views show a modest entrance detail; night paper stays dark. Twelve focused
checks and build pass. Live-street distance/motion and batch-cost acceptance
remain open; do not expand the library yet. See [evidence](storefront-notices.md).


## Texture pilot decision revalidated — 2026-10-11

Completed the previously pending concrete specimen comparison after the Anime
pigment fix. Matched on/off captures in both styles and Miniature night show
subtle grain, insufficient broader facade benefit. No live-city promotion or
new generation spend. Refreshed texture inventory includes both development
candidates; plaster remains over the proposed side limit. See
[review and next bounded atlas hypothesis](../art-workshop/tencent-pilot/REVIEW.md#corrected-shader-recheck--2026-10-11).


## Train follow subject visibility — 2026-10-11

The recorded foreground-tower obstruction is corrected with bounded sightline
selection and cached camera offsets. The matched mature station view shows the
full train in both styles, with additional clear in-transit observations at 1×.
These are sampled captures, not continuous motion acceptance. 49 camera tests and production build pass. This is one-view visual
progress, not full route/style/lighting acceptance. See
[comparison and limits](train-follow-visibility.md).


## Door and movement synchronization — 2026-10-11

Removed the separate coarse world-snapshot door clock. Protocol v4 publishes
compact train door tuples with each movement packet, and renderer interpolates
them on the position timeline. First packets preserve restored phases; later world
snapshots cannot reopen closing doors. Two native and 31 frontend focused checks
pass; build and rebuilt WASM replay pass. See [door evidence](train-doors.md).
Visual aperture/motion acceptance remains open. The public Follow train action
also frames a tall occluding building at the mature checkpoint; see the linked
report capture. Next primary intervention: follow-camera subject visibility,
not further rail ornament.


## Authoritative train doors — 2026-10-11

Added engine opening/closing phases, boarding/alighting gating and actual sliding
door geometry on the platform side. Build and 20 focused frontend tests pass.
Native suite passed 423 tests (53 ignored), followed by a focused regression after
final refinements. Rebuilt WASM train views match native; 200-tick save/movement
replay is identical. Browser loaded the ordinary mature open-door checkpoint,
but canopy/building occlusion prevented clear visual acceptance of the apertures.
See [evidence and presentation limits](train-doors.md). Do not mark rail fidelity
accepted or expand rail families before resolving the remaining visual evidence.


## Train reversal correction — 2026-10-11

Train bodies now retain their straight-route orientation at terminal reversal;
separate head/tail lamps switch driving ends. Build and 18 focused tests pass,
including unchanged body transforms through a 180-degree travel-heading change.
See [evidence and door-contract requirements](train-reversal.md). This corrects
physical reversal before adding dwell-linked doors; articulation remains open.


## Commuter corridor wiring — 2026-10-11

Added open-rail contact/messenger wires, cantilever supports, track bed and service
edge details. Platform reservations switch poles away from the reversed far
terminal and trim its curb. Build and 16 focused tests pass; matched-style mature
WebGPU captures show the candidate. 800 m helper cost is 3,648 triangles/five
material batches. See [evidence and remaining limits](rail-corridor.md).


## First commuter rail kit — 2026-10-11

Replaced box coaches with a bounded instanced stainless commuter kit, aligned
wheel gauge and platform height/gap, and tightened sleeper spacing. Production
build and 11 focused tests pass. Mature WebGPU captures cover both styles and
a live departure; no browser errors observed. This is partial fidelity, not
artistic acceptance. See [geometry, cost and remaining scope](commuter-rail.md).
Next rail slice: catenary and corridor edges, then articulated movement and
station behavior; retain distinct future streetcar and ground-rail families.


## Bank planting asset reassessment — 2026-10-11

Two bounded variants using scaled existing tree cards failed to produce readable
low planting in matched mature noon views; both were removed. Tests/build passed
but did not establish visual benefit. See [trial evidence](bank-planting-experiment.md).
Before further placement, validate a dedicated shrub silhouette at neighborhood
scale. Do not repeat scale/quantity tweaks on the same tree-card asset.

## Fine water banding corrected — 2026-10-11

Isolation confirmed the repeated narrow bands came from the fine sine normal
terms. Two drifting 2D noise fields now replace them, preserving footprint
attenuation, roughness, shadows, reflection and simulated rain. Matched WebGPU
Anime/Miniature dusk views lose the regular bands; production build and WebGL
founder smoke pass. Detail remains subtle and water is still short of the
concepts. See [evidence, cost and limits](water-fine-normals.md). Close this
bounded intervention and reassess broader fidelity gaps before more water work.

## Water pass isolation — 2026-10-11

Matched diagnostics identify the broad angular water patches as received
shadows: they disappear only when shadow reception is disabled. Removing planar
reflection leaves both those patches and the fine repetitive bands. Both
experiments are reverted; required shadows/reflections remain. See
[pass evidence and next diagnostic](water-pass-isolation.md). Fine-wave surface
lighting is the next candidate to isolate, not a proven defect in reflection.

## Water mesh hypothesis rejected — 2026-10-11

A bounded 2 m surface grid passed four geometry tests and production build, but
matched waterfront captures retained the broad angular patches. It added nearly
90,000 visible-river triangles without a demonstrated visual benefit, so the
integration and helper were removed. Runtime water remains unchanged from HEAD.
See [experiment and next diagnostic](water-mesh-experiment.md). Isolate shadows
and reflection before further water tuning; do not repeat tessellation blindly.

## Repeatable waterfront comparison — 2026-10-11

Development renderer details now has **Hold waterfront view**. It selects a
deterministic, collision-checked Explore pose while preserving lens and world
state. Moving to Overview and applying it again produced byte-identical browser
captures at the paused mature checkpoint. Root camera/UI tests pass 62/62 and
production build passes. See [reproduction and limits](waterfront-review-camera.md).
Use this fixture before further water tuning; the rejected shaders remain reverted.

## Water experiment rejected — 2026-10-11

Two bounded shader variants were reviewed against the concept water treatment.
The first overemphasized the reflected sky; the second introduced periodic dusk
glare bands. Both were reverted; runtime water is unchanged. Build passed and
WebGPU reported no shader errors, but those results did not meet the visual bar.
See [evidence and next-action constraints](water-radiance-experiment.md).

## Broader elevated corridors — 2026-10-11

Three fixed lateral fairing passes now remove tight street-junction zigzags
from the independent elevated routes before cubic controls and lot admission.
Ten-seed minimum sampled radius improves from 11.460 m to 102.363 m. Native
422 tests pass, native/WASM controls and 800-tick replay agree, and 10 focused
frontend tests pass. Normal mature browser import settles both queues; matched
style captures are recorded. Seed 87231 now has 1839 buildings (15 fewer).
See [evidence and limits](expressway-fairing.md). Current checkpoint:
`.tools/fidelity-review/fair-routes-87231.json`; earlier plan saves below reject.

## Elevated reference and join verification — 2026-10-11

Actual swept mesh sections now have a regression covering deck, median and
barrier joins in both storage orientations. Focused geometry tests pass 9/9.
The ten-seed WASM audit now checks reversed endpoint orientations and fails on
tangent discontinuities; all ten pass. Tight interior curves remain visibly
short of the broad photographic target, and ramps/graded merges are absent.
See [join verification](expressway-boundary-curves.md). This follow-up changes
validation and reference documentation, not runtime appearance.

## Frame timing diagnostics — 2026-10-11

Frame sampling now distinguishes loading/ready/hidden, resets across visibility
and phase transitions, retains real long visible frames, and exposes p95 CPU
and frame intervals. Focused 32 tests and build pass. A normal mature import
settled at a sampled 53 fps, frame p95 26.1 ms, CPU p95 20.8 ms, with queues zero.
This is observational evidence, not a hardware benchmark or proven speedup.
See [measurement method and limits](frame-timing.md).

## Architectural detail — 2026-10-11

Gabled roofs now share bounded rounded ceramic detail in low-rise and anchor
kits. Root architecture tests pass 27/27; production build passes. Fixed
specimen views show an improvement in both styles, while mature-city motion,
cost and complete concept fidelity remain open. A mature 1854-building audit
now measures +3.96% authored triangles and +2.28% source geometry bytes. Both
browser geometry queues settle, and matched styles preserve the held city view.
A 3 fps diagnostic sample keeps real performance acceptance explicitly open.
See [roof evidence](ceramic-roof.md). Current mature save:
`.tools/fidelity-review/ceramic-mature-87231.json`.

## Boundary curve correction — 2026-10-11

Ten-seed audit found a folding 10 m boundary tail on seed 1234. Regional ends
now continue the interior bearing instead of resetting to an axis. Native
418 tests pass; rebuilt WASM corpus has no sampled folds and native/WASM
controls and continuation agree. See [evidence](expressway-boundary-curves.md).
Current local witness: `.tools/fidelity-review/boundary-1234.json`. Earlier
curve/traffic saves below reject after this plan-digest change.

## Elevated through-traffic — 2026-10-11

Saved regional cars now traverse complete open boundary-to-boundary cubic
expressways, with explicit deck surface height in worker protocol 3. Native
417 tests and focused frontend 48 tests pass; WASM and production builds pass.
See [traffic validation](expressway-traffic.md) for current evidence and gaps.
Fresh local checkpoint: `.tools/fidelity-review/traffic-87231.json`.
Follow retirement now returns to Watch and clears expired inspector subjects;
camera/UI 58 tests and production build pass, with a normal browser trip-exit
witness recorded in the traffic validation report.

## Authoritative expressway curves — 2026-10-11

The seam repair has advanced to master-plan cubic controls and curved corridor
clearance. Shared sweep geometry replaces independent rectangular slabs; the
renderer no longer infers bends from realized neighbors. Prior master-plan saves
reject because the digest now includes controls and admitted lots changed.
Fresh native/WASM checkpoint: `.tools/fidelity-review/curves-87231.json`.
See [evidence, density costs and remaining work](expressway-curves.md).
The goal remains active; ramps and artistic acceptance are unfinished. The
through-traffic checkpoint above supersedes the traffic gap recorded here.

## Station layout and movement elevation — 2026-10-11

Engine-owned terminal frames, compact queue slots, saved elevated walking paths,
and shared platform/stair geometry replace the incorrect queue placement.
Worker protocol 2 carries surface height through rendering, following and rain
accessories. Native 405/405 and focused frontend 71/71 pass; the full frontend
suite has 294 passes and three timeouts. Current WASM queue containment/height
and 800-tick continuation pass. See [evidence and open motion gates](station-passengers.md#implementation-checkpoint).
Changes are in progress on `codex/living-city-fidelity`; broader visual acceptance
is not established. That station checkpoint is now superseded by the authoritative-curve checkpoint
above; `.tools/fidelity-review/station-fixed-87231.json` no longer loads.

## Station passenger baseline — 2026-10-11

The next realism correction has a reproducible failing WASM baseline:
seed 87231 at tick 170480 has two station waiters, both on the opposite side
of the tracks from the rendered platform. Source review also finds oversized
queue rows, missing elevation in the movement contract, and no physical entry
stairs. A renderer-only height offset would be incomplete. See
[the evidence and implementation acceptance](station-passengers.md).
No station fix or new visual acceptance is claimed. The goal remains active.

## Expressway crossing correction — 2026-10-11

The mature city's expressways now cross as independent 11 m/20 m routes. Plan
node identity, digest, snapshot heights, deck geometry, furniture and lower-road
pier clearance agree. Native 402/402, focused geometry 5/5, WASM construction
replay and build pass; frontend full suite has 292 passes and the previously
observed movement-test timeout. See [evidence and remaining gaps](living-city-fidelity.md#layered-crossing-verification).
Older master-plan saves reject explicitly; no migration is provided. The goal
remains active, and broader art/performance/behavior acceptance is unfinished.

## Resumed living-city fidelity work — 2026-10-11

The goal has resumed. Current work is isolated on `codex/living-city-fidelity`
above the reviewed PR #3 head. See [the evidence and ranked visual gaps](living-city-fidelity.md).
The mature WebGPU city loads through the normal scheduler. The first bounded
intervention corrects blank elevated-road furniture and lamp height; it does
not establish concept fidelity or hardware performance acceptance.


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
# Road staging candidate — 2026-10-12

[Current staging evidence](road-staging.md): resumable preparation and shared
soft frame budgets preserve eight snapshot geometry hashes. 30 focused tests and
production build pass. Full-job start is 0.44–0.77 ms, maximum measured call
3.98–5.65 ms, and completion 19–25 update calls in a CPU-only diagnostic.
Candidate is **not accepted**: ordinary developing/mature browser scheduling and
actor/development coherence must be checked; 0.32–0.42 seconds equivalent at
60 fps can still display older roads behind the live simulation. The earlier
21-building startup capture predates this preparation revision. No geometry
quality was removed; full living-city/artistic requirements remain unchanged.

Browser follow-up: normal import/growth progressed from 442 to an exported
664-building checkpoint (seed 87231/tick 27769). Queues settle; both styles apply.
Accelerated-growth sample still has frame p95 95 ms / longest203 ms; this is not
performance acceptance. Source audit confirms actors/buildings can consume newer
infrastructure than the committed road mesh. Next fix that dependency contract,
including existing actors entering newly-open roads; merely gating new IDs fails
that case. See the report for exact checkpoint hash, settings, captures and
remaining startup/style latency limitations.

Building portion now implemented: `VisibleInfrastructure` gates queued building
geometry against committed access-road geometry plus reset epoch. 40 focused
tests and build pass; renderer integration confirms release after road commit.
Not yet browser-verified. Actor/surface/transit/ground/access presentation remains
ungated; next establish authoritative movement dependencies and validate ordinary
growth as one coherent scene. Do not treat the partial gate as acceptance.

Building gate follow-up verified: ordinary UI import/growth664→834 buildings,
all geometry queues settle, Miniature applies. Exact final seed87231/tick34591
checkpoint and hash are in the report. Three-seed WASM corpus independently
passes32,604 building observations with valid access roads. Sampled growth still
hitches (longest154.6ms); no performance/artistic acceptance. Next implement
same-traversal movement support metadata, with worker epoch/order/recycling and
presented-pose consistency; avoid another parallel movement-position algorithm.

### Incremental contour integration — 2026-10-12

Primary workstream remains ordinary growth stalls. Latest main was fetched and is already an ancestor (origin/main 2c443a9, branch HEAD 76530ae); dirty work preserved. Previous goal work made progress through profiling and road-motion evidence.

BuildingChunks now registers sources without scanning contour vertices. It acquires/leases a source group, advances IncrementalBuildingContours at a shared 2,000 work units per update, caches the complete result, and publishes contours with the atomic batch. Cancellation disposes the helper before source geometry; completed source contours remain cached. Source factory construction is still synchronous. Root review corrected omitted unmatched edges, rotated bounds, morph prefilter semantics, local Float32 rounding, interleaved attributes, normal calculation and the final unbounded result-copy path before integration.

Root verification: TypeScript and production build pass (existing bundle-size warning); 35 tests pass across incremental-building-contours, building-chunks, building-retirement and construction-batches. New registration/cancellation test observes zero position reads during set(), exactly 2,000 during first update, no partial published batch, and single source disposal on clear.

Root reran scripts/profile-growth-buildings.mjs with output validation/growth-building-incremental-profile.json. Baseline validation/growth-building-profile.json is retained. On Apple M2 Max/Node v26.7.0, seed0 tick9250 production buildings1602/1611 match independent EdgesGeometry output exactly, including final committed batch arrays in all five repetitions. Helper p95 steps 2.295/2.482ms, maximum 2.622/2.908ms; registration medians .017625/.015541ms. Helper takes126/115 updates; whole isolated batch144/131. These are CPU diagnostics, not browser performance proof.

Next gate: ordinary browser scheduler growth recording, both styles and exact camera/tick context. Check frame timing AND publication latency: spreading work can delay openings and grow queues. Do not accept solely on low per-step cost or a manually drained diagnostic. Final cached road-partition motion and matched nighttime apartment opening also remain pending. Goal incomplete; no commit/push/artistic acceptance claimed.

### Browser attribution after incremental contours — 2026-10-12

See validation/contour-growth-browser.md and contour-growth-stage-trace.json. Root used normal seed0 tick9250 import/growth, High Miniature/noon/50mm/DOFoff/Explore1814. Recorded baseline still has224ms frame; backlog naturally clears when paused. A second, temporarily instrumented run attributes21 >200ms samples to pipeline.render(), with separate building stage maximum35.2ms. Do not call the overall stall fixed. Backend pipeline creation vs first-use buffer uploads is the next bounded diagnostic; Three source confirms both paths exist, not which causes these spikes. Temporary city.ts tracing was archived as a patch and removed exactly. Full recording review, Anime and opening-latency gate remain pending. Tab27 is retained but HMR resets the imported scene; reimport normally. No commit/push, goal active.

### Photographic camera steering and repeatability — 2026-10-12

User explicitly questioned camera/depth fidelity. Primary next work is a bounded
photographic lens comparison; growth render-resource attribution remains
outstanding, not resolved. Root captured Miniature50mm DOF on/off at seed0
tick9250/17:00/High/Explore; baseline views and observations are documented in
validation/camera-dof-baseline.md. Office background softens; waterfront effect
is subtle and the framing lacks developed context. Do not claim concept match.

Radius16 experiment was reverted to8: camera differed because old Frame review
building used animated onFollow and Explore interrupted it. Luna implemented
reviewBuilding(id); root inspected and wired a dedicated optional dev callback
through Controls/main. Reuses building follow shot selection but prevalidates
destination, directly holds Explore, preserves lens, and does not change normal
follow. Root TypeScript and69 tests pass(camera, camera-review-building,ui).
New helper tests use mocked OrbitControls; real browser repeatability must still
be checked before repeating cap/focus experiments. No optical change promoted.
Tab27 remains retained but HMR reset world; import fixed save and use new Frame
review building directly, without an extra Explore click. Current goal active.

### Cap-independent lens coverage candidate — 2026-10-12

Root verified new reviewBuilding action in real browser: held-office-50-on and
held-office-50-repeat PNGs are byte-identical after an intervening waterfront
view. Matched cap16 experiment exposed composite coupling: normalized CoC was
also used as blend alpha, so raising cap reduced intermediate blur. Current
candidate in SignedCoCDepthOfFieldNode.js converts normalized radius back to
output pixels for coverage(min(radius/4,1)); city.ts Miniature cap16,Anime4.
Corrected static screenshot held-office-50-cap16-coverage.png is inspected and
retained. Full details in camera-dof-baseline.md. Root TypeScript/build pass,
WebGPU console empty; optical focus/DOF style choice is still provisional.
Next verify Anime (coverage changed there), motion/night silhouettes and repeat
at other lenses/resolutions. Do not treat this as photographic or goal acceptance.
Growth render pipeline/upload investigation remains outstanding. Tab27 retained;
header edit may have HMR-reset it, so inspect and reimport normally.


## Persistent lighting: matched growth trace — 2026-10-12

Revalidated the candidate through normal scheduling in the Codex in-app browser,
WebGPUBackend, on a MacBook Pro Mac14,5 / Apple M2 Max / 32 GB RAM. Browser engine
version and GPU execution timings were not collected. Viewport 1280×720, High
quality, Miniature, DOF off, 50 mm lens, held review building 1814, noon lighting
preview. Imported seed 0 / tick 9250 / 260 buildings while paused; waited for all
four visible geometry queues to settle, then resumed at simulation 1× / growth
16×. No queues were manually drained. Weather was inherited from the checkpoint.

`persistent-light-growth-trace.json` records all 4,725 sampled animation frames
with 9250 < tick < 10300, rather than only slow frames:

| CPU measure | p95 | Maximum | Frames over 30 ms |
| --- | ---: | ---: | ---: |
| Full animate body | 15.1 ms | 49.2 ms | 27 |
| Render submission | 9.4 ms | 43.3 ms | 1 |
| Node-state lookup/build | 0 ms | 30.0 ms | 0 |

Only one new node-state cache miss was recorded (MeshStandardNodeMaterial,
30.0 ms). The previous diagnostic recorded repeated 190–245 ms render submissions
with 174–228 ms spent in node-state work. Those repeated stalls did not recur in
this interval. Earlier traces sampled slow frames only, so percentile comparisons
would be invalid. This is CPU submission evidence, not GPU timing or overall
frame-interval acceptance. Remaining 49.2 ms work spikes still need attention.

Root reran 40 tests across road batches, street lighting, nearby lights and style
warmup; all pass. TypeScript passes after removing instrumentation. The diagnostic
patch is archived in `persistent-light-growth-instrumentation.patch`; production
code contains no trace wrappers. Existing lifecycle audit found no concrete defect
in cancellation, reset, style traversal or deferred shutdown disposal. Both-style
live growth, long-run memory, night motion and broad performance remain unaccepted.
The immediate shader-churn hypothesis is supported; further generic cache work is
not justified by this result. Resume perceptual camera/composition work next.


## Follow subject focus — 2026-10-12

Camera review found that Follow composition intentionally looks 6/8/10 metres
beyond residents/buses/trains, but the depth pass focused on that same look-ahead
point. Director now exposes a separate reusable `focusTarget`: building center,
train center at rail/body height, or the published movement position. The camera
position, orientation and look-ahead remain unchanged. Existing axial depth and
focus damping consume this point. Missing-subject grace retains the last valid
point; manual interruption/cancellation returns focus to the orbit target. Watch
and Explore focus behavior are unchanged; tilted-plane focus remains absent.

Luna implemented the director and regression tests; root inspected lifecycle and
renderer integration, then ran 68 tests across camera, focus, lenses, presentation
and deterministic review framing. All pass; TypeScript and diff whitespace pass.
This is a correctness improvement, not evidence of a major artistic improvement.

Browser smoke/pair: seed 0, tick 9250, 260 buildings, paused at 15:15, checkpoint
weather unchanged, Miniature, 50 mm, High, WebGPU, viewport 847×764, Follow the
Neighborhood Shuttle at its terminal. Normal import queues reached zero before
capture; no queue draining. `screenshots/subject-focus/train-miniature-on.png` and
`train-miniature-off.png` keep the same paused subject/camera/time. Train remains
legible; foreground roof edges soften modestly. The pair compares DOF on/off, not
old/new focus implementations. It does not establish motion or Anime acceptance.

Perceptual priority: the observed ordinary Watch neighborhood view puts large
foreground buildings across the street subject. More blur cannot fix that
composition. The concept boards arrange clear street/transit subjects with layered
context. Next bounded camera work should assess Watch subject visibility using
existing obstacle envelopes and stable shot selection, preserving relaxed motion;
do not add blanket stronger blur. Lighting/material/density gaps remain separate.


## Watch visibility attempt rejected — 2026-10-12

A bounded street-target/sightline selector passed geometric tests but produced a
steep overhead roof canyon in browser review. Reverted all candidate source and
exclusive tests; retained patch and screenshot. Follow focus remains intact.
58 camera/review/focus tests and TypeScript pass after restoration. Next approach
must constrain artistic elevation and choose a different street/subject if needed,
rather than maximizing visibility by climbing. See [experiment](watch-visibility-experiment.md).


## Lower-angle alternate-street candidate — 2026-10-12

The second experiment is retained as a candidate after a materially better browser
composition. Instead of escalating to steep overhead views, it tries up to four
nearby open local/arterial streets and at most 18 poses per street. Slopes are
bounded at .65 (about 33 degrees); boundary clamping cannot silently steepen an
accepted pose beyond that. Each candidate tests the street center and +/-12 m
against building envelopes, separately checking the camera destination. Existing
collision-safe travel still controls transitions. Clear views keep their original
preferred angle; selection is deterministic and cached across unchanged geometry.
The street point stays pinned while neighboring building centroids change.

Root integrated the selector and alternatives; Luna provided selector tests,
integration coverage and a lifecycle review. The review caught road-closure handling:
cache now records the actually selected road, not only the nearest candidate. If it
closes, Watch transitions to a rebuilt district fallback even if there is no new
eligible street. Tests cover that fallback and manual Explore priority.

Browser study uses the same seed 0 / tick 9250 / 260-building paused checkpoint as
the rejected trial, Miniature, 50 mm, High, DOF on, live 15:15 lighting, checkpoint
weather, 847x764, WebGPU, normal import scheduler. The selected shot exposes a
street/rail corridor as a diagonal, with nearby and distant facades forming layers.
It is more readable and photographic than the steep roof canyon. Root switched to
Explore to hold it for `screenshots/watch-visibility/low-angle-candidate-on.png`.
This is a composition study, not a pixel-identical camera comparison or a claim of
locked-concept fidelity. Later changes only corrected closure handling, boundary
angle rejection and internal visibility status; final both-style/motion checks
remain pending. HMR again briefly showed the startup timeout UI, then recovered;
startup reliability remains a separate unresolved observation.

76 focused camera/selector/review/focus/lens tests and TypeScript pass. Production
build passes with the existing bundle-size warning. If no candidate is visible,
the current implementation retains a generic fallback with explicit internal
visible=false; that fallback has no visual acceptance. Street samples do not model
tree, station-canopy or bridge occlusion. CPU cost, ordinary growth motion, lens/
aspect changes, nighttime and Anime review must be checked before promotion.


CPU follow-up: root inspected and reran `scripts/audit-watch-shot-cost.mjs`.
Current `watch-shot-cost.json` reports Node-only seed0/tick9250 measurements on
Apple M2 Max: anchor build maximum 0.914 ms; 75 cold district shot selections
p50 0.790 ms, p95 3.031 ms, maximum 5.054 ms. Twelve of fifteen district anchors
found a clear street candidate; three did not. This supports a bounded candidate,
not browser frame-time acceptance. The additional old growth-575 checkpoint fails
current save validation (`traveler 3011 destination connector does not meet its
route endpoint`), so it was explicitly excluded; no mature-city coverage is claimed.


## Commit checkpoint — 2026-10-12

See [checkpoint summary](checkpoint-2026-10-12.md) for captured scope, fresh
validation and remaining limits. Native release tests and production build pass;
the web contour threshold test remains failing after an isolated rerun. The
movement timeout from the full suite passes in isolation. This checkpoint does
not establish final visual acceptance.
