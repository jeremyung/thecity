# Incremental preparation for interior-access searches

## Recommendation

Keep the current offline search and the production planner unchanged while introducing a serializable search job in a separate `access_search` module. The job should process one bounded stage at a time, with all cursors and frozen input persisted. It must not decide or mutate growth by itself: when preparation and exact evaluation finish, the existing live admission routine remains authoritative for geometry, demand, project capacity, and reservations.

The latest reproducible diagnostic baseline is the seed-87231 save at tick 242000: 2,069 descriptors, 44 pages of at most 48, one exact and face-contained proposal, and successful admission/validation/canonical restore. The earlier 2,061/43 figure is superseded; current files do not establish which interim code or input change caused the eight-descriptor difference, so it should not be used as a separate behavioral claim. The search is still test-only and a one-face discovery is not a density result.

## Separate the work into resumable stages

Use one optional `AccessSearchCheckpoint` in engine state, with `#[serde(default, skip_serializing_if = "Option::is_none")]` so old saves remain unchanged. A minimal shape is:

```rust
struct AccessSearchCheckpoint {
    schema: u8,
    captured_tick: u64,
    captured_graph_revision: u64,
    stage: AccessSearchStage,
    discovery: DiscoveryCheckpoint,
    frozen: Option<FrozenAccessInputs>,
    prepare: Option<PrepareCheckpoint>,
    descriptors: Vec<AccessGeometryCandidate>,
    evaluate_cursor: usize,
    selected: Option<AccessGeometryCandidate>,
}

enum AccessSearchStage {
    CollectLocalGraph,
    CheckPlanarity,
    TraceFaces,
    SampleFaceTargets,
    FreezeSelectedInputs,
    PrepareDescriptors,
    EvaluateCandidates,
    ReadyForLiveAdmission,
}
```

`DiscoveryCheckpoint` owns deterministic cursors for graph collection, edge-pair checks, half-edge tracing, and target-grid samples. It returns one selected face and target set; it does not also prepare every road candidate in the same tick. `FrozenAccessInputs` contains only the selected face, its ordered boundary hosts, candidate nodes, parcel envelopes, and selected unserved targets. `PrepareCheckpoint` contains host/split/heading/length cursors and any small per-split intermediate needed to resume without rescanning an earlier prefix. The exact field layout can change before implementation, but each stage must have an unambiguous cursor and bounded intermediate data.

Do not serialize derived collision contexts or a whole cloned `Engine`. They are rebuilt from the frozen geometry when needed. For exact proposal evaluation, the follow-up implementation should either freeze/rebuild one bounded context per job and process proposals incrementally, or make the evaluator itself resumable. Calling the existing 48-proposal page evaluator repeatedly is not sufficient if every page reconstructs a context and scans all candidates synchronously.

## Work quotas and nested bounds

The quota is deterministic simulation work, never elapsed CPU time. A tick should debit primitive work at the innermost loops, yield when its quota is exhausted, and resume at the exact next item. A “candidate examined” counter alone is not a bound: one candidate can contain thousands of geometry comparisons.

| Stage | Current structural maximum | Work that must be charged |
| --- | --- | --- |
| Local graph capture | At most 512 nodes and 512 ordinary edges for `road_faces`; caller limits boundary hosts to 24 for this search | Each copied/validated node and edge; each stable-order insertion/comparison if input order cannot be relied on |
| Planarity and face tracing | `road_faces` currently caps at 131,072 edge-pair tests, 1,024 half-edge steps, 524,288 adjacency-search steps, and 512 output faces | Each pair test, half-edge advance, adjacency lookup/step, and output-face validation. The current `trace_faces` call is synchronous; it needs a resumable internal cursor before use in a tick |
| Target discovery | At most 12 retained points, but the existing grid discovery can inspect many cells and compare each with geometry | Each grid cell, each parcel/road clearance query, each node-spacing check, and each top-12 insertion/comparison. If spatial indexing is added, index build and query work also count |
| Frozen-input capture | Up to 24 hosts, 5,000 nodes, and 4,000 parcel envelopes | Each visited world record and each canonicalization comparison. Reject over-cap data before freezing; do not silently truncate and then treat the partial geometry as authoritative |
| Descriptor preparation | At most 24 hosts × 4 split points × 69 headings × 4 lengths = 26,496 descriptors | Host interval projection over nodes/parcels; split/target/blocker checks; every ray sample and polygon-edge test; each length endpoint check and segment sample. In the pessimistic shape, a 60 m endpoint can sample 60 points against up to 64 face edges (3,840 edge tests per descriptor). The current preparation counter does not include these nested operations and is not a CPU-work guarantee |
| Exact evaluation | 2,069 current-fixture candidates in the exhaustive diagnostic, paged at 48; page size is not a bound on the whole evaluator cost | Collision-context construction and each node/parcel/road/project test for every proposal, lot, and candidate. Cache only against the frozen source revision; live admission still rechecks current world state |

The current descriptor cap is 26,496. The diagnostic’s `preparation_counted_units` is a proxy (split prefilter bundles plus endpoint checks), not the work quota proposed here. Charge ray and point-in-polygon loops explicitly. Depending on the implementation, the full worst-case endpoint containment portion alone can approach 102 million polygon-edge tests, so the work must be spread over many fixed ticks; it cannot be hidden behind a nominal 48-candidate page.

Face tracing has a stricter 512-node/edge cap than the full city. A production caller must provide a real, complete local graph window with every intersecting edge included. Taking the first 512 global roads or nodes would fabricate faces. If a suitable complete window cannot be found within a bounded, persisted discovery search, the job should end as `UnsupportedWindow` without affecting growth.

## Determinism and persistence

Canonicalize inputs once using stable keys: faces by canonical `RoadFace.identity`; hosts by road ID; nodes by node ID; parcel rectangles by `(x, z, width, depth, rotation)` using `total_cmp`; targets by `(x, z)`. Reject non-finite values and duplicate stable IDs before work begins. Keep the existing scalar arithmetic order and exact candidate ordering so page concatenation reproduces the current diagnostic sequence. No RNG calls are needed.

On restore, validate the schema, stage/payload pairing, all cursors, capture tick, graph revision, finite geometry, canonical order, uniqueness, and per-array caps before rebuilding derived runtime data. Enforce a serialized-size ceiling for the checkpoint as well as record-count caps. Persisting all 26,496 descriptors is simpler and makes direct paging exact, but it can add roughly half a megabyte of binary candidate data (and more as JSON); measure actual encoded save size. A later optimization can persist the scalar preparation cursor and compact intermediates instead, but only after proving byte-identical descriptor order across arbitrary quota boundaries.

Save/resume equivalence must hold when a save occurs after any work quota, including mid-pair-test, mid-ray, mid-descriptor, and mid-page. Never restart a job at cursor zero on restore. The job is part of the authoritative save, like the district measurement checkpoint; the discovery job must not depend on wall time, rendering cadence, or asynchronous completion order.

## Freshness and admission

Freeze the input geometry at the job’s capture boundary. If the road graph revision changes during a job, cancel it deterministically and schedule a later retry; a host split or newly opened route can invalidate the face topology. Do not restart immediately on every unrelated parcel/building edit. Instead, continue against the frozen face and geometry, then recheck the candidate’s host, face/access, parcel and road clearances, project capacity, current demand, and target usefulness against live state in the existing admission call. If a new building or parcel blocks the result, admission rejects without allocating IDs or changing RNG. Record a stable rejection reason and use a bounded cooldown to avoid redoing identical work every tick.

Admission must not trust a frozen proposal as a reservation. Only successful existing admission creates the pending interior-access plan and its real reservation. A stale face or no-longer-unserved target should be rejected; no candidate may force removal or relocation of an existing entity.

## Smallest implementation sequence

1. Add a pure `access_search` checkpoint and incremental descriptor-preparation state machine over an already frozen `RoadFace`, hosts, nodes, parcels, and target list. Give it `advance(work_budget)`, `checkpoint()`, and `restore()`; do not connect it to growth yet.
2. Add a golden fixture test comparing one-shot preparation with incremental preparation at quotas of 1, 7, 48, and an irregular repeating sequence, serializing/restoring at every boundary. Concatenated descriptors must equal the current 2,069-entry sequence exactly, with no duplicates or omissions.
3. Separately make local graph capture/face tracing/target sampling resumable. Keep these discovery cursors outside the descriptor preparer, so each phase has a measurable budget and can fail closed on caps.
4. Refactor exact evaluation to consume a bounded candidate cursor using one frozen collision context, then hand the winner to live admission for all current gates.
5. Only after the pure stages pass same-target save/replay tests should the engine schedule the job. Compare baseline and candidate on multiple seeds, including road/building outcomes and rendered coverage; retain the current density acceptance gate.

Required tests include golden descriptor sequence, quota partition invariance, save/restore at every stage and cursor boundary, shuffled input normalization, duplicate/non-finite/over-cap rejection, graph-revision cancellation, stale parcel rejection without ID/RNG mutation, no-op under unsupported/non-planar windows, exact evaluation work ceilings, and same-seed save/resume replay. Native and WASM replay must each be tested within their own target; cross-target identical geometry is not assumed.

## Limits of this design

The current success is one mature seed and one face. The selected access branch passes the shared proposal/admission/restore path, but no result shows that the incremental search improves density or visual quality. `road_faces` can reject non-planar legacy graphs, and the current candidate geometry only handles straight one-branch roads. This design does not introduce doglegs, move buildings, bypass setbacks, or relax demand/collision checks. It also does not set Tokyo-wide coverage targets; it only makes expensive geometry work resumable and deterministic.

## Current composition gate (2026-10-10)

The composed descriptor stage passes the frozen2,069-entry mature reference under four quota schedules and149 JSON restores per run. The larger fixture saves roughly every16,381 charged units; exhaustive every-boundary save tests remain the responsibility of smaller stage fixtures. The mature gate alone does not prove every possible restore cursor.

Ray-start polygon copying/validation has been removed from descriptor advances: queries now share an opaque validated Arc slice created at job initialization/restore. Host lookup uses binary search over canonical IDs. Before runtime promotion, audit remaining setup and bookkeeping bounds; initialization and restore costs must remain separately measured. Then complete discovery, evaluator, scheduling and multi-seed/WASM/browser gates above.
