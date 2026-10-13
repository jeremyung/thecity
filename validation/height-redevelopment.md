# Station office height redevelopment candidate — 2026-10-12

The full seed87231 city previously had one 18-floor tower and no height renewal.
The first-build quota experiment was rejected because it produced towers too
early. This candidate adds actual staged replacement at mature station sites.

Admission requires 85% lot realization, 300 active buildings, demand >=2.5,
an online station, an office at least three calendar days old, a nonflag
StationCore lot >=280m², spare office capacity elsewhere, and no resident or
traveler references. Anchors, protected parcels, retained original footprints
and pending rear extensions are excluded. A seeded quarter of sites retain
their character. Only one replacement runs at a time, on the existing three-day
renewal cadence, scanning up to24 eligible core sites through a saved cursor.

The same building and parcel IDs remain. The business explicitly closes;
demolition occupies the first30% of the project, then reconstruction adds six
floors. The old floor count remains authoritative during demolition. Reopening
resets building age and adds history. This is an initial office slice, not
housing relocation, lot assembly or acceptance of the mature skyline.

Renderer candidate: remove original midrise/tower masses from the top down,
retain lower dark facade bands, expose the working floor and keep site hoarding.
Demolition signatures rebuild each removed floor. Materials remain palette-owned
and geometry is shared across styles. Close-up continuity against the old facade,
construction transitions, motion and both-style artistic review remain required.

## Evidence

- Root reran seven native redevelopment tests: all pass, including actual step
  replay with demolition and reconstruction restores. Predicate tests use
  constructed engine states; they do not establish natural admission.
- Fresh seed87231, public development rate16, ordinary WASM steps: first admission
  tick72000 with1795 buildings and1813/1839 realized lots. Office1243 on parcel927
  grows6→12floors, capacity30→60, completion tick74450. Canonical live/restored
  saves match for49 checks /2450ticks; every captured phase restores identically.
  See `height-redevelopment.json`. No state edits create these checkpoints.
- 13 web checks pass across demolition, tower geometry and render signatures.
  TypeScript and the final production build pass (existing bundle-size warning).
- WASM rebuild passes. An earlier overlapping build caught an agent's temporary
  missing helper; the stable-source rebuild succeeded.
- Normal UI import of tick72400 succeeds on WebGPU, High, Anime, 50mm, DOFoff,
  dawn preview,847×769. Normal queues settle to zero; warning/error query empty.
  `screenshots/height-redevelopment/import-smoke.png` shows district loading,
  **not** a close-up of the redevelopment site or artistic acceptance.

Reproduce: `npm run wasm`, then
`node scripts/audit-height-redevelopment.mjs`. Saves are under
`.tools/height-redevelopment/` for normal browser import. This script stops after
the first replacement; the separate long-run audit below covers later renewals. No full default test-suite or hardware performance claim.

Next: inspect the actual host at every phase in both styles and in motion;
check later renewals across seeds and retained character, then decide whether
the candidate advances the skyline target. Do not increase tower admission
probabilities to hide a failed redevelopment result. Goal remains active.


## Follow camera and construction review — 2026-10-12

The ordinary field-notes panel now offers **Visit the renewal site**, routed
through the existing building follow action. Building follow frames the full
building volume and chooses from bounded elevated views. Visibility includes
the lower worksite as well as the centre and upper volume; checking only the
upper points previously left demolition hidden behind adjacent roofs. Camera
and engine state remain independent. This candidate does not guarantee an
unobstructed facade from every site or viewport.

Normal browser imports and live progression inspected office1243, parcel927,
seed87231. Captures use WebGPU High, 847×769, 50mm, DOF disabled, clear weather
(intensity0), noon lighting preview, and the ordinary building-follow camera:

| Capture | Tick | Observed state |
| --- | --- | --- |
| `screenshots/height-redevelopment/demolition-anime.png` | 72400 | Six-floor office, demolition progress .5556; exposed lower worksite |
| `screenshots/height-redevelopment/demolition-miniature.png` | 72400 | Matched state and camera in Miniature |
| `screenshots/height-redevelopment/construction-miniature.png` | 73550 | Twelve-floor target, structure progress .4940; generic broad floor plates |
| `screenshots/height-redevelopment/live-completed-miniature.png` | 75181 | Same office reopened at twelve floors after ordinary running from72400 |

The live run requested2× speed; the follow camera widened as the target grew.
Its export remains at `~/Downloads/thecity-world-2026-10-12 (14).json`.
This is interactive observation plus still captures, **not a motion recording**.
The construction baseline was captured again after resuming this work; its
warning/error query returned no entries. The latest build finished successfully
with the existing bundle-size warning. Camera, follow visibility and UI tests:
**69 passed**, including a lower-worksite visibility regression.

Art judgment: demolition is readable but too bare. More importantly, tower
construction uses a full rectangular footprint while the finished tower has a
podium, narrower shaft and recessed crown. This is a real continuity defect,
not an engine height-allocation problem. Next bounded intervention: share the
finished tower's mass definitions with staged construction, retaining floor
progress, parcel containment and both-style geometry. Accept only if the same
normal construction checkpoint visibly preserves those setbacks; reject an
approach that needs a second independent silhouette or hides the transition.
Full finish-stage facade continuity and recorded motion remain required.

## Thirty-day sampled renewal audit

`scripts/audit-height-redevelopment-longrun.mjs` uses fresh public WASM engines,
ordinary steps and development rate16. `height-redevelopment-longrun.json`
records720000ticks for each seed, sampled every200ticks. No state editing or
private admission calls create the renewals. Root inspected both script and
report; the population field is `world.stats.population`.

| Seed | Admitted / completed renewals | Final towers (≥16 floors) | Maximum sampled buildings | Maximum sampled population |
| --- | --- | --- | --- | --- |
| 87231 | 5 / 5 | 4 | 1839 | 17655 |
| 0 | 2 / 2 | 8 | 1730 | 17318 |
| 5 | 4 / 4 | 8 | 2062 | 18850 |
| 77 | 2 / 2 | 7 | 1826 | 17474 |

All four final states have no pending height project. Immediate canonical
restore and200ticks of original-live versus restored continuation match in all
four. Recorded admissions preserve non-anchor character eligibility and add six
floors; all recorded completions are open. Repeated renewal sometimes selects
the same site twice. Final tower totals include original construction, not just
renewals. These samples support finite growth on this corpus, not an indefinite
memory plateau, all-seed correctness, hardware performance or visual acceptance.
The audit exits nonzero for cap/restore/replay failures; other recorded properties
above were independently inspected, not asserted by its exit status.

Reproduce: `node scripts/audit-height-redevelopment-longrun.mjs` after rebuilding
WASM. Housing relocation, lot assembly, mature skyline composition, complete
phase continuity, motion, audio and full concept acceptance remain open.


## Tower construction massing correction — 2026-10-12

Construction now consumes the existing finished towerMassing definitions for
apartments and offices of at least twelve floors. The existing midrise staging
renderer accepts a masses-only layout and is shared with towers; it erects
podium, shaft and crown plates at their actual widths, offsets and floor levels.
No engine rules, new materials or alternate silhouette generator were added.

The identical normal seed87231 tick73550 checkpoint was imported through the UI,
followed via Visit the renewal site, and viewed at noon, clear weather,50mm,
DOFoff,847×769,HighWebGPU. Queues settled to zero through ordinary scheduling.
`construction-massing-miniature.png` visibly restores the shaft setback against
`construction-miniature.png`; `construction-massing-anime.png` uses the same
state/camera after a completed style switch. HMR briefly showed the graphics
startup warning; it cleared without Retry or backend switching.

Root inspected Luna's actual-mesh test and strengthened it with exact erected
slab count and valid unoccupied construction stages. It covers three profiles,
both kinds and both styles at shaft and crown progress. The four-file tower,
construction, demolition and midrise run passed17tests; the strengthened new
test subsequently passed alone. Production build and TypeScript passed (existing
bundle-size warning). This is targeted verification, not a full-suite claim.

Retain the correction: it visibly fixes the structural silhouette. Finishing
facade panels still differ from the completed facade, and no motion recording
establishes a seamless opening. Site richness, later skyline composition and
complete concept/style acceptance remain open.

Finishing follow-up: normal tick74300 import, same seed/site and noon preview,
Anime/50mm/DOFoff/HighWebGPU/847×769. All four queue counters reach zero normally;
warning/error query empty. `finishing-massing-anime.png` confirms the recessed
crown but exposes generic temporary window panels. Do not call the opening
transition seamless. Browser remains paused on this checkpoint for continuation.


## Authored tower envelope during finishing — 2026-10-12

Tower cladding now calls the same renderTower facade builder as the open building,
with an installed-floor limit. The shared podium/shaft/crown definitions stay
fixed while cladding advances from the bottom. Installed floors replace their
structural plates and columns rather than overlapping coplanar faces. The roof
and equipment arrive after all facade floors; entry detail waits for two floors.
Construction uses dark glazing for otherwise occupied panes and lobby lighting;
the aerial lamp also stays nonemissive. Opening enables operational materials.
Temporary scaffold/crane remain until the authoritative stage opens.

Tower render signatures now follow both erected-floor and installed-floor counts,
so changes inside an old20%progress bucket no longer wait for the next bucket.
This does not change engine timing or other architectural families. Floorwise
assembly remains discrete, not continuously animated construction.

Normal browser witness: seed87231 office1243/parcel927, starting tick74300 from
an unedited normal-growth save, Anime, noon preview, clear weather,50mm,DOFoff,
HighWebGPU,847×769, building follow. Queues0 before capture. The authored curtain
wall/roof are visible in `finishing-authored-anime.png`. Ran ordinary1×simulation
and paused at75789; exported `~/Downloads/thecity-world-2026-10-12 (15).json`
confirms the same ID,12floors,open,business_active=true and reopening history.
`opened-authored-anime.png` preserves the same facade and roof with operational
window accents and no crane. Post-run queues0. This is live progression and
before/after capture, **not a continuous motion recording** or both-style browser
acceptance. HMR's startup warning cleared without recovery intervention.

Root reviewed Luna's tests and corrected material selection that initially
omitted roughness-specific frame/glass meshes; also corrected office shadow
selection. Final checks compare actual nonemissive shell/frame/roof geometry,
open glazing as a subset of staged glazing, no construction emission, roof timing,
remaining exposed slab counts and signature invalidation. Five focused suites
passed25tests; final corrected tower-construction suite4pass. The first build
failed on a test Set type mismatch; corrected TypeScript and production build
pass (existing bundle warning). No full-suite or hardware-performance claim.

Next: record construction/cladding/opening in motion and inspect earlier partial
cladding, Miniature and night. Verify no transient overlap or update stalls at
floor boundaries. Demolition richness, later skyline composition and the broader
living-city fidelity gates remain open; do not equate this facade repair with
completed audiovisual acceptance.


## Recorded Miniature night lifecycle — 2026-10-12

A development-only canvas recorder is now available under Renderer details.
It requests30fps, stops after20seconds, limits retained chunks to20MiB, and cleans
up tracks/timers/object URLs. It captures no microphone, screen or audio. Closing
the details panel cancels and releases the component. Root corrected unmount
callbacks and final-chunk overflow handling in Luna's implementation. Production
build and18UIchecks pass; recorder lifecycle has browser smoke evidence, not
exhaustive browser/error-path coverage. Recording overhead precludes treating
this clip as an uninstrumented performance benchmark.

`miniature-night-renewal.webm` in the screenshot directory is a real canvas video,
not a slideshow. Browser decoding confirms1355×1230,19.967seconds; file about3.5MB.
Capture began at normal seed87231 tick73950, office1243, three facade floors
installed, development16, requested simulation2×. Miniature,22:00lighting preview,
clear weather,50mm,DOFoff,HighWebGPU; CSSviewport847×769, canvas backing resolution
1355×1230. Follow camera stays fixed. Normal queues0 at capture start and after
the run; console warning/error query empty. No queue draining/state mutation.
The original live city ran beyond the recorded interval before being paused;
its later UI state is not an exact timestamp for the recording's final frame.

Playback and seek inspection at0,6,12,16and19seconds confirms real changing frames:
partial unlit frame; small lower-facade addition by12s; complete unlit envelope
with crane at16s; lit occupied panes and no crane at19s. The roof/facade identity
is retained into opening. However, the large cladding jump between12and16s means
**smooth progressive construction is not accepted**. Determine whether worker
publication, building queue or neighborhood batch coalescing delays installation;
do not assume the cause from this video alone. Next primary action is a bounded
latency diagnosis at this same site, preserving ordinary scheduling and geometry.
Night treatment remains dark/plain compared with the locked concepts.

`motion-review.html` is a local viewer for this evidence, with a seek field.
Some browsers initially report infinite duration for MediaRecorder WebM until
playback parses the final cluster; the measured duration above is after playback.
The review tab is paused at16s. `night-recording-review.png` records the6s sample.
Full-frame motion/artistic review across other stages/styles, audio and hardware
performance remain outstanding. This clip advances evidence; it does not close
the living-city release gate.

## Renewal batch latency audit — 2026-10-12

The synthetic audit now uses the same `buildExtendedBuilding` factory as CityRenderer,
including persisted original footprints (zero in this fixture), compactMidrise off,
and unchanged transform/merge/contour quotas. Seed 87231, normal save tick 73950,
target building 1243; replacement is its actual tick 74300 finishing record.

The 200 m neighborhood contains 142 buildings. Replacing the target requires 231
`BuildingChunks.update()` calls and regenerates every one of its 141 peers exactly
once. The independently repeated production-factory run took approximately 349 ms
of serial CPU drain, with p95 3.64 ms per update. This is not browser frame timing:
CityRenderer advances only once per render update, and other queues and GPU work
are absent here. The recorded jump is consistent with this delay amplification,
but exact publication-to-visible latency has not been measured in the browser.

An explicit `--isolated-target` experiment submits only the same tower: replacement
commits in one update, approximately 2.77 ms. It preserves the geometry and quotas,
but does not implement or validate dynamic/static migration. Both JSON artifacts
include input hashes, module hashes, machine information and per-update timings.
Run `node scripts/audit-renewal-chunk-latency.mjs` for the neighborhood baseline;
append the two save paths, an output path, `1243 --isolated-target` for isolation.

The engine publication audit also observed a cladding change at tick 74110 without
a world revision change; periodic revision increments at 40-tick intervals bound
that particular delay. Do not treat this as proof that worker publication explains
the whole recorded jump.

Next bounded intervention: isolate actively changing construction geometry from
static neighborhood rebuilding. Require atomic handoff into/out of static batches,
exact descriptor visibility, no duplicate geometry, preserved picking/contours/
shadows/styles, bounded resources and fair scheduling. Test updates arriving during
handoff, removal, reopening, style switching and clear/disposal. Then repeat the
same normal UI import and night motion recording through the production scheduler.
Abandon or revise isolation if it replaces latency with visible handoff defects,
unbounded retained groups, or unacceptable measured draw-call/frame cost. Do not
raise global work quotas or reduce authored detail to hide the delay.

No production rendering change in this audit. Progressive motion and artistic
acceptance remain open.

## Construction batch candidate — 2026-10-12

`ConstructionBatches` now routes non-open buildings into per-building batches and
open buildings into the existing 200 m static batches. The same materials,
geometry factories, contour extraction and per-update quotas are retained. A
work-conserving alternating scheduler advances one underlying pool per update.
Static/dynamic commit gates retain the prior visible representation through
handoff; isolated batch visibility includes contours, and picking ignores hidden
ancestors. Empty committed batches are retired. No simulation state changes.

Root review rejected the initial per-mesh hiding approach because shared static
meshes contain neighboring buildings. Revised handoff never masks static batches.
A root-added overlapping-update regression then found a disappearance at frame
112: an obsolete dynamic removal could commit after construction resumed. The
removal now waits for a visible counterpart, and a superseded blocked build yields
to the newer source. Twenty focused tests pass, including per-update target and
same-material peer triangle ownership. Production build passes with the existing
large-bundle warning. These tests do not constitute full disposal/style acceptance.

The normal-save synthetic wrapper audit (`renewal-construction-batches-latency.json`)
replaces the tower in one update without regenerating its 141 static peers, versus
231 updates in the baseline. This is CPU/scheduler evidence, not browser FPS.
The initial migration of an already-static renewal site still waits for its static
neighborhood rebuild; later isolated phases avoid that cost. This first-handoff
latency remains a limitation to assess in normal live renewal, not just imports.

Next: repeat the tick73950 Miniature/night normal import and 20-second recording,
inspect intermediate cladding and reopening, then test an already-open site entering
renewal and style switching during handoff. Measure draw calls and frame timing.
Do not claim the motion/fidelity gap resolved before these checks.

## Browser batch review exposes reopening delay — 2026-10-12

Two normal-UI recordings use seed87231/tick73950, building1243 follow, Miniature,
22:00 preview, clear weather, 50mm, DOF off, High detail WebGPU. CSS viewport847×769;
canvas1355×1230; growth16×, requestedsimulation2×. Geometry queues reached zero
before each recording. Both captures use the DEV20-second canvas recorder, no
manual scheduler drain. Files: `miniature-night-renewal-batched.webm` (19.954s) and
`miniature-night-renewal-batched-repeat.webm` (19.898s), in screenshots/height-redevelopment.
Root inspected first clip at0/4/8/12/16/19 and repeat via uninterrupted playback,
then8/19. The first clip does not show reopening; the repeat reaches the full
unlit finishing envelope but still has temporary construction geometry at its end.
The live city later shows the completed facade with occupied window lights and
no queued geometry. No missing target/neighbor was observed in the sampled views;
this is not every-frame proof. Console warning/error query returned empty.

Cladding stages appear earlier than in the prior recorded baseline, consistent
with the isolated-phase benchmark. However the opening transition is NOT accepted.
A new synthetic audit (`renewal-opening-batch-latency.json`) replaces the same
surface record with actual completed tick74450: **463 update calls,141 regenerated
peers,~500ms serial CPU drain** until the exact open descriptor is visible. The
wrapper alternates static work with a dynamic removal waiting for static commit.
This measured return-to-static delay explains a remaining risk, not an exact
browser timing attribution: engine tick at each video frame was not captured.

Next bounded correction: make the final open appearance available promptly in
its isolated batch while the identical static representation is prepared, then
transfer visibility atomically. Preserve shared quotas, disposal, picking, styles
and bounded lifetime; avoid a second unbudgeted group generation. Also make the
scheduler avoid spending alternate slots solely on a blocked removal. Measure
both construction-phase and reopening latency, then repeat normal live recording.
Initial static→renewal handoff and style switching during a real handoff remain
browser acceptance work. Do not claim the whole lifecycle is fixed.

Root also rejected two initially superficial subagent tests and had them corrected:
actual resolver change while a 30-mesh build is detached, and descriptor/batch
owned-material cleanup on clear. Independently rerun focused suites:26passed.
No production change this turn. No commit/push; goal remains active.

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
