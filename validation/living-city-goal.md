Implement the browser-based Tokyo living city defined by README.md,
specs/README.md and its eight linked specifications.

SOURCE OF TRUTH

Read these before implementation:
- README.md for setup and current implementation status.
- specs/README.md and specifications 01–08 for requirements.
- .impeccable.md and the locked concepts for design direction.
- validation/HANDOFF.md, validation/fidelity-review-current.md and
  validation/pr3-review-corrections.md for checkpoint evidence and limitations.
- validation/living-city-goal.md for the version-controlled copy of this goal.
- The latest "Consolidated learning record and priorities" section (2026-10-13) in
  validation/fidelity-review-current.md for the current dimension-by-dimension
  diagnosis, evidence limits and next decision.

Use specifications as the requirements baseline. Treat historical validation
reports as evidence about particular checkpoints, not proof of current
acceptance. Resolve contradictions explicitly and keep the documents current.
Do not weaken requirements to match an incomplete implementation.

OVERALL OBJECTIVE

Create a beautiful, believable, living Tokyo-inspired city that runs in the
browser: a relaxing audiovisual simulation to watch and explore, without
scores, management duties or game objectives.

Each seeded world grows organically from a small beginning into a dense,
bustling metropolis within finite boundaries. Infrastructure, construction,
population and activity have coherent causes and histories. Mature cities
continue evolving through infill, renewal and business turnover.

The viewer explores, follows people and places, adjusts time, and watches
connected neighborhoods emerge. Camera direction, day/night lighting,
simulated water, spatial sound and lo-fi music make the city rewarding to watch.

ARTISTIC FIDELITY

Achieve the fidelity established by the locked concepts:
- concepts/04-last-train-first-light.png
- concepts/01-little-tokyo.png

Build Last Train, First Light first, then verify Little Tokyo as a selectable
rendering style. Both styles preserve the same city state, geometry, camera
and time.

Treat the concepts as visual acceptance targets. Match their architectural
richness, density, material character, lighting, atmosphere, water and lens
treatment. Preserve believable scale, urban structure and behavior beneath
the stylization. Style differences must remain evident with DOF disabled.

Quality must hold in motion, during ordinary growth and construction, across
camera views and daily lighting changes. Neither a beautiful static scene
nor a functioning but visually inadequate simulation satisfies the goal.

ARCHITECTURAL BASELINE

Follow specs/04-architecture.md and the established implementation:
- Rust compiled to WASM for the authoritative simulation.
- A dedicated Web Worker with fixed-step deterministic updates.
- TypeScript, Vite and React for the application.
- Three.js WebGPURenderer and TSL for rendering.
- Web Audio for spatial ambience and music.
- IndexedDB for local persistence.

Keep simulation, world generation, rendering, camera, audio, UI and persistence
modular through explicit typed, versioned contracts. Maintain bounded work
and state, worker backpressure, compact snapshots, render interpolation and
background-tab pausing.

Use the seeded master-plan generator for new worlds. Realize roads, services
and lots progressively through causal, demand-driven construction.
Infrastructure must become usable before dependent development opens.

Inspect the actual branch and merge state before starting. Incorporate the
reviewed PR #3 corrections if absent; do not repeat completed work.

Keep these architectural choices unless an evidenced blocker warrants a
change. Document the problem, alternatives, tradeoffs and validation before
introducing a replacement. Avoid speculative rewrites.

IMMEDIATE MILESTONE

Follow specs/08-delivery.md. First bring the existing 800 m city to a
beautiful, believable, watchable and measured state before expanding maps.

- Compare current browser output with the locked concepts and maintain a
  ranked list of perceptual gaps.
- Improve the largest gaps in city structure, composition, architecture,
  materials, lighting and activity.
- Exercise normal startup, development and mature-city loading through the
  production scheduler. Do not manually drain queues to manufacture evidence.
- Review founding, developing and mature scenes at dawn, noon, dusk and night,
  including camera motion, narrower viewports and style switching.
- Correct raised-platform passenger placement and physical flag-lot access
  before accepting the affected scenes.
- Preserve simulated water, full-scene anti-aliasing, coherent sound and
  purposeful movement.
- Profile named hardware and browsers. Record loading time, worker latency,
  frame timing and memory. Software-rendered captures establish appearance,
  not hardware performance.
- Address measured bottlenecks with bounded or incremental work while
  preserving determinism and visual quality.

Larger map frames, improved district generation, bounded OSM/PLATEAU
integration and the 2.4 km domain remain subsequent milestones. This ordering
does not reduce the original scope.

CURRENT PRIORITY AND LEARNINGS — reconciled 2026-10-13

The overall objective remains the complete living city. Visual fidelity is a
required part of that experience, alongside coherent growth, purposeful activity,
audiovisual exploration and technical correctness; none replaces the others.

The latest mature waterfront review exposes a structural and perceptual gap:
large transport decks and open water dominate repeated building forms. Full lot
realization does not establish convincing density. Afternoon errands now improve
behavioral continuity, but have not substantially changed the wide-view impression.
These observations are specific evidence, not proof that every neighborhood fails
in the same way. Reassess priorities as new matched evidence becomes available.

Keep urban fabric and neighborhood composition under review; the current primary
decision is architectural and material hierarchy, as specified below.
Before changing dimensions, measure rendered footprint coverage, gross versus net
developable area, street share, block and parcel sizes, setbacks, frontage continuity,
height distribution and infrastructure relationships. Use existing fabric targets
and audit tools; do not replace them with unsupported numerical targets. Distinguish
corridor union area from overlapping width-times-length sums. Road dominance alone
does not prove incorrect carriageway widths or justify shrinking every road.

Use references according to what they establish:
- Locked concepts define artistic acceptance for both styles.
- Photographs inform camera, proportions, architecture, transit and everyday detail.
- Overhead maps inform urban fabric, waterway and corridor patterns qualitatively.
  Without a verified scale and legend, do not infer precise dimensions, elevations,
  terrain slopes or that all green polygons are parks.
- Generated style studies supplement these references; they are not engine output
  or replacements for the locked concepts.

Choose one normally generated neighborhood spanning a local street, crossing and
waterfront (with rail where the generated layout supports it). Establish a readable
subject with foreground, middle distance and backdrop. Improve reusable generation
and rendering rules rather than hand-place a showcase or alter authoritative state
for a screenshot. Diagnose urban form first, then architecture/frontage, landscape
and materials, followed by camera/style refinement as evidence warrants.

Evaluate each bounded intervention against these gates:
- The neighborhood reads as connected inhabited fabric, with believable street
  hierarchy, continuous useful frontage and coherent building families.
- Rooflines, massing and ground floors provide variation before extra texture
  batches or tiny props are commissioned. Taller buildings are not a density fix
  unless justified by district structure.
- Banks, planting, bridges and paths integrate with streets. Water and materials
  remain expressive in motion; do not repeat rejected reflection-gain experiments
  without a new mechanism and explicit comparison.
- Hold seed, tick, camera pose/lens, lighting, weather, resolution and quality fixed
  for before/after captures. Compare both styles with DOF off and on. Thin-lens DOF
  already exists; framing, sharp subject detail and depth separation must improve
  without hiding weak geometry, causing halos or destabilizing moving occlusions.
  Anime and Miniature must remain distinct with DOF disabled.
- Review normal-speed activity at real destinations, crossings and stations.
  Counts and visible liveliness are separate evidence. Preserve legal routes,
  occupancy accounting and saved lifecycles; broader work-derived demand remains
  outstanding. Do not spawn cosmetic crowds or raise caps simply to fill a frame.
- Check the reusable result across at least three seeds and developing/mature
  states, then through dawn, noon, dusk and night. Preserve the full founding and
  narrower-viewport release matrix in spec08; this slice does not replace it.
- Measure normal startup/import to usable and fully visible scenes, growth stalls,
  motion frame times and memory on named hardware/browser. Temporary blank areas
  and slow visible frames count. A settled rolling frame window is not end-to-end
  acceptance. Audio continuity and quality remain required and need explicit review.

Keep one primary workstream with a baseline, bounded change, visual/behavioral
acceptance and stop condition. After two unsuccessful attempts, reassess before
another tweak. Do not expand maps, import city datasets, rewrite the engine or
commission broad asset batches to avoid the current deficiency. Preserve and test
existing behavior, but do not keep optimizing activity counts merely because they
are easier to measure than the perceptual gap. User review establishes artistic
acceptance; no current screenshot or passing test suite establishes completion.

LATEST EVIDENCE AND NEXT DECISION — 2026-10-13

The three-seed footprint audit narrows the structural diagnosis: compact crops
already reach 47.35–49.62% gross coverage, while station crops reach 28.98–34.12%
with 35.36–37.90% open-street union. Preserve successful compact fabric. The
street-adjusted diagnostic is not net developable coverage: rail, water and
protected land remain in its denominator. Do not derive new targets from it.
An unobstructed held station view reveals denser fabric than the waterfront view
suggested. Separate composition, projected screen area and measured urban form.

The production-helper audit found all 103 eligible apartment midrises in three
250 m station crops used one walkup composition despite 32–40 active shops/cafes
per crop. Counts alone do not establish facade variety, useful frontage or footfall.
A bounded terrace-apartment family is retained after three-seed still review,
physical/construction tests and sampled activity/orbit checks. Final seed87231
captures now include the neutral terrace finish. Stop apartment embellishment:
repeated roof/window patterns and weak material/style distinction remain larger
perceptual gaps. Sampled video frames are not continuous-motion acceptance.

Two station-detail experiments were rejected: roof seam/pitch/fascia changes were
too subtle at neighborhood scale, and a covered entrance obscured readable stairs.
Neither remains in production. Preserve the negative evidence and do not repeat
these approaches without a new demonstrated need.

Generic homes of four or more floors now have a centered entrance, with the ground
window opening reserved. Focused tests and a generated home457 browser review in
both styles support retention. This does not establish walking-height traversal
or every parcel variant. Do not change authoritative building use to hide a visual
access defect. The historical gateway-stair boundary failure now has a retained correction and
a three-seed plan-view audit with no boundary or building-envelope overlaps.
Actual city-station exit samples support descent and approach walking. Neither
plan-view non-overlap nor sampled frames prove complete mesh/height clearance.

A low-rise shop canopy correction now has matched specimen A/B, static construction
stage checks and a generated shop3449 review in both styles. Retain its modest
benefit without further embellishment. The cafe3649 view was occluded and does not
establish acceptance; live construction and broader parcel checks remain open.

Daylight glazing now has an explicit material response shared by source and
batched buildings: cool glazing by day, warm interiors by night, separate from
lamps and signs. Retain the candidate on bounded evidence from seeds87231 and0,
both styles, lighting previews and construction specimens. These checks do not
prove continuous motion, true daily occupancy, live construction or seed5 behavior.
Repeated window patterns and limited style separation remain unresolved.

The shop3449 ground audit found the sampled green margin outside emitted ground
polygons and released parcels. It is not evidence of a missing pavement mesh.
Do not blanket-pave residual land: classify its plan ownership, corridor/access
role and protection before choosing a surface. The audit does not establish that
all bare margins are correct or visually integrated.

Then rank work against all of these dimensions, using the latest evidence:
- Urban form and overhead maps: connected blocks, corridor hierarchy, differentiated
  districts, setbacks and frontage; preserve already-successful compact coverage.
- Reference fidelity and architecture: layered massing, rooflines, occupied ground
  floors and believable Japanese transit/infrastructure; fix physical continuity
  and scale before small decorative details or broad texture batches.
- Look and feel: distinct material and lighting response in both locked styles,
  with DOF disabled. Both material paths exist; do not assume a switching bug.
- Camera and depth: deliberate subject/framing, perspective and depth separation,
  with thin-lens DOF supporting readable geometry rather than hiding it. Validate
  moving occlusions and focus transitions, not only blurred still images.
- Ground, water and landscape: coherent street edges, banks, crossings, planting
  and paths; avoid disconnected surfaces and featureless paved margins.
- Living behavior and audio: observable journeys to real destinations, construction
  and renewal, daily rhythms and spatial sound. Lighting previews are not actual
  day-cycle tests. Errands do not establish broader work-derived demand.
- Browser delivery: normal startup/import, growth and camera motion, including
  stalls and temporary blank areas. Recorded loading/growth/orbit maxima remain
  unresolved observations, not hardware acceptance or isolated cause diagnoses.

The station construction-phase and foliage-clearance candidates now have focused
native/web tests, three-seed ordinary WASM phase/save checks, both-style static
captures and sampled live construction evidence. Retain these bounded corrections;
they do not establish every-frame, full physical or spec08 release acceptance.
See validation/station-construction-current.json and the latest fidelity log.

The bounded station-to-street walking correction is retained. Shared
engine-owned approach geometry/width and mitered renderer paving are implemented;
do not repeat that implementation. The production-outline audit covers both
terminals in three seeds at ticks10000/80000 (12 cases), with no checked boundary,
building, parcel, pier, protected-land or river-corridor overlap. This is plan-view
evidence, not terrain-height, road-join or all-object acceptance.

The road-type walking-offset correction is implemented in movement, route
endpoints and building access. Fresh seeds87231/0/5 pass save/movement equivalence
and continuation checks through80000. Ordinary exit probes now cover all330
centerline samples per seed, compared with24 uncovered samples before the fix.
A bounded0.4m station road-end apron now supports the turn without moving the
route. Ordinary road ends and boundary gateways are unchanged. Three-seed plan
probes have no missing center/lateral samples; a production-mesh ray audit checks
5610 center/body-perimeter samples per seed with no missing support or unexpected
height. Heights are0.27m paving,0.30m road and up to the first0.176m stair rise at
the foot. Added-area checks clear building/parcel/station-pier/protected/river
and world-boundary envelopes. This is sampled ground-route evidence, not a
whole-city collision proof.28 focused tests and TypeScript pass.

The final apron has matched Miniature and Anime ground-traversal recordings,
each with 12 decoded samples showing the landing, paved approach and street join
without a visible disconnect. Both start from the ordinary tick83829/live17:49
checkpoint and normal scheduler readiness. Together with the three-seed support
and clearance audits, this closes the bounded correction for retention. Sampled
frames and mesh probes do not establish every-frame or whole-city acceptance.
Preserve the earlier failing probes. Stop station detailing unless new evidence
shows a correctness regression. See the station-road-landing recordings and
contact sheets, station-road-landing-height.json, station-road-landing-clearance.json
and station-road-landing-{87231,0,5}-probe.json.

RETAINED WORKSTREAM: NEIGHBORHOOD FRONTAGE AND LANDSCAPE INTEGRATION

The completed bounded planting review used one ordinary generated neighborhood
and its wider waterfront context. Its matched broad waterfront baseline showed bare hard banks, weak planting
integration and repetitive frontage beside transport and water. The three-seed
production-rule audit now finds no generated trees within 17 m of the water edge
and only 13/11/11 retained candidates in the 17–50 m band. A feasibility-only line
of 82 small trees per seed at a 3.5 m bank offset retains none under existing
clearance and conservative transport checks. Do not implement that failed line or
disable clearance. The subsequent cross-section and foliage-bound inspection established the valid
placement described below; do not restart that completed diagnosis. This is a
placement/development interaction, not proof of a renderer bug or universally low
density. Preserve successful compact fabric. See riverbank-planting-baseline.json
and screenshots/riverbank-planting/baseline.json in validation. The ordinary review
export is .tools/riverbank-review-save.json (seed87231/tick85169); lighting preview
is noon, both-style captures use the same held waterfront camera with DOF off.

A measured bank-tree candidate is implemented at scale1.8 and2.7m bank offset,
using existing meshes and batches. Growth review caught two crown/bridge-sidewalk
conflicts per mature seed; bridge clearance now includes sidewalk and crown width,
with matching broad-phase reach. The corrected candidate retains74new trees in
each mature seed without changing outer/inland retention. All41ordinary checkpoints,
including observed bridge construction stages, pass the checked bridge/rail crown
envelopes. Eight focused tests and TypeScript pass. Earlier failing evidence stays
preserved. Both-style stills showed modest benefit before the crossing correction;
corrected Anime orbit/hold has six sampled frames with coherent bank/bridge edges.
See validation/riverbank-planting.md. These checks do not establish every-frame
stability, curved expressway-pier clearance, controlled cost or artistic completion.
A57ms rolling frame observation during orbit remains un-attributed.

Corrected Miniature motion is now recorded from the same ordinary checkpoint;
six sampled frames support coherent bank/bridge edges. Retain the bounded planting
correction and stop this slice. The41checkpoint audit and both-style sampled motion
do not establish complete release acceptance. Do not add further planting variants.

NEXT DECISION: NEIGHBORHOOD ARCHITECTURAL AND MATERIAL HIERARCHY

A normal Watch composition after the bank review shows dense compact fabric;
a held DOF-off/on pair confirms existing near/far blur with a sharper middle band.
See the watch-neighborhood images and latest reassessment in
validation/fidelity-review-current.md. Repeated rooftop enclosures/window rhythms,
subdued surfaces and a weak subject remain visible. Do not diagnose absent DOF,
insufficient density everywhere or a broken style switch from the wide river view.

Inspect the building-family distribution and light/material response of this
ordinary neighborhood, then choose one scene-scale intervention with a matched
baseline. Do not restart tiny station/apartment embellishment, blanket-pave gaps
or use stronger blur to conceal repetition. Test whether framing/lighting alone
accounts for a perceived improvement before attributing it to new geometry.
Keep full day-cycle, construction visuals, audio, hardware and user artistic gates
open. Current Watch appearance is evidence, not acceptance of camera direction.

The first bounded roughness-only trial is rejected: matched deterministic station
views in both styles showed no useful neighborhood-scale benefit. Original kit
materials are restored and all15focusedtests pass. Preserve the negative evidence
in validation/neighborhood-material-response.md. Do not continue scalar roughness
or ambient-fill tuning. Reassess facade/roof composition and subject hierarchy
before selecting a different visible mechanism. This is one unsuccessful attempt,
not acceptance of the existing materials or completion of the workstream.

An office side-facade candidate now carries existing ribbon glazing onto side
walls with >=0.24m clearance, replacing residential window grids there. Initial
station comparison shows a visible architectural distinction. TypeScript and
15focusedtests pass; actual meshes for173/173/211offices across three seeds have
no new or worsened checked parcel overhangs. See validation/office-side-facades.md
and its report. Final guarded stills in both styles show a modest visible benefit.
Both-style ordinary motion now has six decoded samples per clip supporting
coherent facade bands across orbit. This is sampled evidence, not every-frame
stability. Finish developing/live construction review before retention or another
variant; mature motion does not establish that gate. The audit also exposes30/29/64pre-existing
parcel-box violations; their IDs/bounds are recorded. Investigate narrow-parcel
body/detail fitting as a correctness follow-up, not as evidence of full clearance.

For the architectural/material intervention, follow these gates:
1. Capture a reproducible ordinary neighborhood baseline in both styles, recording
   seed, tick, numerical camera pose, lens, lighting, weather, quality and resolution.
   Include a wider context view and DOF-off geometry views. Existing qualitative
   Watch images lack an exported pose/tick and must not be treated as exact baselines.
2. Audit the visible production building families, roof silhouettes, facade rhythms,
   occupied frontage and material/light response. Compare framing and lighting
   controls separately. Distinguish generated repetition from camera selection,
   occlusion, intentional open space and shading.
3. Select one bounded reusable correction with demonstrated neighborhood-scale
   benefit. Architectural/material hierarchy is the leading hypothesis, not a
   predetermined implementation. State the expected visible difference before coding.
4. Compare the same checkpoint before/after in both styles, then check at least
   three seeds, developing/mature states and ordinary motion. Review actual dawn,
   noon, dusk and night separately from lighting previews. Record rendering cost.

Acceptance for this slice: a more legible inhabited neighborhood with coherent
building families, varied roof/facade rhythms and readable material hierarchy at
normal Watch distance. The improvement must survive matched framing/lighting and
DOF-off review, preserve style distinction and retain legal access and clearance.
Current saves, deterministic behavior and normal scheduling must remain correct.
Reject changes visible only in specimen close-ups, changes dependent on stronger
blur, arbitrary color noise, blanket paving or a hand-arranged showcase. After two
unsuccessful attempts, re-rank the deficiency before adding variants or props.

Keep the multidimensional scorecard in the latest consolidated learning record
current. Classify each claim as observed, measured, hypothesis or unverified;
record its checkpoint and scope. Retained corrections are not full acceptance.

Keep retained apartment, entrance, canopy and glazing changes bounded. Broad texture
batches, larger maps, external city imports and architectural rewrites require a
specific evidenced need; they are not substitutes for this comparison. Camera and
lens work remains required, but existing thin-lens DOF must support sharp subject
detail and foreground/middle/background separation rather than mask weak geometry.
Maintain other dimensions as acceptance obligations, not simultaneous speculative
workstreams. Re-rank when a measured correctness or delivery blocker takes precedence.

Use validation/urban-form-baseline.json, urban-form-street-adjusted.json,
station-access-envelope-audit.json, station-architecture-mix.json,
station-architecture-candidate.json and fidelity-review-current.md as checkpoint
evidence. Continuous motion, true day-cycle behavior, construction transitions,
audio, narrow viewports, named-hardware performance and user artistic review remain
open. No focused test, generated style study or retained candidate satisfies the
complete spec08 release matrix.

CURRENT EXECUTION ORDER — learning reconciliation, 2026-10-13

1. Close the existing office side-facade candidate with developing/live construction
   checks. Final both-style stills and sampled ordinary orbit review are complete; do not repeat
   them without a changed candidate or a specific evidence gap. Retain or reject
   the bounded change, then stop facade variants.
2. Diagnose the recorded baseline parcel overhangs using actual body/detail bounds.
   Narrow-lot fitting is a hypothesis, not an established cause of every violation.
   Correct proven physical defects before further decorative work. Preserve
   authoritative parcel/use data and verify affected construction and access.
3. Reassess the same ordinary neighborhood and wider context against concepts04/01
   across urban form, architecture/frontage, material/style, camera, transport and
   landscape, activity/audio, and browser delivery. Select the largest remaining
   evidenced gap; do not automatically choose another building detail.

Maintain a short current decision record separate from historical experiment logs:
what is observed or measured, what remains a hypothesis, the next bounded change,
its visible/behavioral acceptance test and stop condition. Reference images and
unscaled overhead maps guide qualitative judgment, not invented dimensions or
terrain data. Generated comparison images cannot stand in for browser evidence.
Do not optimize a single frame at the expense of normal growth, connected journeys,
current-save replay, motion stability or browser cost. The full spec08 release
matrix and user artistic acceptance remain the completion gates.

WORK SELECTION AND GUARDRAILS

Maintain one primary workstream. Each intervention must identify:
- A concrete deficiency in the living-city experience.
- Baseline visual, behavioral or performance evidence.
- A bounded change and explicit acceptance criterion.
- Conditions for abandoning the approach.

Prioritize the weakest part of the complete experience. Do not repeatedly
improve whichever subsystem is easiest to measure.

Every workstream must advance coherent growth, believable activity,
concept fidelity, audiovisual exploration, correctness or measured browser
performance. Defer unrelated research, features and broad refactors.

Evaluate changes through short feedback loops using matched browser captures,
motion recordings, tests or profiles. Do not stack speculative changes before
evaluating them. After two unsuccessful attempts, reassess the approach.
Revert experiments that add complexity without demonstrated benefit.

CLEANUP AND SAVES

Follow issue #4 for a separate, reviewable cleanup of obsolete planners.
Audit dependencies and preserve required renewal behavior.

Retain or replace generic regression coverage for travel, buses, rail,
business activity, connectivity, bounded state, determinism and current-format
save/resume. Never delete a failure merely to make the suite green.

Backward save compatibility is not required. Do not build migrations for
obsolete formats. Reject incompatible saves explicitly and offer a fresh world.

Current-format saves must validate plan mappings, entity identities,
geometry, frontage/access and infrastructure progress. Replay tests must
compare the original live engine with a restored copy, including construction
in progress.

DELEGATION

Use Luna (gpt-6-luna) subagents for bounded implementation and testing tasks.
Give each relevant specifications, owned files, agreed interfaces, acceptance
criteria and required verification.

Delegate independent tasks supporting the current workstream. The primary
agent owns architecture, integration, artistic judgment and final verification.
Independently inspect and test every contribution. Delegation must preserve
quality and scope.

ACCEPTANCE AND DELIVERY

Apply the complete acceptance criteria in specs/08-delivery.md.

Verify deterministic replay, current-save equivalence, infrastructure-first
growth, connectivity, bounded long-running state, worker backpressure,
rendering stability and audiovisual continuity.

For visual comparisons, record seed, tick, camera, style, weather, lighting
and DOF settings. Confirm geometry becomes visible through normal scheduling.
Review motion as well as still images.

Report failures and timeouts accurately. Distinguish diagnostic reruns from
a passing default suite, and measurements from assumptions.

Keep README, specifications, validation reports and handoff documents current.
Deliver a runnable application, setup instructions and an accurate account
of remaining limitations.

Completion requires the full living-city experience, fidelity to the locked
concepts and technical acceptance. No single dimension substitutes for another.
User review establishes artistic acceptance; tests and measurements support
technical claims.

Follow the current goal status and explicit user pause/resume instructions.
Updating this objective does not itself change execution status.
