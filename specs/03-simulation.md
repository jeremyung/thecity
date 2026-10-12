# Simulation Engine

The Rust engine owns authoritative city state. It can run without graphics for testing, replay, profiling, and seed validation.

## State model

Use stable entity identifiers with generation counters or equivalent stale-reference protection. Separate definitions from instances. Keep hot movement and occupancy data in contiguous arrays; cold history and appearance metadata can use ordinary structured storage.

Core resources: seeded geography, transport graphs, parcel index, demand aggregates, clocks, scheduled event heap, project reservations, random stream states, and bounded history.

Core components:

- Parcel: polygon, zoning suitability, terrain constraints, access, reservation. Parcels are released from the seeded master plan's gap-free lot partition when their frontage road opens; they are not searched for at runtime.
- Master plan: regenerated deterministically from seed and engine version on create and restore. Only realization progress (which plan roads and lots exist) is persisted.
- Building: parcel, footprint, type, capacity, occupancy, construction stage, age, appearance seed.
- Road or rail segment: endpoints, geometry, lanes or tracks, lifecycle, capacity, graph revision.
- Traveler: route, progress, speed, destination, travel mode, activity identity.
- Resident: home, recurring destinations, schedule template, preferences, current activity.
- Project: dependencies, stage, crews, deliveries, progress, expected next event.

The represented population exceeds individual agents. Aggregate households and jobs generate bounded trip demand. Persistent residents contribute their trips once, without double-counting them in aggregate cohorts.

### Housing occupancy and commercial activity

Residential occupancy counts people housed in homes and apartments. It is not a universal measure of whether a building is in use: an operating shop, cafe, or office can have zero residential occupants. Commercial premises need explicit tenant/activity state. Construction stage, tenant activity, household occupancy, and transient visitors are distinct facts.

Demand, local development prerequisites, vacancy aging, and renewal must use the appropriate activity predicate. Operating commerce contributes to an active neighborhood and does not accrue vacancy solely because nobody lives there. New jobs and deliveries target operating premises. Closing a business must either preserve/reassign existing recurring destinations safely or defer closure while those assignments and journeys are active; no traveler may retain an unreachable destination silently.

Business turnover is an explicit, seeded, bounded transition with a reason and history. A closed tenant can leave an empty premise eligible for reuse or renovation; reopening clears genuine vacancy age. Protect anchors and essential services. Do not create forced closures merely to manufacture visible change, and do not exempt all commerce permanently from renewal.

Legacy saves that recorded only residential occupancy cannot reveal commercial vacancy. Migrate their open commercial stock as operating, normalize any former non-residential occupancy marker, and clear commercial vacancy age derived from the old ambiguous rule. Preserve household state and pending renovations. Version the activity representation explicitly, and make migration deterministic, idempotent, and independent of RNG advancement or elapsed time.

## Clocks

Proposed base step: 50 ms of movement time, or 20 Hz. Never use rendering delta time as an authoritative input.

At normal pace, one city day lasts about 20 real minutes. Proposed development pace maps one city day to about one month of planning activity. Treat this as narrative acceleration rather than a literal historical calendar. Construction uses defined development durations.

All clocks derive from an integer tick and explicit rate settings. Use rational or fixed-point accumulation to avoid accumulating calendar drift. Persist rate changes as commands at tick boundaries.

Trips must remain feasible at the calendar rate: use compact distances, schedule windows, and congestion-aware demand. Do not generate departures faster than destination and transport capacity can handle. Fast-forward runs more fixed steps per real second; it does not increase dt.

## Tick order

1. Apply ordered commands assigned to the tick.
2. Advance clock accumulators.
3. Pop due scheduled events using stable tie-breaking.
4. Update demand, candidate projects, and construction when their cadences are due.
5. Update departures and fixed-budget routing jobs.
6. Resolve signals, lane queues, and traveler movement.
7. Commit deferred structural changes and graph revisions.
8. Produce activity events, dirty entity updates, and a snapshot when due.

New graph connections take effect at a declared commit boundary. Network systems invalidate affected route caches before the next movement phase.

## System cadence

Movement runs every tick. Schedule checks are bucketed or event-driven. Demand and parcel evaluation run every two real-equivalent movement seconds initially, staggered by region. Construction visual progress is derived from authoritative stages and time. Project decisions happen at slower development intervals.

Evaluate a fixed number of candidate parcels and route search expansions per tick. A deterministic cursor ensures all eligible areas receive attention. Work completion affects outcomes only at reproducible boundaries. Wall-clock CPU budgets govern how many ticks to execute, not which parcel gets selected within a tick.

## Travel

### Proposed incremental road-planning queue

Research follow-through (2026-10-09), not an implementation claim: apply the priority-queue structure described by [tmwhere](https://www.tmwhere.com/city_generation.html) to road proposals. Order candidates by simulation due tick and stable proposal ID; process a bounded number per tick. Separate growth goals from local geometry and access validation. Proposal acceptance reserves capacity for construction; only an opening event changes traversable connectivity. Revalidate stale proposals against graph revisions and current land reservations. Bound queue size, deduplicate proposals, persist pending work, and use deterministic retry/defer rules. Seeded suitability is an input alongside actual accessibility and sustained demand, not a substitute for them. Preserve independent movement and development clocks.

Implementation checkpoint (2026-10-10): the diagnostic geometry-derived access search now discovers an admissible two-lot branch in the mature seed87231 fixture without hand-picked route coordinates. It enumerates2,069 descriptors in44 pages of at most48 and passes shared admission, validation, and canonical restore. The discovered road also completes through normal engine ticks (observed open and connected at242400), with dependent lots withheld until opening and save-identical mid-construction replay. This remains test-only. The first resumable stage now reproduces44 mature-face host split samples bit-for-bit at quotas1,7,and48 with JSON resume checks; A separately tested ray/segment primitive now budgets individual polygon-edge checks and preserves JSON resume; the composed heading/endpoint preparer now matches all2,069 frozen mature descriptors at quotas1,7,48,and irregular partitions, including149 JSON restores per run. All runs charge2,422,472 work units; the largest observed checkpoint is451,864 JSON bytes. This proves native sequence/resume parity on this fixture, not full runtime acceptance. Ray initialization now shares a polygon validated once per job; host lookup uses canonical binary search. Every-step small-fixture restore validation and the remaining setup/evaluator budget review are still required before promotion. A pure graph pipeline now reproduces all49 mature reference faces through budgeted preflight, bridge detection, topology and complete face walking at quotas1,7,48 with JSON resumes. Its stage work counts are119,232 /2,022 /2,427 /19,630 respectively; this is native frozen-input evidence. Live graph capture, target sampling and pipeline scheduling remain outstanding. Before runtime promotion, incrementally budget live face/target discovery and exact evaluation; persist deterministic cursors and frozen inputs, validate restored bounds, and recheck current geometry at admission. Latest integration evidence: the persisted frozen discovery wrapper now runs through a separate WASM diagnostic boundary, with native and WASM producing the same2,069 ordered descriptor bits after shared algebraic heading construction. Checkpointed and uninterrupted WASM results agree. Native and WASM logical work counts still differ, so cross-target scheduling equivalence is not established. A frozen quota-stepped proposal validator now independently matches the existing evaluator on all2,069 descriptors with JSON resume; production capture, stale-geometry checks, split preflight and live scheduling are still pending. See `validation/spec-audit.md` and `validation/incremental-access-admission-plan.md`. A single successful face is not evidence of general density improvement. See `validation/face-access-paged-search.json` and `validation/face-access-paged-root-test.txt`.

Use walking, driving, and rail graphs with explicit transfers. Initial routing can use A* with cached origin-destination corridors and local refinement. Share routes where appropriate. A graph revision invalidates only impacted routes when practical.

Vehicles use left-hand lanes, speed limits, following distance, and intersection reservations or signals. Pedestrians wait at controlled crossings. Initially use kinematic movement rather than a general rigid-body engine. Detect blocked routes, safely replan, and record cancellations when no valid route exists. Never let unreachable travelers accumulate forever.

Arrival changes an agent's activity and occupancy records at a tick boundary. At an external gateway, entering and leaving represent the region beyond the simulated domain.

Implementation checkpoint (2026-10-11): bus and inbound train passengers can
alight into a persisted, visible waiting-at-stop state when the final walking
route is unavailable. Recovery caps are 256 waiting residents and eight examined
entries/route queries per tick; unchanged failed-route geometry suppresses repeat
searches. Reconnection preserves the physical waiting position and original trip
intent. Native and WASM replay evidence is recorded in
[the handoff](../validation/HANDOFF.md). A permanently unavailable route still
needs an explicit cancellation/replanning policy; this checkpoint does not close
the complete travel requirement.

## Determinism

Use separate seeded streams for geography, growth, schedules, weather, and cosmetic definitions. Rendering randomness has no access to simulation streams. Stable iteration order and deterministic event ordering are required. Avoid dependence on hash-map iteration, wall time, or asynchronous job completion order.

Reproducibility is guaranteed for an explicitly versioned engine and tested execution targets. If cross-platform identical hashes are required, use integer or fixed-point authoritative decisions and prohibit platform-sensitive floating-point decisions. Appearance and render interpolation may use floats freely.

Implementation evidence (2026-10-10): shallow-frontage continuation is repeatable within the tested WASM binary, but native and WASM are not evolution-identical. Matched 200-tick checkpoints first differ in frontage geometry at tick 243200 and choose different candidates by tick 251600. Do not use native continuation counts as exact browser predictions. See `validation/shallow-native-wasm-diagnostic.md`; the new `tests/shallow-infill-wasm.test.ts` covers real construction, connected access, and same-target replay in the browser engine.

## History and persistence

Record meaningful facts such as road opening, business establishment, station service, and redevelopment with causal references. Keep compact permanent landmark summaries and a bounded recent event ring, initially 10,000 records. Aggregate old minor events instead of retaining an infinite log.

A complete save includes state arrays, identifiers, pending projects, graph revisions, event queue, rates, tick, seed, version, random states, and incremental job state or a defined deterministic restart procedure. A world snapshot alone is insufficient if pending work is omitted.

## Evolution without runaway demand

Use occupancy feedback, bounded external migration, project cooldowns, and moving-average demand. Require sustained pressure before approval and a lower threshold for cancellation to avoid oscillation. Construction capacity is finite. At saturation, population stabilizes while activities and limited renewal continue.
