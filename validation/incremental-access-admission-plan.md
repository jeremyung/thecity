# Incremental access candidate validation and admission

This plan covers the final proposal evaluator and road-split admission. It assumes the immutable face/target/descriptor job and live capture described in `validation/live-access-capture-plan.md` already exist. The goal is to preserve the current candidate order and every geometry gate while making validation resumable and preventing a proposal from committing against stale state.

## Existing behavior to preserve

`Engine::try_plan_interior_access` calls `propose_interior_access`, advances `interior_access_cursor`, sorts the page by `CandidateScore::compare`, and calls `admit_interior_access` in that order until one succeeds (`engine/src/lib.rs:5789-5807`). `CandidateScore::compare` orders by descending site-area-per-road-length at the call site, then stable host/fraction/side/length fields (`engine/src/interior_access.rs:47-70`). The candidate grid is 3 fractions × 2 sides × 4 branch lengths per host; explicit face-derived descriptors provide exact `host_t` and endpoint (`interior_access.rs:60-68, 86-91`). The page cap is 48 (`interior_access.rs:4-5`).

The scan has cheap admission gates first: no existing interior or neighborhood plan, demand above 0.8, project slot available, enough road/parcel/id capacity, and no active road project (`lib.rs:5930-5964`). It then filters open, connected local/arterial host roads and sorts them by id (`5966-5992`). Preserve this source filter and the host ordering. For explicit descriptor input, host lookup currently builds a `BTreeMap`; a frozen host index should replace repeated linear lookup, but it must map to the same host record and must not alter the sorted candidate order.

For each candidate, preserve this predicate sequence because early rejection and current float arithmetic are observable in the accepted-proposal oracle (`lib.rs:6000-6278`):

1. Decode candidate fields and validate finite `host_t`, grid side/length indices, and the explicit or grid-derived branch length. Geometry candidates use side `+1` and their actual endpoint; grid candidates derive a normal from the oriented host.
2. Require the host’s two endpoint nodes, finite positive host length, and at least 10 m of host remaining on either side of the split. Compute split point, branch end, and branch length with the existing arithmetic; require 24–60 m and current world bounds inset 24 m.
3. Check branch-to-river clearance using the four current river control segments and half-width 4.5 m; reject protected-land crossings; reject existing interior-plan road/lot conflicts and any parcel-envelope centerline distance under 5 m.
4. Reject split points within 1 m of any other node or within 0.5 m of any other road segment. The road loop includes all roads except the host and treats a road with missing endpoints as a rejection. Then reject endpoints within 18 m of any node.
5. Require at least 8 m between the branch and every other open road and every active road project. The host is exempt from the open-road branch-spacing check. An active road project with missing endpoints is rejected; one without both endpoint ids is skipped.
6. Build branch frontage intervals with `subdivide_frontage(length, [], endpoint-clearance=9, min-width=8, target-width=14, gap=2.2, max-lots=8)` (`frontage.rs:19-135`). For each side `[-1,+1]` in that order and each interval in returned order, compute the lot center, dimensions, rotation, normalized along value, and side. Preserve the distinction between grid normal and explicit branch tangent.
7. Test each lot using `progressive_frontage_geometry_clear`: river clearance using the lot bounding radius; world bounds; protected rectangle overlap; every existing legacy/rendered parcel footprint at 2 m gap; reserved interior lots; every open road at 5 m rectangle-to-segment distance; every pending road at 7 m; coordinated-neighborhood interior and remaining neighborhood segments; then centerline clearances against all open roads (`max(width,depth)/2+4`, minimum 9 m) and pending roads (`+2 m`). Accept a lot only if the full predicate passes and selected lots remain below 16.
8. Require at least two selected lots and capacity for all selected parcels/frontages. Compute integer `site_area` as the sum of rounded lot area in cm² and `road_length_mm` as rounded branch length in millimeters; preserve the exact score fields and deterministic tie breaks.

## Work hidden by the 48-item cap

The 48 value limits candidate count, not evaluator work. Each candidate can visit:

- all four river segments for road clearance;
- up to 16 existing interior reservations and up to 4,000 parcel footprints for branch corridor clearance;
- up to 5,000 nodes, then up to 5,000 roads for split clearance;
- up to 5,000 nodes for endpoint proximity;
- up to 5,000 open roads and up to 12 active projects for branch spacing;
- up to 16 lots × (up to 4 river segments + up to 4,000 footprints + 16 reserved lots + 5,000 open roads + 12 pending roads + up to 1,024 neighborhood segments + 5,000 centerline roads/projects).

Most paths short-circuit earlier, but the worst legal input still has multiple nested world-sized loops. Additionally, `Engine::node(id)` linearly scans `state.nodes` (`lib.rs:1044-1046`), so looking up both endpoints inside each road loop can turn a nominal 5,000-road visit into quadratic node lookup work. A capture-time id-to-index table is appropriate, but building it must happen once under an explicit bounded capture cost and preserve duplicate-id behavior (the engine’s current `collision_context` maps retain the first source-order occurrence).

The candidate setup also clones `CollisionContext` for every accepted branch and appends the candidate branch to its open-road list (`lib.rs:6230-6237`). Avoid this full-vector clone: keep the captured arrays immutable and pass the one candidate branch as an additional final geometry item. `subdivide_frontage` is bounded here because exclusions are empty and its `max_lots` is 8; retain the function’s exact arithmetic or precompute intervals keyed by exact branch length only if parity tests prove bit-identical results.

## Typed resumable validation contract

Keep the candidate source and the validation snapshot immutable for the lifetime of one candidate attempt. A practical separation is:

```rust
struct AccessAdmissionJobCheckpoint {
    schema: u16,
    capture_tick: u64,
    validation_geometry_revision: u64,
    descriptor_cursor: u32,
    phase: AdmissionPhase,
    candidate: Option<AccessGeometryCandidate>,
    validation: Option<ProposalValidationCheckpoint>,
    ready: Vec<AccessProposal>, // bounded by the candidate-source cap
    ready_cursor: u32,
    split_preflight: Option<RoadSplitPreflightCheckpoint>,
}

enum AdmissionPhase {
    ReadCandidate,
    ValidateCandidate,
    OrderReadyProposals,
    RevalidateLiveScalars,
    PrepareSplit,
    Commit,
    Done,
}

struct ProposalValidationCheckpoint {
    candidate: AccessGeometryCandidate,
    host_index: u32,
    phase: ProposalPhase,
    cursor: u32,
    interval_index: u8,
    side_index: u8,
    lots: Vec<InteriorAccessLot>, // at most 16
    // Derived candidate start/end/length and score inputs, validated on restore.
}
```

The immutable `AccessValidationInput` should own, in original engine order: all nodes `(id,x,z)`; all roads `(id,from,to,kind,stage,connected)`; active project road endpoints; all parcel/frontage-derived legacy and rendered oriented envelopes; pending interior reserved lots and branch corridor; open and pending segment projections; current neighborhood interior and remaining segments; river control segments; and the host records used by the descriptor source. This is deliberately distinct from `FrozenViabilityInput`: candidate split checks also inspect closed/rail roads and all nodes, while final proposal admission depends on current active project and reservation state. The same ordered projection can be shared with descriptor generation to avoid rebuilding it.

Use a `ProposalPhase` cursor for the actual scans, with a unit charged for each visited node, road, project, footprint, reservation, river segment, or neighborhood segment. Constant arithmetic and the four-point SAT test stay with the charged geometry entry. Keep short-circuit order and rejection code identical to the synchronous oracle. `advance(0)` must be a no-op; terminal phases use no work; the checkpoint must validate every cursor against its phase’s corresponding input length. Do not use a hidden `.any()`, `.all()`, `.find()`, sort, map build, or `node()` call inside `advance` if its data-dependent traversal is not charged.

For descriptor input, the source is already finite and bounded by `access_candidates::MAX_PREPARED_DESCRIPTORS` (`access_candidates.rs:60-62`). Validate candidates one at a time. Store only accepted proposals, each with at most 16 lots; enforce an explicit serialized-byte ceiling in addition to descriptor count. Sort accepted proposals by the current score comparator through a resumable insertion/merge phase with charged comparisons, or retain a bounded ordering structure whose tie behavior is proven identical. Do not change to “first descriptor wins”: current code sorts valid proposals by area-per-meter before admission.

### Snapshot freshness and retries

At the start of each candidate validation, bind the validator to one `access_geometry_revision` and one frozen `AccessValidationInput`. If the generation changes before validation completes, discard only that candidate’s partial result and advance to the next descriptor; do not restart the face/target/descriptor job. Repeated churn must be observable and bounded by a per-candidate retry limit. Capturing the latest per-candidate input is an atomic tick-boundary operation; it cannot reuse array references while engine vectors mutate.

Once a candidate passes frozen validation, run cheap current scalar gates immediately before preflight. If the geometry generation changed since that candidate’s validation capture, run bounded validation again for that candidate against a fresh snapshot or skip it and continue. Never commit against a stale geometry projection. Demand, active-project count, parcel/road/id capacity, and no-existing-plan checks must be read from current state, not the old search snapshot.

The current broad `state.revision` is unsuitable for this check because weather and movement cadence increment it (`lib.rs:2100-2139`). `BusLine.graph_revision` is a road-count snapshot (`lib.rs:7952-7964`). Add a specific persisted generation at the actual geometry mutation transaction points listed in `live-access-capture-plan.md`: road open/close/connectivity, parcel/frontage and rotation changes, pending road corridor membership, and plan/reservation changes. Project progress changes that retain the same pending segment do not alter this projection; crossing its active/complete threshold does. Business activity and occupancy affect scalar catchment gates, not geometry.

The concrete writer hooks are `initialize_world`, `add_node`/`add_road`/`add_parcel` (`lib.rs:~1100-1235`), `complete_project` and `complete_interior_access_project` (`2331-2678`), `try_progressive_frontage_infill` parcel/frontage admission (`4264` onward), neighborhood plan creation and `plan_next_neighborhood_segment` (`~5460-5521`), `plan_building` parcel reservation (`7200` onward), `renewal` reservation/stage changes (`7405` onward), `recompute_connectivity` (`7375-7404`), and `commit_open_road_split` (`road_split.rs:~330-425`). `trim_completed_projects` (`lib.rs:~2500`) invalidates only if removal changes the active-road projection, not merely because completed history shrinks. Put the bump at committed transaction boundaries; tests should assert whether each mutation changed the effective projection.

## Atomic split preflight and final commit

`admit_interior_access` currently checks road/project/parcel/frontage capacity, next-id headroom, and project availability before calling `try_split_open_road`; only a successful split creates the branch endpoint/road/project and persists `pending_interior_access` (`lib.rs:6287-6355`). Keep all these checks immediately before the mutation. No id allocation, cursor write, road split, or plan reservation may occur on a rejected candidate.

`Engine::try_split_open_road` is mutation-free until `prepare_open_road_split` succeeds; then it commits in one call (`engine/src/road_split.rs:458-462`). This is the correct atomic boundary but its preflight has variable work that also needs metering before production:

- Find and validate host, active-project conflict, capacity, endpoint geometry, split proximity against nodes and all roads (`road_split.rs:19-117`).
- Scan `parcel_frontages`; for each matching frontage, linearly find its parcel and validate remapping. Worst case is frontage count × parcel count (`117-151`). Freeze a parcel-id index or preserve first-match semantics with a bounded captured index.
- For every traveler, validate its route and route lengths, expand/remap the route, and allocate replacement lengths (`155-218` onward). Also expand the bus route and every train route (`~220-276`). Routes each have caps, but a loop over all travelers/trains is still variable work.
- Commit adds two ids/graph objects, applies remaps, and calls `recompute_connectivity`, which traverses the graph (`road_split.rs:~330-423`).

Introduce an opaque `PreparedRoadSplit` runtime/checkpoint that captures `expected_geometry_revision`, `expected_next_id`, host id/endpoints, split point, and fully validated remap output. Prepare it with explicit cursors for node, road, frontage, parcel lookup, traveler, route-edge, bus, and train work. Once complete, commit in one tick only if the generation, `next_id`, host identity/endpoints, and all cheap capacities still match. If any check differs, discard the prepared split with no mutation and continue to the next candidate. The commit itself should be separately profiled at maximum world limits; if its bounded remap writes/reconnect traversal cannot fit the tick budget, stage replacement arrays privately and atomically swap them at the commit boundary rather than exposing partial state.

`recompute_connectivity` and `state.revision` currently run within split commit (`road_split.rs:~400-425`), and `BusLine.graph_revision` is recomputed from open-road count there. Keep these existing effects, but additionally bump the access geometry generation after the new graph and connectivity are committed. Revision must not bump during preparation or failed attempts.

## Required parity and failure tests

- Run the old synchronous evaluator and the new quota-stepped evaluator against identical frozen fixtures. Compare rejection category, each accepted lot field by `to_bits()`, `CandidateScore`, and sorted proposal order, including the mature host3317/t.78/two-lot accepted case.
- Exercise quotas 0, 1, 7, 48, and irregular boundaries; restore JSON at every phase/cursor on a small fixture and selected boundaries on the mature fixture. Logical work totals must not depend on quota.
- Use instrumentation to assert no input-array visit or endpoint lookup occurs without a work debit. Include a maximum-node/road/parcel synthetic validator fixture and record the maximum units per `advance`.
- Mutate each relevant generation between candidate phases, after validation but before split preparation, and after preparation but before commit. The attempt must be discarded without partial state; the next candidate can still make progress.
- Test stale/closed/disconnected host, missing node, active project added, reservation added, parcel added/rotated, capacity exhaustion, and actor route-cap failure. Each failure preserves IDs, roads, frontages, routes, reservations, RNG, and save bytes.
- Compare split-preflight prepared output against the current `try_split_open_road` result on representative travelers, buses, trains, and remapped frontages. Then verify one atomic commit yields identical routes, lengths, connectivity, and canonical save after restore.

This keeps the current rule set intact: bounded work changes where the loops pause, while all proposal and split decisions still use the same ordered data and geometry predicates.

## Integration correction: moving actor state is independently mutable

`PreparedRoadSplit` currently stores every traveler's route, edge index and edge progress, then overwrites them in commit. Geometry freshness alone cannot make a multi-tick prepared split safe: travelers can advance, finish, be removed, or be added while geometry stays unchanged. The same issue applies to bus/train route replacement. Do not promote this object into a persisted multi-tick transaction using only the proposed geometry token.

Before choosing incremental split preflight, measure the existing bounded atomic split at representative and maximum actor/route loads. If atomic preparation and commit fit the measured runtime budget, keeping them together avoids stale movement copies. Otherwise prepare topology/route transformations separately, track actor identity and route versions, and rebase current movement onto unchanged route mappings at commit; new/changed/removed actors need explicit handling. A global actor-movement revision would change every tick and can starve preparation, so it is not sufficient by itself. Any change must verify that advancing actors between preparation and commit never rewinds movement or transfers one actor's route to another.

## Measured atomic path after indexing

The native cap experiment now includes both detached and gateway-connected road networks, with 2,000 travelers and near-limit routes. Removing repeated endpoint/parcel scans and indexing the connectivity traversal reduced the detached cap mean from about40.75ms to3.66ms; the connected cap measured5.90ms mean/8.31ms p95. See `road-split-budget.md` for sample counts and limitations. Pending WASM confirmation, the preferred integration is bounded incremental discovery/validation followed by the existing single-tick atomic split, with fresh scalar/geometry checks immediately before it. The split itself then reads current actor state and commits its remaps in the same tick. Do not introduce multi-tick actor snapshots merely to follow the earlier unmeasured preflight proposal.
