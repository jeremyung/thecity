# Engineering checkpoint — 2026-10-10

This is a working checkpoint, not completed visual or live-growth acceptance.

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
