# Arterial sidewalk continuity candidate — 2026-10-11

Status: candidate retained, **not integrated into the live city**. The prior
turn made verified progress by identifying sidewalk ends independently of the
ground-depth defect. This turn implements and reviews replacement geometry,
then removes live integration because of measured synchronous rebuild cost.

The pure `arterialSidewalkPolygons` builder starts with full-length strips and
subtracts incident at-grade carriageways, wider bridge approaches, the existing
junction disks, construction corridors represented in roads, and already emitted
sidewalk faces. It excludes elevated rail/expressway. Stable road-ID order makes
input ordering irrelevant. Geometry stays inside the reserved sidewalk ribbons;
this does not implement crossings, curb ramps or pedestrian routing.

`sidewalk-kit.ts` extrudes to the existing .475 m sidewalk top and merges into
one batch. The temporary integration replaced only open arterial slabs; staged
construction retained its existing treatment. That integration was removed from
`city.ts`; no runtime imports of the candidate remain.

## Browser evidence before optimization

Imported `.tools/fidelity-review/isolated-ground-growth.json` normally: seed87231,
tick3066, 38 buildings, clear/intensity0, paused day0 09:03, WebGPU High, 50mm,
DOF off. Overview plus two upward scroll pages gave the comparable junction view.
Camera reacquisition means this is not pixel-identical to the previous baseline.
The pair below holds camera/time across style switching:

- [Anime candidate](screenshots/ground-depth/sidewalk-candidate-anime.png)
- [Miniature candidate](screenshots/ground-depth/sidewalk-candidate-miniature.png)
- [Earlier baseline](screenshots/ground-depth/city-motion-miniature-a.png)

The central corner visibly joins where the baseline had abrupt ends; the road
remains open. These do not prove every junction or mature-world acceptance.
A transient startup delay warning appeared after source reload and cleared
without a successful Retry click; it is not counted as a clean startup pass.

## Root validation and cost

Luna implemented the two pure geometry/test files; root inspected and corrected
weak crossing/bridge assertions, independently tested, integrated, captured and
measured. Seven focused geometry checks cover straight, mixed T, oblique/four-way,
construction, zero-length, bridge width, grade separation and ordering, including
positive-area intersection assertions. Combined elevation/access/ground coverage
passes 20/20. Production build passes with the existing bundle warning.

Root CPU measurements use Apple M2 Max, Node, the 564-road/89-open-arterial
seed87231 tick160000 snapshot. They do not measure browser/GPU frame timing.

| Candidate | Polygons | Triangles | Batches | Five measured construction samples |
| --- | ---: | ---: | ---: | --- |
| First | 2371 | 28596 | 1 | 95.6–110.2 ms |
| Convex intersection rejection + cached emitted bounds | 1192 | 14360 | 1 | 22.1–30.7 ms |

Evidence: [first](sidewalk-cost.json), [optimized](sidewalk-cost-optimized.json).
Reproduce with `node scripts/audit-sidewalks.mjs .tools/fidelity-review/mature-layered-87231.snapshot.json`.
The optimized version passes geometric tests but has not had a new browser
capture; the captures above belong to the first version. No claim of exact
polygon equivalence across that optimization is made.

After these two synchronous variants, reassess scheduling rather than repeatedly
tuning geometry. Next: retain unaffected geometry and construct changed junctions
in bounded work, with lifecycle cancellation and no temporary holes. Profile
geometry generation separately from extrusion/batching to choose the smallest
change. Review access-landing coplanarity and bridge/curb transitions, which remain
open; do not promote solely on unit tests or the small developing-city picture.


## Resumable clipping checkpoint — 2026-10-11

The previous turn made progress through geometric implementation and a measured
rejection of synchronous integration. Stage profiling now identifies both work
sources: clipping 11.3–16.8 ms, extrusion 10.1–13.1 ms and merge .78–.97 ms
on the same Node/M2 Max fixture. See [stage profile](sidewalk-stage-cost.json).
Extracted `sidewalkGeometry` preserves the extrusion primitive for future staged
construction; this alone changes no live rendering.

Luna converted clipping to `arterialSidewalkSteps`; root reviewed every yield
boundary and independently compared the full serialized mature polygon result
against a baseline frozen before conversion. All 1,192 polygons match exactly
at quotas 1, 128 and 2048. The job yields per road preparation, reservation scan,
fragment subtraction, prior-polygon check and output emission. The synchronous
wrapper is retained for offline tooling. It must not be used in a live frame.

[Resumable profile](sidewalk-resumable-cost.json): 209,187 iterator calls;
quota2048 completes in103 updates, p95 .486 ms/max .606 ms. These are one-process
Node measurements, in quota order, not browser timing or a hard frame guarantee.
Quota1/128 maximum observations were1.53/1.73 ms. Initial node-map/sort, node
incidence filtering and final array collection are still synchronous; degree
and city size bound those operations. Generator overhead increases full-drain
clipping to15.7–25.7 ms in the separate [stage run](sidewalk-stage-cost-resumable.json).
No throughput improvement is claimed.

Nine geometry tests plus the elevation/access/ground suites pass22/22. Small-case
quota tests alone share the synchronous drain implementation and are not proof
of algorithm preservation; the independent frozen mature comparison supplies
that evidence. Cancellation currently stops an iterator without publishing a
partial result; stale-world GPU disposal is not yet implemented or tested.

Next bounded step: a detached mesh job that quotas `sidewalkGeometry`, uses the
existing `IncrementalGeometryMerge` if merge/copy work warrants it, and retains
committed geometry until completion. Integrate reset/cancel, current-style material
resolution, compile-aware retirement and pending-work diagnostics together.
Do not merely queue the synchronous wrapper, publish partial holes, or claim the
running city has this geometry yet.


## Detached mesh construction — 2026-10-12

`SidewalkBuild` now owns clipping, extrusion and merge resources outside the
scene. Each `step()` advances one phase: at most 2048 clipping iterator calls,
16 fragment extrusions, or 12000 vertices/36000 indices of merge copying.
`takeGeometry()` transfers completed output once. Cancellation releases staged
fragments, unfinished merge output, and completed output unless transferred.
Materials are deliberately absent so integration can resolve the active style
at commit. The caller must cancel stale jobs and preserve old visible geometry.

Root reviewed Luna's two-file contribution and strengthened ownership tests.
The original merge-cancellation test actually completed the merge before
cancelling; the corrected test asserts the merger is unfinished and checks
exactly one disposal per owned fragment plus destination. Transferred output
must survive job disposal; repeated disposal must be harmless. Five focused
suites pass 32 tests (sidewalk build/polygons, incremental merge, ground surfaces,
road elevation).

The repeatable `scripts/audit-sidewalk-build.mjs` compares every vertex attribute
and numeric triangle index against synchronous extrusion plus Three's merge.
All six mature-fixture runs match exactly. [CPU measurements](sidewalk-build-cost.json)
record seed87231/tick160000, fixture hash and Apple M2 Max: 182 updates,
29.55–36.12 ms total after two warmups, maximum step .80–1.31 ms; first cold
maximum 3.77 ms. No browser frame, GPU upload or visual acceptance is inferred.
Constructor allocation/schema validation in the merger, per-fragment extrusion,
initial clipping preparation and final fragment disposal remain synchronous
within their phases. Quotas are operation limits, not hard millisecond bounds.

The live renderer still uses existing slabs. Next integrate an independently
invalidated sidewalk owner: compare nodes plus ground road topology/open status,
not construction progress (which currently causes frequent road rebuilds).
Preserve old committed geometry until complete, cancel stale/replaced worlds,
resolve current style on commit, use compile-aware retirement, and count pending
sidewalk work in loading diagnostics and adaptive-quality gating. Then verify
normal scheduler loading and matched visual/motion views before accepting the
junction repair. No new browser screenshots in this checkpoint.


## Live staged integration — 2026-10-12

`CitySidewalks` is now wired into CityRenderer. It advances once per animation
frame, keeps committed geometry during a rebuild, and resolves the shared active
material only at commit. World reset cancels staged work and detaches prior
geometry; visible geometry retirement goes through the renderer's compile-aware
callback. The job's detached resources are released immediately. Topology
invalidation uses copied sorted ground-road endpoints/kinds/open status and
node coordinates; construction-progress snapshots do not restart it. Rail and
expressway changes do not affect ground sidewalks. Rendering diagnostics and
loading/adaptive-quality eligibility now include pending sidewalk work.
Open arterial boxes were removed from roadKit; construction presentation and
local painted edges remain. Authoritative simulation/access geometry is unchanged.

Root inspected Luna's owner implementation, integrated it, and added a material
failure regression preserving the prior visible mesh. Five focused suites pass
44 tests. Production build passes with the existing bundle-size warning;
`git diff --check` passes. No commit/push.

Browser: existing IAB tab9 / WebGPU, Apple M2 Max, 1099x1071, High detail,
50mm, DOF off. A stale HMR startup warning was present before fresh reload;
fresh reload reached the city. Imported isolated-ground-growth through normal
UI: seed87231/tick3066, clear weather/intensity0, paused day0 09:03,38 buildings.
Overview then two scroll-up pages produces the new close framing. The same
camera/time in [Miniature](screenshots/ground-depth/sidewalk-live-miniature.png)
and [Anime](screenshots/ground-depth/sidewalk-live-anime.png) shows joined
foreground sidewalk corners and clear carriageways. These are cross-style
matched captures, not registered A/B with the prior implementation. Diagnostics
show zero building/chunk/sidewalk queues; settled Anime rolling120-frame window
p95 10.1ms, max11.2ms. This is not transition or long-session acceptance.

Normal Review mature city UI imported the existing seed87231/tick242000 save,
850 buildings, paused day10 08:00. At initial observation the UI reported
848 queued buildings,2 chunks,1 sidewalk rebuild, CPU410.1ms. Later all queues
reached zero without manual draining. Settled rolling120-frame diagnostics show
p95 17.5ms/max22.2ms. The initial410ms cost is unresolved and not attributable
from aggregate metrics. [Mature diagnostics](screenshots/ground-depth/sidewalk-mature-diagnostics.png)
and [Anime Overview](screenshots/ground-depth/sidewalk-mature-anime.png) retain
city/roads/water. Mature weather was not captured; do not treat this as a matched
lighting benchmark. Console read returned no warnings/errors after reload.

Remaining: camera-motion/video and WebGL checks, continuous fast-growth
invalidation/starvation, bridge/crossing/doorway transitions and construction
replacement. Road junction disks and lane markings still leave visible seams
in the close Miniature view; this repair does not accept all road surfacing.
Mature loading latency remains open. The full concept-fidelity gate is not met.


## Fallback and continuous-growth review — 2026-10-12

The previous turn integrated the owner and verified normal loading; this turn
checks two missing conditions rather than inferring acceptance from paused views.
New IAB tab13 used `?webgl=1`, ordinary import of seed87231/tick3066,
clear weather0, paused day0 09:03,38 buildings,50mm,DOF off,High detail.
Viewport1280x720 differs from the WebGPU checkpoint, so these are not registered
backend A/B captures. Overview plus two upward scroll pages produced
[Anime](screenshots/ground-depth/sidewalk-webgl-anime.png), then the same
camera/time [Miniature](screenshots/ground-depth/sidewalk-webgl-miniature.png).
An orbit drag(570,390)->(650,320) produced the
[lower view](screenshots/ground-depth/sidewalk-webgl-orbit.png).
Sidewalk corners remain joined, without visible striped ground in these samples.
The lower view exposes the preexisting raised circular road-junction seams
especially clearly. Settled screenshots after movement are not video evidence
of temporal stability.

[WebGL diagnostics](screenshots/ground-depth/sidewalk-webgl-diagnostics.png)
confirm all three queues zero and WebGLBackend. Sample rolling120 frames:
frame p9543.5ms/max46.4ms, CPU p956.6ms; style preparation106 samples,
submit6965ms(largest196ms), await20352ms. This fails any claim of smooth
high-detail fallback performance or fast style switching. No warnings/errors
were returned by the tab console. Diagnostic tab13 was closed; tab9 retained.

A real-WASM offline scheduler harness now drives the production sidewalk owner
at an ideal30 renderframes/s, speed8, growth16 and500ms structural publish
opportunities for120 simulated wall seconds. It deliberately does not drain
queues at the end. Developing-world cancellations can postpone a commit for
roughly22 simulated seconds. See the reproducible script
`scripts/audit-sidewalk-growth.mjs` and
[growth report](sidewalk-growth-scheduling.json). Node CPU execution time is not
browser frame timing; worker cost/backpressure may reduce actual throughput.

Next scheduling intervention: replace unconditional restart on every topology
update with bounded coalescing or affected-region retention. Finishing an active
same-world snapshot and then building only the latest queued snapshot is the
smallest candidate: memory must remain one active build plus one latest input;
world replacement/reset must still cancel all work, and committed state must
advance monotonically. This permits bounded presentation lag, which must be
measured and checked against new road openings/removals. Do not increase quotas
or publish partially clipped meshes to hide starvation. Re-run this exact
fixture and compare commit gaps before accepting the change. No production
scheduler policy changed in this review.

Root independently reran the corrected harness and all scheduling result fields
match exactly:19200 ticks,153 relevant snapshots,139 replacements,13 commits,
652 continuously pending frames(21.733s),final pending with an older committed
signature. Root review corrected initial harness resource leaks and a one-tick
floating-accumulator loss before retaining the final report.


## Bounded same-world coalescing — 2026-10-12

Evidence from the previous turn rejected unconditional active-job cancellation.
CitySidewalks now preserves one active build and one latest copied pending input;
newer same-world snapshots replace only that pending input. Returning to the
active signature drops the unnecessary queue. Completion commits one whole mesh
(including an empty result), then starts but does not advance the next job in
that frame. World reset cancels both. Material lookup, geometry ownership,
compile-aware retirement and per-frame geometry quotas are unchanged.

This deliberately permits presentation lag. It improves on retaining a much
older committed mesh while repeatedly cancelling work, but does not promise
instant agreement with new road openings/removals. No FIFO history accumulates.
Alternative considered: rebuild only affected regions. That could reduce lag
further but requires geometric dependency tracking; it is deferred until this
smaller correction is judged against actual growth and camera views. Increasing
quotas would risk frame cost and would not guarantee progress under churn.

Root reviewed Luna's implementation/tests and added geometry-bounds checks
against every committed input in the continuous-update test. The new queued-job
regression checks no second builder step occurs on the commit frame. Reset,
A-B-A requests, material failure and style-at-commit coverage remain. Five
focused suites pass34 tests. The first broader run exposed4 harness failures:
`ground-surfaces.test.ts` constructed a partial CityRenderer without the new
sidewalk owner. Root added its setWorld stub; no runtime null guard or weakened
ground assertions were introduced. Production build passes with the existing
large-bundle warning; diff whitespace checks pass.

The updated offline harness reads committedSignature rather than incorrectly
labelling a completed older input as the latest request. It also counts actual
job identity replacement separately from coalesced requests. The
[same fast-growth fixture](sidewalk-growth-coalesced.json) now records76 commits
versus13, zero active replacements versus139, and142 coalesced requests.
The maximum committed input age is92 frames(3.067 simulated seconds at30fps).
Longest interval between commits is224frames(7.47s), including periods without
new relevant inputs; the old commit list's longest gap was660frames(22s).
The pending flag can stay true while commits occur (now33.57s longest interval),
so continuous pending duration is not a starvation metric after coalescing.
At120s a new snapshot is still pending and the final mesh differs from the
latest world. No manual draining was used to turn that into a passing result.
Tests separately verify convergence after updates stop.

Remaining acceptance: real-browser continuous growth/presentation lag, road
opening/removal transitions, lower-angle road seams, and current motion evidence.
The algorithm's exact geometry is unchanged; prior paused visual captures do
not verify this new scheduling policy under growth. No new screenshots this turn.


## Real-browser growth and remaining road seams — 2026-10-12

The prior turn changed scheduling with an offline witness. This turn reloaded
WebGPU after a stale HMR graphics-start warning, imported the same developing
fixture normally, selected growth16 and speed8, and let the ordinary worker and
render loops run. The visible city advanced from38 to441 buildings, with street
opening events and occupied construction visible. No job was manually drained.
While running, diagnostics showed pending sidewalks and neighborhood work;
after Pause all three queues reached zero. This proves growth/recovery in this
sample, not the age of each browser-side commit or continuous motion stability.

The performance result is **not acceptable**: running sampled rolling windows
reported frame p95197.3–241.9ms, CPU p95187.0–192.3ms. After pause/settling:
frame p9520.6ms,CPU p9510.2ms, maxframe22.6ms. The aggregate UI cannot isolate
which stage causes the growth hitch. Full-road rebuilds on progress are a source
candidate, not an established attribution. Profile named stages next; do not
increase sidewalk quotas or remove required effects without attribution.

Retained actual exported save `.tools/fidelity-review/sidewalk-live-growth.json`:
seed87231,tick18918,clearweather0,441 buildings, growth16. Export was taken
through the UI and retains engine-format bytes. [Capture metadata](sidewalk-live-growth.json)
records hash, lens50mm,DOFoff,High detail,Miniature,WebGPU,1099x1071 and sampled
metrics. [Running](screenshots/ground-depth/sidewalk-growth-running.png),
[settled](screenshots/ground-depth/sidewalk-growth-settled.png), and
[grown city at09:00 preview](screenshots/ground-depth/sidewalk-growth-daylight.png).
Final live clock00:55; daylight preview changes lighting only. Console returned
no warnings/errors after reload. Tab9 is retained paused on the grown city.

Root verified the delegated seam diagnosis against current source: all nodes
with >1 open at-grade road get a cylinder topy=.365, versus ordinary road top.30
and lane top.27. Arterial marks top~.3575, below that cylinder. Even straight
same-width continuations get an8.3m-radius pad, widening a16m road and exposing
a raised curved edge. The sidewalk clipper subtracts the same pad. Removing it
alone leaves a sidewalk cutout; removing both cuts without evaluating adjacent
road geometry can introduce overlap. Lowering it alone risks coplanar overlap.
A coordinated road-surface/junction/marking treatment is needed, with straight,
T,oblique,bridge and local-lane cases. The agent's proposed union of road mouths
is a starting hypothesis, not proof that every bend wedge is filled or overlap
eliminated. No junction geometry changed in this review.
