# Movement support candidate — 2026-10-12

## Historical fixture regression resolution

The26 native failures reported below are resolved without weakening validation.
Static derived test inputs now live under
[`tests/fixtures/access-joins/`](../tests/fixtures/access-joins/README.md).
Their manifest records6,648 corrected joins across17fixtures and two active-path
progress adjustments. An inverse-edit check proves all unlisted state unchanged;
all17historical source files were independently verified byte-identical to HEAD.
Existing scenario assertions remain, including moving-actor continuation and
long-running growth/replay tests. Native release rerun: **437passed,0failed,
53ignored**. No runtime save migration was added.

Frontend run initially passed415tests and failed2 because a dynamic fixture path
still pointed to the historical captures. That reference has been corrected;
the final frontend run passes **417 tests across77files**. Production build and
diff check pass; the existing large-chunk warning remains. Evidence:
[native](access-route-native-release-final.txt), [frontend](access-route-web-final.txt),
[build](access-route-build-final.txt). This is not browser visual acceptance or
production integration of the support packet.

## Saved route joins and delivery returns

The engine now validates both saved connector joins: origin tail to route start,
destination head to route end, within 0.01 m. Finite but disconnected geometry
rejects explicitly at restore; paths are not silently rewritten. This is a join
invariant, not proof that every interior segment clears all obstacles.

The first fresh-world restore audit exposed delivery returns reusing a reversed
inbound path. Because travel direction changes the traffic lane, reversal alone
left the connector in the wrong lane. Returns now regenerate their origin access
through the existing building path builder for the outgoing route.

Fresh WASM seed87231/development16 through40kticks now validates/restores each of
50 sampled checkpoints without repair; saved JSON and movement packets match.
All7,283 sampled actor observations have resolved support. Two native tests from
ordinary current-world growth cover malformed-origin/destination rejection,
100-tick live/restored save+packet equality, and natural delivery-return joins.

The first native `bus_` filter failed two of19 tests. One manually constructed
road-split test connector did not join its route; its fixture now uses the real
route endpoint while retaining the split/position/progress assertions. The
historical seed5 reverse-bus fixture had50 connector endpoints displaced1.25m by
old street-width assumptions. Only those endpoints were updated (99 coordinate
values; semantic comparison confirms all other state is unchanged). The reverse
queue, boarding and completed-return assertions remain. The rerun passes19/19.
No production save migration or compatibility bypass was introduced.

The earlier 783-pose oracle report below is historical: its malformed saved
paths can now reject. Do not present it as a passing current restore corpus.
Full native release validation completed: **411 passed,26 failed,53 ignored**.
[Raw output](access-route-native-release.txt). All26 reported failures occur
while historical fixture restores encounter disconnected connector endpoints.
The validator is not accepted as a clean checkpoint yet. Next update the affected
regression inputs without changing their topology, actor identities, scenarios
or assertions; preserve historical visual/economic evidence separately. Do not
add a runtime migration, bypass validation for legacy data, or delete coverage.
Browser motion/art review, movement packet integration and allocation/revision
work remain outstanding. Goal active.

## Transit support and endpoint correction

The latest candidate resolves directional bus queues using the same helper that
places them. Rail queues use the specific station–gateway service axis, so an
additional rail branch no longer falsely makes platform support ambiguous.
Access support includes both the walking route edge and the building frontage,
bus queue edge, or platform. Transit connectors must match the authored path
and route endpoint; the lookup does not choose a nearby road.

This exposed an existing behavior bug: bus boarding access began at the start
of the walking route, while bus alighting and pending-alight recovery connected
to the route's far end. Three endpoint arguments now select the correct join.
New journeys therefore connect at the stop, rather than taking an incorrect
cross-block shortcut before or after route travel. Saved paths already carrying
the old error are not rewritten by this fix; retained historical fixtures are
not proof of current path validity.

`node scripts/audit-movement-support-fresh.mjs` exercises ordinary seeded growth
(seed 87231, development 16, 40,000 ticks, sampled every 800). It records **7,283
actor observations, zero unresolved support**, matching packet/legacy poses and
validating referenced roads, endpoints and platform layouts. See
[fresh corpus](movement-support-fresh.json). This is one seed and sampled ticks,
not exhaustive journey, multi-seed, renderer or motion acceptance.

The old 15-sample corpus still preserves all 783 baseline poses. Its unresolved
count is now **27**, down from 55. Witnesses include saved malformed bus paths;
do not treat successful old-save restore as validation of those paths. Remaining
old-fixture cases need classification before any production visibility gate.

Latest checks: seven movement-support native tests and two transit endpoint
regressions pass. The endpoint regressions exercise scheduling, direct alighting
and deferred rail alighting through real engine methods. Twenty-one frontend
decoder/visible-infrastructure tests pass. The native `bus_` filter also passes
all 19 selected tests, including named journeys, capacity, reverse service and
save/replay. WASM rebuild and diff checks pass.
The earlier empty-fixture failure and cancelled full-suite runs below remain
historical facts, not current passing full-suite evidence.

The packet remains unwired. Next address current-save path validation, legacy
allocation measurement and revision/platform commit ordering, then exercise the
actual presentation gate through browser growth. No artistic acceptance claimed.

## Earlier packet checkpoint

The active deficiency is presentation continuity while infrastructure batches
are staged: an authoritative actor can advance before its supporting road or
platform is visible. Building admission already checks committed road geometry;
movement needs equally precise support and snapshot ordering.

The Rust candidate emits a versioned integer packet containing pose bits and
road/platform IDs in one traversal. The TypeScript decoder validates record
lengths, finite poses, identities and support flags. It is **not wired into the
production worker or renderer**. Unresolved support remains explicit.

## Independent evidence

`node scripts/audit-movement-support.mjs` compares rebuilt WASM against movement
output captured before the candidate build. Fifteen samples across three local
developing/rail checkpoints contain 783 actor observations. All poses match the
old output; decoded candidate poses also match legacy output including float
bits. The JSON oracle cannot preserve signed zero, so the separate packet/legacy
comparison and decoder test cover that distinction.

[Corpus report](movement-support-corpus.json) records WASM, baseline and fixture
hashes. The local fixture/oracle files are ignored, so reproduction requires
retaining those files; this is not a portable clean-checkout fixture suite.

55 observations have unresolved support, including bus queues and passengers
walking from transit. These are repeated observations, not 55 unique people.
Resolved support references existing open roads with endpoint nodes and actual
platform layouts. Expressway `connected=false` is intentional: that flag belongs
to the ground-access graph. Expressway continuity uses its separate route check.
The first audit incorrectly required that flag for expressways and failed on
road 1881; the audit now follows the actual engine contract.

Review also found a real candidate defect: bus support duplicated edge selection
without the direction-sensitive junction tolerance used by `bus_pose`.
`bus_pose_with_edge` now returns provenance from the same calculation as the
pose. A regression checks both directions at a junction and ±0.0005 m, including
opposite selected edges at the exact junction.

## Checks and limits

- Four focused native tests pass: encoding, pose/support checks, current-format
  save/resume over 24 ticks with a moving test traveler, and the bus junction.
- Twelve decoder tests pass; TypeScript and diff whitespace checks pass.
- WASM rebuild succeeds, with existing warnings. Corpus rerun preserves pose
  parity; unresolved count remains 55.
- The initial save/resume test had no actors and failed its nonempty assertion;
  the corrected controlled fixture includes a valid moving traveler.
- Three accidentally duplicated full native runs were confirmed live and
  terminated. They are cancellations, not full-suite passes.

No browser, visual or performance acceptance is claimed for this candidate.
The legacy path now builds intermediate rows: allocation/latency must be measured
before integration, or the extra allocation removed without duplicating poses.

## Next bounded steps

Resolve transit access and bus queues from their authoritative path/edge
selection. Do not infer support from nearest roads or hide ordinary passengers
indefinitely. Add snapshot revision/epoch ordering so metadata cannot authorize
a newer pose against an older geometry version. Include platform geometry in
commit identities, preserving last valid presented poses for actors and follow
cameras. Only then wire the packet and test ordinary growth, style switches and
restore through the production browser scheduler. Abandon the gate approach if
it cannot preserve bounded, coherent movement without prolonged visible stalls.

Original ground-striping camera/motion validation and locked-concept fidelity
remain open; this work does not close either visual gate.
# Visibility dependency gate — 2026-10-12

## Production presentation integration

Protocol v5 now transfers the integer support packet through the existing
two-buffer pool. The client validates word count, epoch and presentation token,
decodes before recycling, and requests resync on malformed/mismatched data.
`MovementPresentation` retains one latest target and one displayed pose per actor.
New actors await committed dependencies; existing actors freeze at their last
rendered pose while support is unavailable. Retention ends if that exact committed
geometry changes. Interpolation holds both source and destination dependencies;
retired actors and reset worlds clear their state. A paused packet is reconsidered
after geometry commits without needing another worker message.

The renderer now sends interpolated visible positions to the director each frame
before camera update. Withheld IDs suppress structural train/resident fallbacks.
Spatial audio samples those presented poses. Bridge elevation and train platform
metadata use committed road batches. This removes the legacy float-row production
publication path, but packet allocation and per-frame dependency costs still need
profiling; no performance win is asserted.

Validation: delegated transport tests 28 pass, reviewed by root. Focused actor,
presentation, follow fallback and camera suites: 57 pass. The full frontend run
reported **442 passed / 3 timed out**, in lowrise-kit, tower-kit and wasm tests.
Those three suites subsequently passed **22 tests** in a diagnostic rerun; this
does not turn the failed full run into a pass. Final build/TypeScript pass with
the existing large-chunk warning. Raw logs are `movement-presentation-web.txt`,
`movement-presentation-rerun.txt`, `movement-presentation-camera.txt`, and
`movement-presentation-build.txt`. No engine code changed this integration turn.

Browser: fresh seed87231 via normal startup, development16×/simulation8× grew to
323 buildings by day0 18:36. Miniature, High detail, WebGPU, DOF off, 50mm,
1280×720. Pausing let all four visible geometry queues drain naturally. The
loading sample reported frame p95 180.4ms / longest230.9ms; a settled sample
reported p95 14.7ms / longest22.1ms. These were uncontrolled rolling UI samples,
with tests/build work also running: neither a benchmark nor growth acceptance.
Console query returned no warnings/errors at that point.

At simulation1×, followed the shuttle from terminal to In Transit. The retained
[Miniature capture](screenshots/movement-presentation/train-follow-miniature.png)
is day0 19:22, with six passengers, using the actual follow camera. No exact tick,
weather or camera transform was exported; this is a sampled smoke check, not a
matched fidelity/motion comparison. A later source-comment edit triggered Vite
reload during the Anime comparison, so that comparison was not completed. Tab15
is now a fresh Anime settlement paused at07:33. Next verify matched current-save
scenes and measure the worker publication/actor sampling costs during growth.

## Structural ordering follow-up

`Engine::presentation_revision()` and the snapshot now share an exact string
token combining the existing access generation with station visual threshold,
ready and online phases. The worker compares it before movement and force-posts
a matching world when it changes, including during normal 500 ms throttling.
Initialization and rollback record the world token; an epoch reset clears it.
On access-generation exhaustion, the token uses the tick so changing simulation
steps cannot silently reuse an exhausted generation. This is a publication
ordering prerequisite, not the completed actor visibility integration.

Six native tests pass, including expressway completion through the real caller,
12k ticks of ordinary seeded growth and restore, phase transitions, exact u64
encoding and exhaustion. One initial test incorrectly expected development rate
zero to prevent reservation changes; it now isolates clock/weather writes while
the separate real-growth audit remains. 65 focused frontend tests, TypeScript,
WASM and web production builds pass (existing Rust warnings and web chunk-size
warning remain). Access generation is conservative: reservation changes can
cause extra world publication. Measure that cost during browser integration.
The rebuilt WASM fresh corpus also passes: 50 checkpoints through 40k ticks,
7,283 actor observations, zero unresolved support. Getter/snapshot token equality
and restored/live token equality are now checked at each sample. This is one
seed's sampled coverage, not all-tick or browser acceptance.

The shared infrastructure gate now captures exact rail-platform layout identities
alongside road identities. Candidate movement support is accepted only when it is
resolved, nonempty, and every dependency matches the requested and committed
geometry. Open expressways may be outside the ground connectivity graph. Road
identity includes lanes, and the completed-road batch signature now includes
lanes too, so a changed identity actually schedules a replacement commit.

Root reviewed the delegated gate and added renderer integration tests: platform
replacement and lane changes block support until normal staged batch completion.
Focused gate, road-batch and packet-decoder suites: **44 passed**; TypeScript
also passes. These tests
explicitly drain staging to verify commit semantics; they are not browser timing
or ordinary-growth acceptance evidence.

The worker packet is still unwired. The initial inspection incorrectly inferred
that expressway opening bypassed the access generation: its `complete_project`
caller advances that generation after `masterplan_complete_project` returns.
The actual missing dependency is the station's visible construction threshold
and readiness. A movement packet must be associated with its structural world before
the new predicate is used; otherwise reused IDs could incorrectly pass against
an older requested world. Next resolve that ordering, then integrate last-valid
actor presentation and camera positions together. Do not claim the visible
movement defect fixed from this predicate alone.
