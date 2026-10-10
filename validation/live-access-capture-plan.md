# Live access search: capture, scheduling, and admission

This plan connects the verified frozen-input pipeline (road graph → faces → target selection → ranked targets → access descriptors) to the live engine without pausing growth or restarting a long search whenever the city changes. It is a design boundary for production integration; the verified pure stages do not yet make a production policy.

## Current runtime facts

`Engine::step` runs fixed 50 ms ticks and caps a caller batch at 2,000 ticks (`engine/src/lib.rs:2089-2095`). In each `step_one`, growth evaluation runs every 40 ticks, project construction advances on development days, transit and actors advance, and the broad `state.revision` increments every 40 ticks (`2096-2139`). Weather also increments that revision at day changes (`2100-2117`). The revision is therefore a snapshot/UI revision, not an access-geometry revision.

`BusLine.graph_revision` is initialized from the current count of open roads (`lib.rs:7952-7964`); it is not a monotonic graph generation and does not describe parcels, pending projects, or reservations. Neither field can safely validate a frozen access result.

The complete mutable source lives in serialized `WorldState` (`lib.rs:633-710`). It includes the graph, parcels/frontages, buildings, projects, current neighborhood and interior reservations, access cursors, and district measurement checkpoint. `save_json` serializes that state and `restore_json` validates it and rebuilds runtime state (`10449-10495`). The live access job and its frozen inputs must follow the same pattern: persisted checkpoint data in `WorldState`, ephemeral runtime cursors rebuilt from validated checkpoints.

Today `advance_district_measurement` demonstrates resumable work but not bounded capture: it synchronously calls `make_samples` across all buildings and frontages, begins an accumulator, then advances a fixed sample quota and persists its cursor (`2152-2217`). Reuse its checkpoint/runtime split, not its synchronous full-world capture as proof that access capture is cheap.

The production search entry is `try_plan_interior_access` (`5741` onward), called from growth evaluation. Its scan implementation is `propose_interior_access_scan` (`5891-6285`). It caps inspected proposals at 48 (`5972`), but each proposal can scan all roads, projects, nodes, parcels, river segments, and reserved/neighborhood geometry. `collision_context` also projects the current parcel and road state before candidate evaluation. Thus “48 proposals per evaluation” is not a bound on primitive geometry work.

The diagnostic face pipeline is not yet the production caller. Its frozen-input adapters and verified mature fixture are in `engine/src/face_access_exact_tests.rs`; the pure face-selection modules remain test-only. The mature seed 87231 checkpoint at tick 242000 verifies 49 faces, 34 centers in the selected face, six final targets, and 2,069 descriptors. Parent’s end-to-end frozen run reproduced all 2,069 bits and the exact accepted proposal (host 3317, t .78, two lots), at 5,698,020 logical work units before descriptor initialization changes. The measured pipeline is much longer than a few ticks: at 4,096 work units/tick it is about 70 seconds at normal speed; at 48/tick it is about 99 minutes. Those are work-count estimates, not a chosen production budget or wall-time guarantee.

## Mutation inventory and revision scope

Do not bump one revision for every simulation-state change. Add explicit monotonic generations with documented input scope:

| Generation | Changes that invalidate it |
| --- | --- |
| `access_geometry_revision` | Coordinates, membership, eligibility, or connectivity of nodes/roads; parcel dimensions/rotation/protection/frontage; active road-project corridor set; current interior/neighborhood reserved geometry. |
| `access_demand_revision` (or capture values directly) | Demand and live housing/commercial catchment capacity/vacancy/activity used by gates and ranking. Usually cheaper to copy these values into the frozen job and always recheck live at admission. |

The geometry generation must advance only at committed mutations and must be persisted. Centralize it at transaction/writer boundaries; `add_node`, `add_road`, and `add_parcel` append without incrementing a revision themselves (`lib.rs:1150-1235`), so adding a counter only to those helpers will miss in-place stage and reservation edits. The required mutation sources are:

- World creation and graph/object insertion: `initialize_world` and the add helpers (`lib.rs:~1090-1235`, `2042-2085`).
- Road project lifecycle: `advance_projects`/`complete_project` mutate project progress and road stage; road completion opens the road, recomputes connectivity, and creates frontage parcels (`2331-2520`). A project enters/leaves the pending-corridor set when its kind/progress crosses the query’s `road && progress < 1` predicate.
- Interior access: admission splits an existing road, adds endpoint/branch objects, and stores selected lots as reservations (`6287-6355`); completion opens the branch, creates parcels/frontages, and clears the plan (`2520` onward). Retry state changes also affect pending geometry.
- Neighborhood plans: starting a plan reserves lots and its segment list; each scheduled segment creates a road project; completion admits lots and retires the plan. See plan creation around `5460`, `plan_next_neighborhood_segment` (`5474-5521`), and `neighborhood_spine.rs` reservation/completion methods.
- Progressive frontage: `try_progressive_frontage_infill` admits parcels and frontages and increments the broad revision (`4264` onward; admission near its end). Parcel rotation/frontage association is part of the collision input.
- Building lifecycle: `plan_building` reserves a parcel (`7200` onward); completion clears the reservation (`complete_project`, `~2400`). This affects reserved-lot collision geometry. Renewal changes building/parcel stage and reservation (`renewal`, `7405` onward), while business turnover changes `business_active`/occupancy (`try_close_business_for_turnover`, `7461` onward). Activity and occupancy affect live demand/catchment, not face geometry.
- Connectivity recomputation (`recompute_connectivity`, `7375-7404`) can change the eligible connected-road graph without changing road coordinates. Include a generation bump when the effective connected flags change.
- Completed-project trimming (`trim_completed_projects`, after completion) is harmless only if it cannot remove an active pending-corridor input; generation should be tied to the effective projection, not vector length alone.

Implement a small `mark_access_geometry_changed()` at transaction boundaries after a committed mutation, and one `mark_access_demand_changed()` for live score inputs if they are not simply recaptured at admission. Add tests that each writer either changes the geometry generation or proves its mutation is irrelevant to the projection. Do not use raw `state.revision`, project count, or `BusLine.graph_revision` as substitutes.

## Snapshot and job contract

Capture a **single immutable input at one deterministic tick boundary**, then let the analysis consume it over later ticks while simulation continues. Do not walk live vectors across ticks: insertions/removals shift cursors and can combine road state from different moments. Do not restart on every geometry revision: at current measured work, normal construction changes would prevent completion indefinitely.

The initial capture should run at the end of a fixed tick after that tick’s growth/project mutations have committed (the current `step_one` order is visible at `2096-2140`). It must have explicit maximum item counts and serialized-byte limits. Before adoption, measure the synchronous projection at a mature fixture and the worst legal synthetic state. If the bounded copy itself exceeds the tick budget, introduce immutable copy-on-write/versioned component snapshots; do not silently spread capture across mutable vectors. This is a measurable one-time boundary cost, separate from the resumable search quota.

Persist at most one `LiveAccessJobCheckpoint` with fields equivalent to:

```rust
struct LiveAccessJobCheckpoint {
    schema: u16,
    capture_tick: u64,
    capture_geometry_revision: u64,
    capture_demand_revision: u64,
    trigger_road_id: Option<u32>,
    source_world_seed: u32,
    phase: AccessJobPhase,
    // Frozen, ordered graph and query arrays required by exact stages.
    graph: FrozenRoadGraph,
    viability: FrozenViabilityInput,
    ranking: FrozenRankingInput,
    live_catchment_at_capture: LiveCatchment,
    // Stage checkpoints and deterministic cursors; no wall-clock deadlines.
    trace: Option<TraceCheckpoint>,
    selection: Option<SelectionCheckpoint>,
    ranking_job: Option<RankingCheckpoint>,
    descriptor_job: Option<DescriptorCheckpoint>,
    proposal_cursor: u32,
}
```

Exact types can use the existing serializable checkpoint DTOs. Store the original ordered arrays, not hash maps whose iteration order can vary. The capture contains eligible graph edges and node coordinates; source-order parcel footprints and rotations (including both legacy and resolved envelopes where current collision semantics require them); open-road and pending-road segments; river segments; neighborhood segments and interior; ranking roads/nodes; frozen district/served-catchment inputs; trigger identity. Exclude presentation-only objects and actor positions. Record capture versions so diagnostics can explain why a candidate was later rejected.

On restore, validate schema, caps, finite coordinates, references, cursor/phase consistency, and capture tick ≤ saved tick; reconstruct runtime objects once from the frozen checkpoint. A job must continue from its exact phase and logical cursor. Do not discard it and restart from the current city on restore.

For mature graph limits, face tracing currently caps at 512 nodes/edges (`road_faces.rs`, `road_face_trace.rs`, `road_face_walk.rs`). The engine allows up to 5,000 road segments (`lib.rs:71-73`). A live adapter must either prove the eligible connected subgraph fits the trace cap or defer with a surfaced `OverGraphCap` reason. Never truncate a large graph and call the resulting faces complete.

## Scheduling and stale candidates

Use a persisted state machine with one active job: `Idle → CaptureDue → Trace → Select → Rank → Descriptors → Revalidate → Admit/Exhausted`. Trigger at a deterministic existing growth evaluation (every 40 ticks) or a qualifying road opening, only when demand and capacity gates allow, no conflicting interior/neighborhood plan exists, and one job slot is free. A trigger while a job is active is coalesced into a pending boolean/road id under a documented deterministic rule; it does not replace the active snapshot. Each tick advances a fixed integer work budget independent of `development_rate`, display rate, render cadence, or wall-clock time. The engine already advances projects at development-day cadence; analysis progress must not accelerate with it.

Geometry changes while a job runs are expected. Preserve progress on the frozen job. When descriptors finish, stream candidates in the verified deterministic rank/order and revalidate each against the **current** world before admission: host still exists/open/connected and endpoints match; the current face boundary has not been invalidated for that proposal; candidate split/corridors remain clear; no reservation or active project blocks it; current demand, catchment, capacities, ID budget, and project admission still pass. Reuse the atomic mutation-free preflight in `admit_interior_access` / `try_split_open_road` (`lib.rs:6287+` and `road_split.rs`), then commit at that tick boundary. On rejection, advance to the next frozen proposal. Do not restart the whole graph search just because the geometry revision changed. If the face boundary changed, reject only candidates tied to that stale face and continue to another face/proposal if available. If all are stale/exhausted, finish the job, record the reason, then observe a deterministic cooldown before capturing again.

Candidate revalidation itself must be quota-stepped. `propose_interior_access_scan` currently builds a whole `CollisionContext`, iterates up to 48 descriptors, scans every node/road for split and endpoint proximity, checks all open/pending roads, then tests each side’s frontage intervals against all footprint/reservation/river/street arrays (`lib.rs:5960-6285`). The 48 cap does not bound this nested work. Extract a serialized `ProposalValidationJob` with cursors for each ordered array and explicit work debit per geometry item, preserving existing loop order, short-circuit order, float arithmetic, and first rejection reason. A proposal is admitted only after its revalidation job completes successfully. Freeze the candidate validation inputs at a deterministic tick and record their geometry generation. At commit, require that generation still matches the live geometry generation, in addition to rechecking cheap live scalar gates. Otherwise discard/restart only that candidate validation (or apply a verified bounded mutation-log recheck); never admit on scalar checks alone after geometry has changed during multi-tick validation. Define a deterministic retry/cooldown limit so a perpetually changing candidate cannot monopolize admission. The full frozen search continues to retain its progress.

The existing `AccessScan` cursor is a proposal cursor, not a complete job checkpoint. Existing `access_candidates` has prepared resumable descriptor scanning, but its initialization and the full face/context adapter must also be bounded or explicitly measured. Avoid constructing BTreeMaps/BTreeSets or sorting long vectors inside a tick without charging or documenting that cost.

## Required integration evidence

1. Preserve mature frozen parity end to end: the mature fixture still yields the same trace/face, target order, descriptor bitset, and first accepted proposal when the new live wrapper captures without intervening mutation.
2. Determinism: run identical seeds with different worker batch sizes and development rates; job phase/cursors, final proposal, RNG state, and saved bytes match at equal ticks.
3. Save/restore: serialize and restore during capture completion, trace, selection, ranking, descriptor generation, and candidate revalidation; continuing yields the same result and work count as uninterrupted execution.
4. Growth during search: keep advancing roads/buildings while the job runs. Verify the job does not restart, memory stays within declared checkpoint caps, and a now-invalid first proposal is rejected while the next valid proposal is considered.
5. Geometry generation audit: tests cover every mutation listed above, including in-place stage/connectivity/reservation changes. Nongeometry actor movement and weather must not invalidate the face job.
6. Boundedness: instrument capture item/byte count and wall-time at mature and maximum legal states; instrument per-tick logical work so every stage—including proposal revalidation—stays within its quota. Report max/p95 separately from simulation ticks; do not derive a production quota from the test quotas (1/7/48) used to establish resumability.
7. Cap and failure behavior: over-cap graph, malformed saved checkpoint, stale host, exhausted proposals, or failed preflight leave the world unchanged and produce a deterministic diagnostic/cooldown rather than partial IDs, roads, or reservations.

The key integration choice is: snapshot once, compute incrementally on that immutable snapshot, then validate each proposed mutation against current state. This allows city growth to continue through the multi-minute search while keeping the final road admission safe and deterministic.

## First capture measurement

`access-capture-native-profile.json` records32 native-release captures of seed87231/tick242000:median0.111ms,p950.135ms,max0.240ms;serializedprojection120,794bytes. The timing excludes JSON serialization, separatelymedian0.209ms. These figures apply only to the850-building fixture on this host. Maximum-size source projection and WASM/browser timing remain required before selecting the atomic-capture boundary for production.
