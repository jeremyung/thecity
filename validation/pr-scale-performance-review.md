# PR scale and presentation performance review

PR #12 must not merge in its current form. GitHub reports conflicts with main,
and its history contains large evidence blobs. A follow-up deletion commit or
adding LFS rules now would not remove those historical blobs.

## Review disposition — current checkpoint

- **Scale remains a merge blocker.** The local source/evidence export is preparation,
  not a published clean-history replacement. Do not merge the original branch to
  carry the source split forward: that also carries the evidence history.
- **Movement allocation fix is present in the working source.** Rechecked the
  movement-presentation, visible-infrastructure and actor-presentation suites:
  35 tests pass; TypeScript and whitespace checks pass. This is focused validation,
  not a claim that the whole checkpoint passes.
- **Snapshot bursts were not observed in the recorded nine-case dev16 audit.**
  Browser transport, rebuild and GC costs still need profiling; retain publication
  ordering until a measured replacement preserves its correctness contract.
- **Privacy cleanup is partial at repository scope.** Everyday-city metadata has
  no remaining local Codex paths, and the export sanitizes those paths. A complete
  scan of the eventual replacement commits is still required before publishing.
- **Integration remains isolated.** Main integration work under `.tools` has not
  been promoted into this branch. Resolve remaining documentation conflicts and
  native-test failures, regenerate the split, provide durable evidence hydration,
  and validate a fresh checkout before recommending merge.

Keep this remediation bounded to reviewability, correctness and the reported
performance issue. Further visual features belong in subsequent checkpoints.

## Changes made

- Movement presentation checks numeric per-road/per-platform commit revisions.
  Geometry signatures are still computed at request/commit time, not per frame.
  Equal geometry commits preserve revisions; removal/reinsertion and reset invalidate them.
- Two reusable frame maps preserve the renderer's previous-frame motion and audio
  speed calculations. Actor pose objects are reused; interpolation origins are
  copied when targets change. Frame results are borrowed until two samples later.
- Settled actors do not repeatedly capture support identities. Allocations still
  occur on new movement packets, changed targets, and support transitions.
- Everyday-city study provenance and prompt inputs now use repository-relative
  image paths, including retained originals, instead of personal/session paths.

## Snapshot measurement

`node scripts/audit-presentation-publication.mjs` measures the existing publication
policy using current default WASM. Seeds 87231, 0, 5; starting ticks 0, 10000,
83000; development rate 16; playback speed 1; 600 single-tick turns (30 logical
seconds) per case. Warmup verifies the actual engine tick.

All nine cases produced 60 snapshots, at most two in any one-second window.
Forced geometry publications replaced periodic publications in these samples;
no extra burst was observed. Mature snapshot serialization plus JSON parsing had
p95 18.5–18.8 ms and maxima 20.0–20.9 ms in Node; each mature case serialized
181–185 MB over 30 logical seconds. This is meaningful worker work, even without
an observed burst. Results exclude browser postMessage/structured cloning,
main-thread processing, GC, GPU work, and higher playback speeds. They neither
explain nor rule out the previously recorded 80 ms frame spikes.

Preserve world-before-movement ordering. Do not remove forced publication merely
to reduce counts. A subsequent browser profile should separate serialization,
transport and renderer rebuild cost before choosing coalescing or geometry deltas.
Raw measurement: `.tools/pr-review/publication.json` (local evidence).

## Clean-history split prepared

`python3 scripts/prepare-review-package.py` reads the worktree and merge base,
without changing the index, branch, files or remote. It creates:

- `.tools/review-package/runtime.patch`: engine, web, tests and shared source.
- `.tools/review-package/tooling.patch`: scripts and diagnostic tools.
- `.tools/review-package/documentation.patch`: specs, prose and reference metadata.
- `.tools/review-package/evidence.tar.gz`: raw validation output, concept images,
  generated plan artwork, and fixtures larger than 256 KiB.
- `.tools/review-package/manifest.json`: base/head IDs, file sizes and SHA-256 hashes.

The current worktree is substantially beyond the reviewed PR: approximately
425 MB of evidence is excluded from the source patches. Package counts are in
the manifest. The bundle retains repository-relative paths, so extracting it at
the root restores existing document links and fixture paths. Archive text and
exported metadata redact local Codex paths. Originals remain locally intact.

This is a prepared split, not a completed replacement PR. Patches are against the
merge base, include uncommitted work and isolated experiments, and are NOT a safe
overwrite of current main. Required next steps:

1. Preserve #12 as the historical evidence branch; do not merge it into main.
2. Reconcile the runtime patch onto current main in a new branch and review which
   isolated experiments belong in the checkpoint. Resolve conflicts explicitly.
3. Publish the hashed evidence bundle to an agreed durable download location and
   document hydration before tests. Rust `include_str!` and WASM tests consume
   large fixtures; removing them without hydration breaks tests at compile time.
4. Keep documentation and diagnostic tooling in separate reviewable changes.
   Link the hosted bundle from validation documents; current local links are not
   a substitute for a published download.
5. Validate from a fresh checkout with hydrated fixtures, then open replacement
   PRs. No history rewrite, force push, evidence upload or new PR was done here.

## Validation

Focused movement/infrastructure/actor/publication-order suite: 41 tests pass.
TypeScript and `git diff --check` pass. Tests cover reused buffers, previous-frame
motion, fixed interpolation origins, no per-frame serialization, geometry
replacement/removal/reinsertion, reset, and platform revisions. This focused
validation does not certify the entire dirty checkpoint or resolve existing
native-suite failures documented in `finite-frontage-validation.md`.

## Current-main integration audit — 2026-10-14

Fetched `origin/main` at `fca58ca` (merged PRs #15 and #16). The previously local
main reference was stale at `2c443a9`. The isolated worktree snapshot is under
`.tools/review-integration-main`; inventory is
`.tools/review-package/main-integration.json`. This is an ordinary ignored
scratch directory, not a Git branch and not the running application's source.

Across the 420 packaged paths, 405 applied without overlap and six merged
textually. Nine files initially conflicted. `kit.ts` and `materials.ts` are now
resolved in the isolated candidate, retaining main's signage additions and the
checkpoint's facade, window-response, roof, demolition and material-disposal
changes. Seven files remain conflicted/unreconciled:

- `engine/src/lib.rs`
- `web/src/render/city.ts`
- `web/src/render/train-kit.ts`
- `web/src/render/trains.ts`
- `tests/train-kit.test.ts`
- `README.md`
- `specs/05-rendering.md`

Rail is a semantic conflict: main has the textured four-car 81.8 m consist;
the checkpoint has a three-car kit with engine dwell/door phases, authoritative
curved coach placement and committed-infrastructure gating. Neither entire
version should replace the other. Next investigation: combine main's authored
train geometry/materials with the checkpoint's curve, support and door handling,
then align engine terminal clearance and platform dimensions to that same consist.
Keep all tree/park, season, shrine and signage work already merged on main.

Seven isolated suites pass, 36 tests total: signage, shop-sign atlas, building and
lowrise massing, painted materials, ceramic roofs and roof services. Integration
found a stale three-label assertion in the atlas test; it now checks every label's
shared material identity and resource disposal with the actual label count. This
test correction is also applied to the active working source. The demolition
suite cannot load while `city.ts` contains conflict markers; it remains pending,
not passed. No browser/visual acceptance or full compile is claimed for this
partial integration. The source split must be regenerated after final integration;
these scratch resolutions have not been applied to the runnable branch.

The audit also exposed Vitest discovering the ignored scratch checkout from the
root. `vite.config.ts` now preserves Vitest default exclusions and excludes
`.tools/**` and `.claude/**`, preventing duplicate or unfinished candidate tests
from contaminating normal test runs. Explicit `--root` candidate testing remains
available. This is a test-discovery change, not a skipped production test.

## Train integration progress — isolated candidate

Extracted `TrainPlacement` into the working source and reused it in the isolated
textured four-car fleet. It samples each physical car at its own route-distance
offset, handles reversed road orientation, and keeps the consist order fixed
when travel reverses. The candidate's separate `trainFacing` instance attribute
switches cab lamp direction without rotating the bodies. World reset clears the
cached path. Current three-car rendering retains its prior behavior and doors.

An actual mesh check exposed another mismatch: the textured kit placed wheels
near the 2.95 m car body's edges, outside the checkpoint's 1.067 m gauge rails.
All three car-role tests failed before correction. In the isolated candidate,
wheelset centers now follow `RAIL_CENTER_OFFSET` independently of body width.
The same three tests pass afterward.

Validation on this macOS workspace with Node/Vitest: working source train curve,
door, reversal and kit suites pass (11 tests); TypeScript passes. The isolated
textured fleet, kit, curve/reversal and wheel-fit suites pass (12 tests). The
candidate patch is `.tools/review-package/train-integration-candidate.patch`,
relative to fetched main `fca58ca`; it is a partial integration, not a deployable
city patch. No browser rendering, shader compilation or visual acceptance yet.

Next blocking integration work: the checkpoint reserves a 50 m straight station
approach and a 48 m platform edge, but main's four-car consist spans 81.8 m.
Unify engine terminal inset, rail straight approaches, platform/stair geometry,
reservation clearances and coach door apertures before enabling the new fleet.
The generated side panels currently have baked closed doors; overlaying sliding
leaves on those solid panels would not produce real openings. Preserve engine
boarding/dwell phases and committed-infrastructure admission. Check pantograph,
wire and floor-height alignment, and signed wheel roll under reversal as part
of that integration. Do not treat the passing placement tests as whole-station
or production acceptance. The main source/evidence packages predate these latest
changes and must be regenerated after integration is complete.

## Four-car station geometry trial — not accepted

In the isolated candidate, resolved the engine's train-length conflict in favor
of main's four-car 81.8 m consist while retaining dwell/door phases. Platform
ends and inward stair runs derive from the consist: terminal inset 40.9 m,
platform far edge 82.8 m, gateway stair foot 101.8 m, straight station approach
84.8 m and straight terminal approach 106.8 m. Renderer slabs, tactile strips,
canopies, support positions and corridor equipment exclusions follow those
lengths. Updated the plan identity salt; no obsolete-save migration is added.

Fifteen focused native tests pass (rail alignment/runtime, terminal access,
queue support, doors, named commuter journey and deterministic replay).
Thirteen isolated web tests pass (station meshes, wheel fit, curved four-car
fleet and fleet lifecycle). These validate a partial integration only.

The broader planner suite fails its existing rail lot-retention guard:
18 passed, one failed. On seed 87231 the straight-layout comparison has 2061
lots and the longer curved layout has 1786, below the unchanged 90% threshold.
A controlled build of the SAME integrated source with only TRAIN_HALF_LENGTH
restored to 23.5 passes that guard. The trial remains isolated; do not loosen
the guard or promote it on the strength of the rail tests.

Three-seed exports compare the existing shorter approaches to the longer trial:

| Seed | Short approaches | Long approaches | Unchanged parent outlines | Those with rerolled subdivisions |
| --- | ---: | ---: | ---: | ---: |
| 87231 | 1860 lots | 1786 lots | 15 | 10 |
| 0 | 1924 lots | 1920 lots | 15 | 10 |
| 5 | 2071 lots | 1926 lots | 14 | 9 |

The parent comparison uses exact depth-zero polygon equality, then compares all
subdivision endpoints, kinds, depths, districts and parent polygons for that
coarse cell (ignoring allocated road/line IDs). Changes occur in later rows 4–5,
even where parent boundaries did not change. Source inspection confirms ordinary
subdivision consumes a shared RNG; independent cell streams currently exist only
behind the earlier `frontage-network-candidate` feature. This is evidence for a
bounded generator correction, not permission to accept fewer lots or add density.

Next investigation: make subdivision randomness independent per parent, compare
short and long rail geometry under the SAME stream policy, and require unchanged
parents to keep their street subdivision. Then rerun retention, access, replay
and scene comparisons. Avoid further ad-hoc rail/lot parameter tuning. Real
textured door openings, full city renderer integration and visual acceptance
remain unfinished.

Local evidence: `.tools/review-package/four-car-planner-tests.txt`,
`short-train-integrated-baseline.txt`, and `rail-length/{short,long}-*.json` plus
`rail-length/comparison.json`. The compiled isolated native test binary is again
built from the four-car source; the shorter baseline was a temporary controlled
comparison and its source override was restored in a finally block.

## Independent random streams — isolated technical validation

The isolated candidate now gives each coarse parent its own street/district RNG,
and each geometric parcel block its own alley/lot RNG. Parcel streams hash the
world seed and sorted millimetre-quantized boundary vertices, excluding transient
block/road IDs. The existing frontage-network feature is not enabled. Updated
the generation identity salt; existing save compatibility is not maintained.

The first invariance assertion was too broad: identical outer polygons can have
changed junctions on shared boundary roads. `nearest_junction` correctly rejects
cuts near those junctions. Added test-only input-junction provenance and require
matching polygons, junctions AND district inputs before asserting identical
subdivision. This preserves the clearance rule rather than forcing stale cuts.
The original global-stream implementation failed the initial regression; after
local streams, 26 planner tests pass (four diagnostic exports ignored), including
the unchanged original 90% rail-retention gate. The same 26 tests also pass with
the shorter consist under the identical random-stream policy.

| Seed | Local-stream short lots | Local-stream long lots | Parents with identical full street inputs | Unexpected subdivision changes |
| --- | ---: | ---: | ---: | ---: |
| 87231 | 1809 | 1811 | 9 | 0 |
| 0 | 1789 | 1812 | 8 | 0 |
| 5 | 1871 | 1830 | 7 | 0 |

This establishes street-subdivision locality, not exact parcel-footprint identity.
Additional footprint comparison finds small numeric differences in several
matched parents and one seed-0 footprint with a 1.34 m dimension difference.
Other inputs, including final road and elevated-clearance geometry, have not been
proven equal for those parcels. Do not claim complete lot invariance.

Nor is this artistic acceptance. A one-time change to the random-stream policy
changes ordinary layouts even with the old train length. Total planned building
footprint (square metres, rounded) compared with the current old-stream layout:

| Seed | Current short / old stream | Candidate long / local streams |
| --- | ---: | ---: |
| 87231 | 242313 | 252968 |
| 0 | 236552 | 242821 |
| 5 | 241070 | 230238 |

Seed 5 is a required weak-view comparison, not an acceptable hidden regression.
Counts and footprint totals cannot prove tighter frontage or a better city.

The isolated four-car binary additionally passes seven masterplan growth tests
(one long-run export ignored), including 40,000-tick growth, legal access,
non-overlapping parcels, and restore/replay; four rail-runtime tests pass (one
export ignored). The named long binary is
`.tools/review-package/local-streams/four-car-tests`. Cargo's ordinary scratch
target currently contains the temporary shorter baseline build; source was
restored to 40.9 m half-length in a finally block. Rebuild before using that
ordinary target. Raw comparisons and baseline log live in the same `local-streams`
directory; `compare-local-streams.py` records the controlled experiment.

Next: complete real door apertures/boarding presentation and the city renderer
merge, then test current WASM and matched ordinary/weak scenes across seeds and
states. The generator and four-car station trial remain isolated pending those
checks and user visual acceptance. Do not begin another lot-count tuning loop.

### Textured train doorway integration — geometry foundation

The isolated integration candidate now has atlas-aligned passenger doorway bounds
for cab/trailer/motor cars (four/four/three pairs per side, matching the supplied
atlas). Body panels can be cut around these bounds, and separate sliding leaf
geometry retains the original UVs and body taper. Cab access doors are excluded.
This replaces the assumption that every car can use three uniformly placed doors.
No new texture generation was needed.

Nine targeted tests pass across doorway apertures, existing train geometry, and
rail fit. Raycasts probe nine points per opening on the cut body, then verify that
individual leaves cover their half-opening when closed and leave it when slid.
The body stays below the existing 2,000-triangle budget. This is geometry evidence,
not browser visual acceptance or complete boarding integration.

Apertures remain opt-in in the isolated `buildTrainCar` API; the running root
renderer is unchanged. Next connect instanced leaves to the authoritative door
packet and platform side, preserving stable physical car order on reversal; then
resolve the city renderer merge and check boarding scenes in both styles. Do not
turn on open body meshes without their leaves and interior presentation.

### Textured fleet door state — isolated integration

The candidate TrainFleet now uses cut body meshes plus atlas-matched sliding
leaves. Leaves are merged into one geometry/draw batch per car role, with signed
per-instance opening and per-vertex travel in the shared train shader. Body,
leaves, and a neutral interior floor share each car transform. The floor prevents
an open doorway exposing the track below. This adds three leaf and three floor
batches, independent of train count; capacity remains bounded by consist roles.

`TrainDoorState` consumes the existing authoritative movement packet. Paused saves
bootstrap opening; later coarse snapshots do not rewind movement. Openings are
suppressed away from the platform stop, and platform side is evaluated in each
car's coordinates (including the reversed tail cab). Reset clears stale samples.
Eighteen focused tests pass (six files), including both terminal sides, closing
interpolation, stale-snapshot immunity, departure suppression, reset, shared car
transforms, real aperture raycasts, gauge and curved placement. Targeted TypeScript
compilation of trains.ts and train-materials.ts and their dependencies passes on
this macOS host. These checks do not execute the WebGPU shader in a browser.

Still isolated: city.ts has unresolved integration conflicts, and the runnable
root renderer remains unchanged. Existing legacy `train-doors.test.ts` targets
the old CityTrains API and is not covered by the focused run. Next resolve the
renderer API/committed-infrastructure binding, adapt that integration coverage,
and inspect station boarding in both styles. Verify atlas sill/platform height,
pantograph/wire fit, picking of shader-displaced doors, and actual GPU motion;
the floor is a minimal interior, not a visually accepted complete cabin.

### City renderer integration — candidate builds

Resolved all twelve city.ts conflict regions in the isolated candidate. Preserved
checkpoint atmosphere, queued/committed infrastructure, sidewalks/ground, staged
tower and demolition signatures, movement presentation and audio timing; retained
main's seasonal tree batches, parks, shrine/signage context and updated train
assets. TrainFleet now receives committed rail geometry and the door packet through
the city renderer, with rail-head elevation and existing interpolated movement.
Signage remains part of the combined building signature.

Integration exposed an access-shape mismatch: main's street-tree clearance only
registered the first segment of building entrances, whereas checkpoint entrances
are polylines. Candidate parks.ts now registers every finite segment. A regression
uses a bent entrance whose later segment intersects a previously planted tree;
that tree is excluded. Existing park signatures already include all path points.

Validation on this macOS host: 49 focused tests passed across parks, demolition,
shop signs, movement identity/presentation and door state. Eight additional tests
passed in the matched station-kit/render-startup files (nonexistent filter names
in that command did not run tests). Runtime-only strict TypeScript check passes.
Fresh candidate release WASM built successfully with existing Rust warnings;
wasm-bindgen bindings generated inside the candidate. Production Vite build passes
(1.49 MB main JS, 445 KB gzip; chunk-size warning remains). A current-WASM
restore/deterministic replay test passes (four other tests explicitly skipped by
name filter). This does not establish native/WASM parity or production scheduler
acceptance across the required seeds/stages.

Full test TypeScript remains incomplete: old CityTrains/buildCommuterCoach test
APIs need migration without dropping their invariants. The renderer source itself
now compiles. Next migrate those tests, verify train-to-wire/platform fit, then
run the isolated application in the browser. No screenshot, shader execution or
user acceptance is claimed. Root source/runtime and Git history remain unchanged
by these isolated integration edits. README/spec conflict resolution and clean
PR/evidence publication are still pending.

### Train fit and first browser smoke check

Migrated remaining legacy train tests to TrainFleet, keeping committed-path,
per-car tangent, stable-body reversal, lamp direction, door timing/side/reset and
real-aperture coverage across the current focused suites. Full strict TypeScript
now passes with fresh WASM declarations. Eighteen tests in seven focused files
passed before the subsequent buffer packing; nine affected fleet/state/reversal
tests plus a new buffer-budget regression pass after it.

The original wire-fit assertion caught a 1.0634 m overshoot in the generated
motor-car pantograph. Replaced the disconnected oversized bars with connected
diamond arms and a transverse shoe meeting the existing contact height. The
unchanged wire-height assertion passes; fifteen corridor/kit/aperture/gauge tests
passed. No corridor height was raised to hide the mismatch.

Browser: Codex in-app browser on this macOS host, candidate localhost:5189,
seed87231 founding stage. A real WebGPU startup error reported nine vertex buffers
against the device's limit of eight. Packed static mesh attributes into one
interleaved buffer before per-instance attributes. No new error entries appeared
in the observed post-change startup or miniature switch; earlier timestamped
errors remain in the tab log. Miniature selection completed, depth of field off.
This is a startup smoke check, not train boarding/mature-city acceptance.

Saved screenshot (ignored evidence):
`.tools/review-package/candidate-miniature-startup.jpg`.
Candidate server session20919 and browser tab80 were left for continued review;
revalidate their live status before use. Candidate simulation paused at06:35.
The scene still shows weak open ground, repetitive riverbank trees, and coarse
background treatment. Do not label this a fidelity gain. Next use developing and
mature candidate states to inspect actual trains/boarding and scene composition,
then run matched urban-fabric comparisons. Root runtime remains unchanged.

### Developed browser scene — both styles, not accepted

Generated a fresh candidate seed87231 at development16 through 40,000 ticks using
200-tick WASM steps, verifying the final tick. Save/world exports stay in ignored
review-package. Snapshot contains1006buildings,275journeys, two rail platforms,
and train231 dwelling with door fraction1. This is a newly generated candidate
layout, not a same-world comparison with the running branch.

Imported through the actual browser/worker save workflow. The file-chooser tool
call reported2463seconds elapsed; this is not a reliable isolated app import
measurement, and cannot be attributed wholly to the renderer. Subsequent UI
observations showed chunk queues progressing11→7→5→2→0. Initial empty blocks were
loading, not a justified density finding. Do not use those frames for comparison.
The queue settled with953draws and roughly11.4million reported triangles. Before
fixed quality, the sampled UI showed approximately50–60fps and19–25ms frame p95;
these are local short samples, not a benchmark. Loading samples were much worse.

Captured a paused station scene in Miniature and Anime with identical numerical
pose [150,25,-74] targeting[200,9,-82.81525],35mm/environment lens,14:00preview,
847x761viewport,DOFoff,fixed review quality. Both styles selected successfully and
no new error logs were returned. One screenshot call failed transiently during
the style switch; a later state check confirmed completion and screenshot worked.
Evidence and exact metadata are under `.tools/review-package/candidate-station-*`.

The four-car body renders beside the platform and into the curved corridor; this
camera does not expose the platform-side openings sufficiently to prove boarding.
Do not call doors visually verified. Still visible: wide grassy strips beneath
and beside rail, generic facade repetition, coarse backdrop and conspicuous
straight corridor composition. A single station scene does not assess the full
urban-fabric milestone. Next capture settled ordinary interior/frontage/overview
views for this seed, then equivalent seed0/5developing/mature cases; profile the
large geometry workload rather than adding train detail. Platform-side boarding
verification remains a required integration check. Root runtime unchanged.

### Urban-fabric allocation and geometry evidence

Generated current candidate seeds87231/0/5 at ticks10000/40000/83000 with dev16,
200-tick steps and asserted tick counts. Raw current-save/world files and a compact
summary remain ignored in `.tools/review-package/scene-matrix`; the generator is
`generate-candidate-matrix.mjs`. At83000 all buildable parcels are occupied in all
three seeds. Each has16unbuilt parcels, all protected and none reserved. Building
counts are1795/1796/1814, footprint250065/240158/227730m². These metrics identify
build-out, not visual success. Seed0's mature weather is overcast(.32355), the other
two clear; cross-seed visual review must account for that instead of calling it a
geometry difference. Captured earlier tick40000 is not full build-out despite UI
stage 'maturity'.

Permanent rail gap cause confirmed in masterplan.rs: RAIL_BAND_HALF=11, two local
frontage road centerlines at ±11m, and the whole intervening row skipped during
block subdivision. Candidate curved alignment carries these offsets along the
curve. Station/platform/stair clearance prevents safely lowering that constant
uniformly. Future corridor intervention must distinguish station envelopes from
ordinary track, preserve legal service/access paths and demonstrate scene gains;
this turn makes no density/layout edit. Settled overview observation saved as
`candidate-developed-overview-observation.jpg`; automatic overview moves, so it
is not a matched numerical-camera comparison.

A separate isolated diagnostic builds actual authored geometry for all1006
buildings in the tick40000snapshot, with signage and retained original footprints:
2,977,964triangles total before render passes. Apartments219buildings/1,170,620tri;
homes384/865,108; offices116/378,938; cafes150/294,452; shops137/268,846. Largest
apartment54304triangles at28floors. These are authored geometry counts, not GPU
visible triangles or post-batching draws; they cannot directly apportion the
11.4M UI counter across shadow/reflection/main passes. Report:
`.tools/review-package/building-geometry-inventory.json`. Diagnostic runner under
candidate/tests is isolated tooling, not a product correctness test. It completed
in1.46s; full TypeScript still passes.

Next: inspect existing building LOD/chunk policy against this inventory before
attempting a bounded distance-detail change, since loading/performance impede
multi-scene review. Preserve close balconies/roofs and compare the same saved city
at identical poses before/after; reject if ordinary-distance silhouette or facade
identity degrades. Urban-fabric work remains the primary milestone; do not turn
this into general renderer optimization or further train embellishment.

### Distant window panels — measured and rejected

Current BuildingChunks retains full authored geometry in200m batches, with no
building LOD selection. It bounds source transforms/merges over updates, but that
does not reduce eventual rendering detail. Tested an isolated authoring option
converting urban window recess/glass boxes to outward-facing panels while keeping
window arrangement/materials, slabs, rails, stairs and roof geometry unchanged.
Default path was untouched; no renderer LOD system was introduced.

On the same1006-building world, total authored triangles fell2,977,964→2,649,604
(11.03%). Apartments1,170,620→948,840; homes865,108→804,108;
offices378,938→333,358. Shops/cafes unchanged. This is an optimistic upper bound
because every building used the cheap option; a distance-limited policy saves
less. It is not an11%frame-time claim. No visual gain or performance benchmark is
claimed. The option was removed from the integration candidate; the baseline kit
was restored and full TypeScript passes. Preserve only ignored evidence:
`window-panel-trial.patch` and `window-panel-trial-inventory.json`.

Decision: reject this isolated variant. Its saving is too modest to justify a
new LOD mode plus risk of flat window reveals. Do not continue tiny detail removal
or broad renderer rewrites. Return to urban fabric; keep the large-load/performance
concern open and only revisit LOD with evidence for a substantially better bound.
Kit edits triggered candidate HMR; previous browser scene/camera must not be assumed
preserved. Current-save matrix and captured screenshots remain available.

## Verified local evidence hydration

`scripts/hydrate-review-evidence.py` verifies the archive SHA-256, every member's
size and SHA-256, and exact membership against the manifest. It accepts regular
files only, restricts evidence paths, rejects traversal and symlink destinations,
and refuses to overwrite different local content. Verification is read-only by
default. `--apply` writes missing files only after the complete bundle and all
selected destination conflicts have been checked. Repeated hydration preserves
identical existing files. The manifest must come from the reviewed release;
matching hashes alone do not establish publisher authenticity.

Example after downloading the eventual matching archive and manifest:

```sh
python3 scripts/hydrate-review-evidence.py \
  --archive /path/to/evidence.tar.gz --manifest /path/to/manifest.json \
  --destination . --fixtures-only
# Review verification, then write the missing fixture files:
python3 scripts/hydrate-review-evidence.py \
  --archive /path/to/evidence.tar.gz --manifest /path/to/manifest.json \
  --destination . --fixtures-only --apply
```

Omit `--fixtures-only` to include screenshots, recordings and concept evidence.
No network calls or Git/index changes occur. Interrupted writes may leave a
partial file, which a rerun detects as conflicting; inspect/remove that partial
file explicitly rather than overwriting local work automatically.

Verified the existing prepared bundle's1,280members and hydrated19large fixtures
into an empty ignored directory. Six focused Python tests pass for dry-run/apply,
repeat behavior, conflicts, archive corruption, member corruption, unsafe paths
and symlink destinations (`python3 scripts/test-review-evidence.py`). This tests
local hydration, not a full fresh-checkout build. The prepared bundle still
predates the latest integration edits; it has not been uploaded or assigned a
durable download URL. Regenerate it with the final selected integration source,
then validate a fresh checkout before publishing replacement PRs.

## Reconciled source export and reconstruction

Prepared five separate review patches against fetched main `fca58ca`, using the
reconciled candidate rather than the old merge-base patches: engine42files,
web78, tests90, tooling97, documentation118. Total425changed files, approximately
5.2MB of whole-file content. Each group remains below300files, but this is a
local review split, not independent merge approval or published PRs. Engine/web/
tests have integration dependencies; tooling and historical documentation still
need scope review before inclusion. No large evidence bundle is embedded.

Reconstructed all five patches over a fresh archive of that main revision and
verified SHA-256 for all425resulting files against the export manifest. Found
and corrected an exporter bug first: blanket path-prefix replacement modified
literal strings inside the exporter script. Header rewriting now applies only
to diff headers, preserving payload bytes. This correction is in the working
exporter as well as the isolated source. Additional concept metadata contained
remote attachment session paths; exports redact those as user-supplied references.

Artifacts are ignored under `.tools/review-package/reconciled`; reconstruction
manifest names the checked archive directory. This verifies patch application
and content identity, not a fresh dependency install, WASM rebuild, complete test
run, hosted evidence availability or visual acceptance. Root full native suite
remains live at session43533; poll before reporting a complete result.

Fresh reconstruction follow-up: TypeScript cannot resolve generated public/wasm
bindings before a WASM build (not present in git archive). Hydration preflight
correctly refuses tracked main's older `tests/fixtures/bus-seed5-legacy-queue.json`.
Next add explicit externalization deletions for bundle-owned tracked fixtures
to the review patch set, then hydrate missing files and rebuild WASM. Do not
force-overwrite baseline fixtures or claim this fresh checkout passes yet.

Full working-root native suite completed successfully (session43533 terminal exit0):
test result: ok. 500 passed; 0 failed; 59 ignored; 0 measured; 0 filtered out; finished in 594.23s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
This includes the explicit diagnostic-ignore scope change described above; it
does not mean the rejected experiment passed. Candidate full suite and fresh
reconstructed-checkout build remain unverified.

## Reconstructed-checkout build and fixture externalization

Added a separate externalization patch: delete the two bundle-owned files already
tracked on fetched main (the old bus fixture and texture-budget output), and
ignore hydrated evidence plus explicitly listed large fixtures. This does not
remove their earlier blobs from main history; it avoids introducing further
evidence history. All other source patches remain separate review groups.

Fixture-only hydration initially exposed embedded validation witnesses missing
from tests/fixtures. The exporter now records native include_str dependencies as
`testFixture` manifest entries; the hydrator includes those roles. All23required
files hydrate after verifying1,280bundle members. Seven Python integrity tests
pass, including validation-role selection without restoring images.

In reconstructed-checkout-v2, built release WASM from source, then `npm run build`
(TypeScript+Vite) passes. Native `cargo check --tests --locked` passes after fixture
hydration. Five focused presentation/train suites pass39tests. Their first run
found the car-presentation harness still mocked the old train API; added empty
`pickable` to that mock, preserving car geometry/admission assertions. No runtime
renderer change. The1.50MBweb chunk warning and existing Rust warnings remain.
These checks use cached local Rust/npm dependencies on macOS; they do not prove
a clean network install, full candidate suite, browser performance or fidelity.

Refresh source patches after these hydration/mock changes. No hosted evidence
release, clean Git branch or replacement PR has been published.
