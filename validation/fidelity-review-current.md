# Current mature fidelity review — 2026-10-12

## Reproducible baseline

Checkpoint e87392b plus the contour-test correction; renderer unchanged. Rebuilt
WASM with `npm run wasm`. The older `.tools/waterfront-growth/fully-realized.json`
was rejected by current route validation (traveler 723 origin connector mismatch).
It was not repaired or overwritten.

`node scripts/generate-fidelity-review-city.mjs` now generates fresh seed 87231
through ordinary fixed steps at development rate 16. At tick 80000 all 1839 lots
and 564 roads are realized. Checkpoints every 10000 ticks round-trip exactly and
preserve movement packets. The final live engine and restored copy agree after
another 80 ticks. Binary hash and counts: `fidelity-review-generation.json`.
Local review saves are in ignored `.tools/fidelity-review/tick-*.json`; regenerate
rather than add obsolete save compatibility. This is single-seed evidence.

Imported tick-80000 normally in the browser. All four geometry queues reached
zero without manual draining. Held existing waterfront review preset, 50 mm,
High WebGPU, 1280x720, DOF off, paused live14:00, clear weather/intensity0.

- `screenshots/fidelity-review-current/ready-diagnostics.png`
- `screenshots/fidelity-review-current/anime-waterfront-14.png`
- `screenshots/fidelity-review-current/miniature-waterfront-14.png`

The same city/camera/time was retained during the style switch. These are new
baselines, not before/after improvement evidence. Paused water phase can differ
between captures. They do not establish motion or day-cycle acceptance. Ready
window reported frame p95 14.3ms, CPU p95 11.4ms, max20.8ms, but no full loading
profile/hardware performance claim is made from that rolling window.

## Ranked perceptual gaps

1. **Sparse ordinary street activity.** At full buildout the roads and promenade
   appear almost empty in this afternoon view. This is an experience deficiency,
   not just a surface-detail problem. Diagnose real trip demand first.
2. **Waterfront composition and structure.** Very broad transport decks, straight
   banks and repeated retaining edges dominate the framing. The concepts connect
   intimate streets, rail, vegetation and water with more varied silhouettes.
3. **Architecture/material variation.** Repeated flat walls, window rhythms and
   muted surfaces still read as generic procedural blocks. More tiny roof props
   will not close this larger-scale gap.
4. **Water/atmosphere/style distinction.** Pale sky reflection and weak surface
   variety dominate the river; Anime and Miniature remain too similar in this
   DOF-off view. Preserve earlier rejected water experiments as negative evidence;
   do not repeat gain-only tweaks without a new mechanism and motion review.

Do not pave vacant land or add fake buildings to mask unfinished development.
Source review confirms green unbuilt lots are intentional. Do not treat the
260-building development view as final-density evidence.

## Primary next workstream: purposeful daily activity

Root verified the source audit: the local traveler population currently comes
from persistent resident schedules and sparse delivery service. External
expressway traffic is separate. No general residual household/job trip demand
is implemented, although spec03 requires it. `MAX_TRAVELERS=2000` is not the
cause of the quiet afternoon. Delivery interval `TICKS_PER_DAY/7` is approximately
3h26m of simulated time, not a frequent neighborhood delivery service.

`node scripts/audit-mature-daily-activity.mjs` advances the fresh mature checkpoint
through one full day and samples every30 simulated minutes. Report:
`mature-daily-activity.json`. Population17655; persistent residents800. Local
travelers range0–486. Samples:10:00=454,12:00=192,14:00=27 (nextday30),15:00=3,
16:00=3,18:00=280,20:00=448. These counts exclude external expressway trips and
aboard passengers. Instantaneous counts are not trip totals or route success.
They establish schedule concentration, not a universally correct activity target.

Bounded first implementation should represent errands/visits from the residual
housing population: occupied household count minus named residents assigned to
that home. Count active aggregate participants once; do not use camera distance
or render randomness. Reserve a real active commercial destination, construct a
legal route from actual entrance to entrance, dwell, then return to the origin.
Validate both legs and cancellation/replanning for closure; never spawn a visual
walker merely to fill a shot. Use seeded schedule windows, bounded dispatch and
route work, fixed concurrency, persisted stable IDs/cursors and deterministic
save/resume. Keep resident/transit/delivery behavior intact. Work-derived demand
and broader activity categories remain part of the full spec, not implicitly
satisfied by a household-only first slice.

Acceptance: accounting conservation and no named-resident double count; legal
routes and entrance positions; bounded state/work; mid-trip and dwelling replay;
continued activity through ordinary afternoon hours in three seeds without
forcing uniformly busy nights. Repeat this matched browser view and watch real
motion at normal simulation pace in both styles. Reject if routes clip buildings,
queues grow unbounded, people teleport, or the added activity is imperceptible at
normal camera scale relative to its cost. Calibrate rates against the measured
day profile, not an arbitrary minimum count that tests merely enforce.

The Mac native-app inventory reported a locked screen; in-app browser inspection,
normal import and captures remained available. No user action was needed.

## Admission prototype review — 2026-10-12

`engine/src/neighborhood_demand.rs` is registered under `cfg(test)` only.
It does not yet create production trips. The helper considers one home per
call, advances a deterministic round-robin cursor, and examines at most eight
rotating commercial destinations. It proposes at most one trip, with a global
128-outing admission ceiling and a 09:00–21:00 window. These are initial bounded
prototype parameters, not calibrated activity targets.

Review corrected a native/WASM pointer-width bug: destination hashing used to
cast the 64-bit hash to `usize` before modulo. It now reduces in fixed-width
arithmetic before casting; destination rotation also handles the full cursor
range without pointer-width-dependent overflow. A regression demonstrates the
old 32-bit truncation would select a different offset. This arithmetic check
is not proof of whole-engine cross-target equivalence.

The caller contract explicitly keeps returning participants deducted from their
home's residual population. Destination reservations cover outbound and dwelling
participants, and must not be counted again as current visitors. Ineligible
homes now report zero destination checks rather than charging unperformed work.
The home scan still costs one check, tracked separately by its caller.

Verification: `npm run test:engine -- --release neighborhood_demand` passes
10 tests, including all possible sole-open destination positions for shortlist
sizes 1, 7, 8, 9, 16 and 100, multi-home fairness, residual accounting, capacity
limits and window boundaries. Existing unrelated Rust warnings remain.

Next integration must be one coherent saved lifecycle: admit only after both
entrance-to-entrance travel legs validate; reserve a participant and destination;
retain the outing during its dwell; return physically; release reservations on
arrival home. Persist the cursor and outing identities/phases. Protect home and
destination during turnover/redevelopment, including invisible dwell, and
subtract aggregate participants before assigning new persistent residents.
Admission must reuse bounded/indexed snapshots rather than rebuild full-city
arrays every tick. Acceptance remains the route, replay, three-seed daily-trace
and browser-motion criteria above. None of those production gates is established
by the pure helper tests.

## Production errand lifecycle candidate — 2026-10-12

Supersedes the test-only status above: `neighborhood_errands.rs` is connected to
ordinary scheduling every ten ticks and traveler arrival. Each represented
visitor deducts one residual household participant, walks through both entrance
connectors, dwells for 250–750 ticks, and follows a prevalidated return plan.
The saved ledger includes invisible dwell and returning participants. Existing
named residents are excluded from residual demand, and new resident assignment
reserves people who are away. Admission examines one raw building slot and at
most eight destination slots; the destination window shifts between complete
home scans so a home is not permanently paired with the same destinations.
The outing limit remains 128. Routing uses the existing whole-route solver, so
bounded candidate counts are not proof of a fixed routing-expansion budget.

Turnover, renovation, height redevelopment and new rear extensions protect
referenced premises. Admission excludes pending extensions. Road splitting
currently defers on saved return corridors until the outing finishes; future
remapping would avoid this growth restriction. Other travelers do not participate
in a universal visitor-capacity system: errand admission considers their current
destination demand, but only aggregate reservations have a hard ledger bound.

The first integrated trace kept all 128 outing slots busy and had route lengths
105–1235m, median522m. This was rejected as implausible for neighborhood errands
under the accelerated day clock. The revised admission caps a walking leg at
180m including its entrance connectors. It does not speed up people to fill the
scene. Rejected candidates no longer allocate an entity ID.

WASM fresh seed87231 still realizes all1839 lots and564 roads bytick80000 through
ordinary steps; every10000-tick checkpoint round-trips, and the final live/restored
80-tick continuation matches. See `errand-local-generation.json` (binary hash).
The full-day trace is `errand-local-daily-activity.json`. At15:00 local travelers
rise from3 baseline to21; at16:00 from3 to23; at14:00 from27 to45. Errands finish
by04:00 and remain absent through08:00. Earlier unconstrained-route evidence is
retained in `errand-review-generation.json` and `errand-daily-activity.json`.
Counts exclude onboard transit and expressway traffic; the separate generations
are not a frozen-state A/B experiment. Browser motion and three-seed acceptance
remain outstanding. Work/job-derived aggregate demand remains outstanding.

Activity schema5 requires explicit errand state; obsolete saves are rejected.
Eighteen regression fixtures were explicitly normalized as test assets, preserving
all other parsed fields (independently checked against HEAD); manifest/provenance
records document this. There is no runtime migration. Focused coverage exercises
actual walking, visits and returns with live/restored state comparison, plus
corrupt return-plan and orphan-ledger rejection.

Final verification for this candidate: native release suite **468 passed,
0 failed,54 ignored** (46.72s). The focused physical lifecycle/corrupt-save tests
all pass after choosing a genuinely local fixture road under the180m leg limit.
WASM build succeeds; `npm test -- tests/wasm.test.ts` reports **4 passed,1 failed**
with the movement-validity test hitting its unchanged5000ms timeout (57.68s total).
That test was previously timing-sensitive, but this run is still failing evidence,
not a green suite or proof of unchanged performance. `git diff --check` passes.

## Three-seed and browser follow-through — 2026-10-12

The generation script now accepts a uint32 seed after the output/report paths;
the daily audit reads seed and development rate from its actual input rather than
hard-coding87231. Current-WASM ordinary growth and live/restored continuation
checks also pass for seeds0 and5. Seed0 realizes1730/1730lots and530/530roads by
80000; seed5 has1999 buildings,2030/2062lots and545/558roads at80000, so do not
label all three worlds fully mature at that same tick. Seed5 reaches2062buildings
within the following day. Reports: `errand-seed0-generation.json`,
`errand-seed5-generation.json`, `errand-seed0-day.json`, `errand-seed5-day.json`.

At15:00 the three seeds87231/0/5 have21/74/250local travelers, of which18/18/15
are walking errands; dwelling participants are separate. Peak active outing
ledgers are34/32/20 respectively, below128. All three have empty errand ledgers
at04:00. These are behavioral observations, not target population counts and not
proof of visual liveliness. There is no before/after baseline for seeds0/5.

The isolated unchanged WASM movement-validity test passes in4.65s. A separate
seed5/rate60/72000-tick diagnostic takes4779.56ms in simulation and about43ms in
snapshot/save/movement extraction; a mirror of123marker checks takes0.352ms.
This explains narrow timeout headroom; it does not turn the earlier full-file
failure into a pass or demonstrate an errand-specific performance regression.
Hardware identification through sysctl was denied; no named-hardware benchmark
claim is made. Diagnostic work came from a Luna audit; the root inspected the
actual test scope and prior failure output.

Browser: reloaded current engine, normally imported the fresh seed87231
80000-tick save, and let geometry schedule normally. It was running briefly
before pausing, so the final paired screenshots are at15:41, day3, not the
baseline14:00. Held waterfront preset,50mm,High WebGPU,1280x720,clear weather,
DOFoff; switched Miniature→Anime while retaining paused state and camera.
Ready UI reports1839buildings,30journeys (includes expressway),600draws and
10,398,158triangles, with all four queues0. Rolling120frame p95=12.1ms,CPU
p95=9.1ms,max13.0ms is only a window observation, not an end-to-end profile.
Screenshots are in `screenshots/errand-review/`.

The first20-second canvas recording was made at1× while neighborhood geometry
was still loading, deliberately retained as loading evidence:
`recordings/errand-review/miniature-loading-waterfront.webm`. Recording completion
and download were verified, but the video has not yet been visually reviewed
frame-by-frame; it is not motion acceptance. Loading screenshots show large
temporary blank building areas; queues later reachedzero without manual drains.
The ready wide view still looks sparse, dominated by broad transport decks and
pale sky reflections. More purposeful trips alone have not closed the concept
fidelity gap. Next inspect close neighborhood movement and normal import latency
before deciding whether to tune this workstream or move to the larger structural
and material gaps. No artistic acceptance is claimed.

## Multidimensional gap assessment — 2026-10-12

Reassessed the latest paired errand-review waterfront captures against both
locked concept boards, the supplied street/rail photographs and the metropolitan
waterways map. This supersedes the earlier priority order, not its observations.
The strongest next priority is urban form and composition: the fully realized
city still reads as transport decks and open water surrounded by repetitive
blocks, rather than connected, inhabited neighborhoods. Increasing trip counts
has not substantially changed that impression.

1. **Urban fabric and perceived density — critical.** Measure footprint coverage,
   street hierarchy, block/parcel sizes, setbacks and continuous frontage before
   changing proportions. Full lot realization is not visual-density acceptance.
   Metropolitan maps are qualitative references without a supplied scale/legend;
   do not infer terrain elevations or equate every green polygon with a park.
2. **Human-scale composition — critical.** Establish readable shop/crossing/rail
   or waterfront subjects with foreground, middle distance and backdrop. The held
   waterfront preset is useful geometry evidence, not an accepted Watch shot.
3. **Architecture and identity — high.** Improve building families, frontage and
   roof/massing variation; local Japanese details should reinforce those forms.
   Additional texture generation alone will not correct repetitive silhouettes.
4. **Landscape, water and lighting — high.** Current banks are hard and uniform;
   pale cloud reflections dominate water. The concepts rely on layered planting,
   varied edges, material response and warm/cool light separation. Test mechanisms
   in motion rather than repeat previously rejected reflection-gain tweaks.
5. **Camera and styles — high.** Thin-lens depth blur exists, with provisional
   coverage correction and reviewed still/selected motion evidence. It is not a
   tilted focus plane. Latest waterfront pairs have DOF disabled and cannot judge
   blur quality; they do reveal weak Anime/Miniature distinction without blur.
6. **Living behavior — high.** Errands improve afternoon continuity, but broader
   work-derived demand and convincing visible activity concentration remain.
   Counts, actual routes and perceptibility must be assessed separately.
7. **Loading and motion — high acceptance risk.** Normal import shows temporary
   blank building areas. A settled frame window is not startup/growth/motion
   performance acceptance. Audio quality was not re-audited in this assessment.

Recommended bounded next slice: diagnose and improve one normally generated
neighborhood spanning a local street, crossing and waterfront, using reusable
rules. Compare equal camera/time/style settings with DOF off and on, then review
normal-speed motion and multiple seeds. Do not expand map scope, redesign the
engine, or add broad asset batches before evaluating the structural result.

## Urban fabric baseline — 2026-10-12

Ran the existing production-kit fabric audit on current-WASM snapshots restored
from the three ordinarily generated tick80000 saves. No runtime/world edits.
Results and snapshot/WASM/script hashes: `urban-form-baseline.json`.

| Seed | Compact projected coverage | Station projected coverage | Station street union | Whole-map projected coverage |
| --- | --- | --- | --- | --- |
| 87231 | 49.14% | 32.91% | 35.82% | 30.87% |
| 0 | 49.62% | 34.12% | 35.36% | 29.92% |
| 5 | 47.35% | 28.98% | 37.90% | 29.74% |

Seeds87231/0 are fully realized; seed5 is still developing at this tick. Compact
coverage is within the existing40–50% target; station coverage is below38–48%
and station street share exceeds20–30%. This narrows the next investigation to
station/infrastructure relationships rather than universally shrinking buildings'
setbacks or increasing density. This is diagnostic evidence, not full acceptance.

Separately unioned station arterial areas are20.26/21.47/24.06%; local streets
17.49/15.88/16.27%. There are no bridge rectangles inside these station crops.
Kinds overlap at junctions, so these percentages must not be added as total street
area. Station-centered building samples are all open (143/122/114 respectively).

Root independently checked the Luna method audit against source: coverage projects
all building kits (including boundary geometry and any non-open stages), while
building/floor statistics select open centers; compact crop selection sums centered
parcel envelopes. Footprint quantiles estimate an initial party-wall rectangle,
before lowrise massing, and are not exact final footprint measurements. Net-of-road
coverage is missing. Road union uses straight rectangles and excludes rail and
expressway;18.8m bridge width matches the current renderer's width+2.8 deck, so the
audit's bridge number is not itself an unexplained error. It does not resolve the
separate bridge reservation/specification mismatch.

Next: correct/clarify audit denominators and footprint proxy, inspect the station
crop visually and identify whether arterial spacing or local partitioning creates
excess street area before proposing a bounded generator change. Preserve compact
fabric that already meets the gross target. No new acceptance targets, road-width
changes or blanket density multiplier are justified by this run.

### Street-adjusted denominator correction

The production-kit audit now reports a separate diagnostic based on the same
raster: (building union outside open streets) / (crop outside open streets).
Street rectangles are unioned before subtraction. River, protected land, rail and
expressway remain in this denominator: this is explicitly not net developable
coverage. The fabric audit consumes that same street mask rather than maintaining
a second road rasterizer. Gross metrics and existing target ranges are unchanged.

`urban-form-street-adjusted.json` records compact versus station percentages:
seed87231 **63.26 / 51.22**, seed0 **62.72 / 52.72**, seed5 **63.81 / 46.59**.
There is still a station gap after road subtraction, but this alone does not
separate rail/protected space from setbacks or missing usable frontage. Do not
introduce a net-coverage target from these three observations.

Seven focused raster tests pass, including a building overlapping duplicated road
geometry, absent road data and a fully excluded domain returning null. TypeScript
checks pass. Luna implemented the helper and tests; root inspected the diff,
corrected a hard-coded grid-size description, integrated the fabric report and
independently reran checks. Snapshot/method proxy limitations remain explicit.
No generator or renderer behavior changed. The next decision still needs a visual
station-layout review, particularly arterial spacing, rail frontage and local cuts.

### Station visual review and held-camera correction

The existing DEV View station invoked entity follow and landed behind a rooftop
in the mature seed87231 city, hiding the station. Saved `station-fabric/before-review.jpg`
under screenshots records the failure at15:41. Added `reviewStation(id)` to select
a deterministic lens-fitted55m neighborhood pose at platform center/height, with
visibility and destination collision checks before state mutation. Only the DEV
review control uses this method; ordinary Follow remains unchanged.

HMR restarted the world. Normally reimported the current tick80000 save while
paused, waited for all four rendering queues to reach0, and captured at14:00,
seed87231/day3,1839buildings,Anime,High WebGPU,50mm,DOFoff,1280x720. This is not a
matched-time before/after artistic comparison. `screenshots/station-fabric/` holds
ready diagnostics and the unobstructed held view. Visiting waterfront then station
again produced identical screenshot SHA256
`19d76e77035988af13b6b5b31a430472e1cd1edf119e8dcfb85f5c4e0d115952`.

The visible district is substantially denser than the earlier waterfront framing
suggested. Repetitive facade/roof families, plain rail margins and weak visible
station entrance/frontage detail are now more specific gaps. Arterial corridors
remain prominent, but blanket densification is not justified. The saved overhead
`seed-87231-parcel-plan.png` is a250m diagnostic plot of parcel envelopes (blue
by floor band), street rectangles(gray),rail(orange),expressway overlay(purple),
station(yellow); it is not a rendered footprint map or a browser image.

Next prioritize a bounded station frontage/rail-edge treatment after inspecting
existing platform/access geometry, preserving legal passenger routes and shared
scale. Keep morphology targets open; the new camera does not resolve their gaps.
Do not turn this diagnostic framing into automatic Watch behavior without review.

Root inspected Luna's implementation and corrected the occlusion regression's
baseline ray to originate at the actual synthetic station.61 focused camera tests
passed before that test-only correction; all5 station tests pass after it.
TypeScript and diff checks pass. Tests cover repeated poses, platform height,
occluding buildings, infeasible and missing stations. Full artistic and motion
acceptance remain outstanding.

### Rejected station canopy experiment — 2026-10-12

Tested a shallow pitched roof, standing seams, muted fascia stripe and outer
platform guardrail within the existing station kit. Root corrected floating
supports and initial footprint expansion before browser review. Platform, queue
and stair geometry remained unchanged;17 focused rail/station/train tests and
TypeScript/build passed for the candidate (existing bundle-size warning remains).

Normally imported the same paused seed87231/tick80000 city, waited for all four
queues0, then used the same held station50mm/Anime/HighWebGPU/14:00/DOFoff view.
The saved `screenshots/station-fabric/canopy-candidate.jpg` shows too little
perceptual improvement over `held-station-anime.jpg` to justify the extra detail.
Ready diagnostics reported698draws/9,979,506triangles versus the prior
686/9,962,034; these are scene counters, not an isolated performance benchmark.

Rejected and reverted only this experiment's station-kit and test edits. The
patch is retained as `station-canopy-rejected.patch` for negative evidence, not
as a pending implementation. The earlier held station camera fix remains.
Do not repeat seam/color/pitch tweaks as the next intervention. Inspect station
entrance/access composition and actual canopy visibility first; choose an
intervention readable at neighborhood scale. No artistic acceptance claimed.

### Access audit and goal reconciliation — 2026-10-12

`station-access-envelope-audit.json` checks platform and stair rectangles against
rotated building envelopes in seeds 87231, 0 and 5 at tick80000. No overlaps were
found. City-station rectangles remain inside the world, but gateway stairs extend
outside its boundary in all three seeds. Seed5 is still developing. This is a
plan-view envelope check, not proof of rendered mesh, height or walking clearance.
The gateway boundary defect remains unresolved and must be checked before acceptance.

The updated `living-city-goal.md` incorporates the measured fabric distinction,
camera framing correction, rejected canopy experiment and access audit. A covered
entrance helper and test exist as an unintegrated candidate; they are not accepted
production behavior. Independent physical-support, clearance and matched visual
review are still required. Keep the next intervention bounded and return to the
overall ranked fidelity gaps rather than indefinitely detailing one station.

### Second station experiment rejected; reassess composition — 2026-10-12

Integrated the covered stair entrance candidate for review. Root corrected its
floating side columns to bear on actual treads over the stringers and meet the
roof underside. Headroom checks now cast upward from discrete tread elevations,
including both sides of a 1.8m aisle. Twelve focused station/rail/train checks and
the production build passed; the existing bundle-size warning remains.

The browser graphics-start warning cleared before Retry could be clicked. Imported
the paused seed87231/tick80000 checkpoint normally and waited for all four queues
to reach zero. Matched held station view: day3/14:00, Anime, High WebGPU, 50mm,
DOFoff, 1280x720. Captures: `screenshots/station-fabric/entry-candidate.jpg` and
`entry-diagnostics.jpg`. Ready counters were700 draws/9,962,382 triangles; this
is not an isolated performance benchmark. No cross-style or motion acceptance
was attempted after the candidate failed this first perceptual gate.

The cover obscures the previously legible stair flight without materially
strengthening station identity at neighborhood scale. Rejected the candidate,
removed its helper/test and reverted its integration. The complete experiment
is preserved in `station-entry-rejected.patch`, not production code. The earlier
statement that the helper exists as an unintegrated candidate is superseded.

Two bounded station detail attempts have now failed to improve the whole scene.
Stop station embellishment and reassess the district composition. The held view
shows repetitive apartment/office vertical window bands and flat roof profiles,
limited hierarchy around the station, and a foreground expressway dominating
the local street. Next inspect the actual distribution of building families and
ground-floor frontage in this crop against the locked concepts before choosing
one reusable architectural or layout intervention. Do not add a third roof/signage
variant, blanket density multiplier or extra texture batch. Keep the gateway
boundary defect tracked separately; it is not fixed by either visual experiment.

### Architecture distribution after station reassessment — 2026-10-12

Added `scripts/audit-station-architecture.mjs` and generated
`station-architecture-mix.json` from the existing three tick80000 snapshots.
The audit imports production envelope, massing and palette helpers, pins snapshot
and bundled-runtime hashes, and records every selected building. It counts open
building centers in 250m and 150m station squares; these are not camera frusta.
Repeated execution produced an identical report, and all six crops reconcile
kind/floor/family/palette totals with unique building IDs.

| Seed | Open buildings, 250m | Active shops/cafes | Apartment midrise family | Office midrise family | Neutral palette |
| --- | ---: | ---: | ---: | ---: | ---: |
| 87231 | 143 | 40 | 36 walkups | 30 setback offices | 88 |
| 0 | 122 | 33 | 34 walkups | 20 setback offices | 82 |
| 5 | 114 | 32 | 33 walkups | 16 setback offices | 74 |

Seed5 remains developing. The 150m crops contain43/52/31 open buildings and
14/16/8 active shops/cafes. All sampled open buildings have access records, which
does not prove visible doors, unobstructed paths or continuous frontage. Counts
do not measure projected visual area or footfall. Office family totals combine
its existing slab and podium variants; palette family counts omit within-neutral
swatch differences. Do not use these observations as diversity quotas.

The generator supplies several uses and a broad height distribution. However,
all103 eligible apartment midrises use the same side-core/two-floor-setback
composition; the renderer then repeats the same fenestration and alternating
balcony rule. This supports a bounded architectural-family intervention before
changing land-use weights, density or material counts. Little Tokyo's locked
board emphasizes quieter wall areas, varied roof profiles, domestic buildings
and strong street-level connections; current repeated detail does not establish
that richness merely because geometric counts are high.

Next candidate: one coherent alternative apartment family with a materially
different silhouette and balcony rhythm, selected deterministically from existing
appearance data. Preserve authoritative uses, floors, parcel bounds and entrance
positions. Carry the same geometry through construction/renovation, check support
and clearance, and compare the same held neighborhood in both styles. Accept only
if the whole street reads less repetitive without becoming random or increasing
unexplained gaps; reject if the difference is only visible in an isolated specimen.
No implementation or artistic acceptance is claimed by this diagnostic.

Separate follow-up found during source inspection:4–8-floor `home` buildings
fall through the generic facade path, whose explicit centered entrance is enabled
only for2–3-floor apartments/offices. Verify rendered doorway alignment for these
homes against their access endpoints; the presence of an access record alone is
insufficient evidence. Do not silently relabel their simulation use to fix visuals.

### Terrace apartment candidate integrated — 2026-10-12

Luna implemented and root inspected a deterministic minority family for4–11-floor
apartments: full-width lower body, rear-aligned top floor and broad front terrace.
Root added solid continuous front balcony bands above ground, retained the centered
entrance, and suppressed terrace rails where the upper rear wall is aligned.
Construction and renovation consume the shared mass layout. Uses, floor counts,
simulation access and authoritative parcel dimensions are unchanged.

Root expanded existing narrow-parcel, construction-glazing and doorway checks with
an explicit new-family seed.25 focused massing/kit/cladding/core/tower tests pass;
an additional physical front-guard ray test passes in both styles (the complete
midrise-kit file passed10 tests before its final material-specific assertion;
the updated guard test was then rerun independently). TypeScript and production
build pass with the existing bundle-size warning. Do not report this as a full
suite run. `station-architecture-candidate.json` records9/12/21 affected buildings
in the250m station crops and3/5/6 in the150m crops for seeds87231/0/5.

Normally imported the paused seed87231/tick80000 city, waited for all four queues0,
and captured held50mm, day3/14:00, HighWebGPU, DOFoff,1280x720. Anime before/after
uses `held-station-anime.jpg` and `apartment-anime.jpg`; candidate Miniature is
`apartment-miniature.jpg`, all in `screenshots/station-fabric/`. The latter has no
matched pre-change Miniature capture. Ready Anime scene counters:686draws and
9,919,042triangles; this is not an isolated benchmark or a hardware-performance
claim. Broader terraces and continuous balcony bands visibly break up the previous
side-core repetition, so retain the candidate for further review rather than
rejecting it at the first neighborhood gate. Material/style distinction remains
weak and this does not resolve the overall concept-fidelity gap.

Pending: visual checks on the other two seeds, construction in motion, DOF on/off,
dawn/dusk/night and user artistic review. Also inspect balcony ends and terrace
access at closer range before accepting physical realism. No new asset batch or
land-use tuning is justified by this candidate. Goal remains incomplete.

### Apartment physical-detail correction — 2026-10-12

Source review found the initial continuous balcony candidate inherited individual
window-balcony slab elevations: the floor sat roughly1m above the apartment floor.
Its ends were also open. Replaced the repeated bay slabs with one continuous slab
per upper front storey, embedded into the wall at floor level, bounded by parcel
clearance. Solid front and return guards now close its exposed edges; tall glazing
represents balcony doors. The broad top terrace now has front and return guards
instead of the old short, interrupted decorative rails. Ground entrance and rear
facade remain outside this treatment.

Independent ray checks verify slab support across multiple bays at the storey
floor, closed balcony ends, three terrace edges and unobstructed ground-door
presentation in both styles.22 focused massing/kit/cladding/core tests passed;
the guard test was rerun after extending its terrace assertions and passed.
TypeScript/build pass with the existing bundle warning; diff check clean.

The earlier `apartment-anime.jpg`/`apartment-miniature.jpg` captures predate these
physical corrections. Fresh browser review is required before claiming their
appearance carries forward. Next capture corrected geometry, then continue the
remaining seed/motion/lighting matrix; no additional family changes in parallel.

### Corrected-family browser review — 2026-10-12

Normally imported seed87231/tick80000 while paused and allowed all four queues to
reach0. Clear weather, day3/14:00, HighWebGPU,50mm,1280x720. The transient graphics
startup warning cleared without a retry. Reviewed generated apartment1197 using
Frame review building, then returned to the held station pose. Captured current
Miniature and Anime with DOFoff, and Anime lighting previews at06:00,18:00,21:00
plus21:00 with DOFon. Restored live14:00 and DOFoff after review.

Files under `screenshots/station-fabric/`: `apartment-corrected-detail.jpg`,
`apartment-corrected-miniature.jpg`, `apartment-corrected-anime.jpg`,
`apartment-corrected-dawn.jpg`, `apartment-corrected-dusk.jpg`,
`apartment-corrected-night.jpg`, `apartment-corrected-night-dof.jpg`.
These supersede the earlier apartment images for current geometry. They are
lighting previews of one paused city, not actual daily-activity checkpoints.

The close view supports the balcony-floor and end-guard corrections visually.
It exposes a remaining material defect: the broad terrace uses the red wall
material from the underlying mass instead of a distinct horizontal walking
surface. Add a bounded horizontal terrace finish using an existing material;
do not commission new textures. The neighborhood silhouette variation survives
the correction, but repetitive window lighting and weak style/material character
still fall short of the locked concepts. Anime night DOF shows no obvious halo
in this still; motion stability is unproven. No full visual acceptance claimed.

Next: correct terrace surface, then finish other-seed and motion review. The
paused browser remains on the live-lighting station view. No renderer or simulation
code changed during this review turn.

### Terrace finish and second-seed review — 2026-10-12

Added a thin existing-concrete finish over only the exposed front terrace strip.
It rests on the lower mass, remains inside the envelope and appears only once
the upper tier is installed. No textures or new materials added. Ray checks verify
the surface material/elevation; a construction regression verifies its absence
before completion and presence at finished/open stages.12 kit/cladding tests
passed before the added construction regression; the complete cladding file then
passed3 tests. TypeScript and production build pass (existing bundle warning).

Normally imported seed0/tick80000 paused,1730buildings/147journeys,day3/14:00,
clear weather,50mm,HighWebGPU,DOFoff,1280x720. Waited for all four queues0. Held
station review and generated building734 detail show the neutral terrace finish
and visible architectural variation without obvious overlap in those views.
Captured both Anime and Miniature. These are another-seed inspection, not a matched
pre-change A/B for seed0. Ready Anime counters702draws/10,132,820triangles are scene
diagnostics, not a performance benchmark. Files in `screenshots/station-fabric/`:
`apartment-seed0-diagnostics.jpg`, `apartment-seed0-anime.jpg`,
`apartment-seed0-terrace-detail.jpg`, `apartment-seed0-miniature.jpg`.

Retain this bounded family while finishing review. Roof/fenestration repetition
and weak style distinction remain visible; the new family is not full concept
acceptance. Seed5 developing-state review and motion remain next. Browser is on
paused seed0 station view,Miniature,14:00,DOFoff. Earlier seed87231 corrected images
predate the neutral terrace finish and must not be presented as its final result.

### Goal reconciliation — 2026-10-13

Updated `living-city-goal.md` to replace the superseded station-detail next step
with the architecture audit, both rejected station experiments, the retained
terrace-apartment candidate and its remaining review gates. The full living-city
objective, locked concepts, architectural baseline and spec08 acceptance remain
unchanged. The next action is developing seed5 and motion/final-geometry review,
followed by a whole-experience priority reassessment—not another speculative asset
or geometry batch. Distinguish lighting previews from daily behavior, stills from
motion, scene counters from performance, and focused tests from full acceptance.
No new visual or runtime acceptance is established by this documentation update.

### Developing third-seed review — 2026-10-13

Imported seed5/tick80000 through the normal UI into a fresh IAB tab after the old
review tab was gone. Paused day3/14:00, clear, 50mm, High WebGPU, DOFoff,1280x720;
1999 buildings,315 journeys,8 works in progress. Despite the maturity label, this
is still a developing city. Waited for all four render queues to reach0 without
forcing work. Miniature ready counters:721 draws/10,896,921 triangles/38 textures.
Captured `screenshots/station-fabric/apartment-seed5-miniature.jpg` and
`apartment-seed5-anime.jpg` at the same held station pose/time. Terrace/balcony
variation is visible and no obvious new overlaps appear in these views, but
highway decks, repeated office grids and uniform ground treatment still dominate.
The styles remain too similar with DOF disabled. No pre-change seed5 A/B exists.

Loading diagnostics included a2553ms longest frame; later settled windows do not
cancel that stall. Anime style preparation reported1187ms submission and7564ms
await. These are observed application diagnostics, not a named-hardware benchmark.

Resumed at1x simulation (existing4x city-growth setting), recorded the canvas for
20seconds using the DEV recorder, then paused at15:25. The city reached2034
buildings/243 journeys/6 active works by pause. Growth enqueued neighborhood batches
and a later rolling window reported100.2ms longest frame. Recording saved as
`recordings/apartment-seed5-activity.webm`; capture is held-camera Anime/DOFoff and
contains no audio. It is captured evidence, not yet a completed motion review:
local ffmpeg/ffprobe are unavailable, so attempted frame extraction did not run.
Camera motion, DOFon occlusion stability and construction transitions remain
unverified. Do not report the recording itself as a passing motion test.

Retain the family provisionally. Next inspect recorded motion using an available
media reader, complete final-geometry seed87231 and moving-camera checks, then
rerank the wider fidelity gaps instead of adding another family immediately.

### Recorded-frame inspection — 2026-10-13

Decoded the existing seed5 activity WebM using the browser's video decoder in a
local `.tools/motion-review.html` contact-sheet viewer; no dependency installation
was necessary. Inspected seven frames at0.1,3,6,9,12,15,18seconds and saved
`screenshots/station-fabric/apartment-seed5-motion-samples.jpg`. The initial viewer
sample at exactly0 was blank before decoding; seeking to0.1 corrected the viewer.
This was not evidence of a blank city frame.

The train changes position along the rail corridor, and visible terrace buildings
retain their silhouette and finish across sampled times. No obvious large geometry
disappearance is visible in these samples. This is a sampled temporal check, not
continuous playback acceptance: brief flicker, smoothness, construction transitions,
moving-camera occlusions and DOF stability remain unproven. No audio was recorded.

Source inspection confirms distinct Toon/Standard material variants in both the
ordinary and batched building paths; the weak observed style distinction is not
explained by a missing second material implementation. They share base colors,
emission rules and much of the glass response. Do not assume a switching bug or
increase contrast globally without a controlled comparison against the concepts.
The remaining appearance gap spans architectural repetition, ground/frontage
integration and style response. Keep the apartment candidate provisional and
complete the moving-camera/final-geometry check before another variant.

### Final-family orbit check and workstream decision — 2026-10-13

Reimported seed87231/tick80000 normally after the review tab had reset to a founding
world. All four queues reached0. Captured final neutral-terrace geometry in Anime
and Miniature with DOFoff at the held station pose:50mm,14:00,clear,HighWebGPU,
1280x720. Files:`apartment-final-87231-anime.jpg` and
`apartment-final-87231-miniature.jpg` in screenshots/station-fabric. These supersede
the pre-finish seed87231 apartment screenshots for current appearance.

With simulation still paused, enabled Miniature DOF and recorded three70px leftward
canvas drags from the same station view during the first five seconds of a20second
capture. Kept the recorder panel mounted: closing it cancels the recording.
Saved `recordings/apartment-87231-camera-dof.webm` and browser-decoded samples at
0.1/1/2/3/4/5/18seconds in `apartment-87231-camera-dof-samples.jpg`.
The camera visibly orbits and changes building occlusions; sampled terrace edges
remain attached with no obvious large halos or missing masses. Wide-frame DOF is
subtle. Samples cannot rule out short flicker, and this is not continuous playback
or construction-transition acceptance. A rolling diagnostic during orbit reported
a122.7ms longest frame despite queues0; recording overhead is not isolated.

Decision: retain the bounded apartment family as an integrated improvement with
limited acceptance evidence, and stop adding apartment detail for now. Three-seed
still review, physical/construction regression tests, lighting previews and sampled
activity/orbit checks support retention; they do not meet the complete release
matrix or user artistic acceptance. Broadening this into more families immediately
would bypass the unresolved perceptual diagnosis.

Next primary decision should compare current matched DOFoff views directly with
both locked concepts and isolate surface/lighting response from architecture and
ground/frontage integration. Repeated roof/window grids, broad transport surfaces
and weak style distinction remain higher-level gaps than additional balcony props.
Preserve outstanding continuous-motion, true day-cycle, audio, narrow-viewport,
performance and boundary/access checks in the delivery backlog.

### Concept comparison and tall-home frontage correction — 2026-10-13

Re-read both locked growth boards and the dense-city concept pair. Compared with
current DOFoff station frames, the concepts have layered building scales, readable
occupied street edges, landscaped infrastructure margins and stronger material/
light separation. Current repetition and broad ground/transport surfaces persist
at neighborhood scale. Global blur or contrast alone cannot address these gaps.

Source tracing confirmed a specific frontage defect previously flagged by the
architecture audit: generic homes of4+ floors never called the entrance helper.
The three current raw review snapshots contain57/52/46 open non-anchor homes of
that height (seeds87231/0/5),155total. Added the existing centered entry treatment
to that branch and reserved its opening in the ground-floor window layout. No
land-use, capacity, simulation geometry or asset changes. This is a bounded
correctness fix supporting believable frontage, not a claim to resolve the larger
concept gap.

A ray regression failed against the old wall-only facade, then passed for4/5/8
floors in both styles and open/renovating stages after the correction. Open doors
must be the first ray hit and remain parcel-contained; renovation retains the
door behind temporary construction fencing rather than asserting an unobstructed
renovating entrance. All23 tests across building-massing,midrise-kit and
midrise-cladding pass. TypeScript passes. Browser appearance and construction
transition review remain pending; no artistic acceptance claimed.

### Tall-home browser verification — 2026-10-13

Normally imported seed87231/tick80000 paused; waited for all four render queues0.
Reviewed generated home457 (Riverside House,5floors,9.489365x14.509381m), whose
saved access joins road13. Frame review building initially placed the camera over
the rear roof; manually orbited to the street side and lowered the angle, then
zoomed out to clear foreground buildings. Captured the identical resulting pose
in Miniature and Anime,50mm,14:00,clear,HighWebGPU,DOFoff,1280x720:
`screenshots/station-fabric/tall-home-457-miniature.jpg` and
`tall-home-457-anime.jpg`. Camera actions were manual; there is no matched pre-fix
browser capture, so these establish current presentation rather than A/B gain.

The centered blue door is visible on the five-storey facade immediately right of
the red gabled home. It aligns with the existing centered step and paved street
connection. No overlapping ground window masks it in either style. Retain the
correction. This view does not establish walking-height traversal or every parcel
variant; the focused geometry tests provide complementary, bounded evidence.
Production build also passed after the correction, with the existing bundle-size
warning. No new source changes during this visual review.

The lower street-facing view exposes a larger ongoing gap: uniform facade bays,
sparse ground-floor identity and bare paved margins. The two styles remain close
in appearance. This is stronger evidence for reusable frontage/material treatment
than for more tall-building silhouettes or extra blur. The camera's automatic
building review did not itself expose the entrance; manual exploration was needed.
Browser retained paused on this frontage view in Anime with DOFoff.

### Shop-canopy frontage candidate — 2026-10-13

Commercial-frontage source review found an envelope error: low-rise shops passed
the front wall plane as `frontBoundary`, and canopy placement treated it as the
parcel boundary. Almost the entire canopy depth sat inside the solid building,
reducing its visible silhouette even when a front setback was available.

Added an explicit local parcel limit for the low-rise canopy. Its back embeds8cm
into the wall and its depth is clipped to the actual remaining frontage (4cm parcel
margin). The valance and its stripes now follow the exposed canopy edge. Existing
facade/window/door planes stay unchanged; anchor and legacy shop paths retain their
previous placement. Shared materials and geometry counts are unchanged. No texture
batch, simulation modification or wider parcel footprint is introduced.

New downward-ray regression failed on the original kit (hit the step at0.46m
instead of a canopy), then passed for shops/cafes in both styles at two parcel
depths. Existing low-rise containment/access tests also pass:15 tests across the
kit and massing files. This establishes exposed geometry and parcel containment,
not artistic acceptance. Next inspect generated shops and the neighborhood before
retaining the visual candidate; reject if projection harms roof/frontage coherence
or does not improve street identity at useful viewing distance.


## Consolidated learnings and goal update — 2026-10-13

This reconciliation supersedes earlier next-action notes, not their historical
results. The overall goal remains the complete living city; concept fidelity is
an essential acceptance dimension alongside behavior, exploration and delivery.

| Dimension | Evidence and learning | Consequence for work selection |
| --- | --- | --- |
| Urban form / overhead maps | Compact gross coverage is already 47.35–49.62%; station crops have lower coverage and substantial street union. Held views change the apparent density. | Preserve compact fabric; distinguish camera composition, screen area and land coverage. Unscaled maps are qualitative, not surveyed terrain. |
| Reference architecture | The audit found one walkup family across 103 eligible apartment midrises. Terrace variation is now retained with bounded evidence. | Stop balcony embellishment; address repetitive roof/window patterns and ground-floor identity where visible. |
| Frontage and correctness | Tall-home doors were absent; corrected and viewed on home457 in both styles. Shop canopies were largely inside the wall. | Correct functional visual relationships before commissioning assets. Keep traversal and construction checks distinct from a still view. |
| Look and feel | Current styles remain too close with DOF off despite distinct material implementations. | Compare surface/light response with locked concepts; do not assume a missing style switch or apply global contrast blindly. |
| Camera and lens | Thin-lens DOF exists; wide-view effect is subtle and manual framing was needed to expose an entrance. | Improve subject, perspective and depth separation; test moving occlusions. Blur is not a geometry repair. |
| Landscape / infrastructure | Broad transport and bare ground margins dominate some views; gateway stairs exceed the world bounds. | Integrate streets, banks, paths and planting; track boundary correctness independently of artistic improvement. |
| Living activity / audio | Errand lifecycles and sampled train movement provide bounded behavioral evidence. Lighting previews and silent recordings leave major gaps. | Review real daily behavior, destination activity and audio continuity; do not raise crowd counts for screenshots. |
| Browser performance | Loading, growth and orbit diagnostics include substantial stalls; recording overhead is not isolated. | Profile normal operation on named hardware; settled counters and queues0 do not establish end-to-end performance. |

The shop-canopy A/B uses the controlled architecture viewer, identical inspected
low-rise pose and daylight, 1280x720, in Anime and Miniature. Evidence is stored in
`screenshots/station-fabric/shop-canopy-{baseline,candidate}-{anime,miniature}.jpg`.
The candidate has a visible canopy top instead of mostly a thin wall strip; its
entrance remains readable. This is a modest specimen-level benefit. The candidate
source was restored after baseline capture. No generated-neighborhood, construction
transition or full release acceptance is added by this comparison.

Updated `living-city-goal.md` to remove stale apartment and entrance review tasks,
retain their evidence limits, and prioritize completion of canopy review followed
by measured frontage/ground integration. Before another intervention, establish a
matched neighborhood baseline and choose the largest visible deficiency. One
primary workstream, bounded cost, explicit acceptance/abandon criteria and a
reassessment after two unsuccessful attempts remain mandatory. Broad asset batches,
map expansion and engine rewrites must not substitute for the current gap.


### Canopy construction specimen review — 2026-10-13

Reviewed the current production kit through the architecture viewer at its fixed
Inspect low-rise street pose, controlled daylight and 1280x720. Anime construction
at50% shows the structural frame without a premature canopy. At95% the canopy is
attached to the completed facade behind temporary worksite rails. Miniature
renovation preserves the canopy and entrance with the existing worksite treatment.
Saved `screenshots/station-fabric/shop-canopy-construction-50.jpg`,
`shop-canopy-construction-95.jpg` and `shop-canopy-renovation-miniature.jpg`.

Source inspection agrees with those states: `renderLowriseConstruction` uses the
open facade/roof kit from progress0.86 and retains the worksite overlay. This is
static stage evidence, not observation of a live construction transition. No new
runtime changes or test runs were made in this review. The specimen benefit remains
modest, with simple repeated facade bays and weak style separation still visible.
Keep the correction provisionally; next review generated shops/cafes and their
street surroundings through normal city loading before closing this intervention.


### Generated shop neighborhood review — 2026-10-13

Normally imported seed87231/tick80000 (`.tools/errand-local-review/tick-80000.json`)
through World and saves. Paused day3/14:00,50mm,clear,HighWebGPU,DOFoff,1280x720.
Waited for production queues to reach0 without forced draining; initial captures
while neighborhood batches remained were not used as final evidence. Readiness
capture: `screenshots/station-fabric/shop-3449-diagnostics.jpg`.

Frame review building3449 exposes a two-storey Corner Store, parcel3422,
8.916416x9.813336m, at(12.084708,-33.199673),rotation3.1365476. The same held pose is
saved in `shop-3449-anime.jpg` and `shop-3449-miniature.jpg`. The canopy has readable
projection and remains attached to the facade, with the entry and narrow frontage
clear in these views. No city pre-fix A/B was captured; the earlier controlled
specimen A/B establishes the bounded visual difference. Retain the correction as
a useful physical/frontage fix, not a major neighborhood fidelity advance.

The larger block still reads through repeated window/balcony grids and uniform
rooftop fixtures. A bare green patch beside the shop interrupts the street edge;
its ownership/use has not been established, so this is not evidence to fill it
with another parcel or label it a park. Both styles remain visually close with
DOF disabled. These are higher-impact leads than further canopy detailing.

Also attempted cafe3649 (7.2930255x11.850749m,2floors) in the same city. The review
pose was predominantly rooftop; manual orbit exposed its canopy from above, but
lower angles were blocked by taller neighbors. `cafe-3649-occluded-review.jpg`
records that limitation. Do not count it as a clear cafe-frontage pass. Automatic
subject framing/occlusion remains a camera concern independent of kit correctness.

Miniature style-switch diagnostics reported2576ms submission and4810ms await.
These are application observations, not named-hardware performance acceptance.
No code edits or new tests in this review. Live construction transitions, other
seeds and the full release matrix remain open. Browser returned to shop3449,
Miniature,paused14:00,DOFoff. Next investigate ground-edge ownership/treatment or
material response from this baseline; do not add more canopy props.


### Ground provenance beside shop3449 — 2026-10-13

Audited the saved seed87231/tick80000 geometry rather than painting over the visible
patch. `shop-3449-ground-audit.json` records the source hash and four probes west
of the shop at x3/5,z-33/-30. All four are outside emitted ground polygons and
released parcel rectangles, and outside terrain protection polygons. The shop
center control is inside ground block88 and parcel3422. `shop-3449-ground-audit.svg`
shows nearby realized surfaces, parcel outlines and road centerlines in plan.
Centerline distances are not road-surface or clearance tests; the snapshot does
not expose future unreleased plan cells or legal ownership.

Source provenance: `masterplan_ground()` in engine/src/masterplan_growth.rs emits
retained lot outlines only for parcels with building records. The renderer's
`buildGroundSurfaces()` draws those outlines over the natural terrain underlay.
This matches spec05's rule that unrealized cells retain natural ground. The patch
therefore is not evidence of a missing shop ground mesh or of a blanket paving
bug. Preserve this distinction; do not pave a whole block or add cosmetic buildings
just to remove green from a screenshot.

Decision: no runtime change to this patch without a world-level residual-space
classification and actual corridor/access clearance. Its current presentation is
visually weak, but the bounded shop-canopy intervention is not the place to invent
land use. The next appearance comparison should isolate material/light response
in the matched shop3449 views against both locked concepts, while preserving this
residual-land issue in the urban-fabric backlog. Do not reopen canopy detailing.


### Daylight glazing response candidate — 2026-10-13

Compared locked concept day/night panels with the matched shop3449 frames. A
specific material gap is the saturated yellow diffuse color of selected windows
in full daylight: lowering emission alone leaves those panes reading as painted
panels. Added an explicit illuminated-window material class (`ArtMaterials.window`)
that blends existing blue-gray glazing toward the warm interior tint using night
and activity uniforms. Day emission is reduced; the existing analytic sky response
is reused. No images, textures, geometry, simulation or camera changes are added.
Lanterns/signs retain their separate material path; a small shop lamp previously
sharing window glow now uses the lantern material. Unfinished low-rise panes use
the existing unlit glass rather than warm non-emissive paint.

Both source and neighborhood-batched materials use the same response helper.
Window batches explicitly consume the per-vertex warm tint and disable automatic
vertex-color multiplication, preventing the blue daytime base being tinted gold.
A new regression verifies separation from equally colored lamps through batching
and style switching. Existing geometry tests now identify the explicit window
material instead of the old generic emission lookup.

Validation: initial TypeScript and34 tests across painted-materials,building-chunks,
midrise-cladding andlowrise-kit passed; after the lamp/construction adjustment,
19 tests across the latter geometry files and painted-materials passed. The new
material suite has4 passing tests. Final production build passes with the existing
bundle-size warning (JS1425.83kB,gzip420.75kB). These checks do not evaluate rendered
shader pixels or constitute the full suite.

Controlled low-rise viewer captures `window-candidate-{day,night}-{anime,miniature}`
show cool daylight glazing and retained warm night panes. Normally reimported
seed87231/tick80000 and waited for all four queues0; `window-city-diagnostics.jpg`
records readiness. At the previous shop3449 pose,50mm,14:00,clear,HighWebGPU,DOFoff,
1280x720, Miniature `window-city-day-miniature.jpg` removes the yellow daytime
panels seen in `shop-3449-miniature.jpg`. `window-city-night-miniature.jpg` uses a
21:00 lighting preview of the paused city and retains warm panes. This is not a
real day-cycle behavior check. A startup watchdog appeared during hot reload and
cleared without retry; no shader errors were returned by the browser log query.
Do not equate that recovery with startup-performance acceptance.

Candidate remains under validation: check production Anime, dusk/late-night
activity response, construction transitions, additional seeds and moving camera
before broader acceptance. This fixes day/night material semantics, not the
remaining generic architecture, ground integration or weak style distinction.

Production Anime daylight check also completed after style preparation at the same
shop3449 pose: `window-city-day-anime.jpg`. Daytime glazing is readable in this
batched path as well. Browser retained Anime,paused14:00,DOFoff. Broader night,
motion, construction and daily-activity validation remains open.


### Glazing dusk and late-night preview — 2026-10-13

Continued the held shop3449 view without reloading or altering the mature world.
At50mm,HighWebGPU,DOFoff,1280x720, captured Anime18:00 (`window-city-dusk-anime.jpg`),
Anime21:00 (`window-city-evening-anime.jpg`) and settled02:00 in both styles
(`window-city-late-night-anime.jpg`, `window-city-late-night-miniature.jpg`). All are
lighting previews of paused seed87231/tick80000; they are not overnight simulation
or destination-activity evidence.

Dusk panes take on warmth;21:00 windows and shop lighting read brighter than02:00.
The late-night panes remain visibly warm, consistent with the existing nonzero
activity floor: home0.32,office0.1,shop0.18. Do not describe this as switched-off
interiors or proof of per-resident occupancy. The material helper consumes those
uniforms in both diffuse interior blend and emission; source inspection confirms
they are updated continuously with smoothing. No new tuning was justified by these
stills. Retain the candidate provisionally, with additional-seed, construction and
moving-camera checks still open. Night roofs remain dark and overall style
separation remains limited; these captures do not pass the locked concepts.

Restored live lighting to the unchanged paused14:00 city,Miniature. No runtime
changes or additional tests in this review; documentation diff check passes.


### Glazing construction and second-seed checks — 2026-10-13

Controlled low-rise viewer,Miniature/night:95% construction panes remain unlit;
switching to open activates the existing warm window pattern. Captures are
`window-construction-night-miniature.jpg` and `window-open-night-miniature.jpg`.
This verifies static stage behavior, not a live engine construction transition.

Normally imported seed0/tick80000 (`.tools/errand-seed0/tick-80000.json`),paused
14:00,1730buildings/147journeys/0works,50mm,HighWebGPU,DOFoff,1280x720. Waited for all
four queues0; `window-seed0-diagnostics.jpg` records readiness. Framed shop2069,
13.786893x10.067421m,2floors,at(1.543953,-3.6364112),rotation1.5421547. Matched current
views `window-seed0-day-miniature.jpg` and `window-seed0-day-anime.jpg` show readable
cool glazing without obvious lost panes. No pre-change seed0 capture exists, so
this is a generalization check rather than quantified A/B improvement.

Enabled Miniature DOF and made one65px horizontal orbit. The resulting view shows
expected foreground defocus and intact panes/occlusions; saved endpoint as
`window-seed0-orbit-dof-miniature.jpg`. An endpoint does not establish continuous
motion stability or exclude brief flicker. Restored the deterministic building
pose andDOFoff before the Anime comparison. Anime has more visible facade pigment
variation here, but repeated window layouts remain conspicuous and the concepts
are still unmet. No further material tuning was added.

Retain the glazing candidate with these bounded checks. Seed5/developing review,
continuous motion, true daily behavior and release-scale performance remain open.
The loading window included86.5ms longest frame; settled readiness does not cancel
that observation. Browser retained on seed0/shop2069,Anime,paused14:00,DOFoff.
No runtime edits or tests in this turn; documentation diff check passes.


## Updated goal and gap priorities — 2026-10-13

This reconciliation supersedes earlier “next workstream” instructions in this
chronological report. It records existing evidence; no new runtime validation was
performed for this documentation update.

| Dimension | Learning and remaining gap | Next acceptance evidence |
| --- | --- | --- |
| Overhead map / urban form | Compact crops already have substantial coverage; station/waterfront composition differs. Gross coverage and street-adjusted area are not net developable density. | Preserve compact fabric; measure corridors, frontage and block structure before changing density or road widths. Maps without scale/legend cannot establish terrain elevations. |
| Reference architecture | Terrace family, entrances and shop canopy help locally; repeated roof/window rhythms and generic facades remain. | Matched neighborhood views in both styles and multiple seeds, including developing state; stop details with negligible screen-scale benefit. |
| Look and feel | Daylight glazing no longer depends on yellow diffuse panels; two-seed checks support retention. Style separation and material richness remain weak. | DOF-off style comparison plus real day-cycle and continuous-motion checks; lighting previews alone are insufficient. |
| Camera / depth | Thin-lens DOF exists; favorable framing changes the apparent density substantially. | Identical lens/pose comparisons, focus transitions and moving occlusions; blur must not conceal weak geometry. |
| Ground / water / landscape | Sampled shop3449 green margin is outside emitted ground and released parcels. Its intended residual-land role is unproven. | Classify ownership/access/corridor/protection before changing surfaces; assess continuous banks, planting and paths in motion. |
| Infrastructure | Gateway stairs extend outside the world in three audited seeds. Plan-view non-overlap does not establish vertical clearance. | Correct shared access/geometry, verify finite bounds, usable connections, mesh/height clearance and construction. This is the immediate correctness gate. |
| Living activity / audio | Errands improve continuity but do not establish broader work-derived demand or convincing visible daily rhythms. | Normal-speed real destination journeys and true day-cycle review, plus explicit spatial sound/music continuity review. |
| Browser delivery | Settled readiness and isolated passing tests do not explain loading/growth/orbit stalls. | Normal scheduler, named hardware/browser, end-to-end loading, frame times and memory; retain narrow-view and full spec08 gates. |

Retain the canopy and glazing candidates with their documented limitations. The
immediate next task is the gateway-stair correctness defect, followed by a fresh
matched-view decision within frontage/ground integration. Keep one primary
workstream, a bounded change and explicit stop criteria; reassess after two failed
attempts. No broad asset batch, larger map or engine rewrite substitutes for these
gaps. The overall objective remains the complete living city, including locked
concept fidelity, coherent simulation, audiovisual exploration and browser delivery.


### Gateway stair candidate — 2026-10-13

Source diagnosis: both terminal layouts used local stair bottom -20/top -1.
At the boundary gateway this points beyond the world. The candidate keeps the
platform and queue frame unchanged, but uses the inward slab edge: bottom67,
top48, the same19m run and44 risers. Renderer tread depth now uses the absolute
run; signed placement and rail slope support either direction. Engine entry/exit
paths derive stair endpoints from the rendered layout rather than duplicate them.

Mesh raycasts cover every tread, queue position and both slab joins in four rotated
frames for both access directions. TypeScript and7 focused station/rail tests pass.
An ad-hoc SAT comparison of the candidate stair rectangle against saved building
envelopes in the existing seed87231/0/5 snapshots found no overlaps. Its x extent
is325–344m instead of extending beyond400m. This is a proposed-footprint check,
not a regenerated engine snapshot or rendered clearance review.

The direct ground approach still needs work: a path from the terminal toward the
stair bottom can pass underneath the low stair end. The candidate is NOT accepted
as usable access yet. Next: route the ground approach around the stair footprint,
check head clearance/supports, reconcile persisted access-path validation (no old
save migration), then rebuild WASM and review normally generated cities and
construction. Browser WASM has not been rebuilt for this candidate. Do not use the
passing tread tests as proof of pedestrian approach clearance.

Native `rail_` checks:13 passed after correcting the new test fixture to open
rail and recompute connectivity. Existing rotated connector bounds were updated
for inward gateway access; the new four-edge regression checks full tread bounds
and engine entry/exit agreement with the platform layout. Earlier diagnostic
runs failed on the obsolete range and incomplete test setup; final focused run
passes. The full engine suite was not run.


### Stair approach clearance and fresh WASM city — 2026-10-13

Completed the candidate ground detour for both terminal frames. Pedestrians use
local x6.9 outside the stairs and platform columns, turn onto the stair axis1.5m
beyond its bottom, then ascend. Entry and exit reverse the same approach. The
rail connector now has8 points; ordinary access remains capped at5. Save validation
compares explicit rail connectors with their canonical engine path and rejects
obsolete/perturbed geometry with a fresh-world message; no migration is added.

Tests cast horizontal rays along all authored ground-approach legs at lateral
body offsets -0.33/0/+0.33m and heights0.4/1.2/2.07m, across four terminal rotations
and both stair directions. They find no station-kit mesh intersections. This
covers the authored platform/stairs, not all city infrastructure or arbitrary
remote route endpoint connections. An ad-hoc body-expanded building-envelope
sample of the x6.9 gateway side path finds no overlap in the three prior snapshots.
Full city approach/terrain/other-infrastructure review remains necessary.

TypeScript passes,7 focused renderer tests pass, and13 native rail tests pass,
including current-save replay midway up the stairs and rejection of a perturbed
approach. Updated the existing stair-progress test to exercise the actual stair
segment after adding the ground detour. Earlier run failed on its old5-height
expectation; final focused run passes. Full suites have not been run.

Rebuilt WASM successfully. Fresh seed87231 ordinary fixed-step generation reaches
tick80000 with1839 buildings/lots and564 realized roads. All eight10k checkpoints
restore identically and preserve movement packets; final live/restored80-tick
continuation agrees. See `stair-review-generation.json`. Fresh review save:
`.tools/stair-review/tick-80000.json`; snapshot `.tools/stair-review/world.json`.
Snapshot confirms gateway bottom67/top48 and unchanged station bottom-20/top-1.
This is engine generation, not browser scheduler or visual acceptance.

Next: normal browser import, readiness capture, both-style gateway review and
moving pedestrian clearance; then additional seeds/developing infrastructure.
The boundary/access candidate remains under validation. No commit or push.


### Gateway browser review and behavioral limit — 2026-10-13

Reloaded rebuilt WASM and normally imported `.tools/stair-review/tick-80000.json`.
The simulation continued briefly before pausing at day3/14:43 (1839 buildings,
44 journeys,0 works); do not label these screenshots exact tick80000. Framed
building1674, then widened/panned through normal camera controls. HighWebGPU,
50mm,1280x720,DOFoff, live lighting/weather; no lighting preview. All four geometry
queues reached0 without draining. Saved `gateway-stair-ready.jpg`, matched
`gateway-stair-anime.jpg` and `gateway-stair-miniature.jpg` under
`screenshots/station-fabric/`. Stairs join the platform and descend inside the
visible rail corridor. No missing tread or gross slab disconnection is apparent.
The ground approach remains grass with no legible entrance apron or footpath;
this is a remaining ground-integration/fidelity gap, not a reason to pave unrelated
residual land.

Recorded20s at1x simulation in Miniature with DOFoff; paused at15:28 afterward.
`recordings/gateway-stair-activity.webm` and seven decoded samples
`gateway-stair-motion-samples.jpg` preserve the observation. The log reports two
regional-gateway alightings, but samples do not demonstrate stair traversal or
continuous-motion clearance. Source inspection (`lib.rs`, outbound alighting)
shows an immediate transition to `working_external`; the production exit-walking
call targets the city station. The initial statement that the recording captured
a train arrival was too broad: arrival was confirmed by the event log, not by a
clearly visible train in these sampled frames. Do not use this observation to
claim visible gateway access behavior.

Next moving-pedestrian clearance review must target the city station's actual
walking-home exit. Gateway queue/geometry checks remain useful; do not add fake
pedestrians to manufacture acceptance. Longer-term gateway transitions should be
assessed against the living-city requirement without confusing an abstract external
work destination with an implemented ground-level journey.

Style switch diagnostics reported3288ms submit and7792ms await; a ready rolling
frame window does not establish hardware performance. No code/test changes in this
browser-review turn. Goal remains incomplete; additional seed/developing review,
city-station walking clearance and the full release obligations remain open.


### Actual city-station exit evidence — 2026-10-13

Advanced the fresh seed87231 city through ordinary steps to tick83229 and found
Emi Kato (traveler427) beginning a real walking-home rail exit. No actor, queue or
route was injected. Saved `.tools/stair-review/station-exit.json` and its world
snapshot; `validation/station-exit-trace.json` records488 samples through tick84203,
when access_phase changes from0 to1. Body-expanded plan-view building-envelope
sampling at <=0.1m along the complete eight-point path reports no overlaps. This
is conservative building clearance, not a detailed all-object collision proof.

Normally imported the held checkpoint, used View station and ordinary zoom/orbit
to expose the stair, and waited for all four queues0. `station-exit-ready.jpg`
records readiness. HighWebGPU,50mm,1280x720,Miniature,DOFoff,live weather/lighting.
Recorded three consecutive20s segments at1x simulation with pauses for saving:
`recordings/station-exit-{1,2,3}.webm`, spanning day3/17:13–18:26. No audio.
`station-exit-motion-samples.jpg` shows twelve decoded detail samples, corrected
for the video's pixel ratio, with passengers visibly progressing down the stairs,
turning beyond the lower tread and walking along the outside approach. It does
not identify Emi uniquely by appearance or prove every interpolated frame.
`station-exit-evening.jpg` preserves the final full view.

A suspected transverse obstruction is the painted edge of open local road39
(node36 to station38), not a fence: city.ts emits a0.14m-wide,0.03m-thick edge mark
for that road class. Do not create a fake barrier-opening task from the screenshot.
The observed descent supports retaining the candidate with bounded confidence.
The grass approach still lacks a readable footpath/landing; any treatment should
follow the actual infrastructure access, not blanket-pave neighboring residuals.

No runtime change in this review turn. Additional seeds/developing construction,
both-style moving clearance, continuous playback and all spec08 gates remain open.
Observed recording frame maxima63.3/41.2/46.1ms are not named-hardware acceptance.
The browser is paused at18:26 in the station view. Next: additional-seed/access
coverage, then a bounded ground treatment for the demonstrated station approach.


### Fresh three-seed access audit and construction gate — 2026-10-13

Generated fresh seeds0 and5 with rebuilt WASM through ordinary fixed steps to
80000. Eight checkpoint save/movement equivalence checks and final80-tick replay
pass for each. Reports: `stair-seed0-generation.json`, `stair-seed5-generation.json`.
Seed0 reaches1730 buildings/1730 lots/530 roads. Seed5 has1999 buildings,
2030/2062 lots and545/558 roads, so remains developing; do not label it mature.

Added reproducible `scripts/audit-station-access.mjs` using restored current WASM
platform views, not substituted candidate coordinates. `station-access-current.json`
covers seeds87231/0/5 at ticks100,10000,80000:18 stair rectangles,0 out-of-world
corners and0 building-envelope SAT overlaps. The earlier failing audit remains
historical baseline. This is2D envelope evidence, not terrain/all-object clearance
or additional-seed visual acceptance.

The early-stage check exposed a presentation defect: all three tick100 saves have
station_ready=false, station_online=false, station project stage=clearing with
progress0.25 and rail foundation/progress0. The renderer nevertheless builds the
full platform/stairs and enables its roof at progress>.2. `presentation_revision`
and `VisibleInfrastructure` encode the same progress-only threshold. Project
stage is derived from overall progress in the engine;0.25 is still clearing.
No claim is made that passengers use this unfinished structure: service readiness
remains separately gated.

Next bounded correction: coherent station construction phases shared by rendering,
presentation revision and visible-infrastructure support. Preserve mature geometry
and access paths; test progression and restore while work is underway before
adding finished approach pavement. This is within the current infrastructure
correctness gate. Do not stack decorative changes or add compatibility migrations.
No runtime change in this audit turn; no new browser captures. Existing station
scene remains the handoff view. Full goal and release obligations remain open.


## Consolidated learnings and goal reconciliation — 2026-10-13

This is a prioritization update from the recorded evidence, not a new visual
acceptance run. The objective remains the complete browser living city, with
locked-concept fidelity an essential part of its growth, activity and audiovisual
experience. Requirements remain README, specs01–08 and .impeccable.md.

| Dimension | Learning / remaining gap | Decision and acceptance evidence |
| --- | --- | --- |
| Locked references | Repeated architecture, weak frontage and limited material/style separation still limit fidelity. Retained apartment, entrance, canopy and glazing candidates have bounded evidence. | Judge reusable neighborhood changes against concepts04/01 in both styles. Preserve useful massing before adding small props or texture batches. Generated studies are targets, never engine evidence. |
| Overhead / topo maps | Compact crops already have 47.35–49.62% gross footprint coverage; station crops differ. One waterfront composition cannot diagnose citywide density. | Measure block, parcel, street-union and frontage structure. Preserve successful compact fabric. Unscaled maps do not establish slopes, elevations or exact dimensions; green areas are not automatically parks. |
| Transport and physical continuity | Gateway stairs now pass the three-seed, three-checkpoint 2D audit; real city-station exit samples show descent and approach walking. Full clearance is not established. Early saves exposed completed station geometry during clearing. | Finish and verify shared construction phases, render visibility and service support before accepting infrastructure. Keep geometry and passenger paths consistent; do not merely hide broken joins. |
| Ground and landscape | Station approach reads as grass without a legible landing/path. Shop3449's green margin lies outside emitted ground and released parcels. | Surface the demonstrated access route using authoritative ownership/geometry. Do not blanket-pave residual land. Integrate banks, planting, paths and crossings. |
| Camera and lens | Thin-lens DOF exists. Blur does not supply composition, foreground/middle/background separation or believable scale. | Hold pose/lens/time fixed; compare DOF off/on and moving occlusions/focus changes. Keep the subject detailed and both styles distinct without blur. |
| Look, water and atmosphere | Glazing candidate improves daylight response; repeated patterns and style separation remain open. Rejected reflection tweaks did not establish a better water result. | Evaluate material/light behavior in motion and real daily cycles. Revisit rejected approaches only with a new mechanism and comparison. |
| Living behavior and audio | Errands improve continuity; destination counts do not establish visible bustle. External gateway alighting currently abstracts onward travel. | Review real journeys, construction and renewal. Do not inject crowds for screenshots. Work-derived demand, spatial sound and audiovisual continuity remain acceptance obligations. |
| Browser delivery | Normal imports and sampled recordings expose costs outside settled rolling frame windows. | Measure startup, loading, growth, camera motion and memory on named hardware/browser. Do not substitute a ready screenshot or software capture for performance acceptance. |

Immediate bounded work: finish and validate the station construction-phase candidate
already present in the working tree. Its edits are not yet accepted: focused tests,
WASM parity/current-save restore during construction and normal early-stage browser
review remain required. Earlier access reports predate these phase edits. Preserve
mature geometry and routes; verify unfinished geometry cannot support visible service.
Then resume a bounded station-approach/frontage integration change, supported by
matched neighborhood captures and actual routes. Re-rank after that evidence.

Keep one primary workstream, a baseline, a bounded change, explicit acceptance and
a stop condition. Reassess after two unsuccessful attempts. No speculative map
expansion, broad texture campaign, engine rewrite or micro-detail loop to avoid the
largest visible gap. Three-seed and developing/mature checks supplement, not replace,
the full spec08 founding/day-cycle/motion/narrow-viewport/audio/performance matrix.
User review establishes artistic acceptance. Backward save compatibility is not
required; current-format save/replay correctness is required.


### Station construction phases implemented and checked — 2026-10-13

Replaced the premature full-platform/20%-roof rule with engine-derived phases:
site0, supports1, deck/access2, canopy3, complete4. Rendering, presentation revision,
batch identity and visible-platform movement support share that contract. Lamps,
furniture and station signs wait for completion; movement requires a committed
complete platform. Mature geometry and authoritative walking paths are preserved.

The first ordinary-growth regression failed because the proposed92% canopy threshold
was skipped by the station's eight work increments (.875 ->1). Corrected the canopy
threshold to85%, during late surfacing, without changing construction duration.
Final native presentation tests7/7 pass; TypeScript passes; focused web tests51/51
pass across station-kit, rail-corridor, visible-infrastructure, road-batches and
camera-review-station. Geometry rays cover support/deck/canopy/fittings order and
both stair directions. Existing queue/tread/body-clearance tests remain passing.

Rebuilt WASM. scripts/audit-station-construction.mjs checks ordinary growth for
seeds87231/0/5, all five phases at ticks0/200/300/350/400, exact current-save and
one-step continuation/movement equivalence at each boundary. Report:
station-construction-current.json (includes binary hash). Service readiness stays
false before phase4. Native and WASM both pass phase progression; this is not a
full cross-runtime bit-for-bit city comparison.

Normal browser imports of seed87231 phases1 and3 were paused at ticks200 and350,
06:12/06:21, live dawn lighting, HighWebGPU,50mm,1280x720,DOFoff. All four queues
reached0 without draining. View station shows bare supports at phase1 and deck,
stairs and canopy at phase3. Captures under screenshots/station-fabric:
construction-supports.jpg, construction-canopy-ready.jpg and
construction-canopy-miniature.jpg. These are different growth checkpoints, not
matched lighting A/B or continuous construction footage. Ground approach remains
grass. Preserve that limitation and do not blanket-pave nearby residual land.

Next: complete both-style construction/moving transition review, then resume the
bounded access/frontage treatment. Full day-cycle, audio, narrow viewport,
named-hardware performance and user artistic acceptance remain open. No commit.

The held phase3 scene was also switched to Anime and allowed to settle with all
four queues0. construction-canopy-anime-ready.jpg and construction-canopy-anime.jpg
record the same tick/camera/lens/lighting with DOFoff. Both styles retain the canopy,
deck and stairs. This adds static style coverage, not continuous transition proof.
The browser remains paused at tick350/06:21 in Anime for follow-up review.


### Live station construction and foliage clearance — 2026-10-13

Previous turn was progress: construction contract, tests and browser evidence changed
authoritative state. This turn normally imported seed87231 tick200/phase1 paused,
waited for all four queues0, selected View station and recorded20s in Anime at1x
simulation/16x development, HighWebGPU,50mm,1280x720,DOFoff,live dawn lighting.
`recordings/station-construction-anime.webm` records ordinary growth; no phase or
actor was injected. Simulation continued after recording until manually paused
at07:47; that time is not the recording end time.

Twelve decoded samples (`screenshots/station-fabric/construction-anime-motion-samples.jpg`)
show supports through5s, deck/stairs at6–7s, canopy at8–10s and a train at14–18s.
This establishes the visible phase sequence in one live run, not every interpolated
frame, passenger support or full continuous-motion acceptance. The initial recording
start/camera change produced an observed377.3ms longest frame; it is not an isolated
construction bottleneck diagnosis or hardware acceptance. Final ready-window values
do not supersede that observation.

The sequence exposed foliage intersecting the platform area. Tree clearance checked
roads/buildings/ground/rear extensions but omitted platform reservations. Added
`station-clearance.ts` with canopy-expanded station-kit and signed stair footprints;
`updateTrees` uses it and its geometry inputs participate in cache invalidation.
Reservations clear before deck construction, and phase-only changes do not churn
instance matrices. This removes visual trees in reserved infrastructure only; no
engine terrain, planting generation, route or ground ownership is altered.

TypeScript and14 focused tests pass (station-kit plus tree-clearance cache/signature/
grid). New renderer-cache regression covers forward/reverse/oblique frames, both
stair ends, preserved nearby planting, phase stability and moved-platform invalidation.
Normally reimported the identical seed87231 tick350/phase3 checkpoint and waited for
all queues0. Same View station/50mm/Anime/DOFoff/live06:21 comparison saved as
`construction-canopy-anime-clearance.jpg`; the overlapping foliage is absent while
nearby planting remains. This is one-seed static evidence after the clearance change.
The prior recording is the failing foliage baseline, not post-fix motion acceptance.

Next: review the corrected construction sequence in Miniature, then resume the
bounded station approach/path treatment from authoritative access geometry. Wider
clearance, daily cycles, continuous motion, audio, performance and full spec08/user
artistic acceptance remain open. Browser is paused at tick350/06:21 in Anime. No commit.


### Corrected Miniature sequence and approach design audit — 2026-10-13

Previous turn was progress (station foliage clearance and matched evidence).
Normal import of seed87231 tick200, queues0, View station, HighWebGPU50mm,
1280x720,DOFoff,Miniature,live dawn: recorded20s at1x simulation/16x development
with foliage correction present. `recordings/station-construction-miniature.webm`
and `screenshots/station-fabric/construction-miniature-motion-samples.jpg` preserve
12 samples. Supports persist through5s, deck/access appears at6–7s, canopy at8s,
and a train at14–18s. The prior foliage intersection is absent in those samples.
This supports retaining the phase/clearance candidate and proceeding to access
integration; it is not every-frame, passenger-clearance or full release acceptance.
A533.7ms longest frame was observed at recording/camera startup; cause and hardware
acceptance are unresolved. Paused later at07:35, after the recording completed.

Added `scripts/audit-station-approach.mjs` and
`validation/station-approach-candidate.json`. This is a design audit, not emitted
paving: 1.6m-wide butt-joined strips from the terminal node to the engine approach,
round the stair foot, ending at the first tread. Existing side offset6.9m causes
pier overlaps for all three gateways. Candidate offset7.5m clears piers for both
terminals in seeds87231/0/5 tick80000; all six candidates have no world-boundary,
building-envelope, parcel, protected-polygon or supplied river-corridor overlaps.
Seed5 is still developing. River test uses the supplied piecewise-linear horizontal
half-width; no terrain-height or other-road/all-object clearance is claimed.
Corner/landing geometry must be re-audited once authored; butt strips do not prove
watertight or accessible paving joins.

Next bounded implementation: expose authoritative station approach geometry/width
in the platform contract, move the pedestrian side approach to the proven candidate
offset, and derive paving from that same contract. Preserve current-save validation
and mature walking behavior; no migration for incompatible old access paths.
Stage paving with deck/access, retain a legible landing and avoid piers, road paint
and neighboring ground. Add physical support/clearance tests and matched browser
review with actual passengers before acceptance. No runtime change this turn.


### Shared station approach implemented — 2026-10-13

Previous turn was progress: corrected Miniature recording and candidate footprint
audit changed the next implementation decision. Added engine-owned local ground_path
(five street/approach/apron/foot points) and ground_width1.6 to RailPlatformView and
TypeScript contract. Side approach is7.5m from the rail axis. Engine entry/exit paths
now read their three ground waypoints from this same contract. Mature stair geometry
is unchanged. Current-save validation rejects obsolete access paths; no migration.

Renderer uses a single mitered extruded paving outline (0.12m thick, top at ground_y)
in phase2 onward, joined around the stair foot. Avoids coplanar overlapping strips.
Visible-infrastructure identity and tree-clearance signature include path/width;
foliage clearance includes the approach corridor. The earlier butt-strip audit is
not proof of these exact joined corners: re-audit emitted geometry before acceptance.

Initial tests exposed nonindexed extrusion versus indexed Kit batching; corrected
the new geometry to use indices. Native compile required WorldPoint PartialEq for
the existing platform-view comparison. The rotated connector regression retained
its independent envelope check with the new7.5m offset (previous6.9m expectation
failed as expected). Final checks: TypeScript passes;56 focused web tests pass;
13 native rail tests pass, including named commute save/replay and both-terminal
rotated access. Native assertions compare engine ground entry/exit points to the
exported ground_path. New ray tests sample pedestrian-width support along all four
paved legs and absence before deck phase; they do not prove every corner or real
street endpoint. Rebuilt WASM and reran the three-seed construction/save audit.

Normal browser import: fresh construction checkpoint seed87231 tick350/06:21,
Miniature,HighWebGPU,50mm,1280x720,DOFoff,live dawn,all four queues0. View station,
then ordinary zoom/orbit exposed the landing. Captures:
`screenshots/station-fabric/approach-miniature-wide.jpg` and
`approach-miniature-close.jpg`. The connected landing/paving is visible beside the
stair and road. No actual passenger traversal was captured in this early scene.

Next: validate the emitted mitered footprint and actual road-endpoint/ground joins,
then fresh mature passengers in both styles. Route endpoints can differ from the
terminal center, so centerline support tests alone are insufficient. Old mature
saves with active old approach paths may reject; regenerate rather than repair.
Do not declare access or full construction accepted yet. Browser remains paused
at06:21 in the close Miniature view. No commit/push; full goal remains active.


## Recorded learnings and goal reconciliation — station road join — 2026-10-13

This update records existing implementation and artifacts; no new runtime change
or visual acceptance run was performed for this documentation request. It supersedes
older “implement shared paving” next steps, while retaining the multidimensional
assessment above and the locked concepts as artistic targets.

The shared station approach is implemented, not merely proposed. The exact
production mitered-outline audit (`station-approach-built.json`) checks both terminals
at ticks10000/80000 for seeds87231/0/5: all12 cases clear the tested world boundary,
building/parcel envelopes, piers, protected polygons and supplied river corridor.
Seed5 at80000 remains developing. This does not prove terrain height, road joins
or all-object clearance.

The ordinary passenger trace (`station-approach-exit-trace.json`, traveler427,
seed87231, starting tick83229) exposes why route-level checks must follow geometry
checks. Of329 sampled ground centerline points,24 on the final leg lie outside the
probe's paving/straight-road/emitted-ground union, approximately the last2.4m.
`station-approach-ground-support-probe.json` excludes junction rounding, arterial
sidewalks and height: preserve that limit rather than calling it a full collision
failure. Source inspection finds pedestrian half-width+0.72m offsets while rendered
locals/lanes are shared surfaces. A road-type-aware offset/connector correction is
the next candidate; it has not been implemented or validated by this update.

The goal now requires actual passenger-width and height support through the road
join, consistent movement/endpoints/access, current-save replay, construction and
three-seed checks, then actual traversal in both styles. No save migrations required.
Once demonstrated, stop station correctness work and resume the largest visible
neighborhood gap: frontage/architectural repetition, ground integration or material
and lighting distinction, selected using matched evidence.

The overall goal remains the living city. Compact coverage is already successful
in some crops; avoid global density or road-width changes from one view. Thin-lens
DOF exists; camera composition and moving focus still need review. Texture volume
is not a substitute for massing, physical connections or useful frontage. Purposeful
activity, real daily cycles, spatial audio, narrow viewports and named-hardware
performance remain open obligations. One primary workstream, explicit stop criteria,
and reassessment after two unsuccessful approaches prevent decorative detours.


### Road-type walking offset correction — 2026-10-13

Previous turn was progress: recorded findings and synchronized goal. Implemented
one pedestrian_road_offset helper for route endpoints, movement, building frontage
segments and building-access detours. Local/lane walkers now stay0.72m inside the
shared carriageway edge; arterial/other walkers retain0.72m outside. Vehicle lane
positions unchanged. Regression checks cover local/lane/arterial, both route
orientations and walking sides, physical width and endpoint agreement.

Checks pass:1 expanded movement test,13 rail tests,3 access-path tests (including
current-growth tamper rejection and replay). Rebuilt WASM. Fresh seed87231 reaches
80000 with1839 buildings and564 roads; every10000 checkpoint matches save/movement
and final80-tick live/restored continuation matches. Report:
`walking-offset-87231-generation.json`. No old-save migration or fixture repair.

The same ordinary Emi Kato exit starts83229, traveler427. Its street endpoint
moves1.44m inward; origin access completes84215. See
`station-walking-offset-exit-trace.json`. Added reproducible
`scripts/probe-station-walking-ground.mjs`; production paving/straight-road/ground
union covers all330 centerline samples, including polygon boundaries (1e-5m
boundary tolerance). Previous probe found24 uncovered samples. This is not
passenger-width, terrain-height, junction/sidewalk or continuous browser evidence.

Next: audit actual body-width support and height/road-end junction transitions;
regenerate seeds0/5 with this WASM, preserve construction/save gates, then inspect
ordinary traversal in both styles. Do not mark access or artistic fidelity accepted.
Goal remains active. No commit/push; prior dirty work retained.


### Three-seed walking support and recorded descent — 2026-10-13

Previous turn was progress: corrected shared-road offsets and validated actual
seed87231 centerline support. Fresh seeds0/5 now also reach80000 with checkpoint
save/movement equivalence and80-tick continuation. Counts remain1730/530 and
1999/545 buildings/roads; seed5 remains developing. Their ordinary station exit
traces are `station-walking-offset-0-exit-trace.json` and
`station-walking-offset-5-exit-trace.json`; no actor/route injection.

Extended `scripts/probe-station-walking-ground.mjs` to test lateral body edges
+/-0.33m and include production junctionFootprint and roadWidth. All three cases
have330 supported center samples. Each has one unsupported lateral sample at the
exact final endpoint, about0.0449m beyond the flat road cap. This persists with
production junction footprints (a single-arm terminal adds no junction pad).
Do not hide this by relaxing tolerance or report full passenger-width acceptance.
Reports: `station-walking-offset[-0/-5]-ground-probe.json`. Probe remains plan-only,
excludes arterial sidewalks and does not model complete body sweep or every object.

Height source inspection: station paving is flat at0.27m, ordinary carriageway
surfaces top at0.30m, ground-patch meshes at0.035m; default pedestrian surface is
0.27m. The exit's explicit0.27m to negative-sentinel transition preserves the
existing ground policy. No new cliff is suggested by these constants, but this is
source evidence, not a complete mesh-height or contact audit. Do not count generic
ground polygons as equal-height paving in a future height acceptance test.

Re-ran ordinary three-seed station phase/current-save audit with current WASM;
all0..4 phases retain expected boundaries.20 web tests pass across station-kit and
visible-infrastructure. No new production geometry change this turn.

Browser: normal import of seed87231 tick83229, Miniature,HighWebGPU,50mm,1280x720,
DOFoff,live17:13, all four queues0 before recording. Saved20s recording
`recordings/station-walking-offset-miniature.webm` and12 sampled frames in
`screenshots/station-fabric/walking-offset-miniature-motion-samples.jpg`.
Samples show real passengers descending and reaching the paved foot at18s; they do
not demonstrate the complete street join or every-frame clearance. The paused still
`walking-offset-miniature.jpg` predates confirmation of all queues0, so it is not a
complete-city loading acceptance capture. Later pause18:31 is after recording end.

Prepared `.tools/walking-offset-87231/station-ground-exit.json`, advancing the genuine exit600 ticks to83829/17:49.
Emi remains on origin access at45m progress. Imported paused and restored the
station View/zoom2/orbit140px camera for the next ground-join review. Normal import
again showed temporarily missing building batches; no forced draining. Verify
queues finish before recording. Browser tab39 retained; tab38 specimen retained.
Next: inspect actual ground traversal and the cap edge in both styles, resolve the
small physical connection only if needed with bounded geometry, and finish height
support checks. Broader concept, audio, day-cycle and performance gates stay open.


### Station road-end apron and mesh support — 2026-10-13

Previous turn was progress: three-seed checks isolated the road-cap body overhang
and retained actual descent evidence. Added a0.4m end apron only for ground roads
at station nodes. It is a continuation of the existing road width and datum, not
an authoritative route change. Other street ends and boundary gateways are unchanged.
New `ground-road-footprint.ts` shares both footprint and road-surface mesh constructor
with production city rendering and acceptance probes. Surface partitioning remains
in place; no overlapping coplanar patch is added. No WASM rebuild needed this turn.

28 tests pass (ground-road-footprint, station-kit, road-batches); TypeScript passes.
Tests ray-check the full0.33m turning disk at both station ends, both street sides
and cardinal/oblique axes, and preserve ordinary/gateway extents. Exact added-area
three-seed audit clears buildings, parcels, station piers, protected/river polygons
and world bounds: `station-road-landing-clearance.json`. Center/lateral probes now
have0 misses for all330 samples per seed; old failing reports are preserved under
the walking-offset names and new reports under `station-road-landing-*-probe.json`.

Added `scripts/audit-station-walking-height.mjs`: actual exit ground legs, sampled
every<=0.1m at the center and16 perimeter points of a0.33m disk, ray-cast against
production station and ground-road mesh constructors. All5610 samples per seed
87231/0/5 have support. Heights range0.27000001–0.44568181m; upper value is the first
stair tread at the foot, with ordinary road0.30m. No unexpected height under the
explicit first-tread exception. This is sampled ground-route support, not continuous
collision clearance, all terrain, all junctions or every construction state.

Browser reload after source changes reset the review to founding and briefly
reported graphics startup taking too long. The Retry control disappeared before
activation as startup recovered naturally; do not claim a retry fixed it. Final
candidate imported paused from ordinary `station-ground-exit.json` tick83829/17:49,
then View station/zoom2/orbit140px, Miniature High WebGPU50mm1280x720 DOFoff.
Normal loading still had8 neighborhood batches at the last recorded readiness
check. A DOM wait returned a tooling timeout, which is not evidence that rendering
stopped. Preserve tab39 and inspect current queues, then record actual ground
traversal in both styles before closing this correction. Do not force queues.
No new candidate motion acceptance yet. Full living-city goal stays active.


### Final apron Miniature traversal — 2026-10-13

The earlier tooling timeout did not indicate stopped rendering. A fresh UI check
confirmed ready/all four queues0; then recorded20s ordinary motion from the paused
83829/17:49 checkpoint, Miniature High WebGPU50mm1280x720 DOFoff,1x simulation.
`recordings/station-road-landing-miniature.webm` and
`screenshots/station-fabric/road-landing-miniature-motion-samples.jpg` preserve the
result. Twelve decoded samples show passengers on the stair-foot landing, along
the paved side approach and joining the local street. No visible disconnect in
these samples; preserve the distinction from every-frame or both-style acceptance.
The later pause18:48 was after capture end. UI observed longest frame50.4ms at that
pause, not an attributed recording-wide maximum or hardware performance acceptance.

Retain the bounded apron on physical tests/three-seed mesh evidence and this
Miniature traversal. Next finish matched Anime ground traversal, then close further
station detailing and return to ranked neighborhood fidelity gaps. Tab39 paused
at18:48 with details open; tab38 retained. No commit/push. Goal active.

## Recorded learnings and next goal — 2026-10-13

This reconciliation supersedes the earlier instruction to finish Anime station
traversal. It records learning and work selection; it does not grant artistic or
full-release acceptance.

### Station correction: bounded review complete

The final Anime recording uses the same ordinary seed87231 tick83829/live17:49
checkpoint, station framing, 50 mm lens, High WebGPU, 1280×720 and DOF off as the
Miniature review. All four rendering queues reached zero normally before capture.
The 20-second recording and twelve decoded samples show the passenger sequence
from stair landing, around the foot, along the paved approach and onto the street.
No visible disconnect appears in those samples. Evidence:

- [Anime recording](recordings/station-road-landing-anime.webm) and
  [sampled frames](screenshots/station-fabric/road-landing-anime-motion-samples.jpg).
- [Miniature recording](recordings/station-road-landing-miniature.webm) and
  [sampled frames](screenshots/station-fabric/road-landing-miniature-motion-samples.jpg).
- [Three-seed mesh support](station-road-landing-height.json),
  [added-area clearance](station-road-landing-clearance.json) and the three
  `station-road-landing-*-probe.json` reports.

Retain the correction and stop station embellishment. These sampled observations
are not continuous collision proof, a complete motion review or hardware acceptance.
The observed UI frame windows are not recording-wide performance measurements.

### What the references teach and what remains

| Dimension | Learning / current gap | Consequence for the next work |
| --- | --- | --- |
| Locked concepts | Layered architecture, landscape, light and activity create the intended experience together. Small retained corrections have not established that overall fidelity. | Compare ordinary engine output directly with concepts 04 and 01; user review remains the artistic gate. |
| Overhead / topo-style maps | Connected blocks, corridor hierarchy, varied parcel grain and water/land relationships matter more than occupied-lot totals. Compact crops already have 47.35–49.62% gross coverage; station crops differ. | Preserve successful compact fabric. Separate gross coverage, street union and actual developable land. Unscaled images do not establish elevations, slopes or precise dimensions. |
| Neighborhood composition | Water and transport dominate the broad waterfront view; a closer unobstructed station view reveals substantially denser fabric. | Separate camera projection from physical density. Use a repeatable local subject plus a wider context view; do not globally shrink roads or raise buildings from one shot. |
| Architecture / Japanese identity | Repeated roof/window rhythms and limited frontage variation remain. A terrace family and bounded canopy/entrance changes help locally. | Improve reusable massing and frontage where visible at neighborhood scale. Transit realism also needs coherent curves, joins, platforms and vehicle proportions; decorative detail cannot repair geometry. |
| Ground, banks and planting | The latest broad noon-lighting preview still reads as hard banks, weak planting integration and repetitive frontage. The cause of sparse visible vegetation has not been isolated. | Inspect generated placement, clearance/culling, bank/path geometry and land ownership before changing them. Do not blanket-pave residual margins or assume a missing-tree bug. |
| Materials / look and feel | Daylight glazing is implemented, but repeated surfaces and weak style separation remain perceptual concerns. | Compare Anime and Miniature with DOF disabled and identical state. Texture generation should address a named material deficiency after form and scale are sound. |
| Camera / lens | Thin-lens DOF exists. The photos combine selective focus with framing, perspective, a sharp subject and real depth layering. | Check focus transitions, moving silhouettes and occlusion halos; stronger blur alone is not a fidelity improvement. |
| Living behavior / sound | Errands and real station journeys provide causal motion, but counts do not establish believable concentration or daily rhythm. Broader work-derived demand and explicit audio review remain open. | Inspect ordinary journeys at destinations and a real daily cycle. Lighting previews do not test activity or audio over time. |
| Browser delivery | Normal loading, growth and motion still need end-to-end measurements on named hardware. | Count temporary blank scenes and stalls. Preserve browser budgets, bounded state and normal scheduling; settled frame windows alone do not pass delivery. |

### Updated decision

The primary workstream is **neighborhood frontage and landscape integration**.
Bank-to-street integration is the leading hypothesis, pending a reproducible
matched baseline and source diagnosis. Select one reusable correction with the
largest visible benefit; do not start separate architecture, planting, camera and
texture projects simultaneously.

For that correction, record baseline, hypothesis, bounded change, comparison,
rendering cost and stop condition. Review equal seed/tick/camera/light/settings in
both styles, with DOF off and on, then ordinary motion and developing/mature states
across three seeds. Reject close-up-only benefits, access obstruction, arbitrary
paving and blur used to conceal defects. Reassess after two unsuccessful attempts.
The full spec08 matrix, current-save/replay correctness and user artistic review
remain mandatory. Larger maps, broad imports, texture batches and rewrites stay
behind a demonstrated need. The overall objective remains the complete living city.

### Riverbank planting diagnosis — 2026-10-13

Added `scripts/audit-riverbank-planting.mjs` and its
[three-seed baseline](riverbank-planting-baseline.json). The audit extracts current
renderer placement and exact visibility predicates and bundles their production
helpers; it records source/input hashes. It does not alter the renderer or worlds.

| Seed | Generated within 17 m of water edge | Generated 17–50 m out | Retained 17–50 m out | All retained trees |
| --- | ---: | ---: | ---: | ---: |
| 87231 | 0 | 450 | 13 | 55 |
| 0 | 0 | 460 | 11 | 23 |
| 5 | 0 | 414 | 11 | 31 |

Distances use the renderer's X-offset river convention. Retained means not hidden
by the production clearance predicate, not necessarily visible on screen. Audit
snapshots are ordinary exit worlds at ticks83229/83255/82901. Paving alone rejects
240/221/198 candidates in the bank band; other road/building/station/extension
clearances also apply. This supports a placement/development interaction behind
sparse mature banks, rather than a claim of a broken style switch or missing mesh.

A feasibility-only line of 82 small-tree candidates, 3.5 m from the water edge at
18 m spacing on both banks, retains zero candidates in every seed. All hit the
production clearance and a separate conservative transport-envelope check. For an
example seed87231 candidate near z=-360, the nearest arterial is 11.4745 m away,
inside the current 11.5 m road clearance. The renderer's nominal unpaved strip is
not automatically suitable for this tree line. No candidate was added to runtime;
do not override clearance to make the experiment visible.

Saved matched [Anime](screenshots/riverbank-planting/baseline-anime.jpg) and
[Miniature](screenshots/riverbank-planting/baseline-miniature.jpg) waterfront baseline
views and [metadata](screenshots/riverbank-planting/baseline.json). The browser was
paused at seed87231/tick85169, 50 mm, 1280×720, High WebGPU, DOF off, noon lighting
preview; the same held camera was preserved between styles. Miniature switch
completed with all queues zero. Export `.tools/riverbank-review-save.json` preserves
the world; camera pose is described by the preset, not numerically exported.
These images remain baseline evidence, not acceptance of a new visual change.

Next: inspect the physical bank cross-section and actual foliage bounds to choose
between a smaller planting layer or a revised placement inside legal land. Keep
paths and arterial clearances intact. First establish a viable candidate, then
review its neighborhood-scale benefit using this checkpoint before expanding it.

### Bank planting candidate and first matched review — 2026-10-13

Implemented `riverbankTreePoints` and integrated it into the existing tree batches.
Actual far-crown radius2.3772m supports scale1.8 trees at2.7m bank offset; the failed
3.5m placement is not used. No clearance rules were weakened. Three mature seeds
retain76 of84 new positions each, preserving all previous outer/inland counts.
Ten focused tests and TypeScript pass. Tests cover actual near/far crown envelopes,
sloping banks, world bounds and trunk/branch clearance from coping.

Ordinary import reached zero queues. Both-style candidate captures at the held
waterfront/noon preview show a modest, visible bank rhythm; see
[full evidence](riverbank-planting.md). This is candidate retention for further
review, not artistic acceptance. Motion, developing/construction states, all
crossing conflicts and controlled cost comparisons remain open. Do not add another
planting variant before checking this one. Source reload briefly showed slow
startup and recovered naturally without Retry or scheduler overrides.

### Bank crossing correction and sampled motion — 2026-10-13

The growth audit exposed two retained crowns per mature seed overlapping bridge
sidewalk envelopes. Increased only bridge clearance to include2.5m sidewalks and
scaled2.38m crown radius, with matching broad-phase reach. Saved the failing audit.
Corrected retention is74new trees per mature seed; outer/inland counts unchanged.
All41ordinary checkpoint cases, including observed bridge construction stages,
pass checked bridge/rail envelope clearance. Eight focused tests and TypeScript
pass. See [planting evidence](riverbank-planting.md).

An ordinary20sAnime orbit/hold recording and six decoded frames show the corrected
bank rhythm and readable bridge openings. Sampled frames do not establish every
frame or hardware performance. One rolling UI window showed57ms during orbit;
no planting-specific cause was isolated. Next verify corrected Miniature motion,
then stop this bounded planting work and return to the ranked neighborhood gaps.

### Planting retained; ordinary neighborhood reassessment — 2026-10-13

Corrected Miniature orbit/hold completed after ordinary import and zero queues,
from the same85169review save and settings as Anime. Six decoded frames show
coherent bank planting and bridge openings. Retain this bounded correction and
stop more planting variants. See [evidence and limits](riverbank-planting.md).

A normal Watch view selected after the review shows much denser local fabric than
the wide waterfront view. Held that pose in Explore and captured DOF off/on:
[without blur](screenshots/riverbank-planting/watch-neighborhood-dof-off.jpg),
[with blur](screenshots/riverbank-planting/watch-neighborhood-dof-on.jpg).
Miniature,50mm,High,1280×720,noon preview,paused after normal recording; this is not
the original85169actor frame or a measured camera pose. The paired captures show
visible near/far depth separation with a sharper middle band. Thus absent DOF is
not the current diagnosis; moving focus/occlusion acceptance remains incomplete.

Re-rank toward neighborhood architectural/material hierarchy and a readable subject.
Repeated rooftop enclosures, similar window rhythms and subdued surfaces remain
visible even in the closer view. Do not globally increase density, strengthen blur
or restart tiny station/apartment detailing. Before a change, inspect the visible
building-family distribution and material/light response in this ordinary view,
then choose one scene-scale intervention. Existing camera/DOF controls can already
reveal the physical density; compare equal lighting and framing to avoid attributing
a better shot to new geometry. This reassessment is qualitative, not proof of a
specific material bug or acceptance of the Watch director.

## Consolidated learning record and priorities — 2026-10-13

**Superseded priority:** the subsequent user review identifies city-wide openness,
regularity, weak Japanese identity/vegetation, flat atmosphere and visible world
edges as leading failures. The current workstream is city-wide spatial composition
and environmental depth, not further office/facade detail or actor counts. See
[user review and revised decision](user-visual-review-2026-10-13.md). Existing
coverage statistics and feature code do not rebut the perceived failure. The
historical table below retains checkpoint evidence, not the current ranking.

**Latest correctness follow-up:** the office part-level mesh audit now attributes
all123recorded parcel overhangs across557offices.99are on parcels narrower than
the minimum body plus foundation; the remaining24are rear AC assemblies exceeding
shallow setbacks. This is measured geometry, not a density or style judgment.
See [diagnosis and bounded acceptance](office-parcel-fitting.md). Correct fitting
before further visual embellishment, preserving authoritative parcels and access;
no production correction or broader acceptance is established by this audit.

This reconciliation supersedes earlier **next-step rankings**, while retaining
historical measurements, failed experiments and their checkpoint limits. It adds
no new browser observations or test results.

| Dimension | Learning from available evidence | Remaining gap and acceptance evidence |
| --- | --- | --- |
| Locked concepts and reference photos | Concepts 04/01 remain artistic targets; photos explain proportions, layered buildings, Japanese transit and photographic depth. Generated studies are supplementary, not engine results. | Architecture and material hierarchy still lack reference richness. Compare ordinary neighborhoods at normal viewing distance in both styles; user review remains required. |
| Overhead map and urban form | Compact crops measure 47.35–49.62% gross footprint coverage. Station crops are less covered and more street-dominated. One broad waterfront view does not describe the whole city. | Check district differentiation, block/parcel rhythms, corridor continuity and infrastructure relationships. Unscaled map images cannot establish exact widths, elevation or terrain; street-adjusted coverage is not net developable coverage. |
| Architecture and frontage | Terrace apartments, entrances and shop canopy corrections are retained within their evidence limits. Ordinary Watch still shows repeated rooftop enclosures and window rhythms. | **Next primary diagnosis:** visible family distribution, roof/facade rhythms, inhabited ground floors and material response. Choose one reusable intervention with a scene-scale benefit. |
| Look and feel | Both rendering material paths exist. Daylight glazing has bounded supporting evidence; subdued surfaces and weak style separation remain perceptual concerns. | Separate lighting/framing effects from geometry/material improvements. Compare equal conditions with DOF off; avoid arbitrary color variation or broad texture batches without a demonstrated need. |
| Camera and lens | The held Watch DOF-off/on pair shows sharp middle-distance detail and near/far blur. Missing DOF is not supported by this evidence. | Subject selection, composition, moving focus/occlusion and transitions remain open. Record numerical pose and exact tick for new baselines; existing Watch images lack them. |
| Ground, landscape and transport | Station walking support and bank planting corrections are retained. Planting passes the checked envelopes at 41 ordinary checkpoints and sampled motion in both styles. | Whole-city clearance, smooth infrastructure continuity and integrated street/bank edges still need wider review. Stop station/planting embellishment unless new failures justify it. |
| Living behavior and sound | Errands improve causal activity continuity; more travelers did not resolve the wide-view visual gap. | Broader work-derived demand, real daily rhythms, visible destination use, construction/renewal and explicit audio review remain obligations. Do not substitute cosmetic crowds or counts for believable life. |
| Browser delivery | Ordinary scheduling and sampled recordings provide bounded evidence. Rolling frame observations do not isolate causes or establish hardware acceptance. | Measure startup/import, growth, motion, memory and stalls on named hardware/browser; retain the full spec08 matrix, including narrow views and actual day cycles. |

Proceed with one architectural/material diagnosis and one bounded intervention,
using the [updated goal](living-city-goal.md). Hold seed, tick, pose/lens, lighting,
weather, quality and resolution fixed; include ordinary neighborhood and wider
context views. Verify framing/lighting alone cannot explain the claimed benefit.
Preserve deterministic current saves and legal access; obsolete-save compatibility
is not required. Re-rank after two unsuccessful attempts. Full experience and user
artistic acceptance remain open; no new runtime change is authorized by a finding
alone without its bounded acceptance criterion.

### Production neighborhood family audit — 2026-10-13

Added `scripts/audit-neighborhood-architecture.mjs` and
[the three-seed report](neighborhood-architecture-current.json). The audit imports
production envelope, massing and palette helpers, hashes its bundled runtime and
input saves, and records every occupied 160 m tile rather than selecting a favorable
crop. Open buildings only, at ordinary ticks 83229 / 83255 / 82901.

| Seed | Open buildings | Machiya + shop-house | Neutral facades |
| --- | ---: | ---: | ---: |
| 87231 | 1,839 | 62.86% | 67.92% |
| 0 | 1,730 | 60.69% | 68.84% |
| 5 | 2,062 | 62.27% | 69.01% |

All eleven wall swatches occur in each city. Among tiles with at least 40 open
buildings, neutral shares span 53.33–81.25% across these seeds. Family labels are
not silhouette diversity, and counts are not projected facade area or the previous
Watch frustum. These measurements do not prove sufficient architectural variety.

Source inspection explains two repeated treatments: `renderRoofServices` emits
the same tank/HVAC/headhouse arrangement for sufficiently wide flat roofs, with
seeded mirroring; `kit.ts` requests wall, roof and concrete materials at the same
default roughness (0.85). The palette already distinguishes six neutral and five
accent swatches. Anime and Miniature have separate shader paths. This is evidence
of shared treatment, not proof that roughness or roof services alone cause the
perceived flatness. Earlier roof and daylight-fill changes remain retained.

**Next comparison:** establish an exact ordinary-neighborhood camera/save baseline
and compare light/material response at fixed framing in both styles, including a
DOF-off view. Prioritize a bounded material-hierarchy experiment over another
building family if that comparison supports it; do not indiscriminately brighten
the city or add saturated colors. Inspect projected dominant facades before
attributing repetition to the numerical majority of low-rise buildings. No runtime
rendering change, new visual acceptance or performance claim in this audit.

### Held-camera style and light isolation — 2026-10-13

Captured the ordinary neighborhood at noon and 16:00 lighting preview in both
styles, with simulation paused and the same held Explore camera, 50 mm, High
WebGPU, 1280×720, DOF off. Normal geometry queues were zero before capture.
[Metadata and limitations](screenshots/neighborhood-materials/metadata.json).

The pair visibly distinguishes the styles: Anime has brighter, spatially varied
painted walls; Miniature is darker and smoother. Afternoon shadow direction changes
but does not remove the subdued Miniature facade impression. This contradicts a
blanket claim that style switching is ineffective. Neither style meets the locked
concept richness, and these preview hours are not daily-cycle simulation evidence.

Read the earlier daylight-fill and roof-surface experiments before selecting a
change. Hemisphere fill already received a retained increase; repeatedly raising
it would repeat an explicitly closed experiment. Flat waterproof roof fields have
also already been separated from pale coping. Do not undo those corrections.

The next rendering experiment should distinguish facade/roof/concrete material
response without changing global exposure, adding another family or increasing
saturation broadly. A source-code roughness difference alone is not success:
require a visible neighborhood-scale A/B before retention. First establish a
reproducible saved camera/checkpoint through the existing deterministic review
controls; this held camera lacks exported numerical pose/tick, so it supports the
style/light diagnosis but cannot serve as a reload-matched implementation baseline.

### Roughness trial rejected — 2026-10-13

Completed one bounded material-response trial at deterministic station framing,
saved seed87231/tick86278, both styles, noon preview and DOF off. Wall/roof0.68,
concrete0.96 and pale-concrete0.8 roughness did not produce useful scene-scale
benefit. Reverted exactly to pre-trial kit source; existing dirty work preserved.
All15focusedtests pass after restore. See [trial evidence](neighborhood-material-response.md)
for paired screenshots, source patch, computed pose and actor-frame limitations.
Stop scalar roughness tuning. Reassess facade/roof composition against the concepts
before selecting a different visible mechanism; do not stack another material tweak.

### Office side-elevation candidate — 2026-10-13

Implemented office ribbon glazing on side walls with >=0.24m clearance. These walls
previously used residential punched-window grids despite office ribbon fronts.
Final clearance-limited Anime and Miniature station stills show a modest visible
distinction on several office facades. Ordinary motion and developing-state review
remain pending. No color,
lighting, authoritative-state or extra material-family changes. TypeScript and
15focusedtests pass. Actual open-office meshes across three seeds add roughly
39k–47k triangles, with no new/worsened checked parcel overhangs. The same audit
finds30/29/64existing office parcel-box violations, recorded by ID and bounds.
Do not claim full clearance. See [candidate evidence](office-side-facades.md).

## Current decision record — learning reconciliation, 2026-10-13

This record updates the consolidated assessment above with the subsequent trials;
it adds no new runtime, test, capture or acceptance claim.

- **Measured:** compact coverage is already substantial; family/palette counts show
  variety exists but do not measure visible silhouette or frontage richness. Avoid
  a global density increase or random color variation as a default response.
- **Observed:** held-camera comparisons show DOF and style differences. Composition,
  architectural rhythms and material hierarchy remain gaps; stronger blur is not
  their remedy. Actual daily-cycle and moving-focus review remain open.
- **Rejected:** the matched roughness-only trial had no useful neighborhood-scale
  benefit. Preserve that negative result and stop scalar roughness/fill tuning.
- **Candidate:** guarded office ribbon sides have final stills in both styles and
  bounded test/mesh evidence. Finish motion and developing-state checks, then make
  a retention decision without adding variants. See [office evidence](office-side-facades.md).
- **Correctness follow-up:** the mesh audit found 30/29/64 existing parcel-box
  violations across three seeds, with none new or worsened by this candidate.
  Narrow-lot fitting is a hypothesis requiring diagnosis, not a full explanation.
- **Next selection:** after closing the candidate and addressing proven physical
  defects, compare the ordinary neighborhood and wider context against all eight
  dimensions in the consolidated table. Choose the largest remaining evidenced gap.

The [updated goal](living-city-goal.md) preserves the complete living-city objective,
README/specification references and architectural baseline. It requires one bounded
workstream, matched evidence, a stop condition and reassessment after two failed
attempts. Full behavior, audio, camera motion, day cycle, hardware performance and
user artistic acceptance remain open. No obsolete-save compatibility is required.


## Office fitting ordinary construction evidence — 2026-10-13

Measured: ordinary seed0 office6703 on a6.3795m-wide parcel opens after100ticks,
with exact original/restored state equality at each tick. Observed: both Anime
and Miniature20second recordings, ten decoded samples each, show scaffolding and
partial glazing becoming a completed facade/roof without an obvious sampled
fitting discontinuity.50mm,High/WebGPU,1280×720,DOFoff,noon lighting preview;
normal scheduler reached zero geometry queues before recording. This is sampled
lifecycle evidence, not a true day-cycle or full collision/performance acceptance.
Miniature's observed40.1ms rolling longest frame remains unattributed. Matched
baseline and rear assembly visual checks remain the next bounded action; no
further facade variants. See `office-parcel-fitting.md`. Broader urban form,
reference fidelity, audio, camera and browser release gates remain unchanged.


## Office fitting matched opening comparison — 2026-10-13

Observed: matched seed0/tick58300 office6703 before/after in both styles shows
restored side clearance and narrower body/foundation, with existing ribbon glazing
now enabled by its clearance gate. Roof and street frontage remain coherent. Fixed
50mm,1280×720,High/WebGPU,DOFoff,noon preview,rain0.82715786. Numerical production
pose and source hashes are recorded in the comparison metadata. This supports the
correctness correction visually; it does not establish overall architectural or
material fidelity. Rear-equipment inspection remains before retention. No new
facade variants or production edits were made during this comparison. See
`office-parcel-fitting.md` and its four matched captures.


## User review supersedes next-work ranking — 2026-10-13

See [the seven-dimensional review, source reconciliation and acceptance plan](user-visual-review-2026-10-13.md). City-wide composition and depth now lead. No new rendering result or acceptance is claimed.


## City-wide horizon diagnosis — 2026-10-13

Measured: production Overview excludes the horizontal horizon at35/50/85mm in18
poses across three seeds and two stages;35mm can include outer terrain boundary
vertices. These are geometric probes, not visibility/coverage measurements.
Observed: seed0/tick58300 ordinary overview and lower skyline show hills but an
abrupt developed-city transition to empty terrain, plus dominant straight elevated
decks. A missing-mountains diagnosis is insufficient. No production edit yet.
See [evidence and next decision](horizon-framing-diagnosis.md). Continue corridor,
frontage and urban-edge diagnosis; do not substitute background ornament for it.


## Whole-city road hierarchy — 2026-10-13

Measured whole-city three-seed audit finds2.85–3.65%lane share of mature ground-street chord length and about1.57km of20m elevated expressway per seed. Source assigns Compact subdivision cuts to Local; only selected Strips cuts become lanes. This supports testing district-aware secondary-lane generation, with collectors and route semantics preserved. It is not a projected-area or frontage measurement. See [diagnosis and bounded next candidate](city-corridor-hierarchy.md). The newly user-edited concise goal is mirrored unchanged in `living-city-goal.md`; retain experimental evidence here.
