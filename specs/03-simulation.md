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

Legacy saves that recorded only residential occupancy cannot reveal commercial vacancy. Backward save compatibility is not required (user decision, 2026-10-11); reject incompatible activity representations explicitly and offer a fresh world rather than inferring tenant history or building migrations. Current-format saves must preserve explicit tenant activity, household state and pending renovations, with deterministic live-versus-restored continuation. Version the activity representation explicitly.

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

Use walking, driving, and rail graphs with explicit transfers. Initial routing can use A* with cached origin-destination corridors and local refinement. Share routes where appropriate. A graph revision invalidates only impacted routes when practical.

Vehicles use left-hand lanes, speed limits, following distance, and intersection reservations or signals. Pedestrians wait at controlled crossings. Initially use kinematic movement rather than a general rigid-body engine. Detect blocked routes, safely replan, and record cancellations when no valid route exists. Never let unreachable travelers accumulate forever.

Arrival changes an agent's activity and occupancy records at a tick boundary. At an external gateway, entering and leaving represent the region beyond the simulated domain.

## Determinism

Use separate seeded streams for geography, growth, schedules, weather, and cosmetic definitions. Rendering randomness has no access to simulation streams. Stable iteration order and deterministic event ordering are required. Avoid dependence on hash-map iteration, wall time, or asynchronous job completion order.

Reproducibility is guaranteed for an explicitly versioned engine and tested execution targets. If cross-platform identical hashes are required, use integer or fixed-point authoritative decisions and prohibit platform-sensitive floating-point decisions. Appearance and render interpolation may use floats freely.

## History and persistence

Record meaningful facts such as road opening, business establishment, station service, and redevelopment with causal references. Keep compact permanent landmark summaries and a bounded recent event ring, initially 10,000 records. Aggregate old minor events instead of retaining an infinite log.

A complete save includes state arrays, identifiers, pending projects, graph revisions, event queue, rates, tick, seed, version, random states, and incremental job state or a defined deterministic restart procedure. A world snapshot alone is insufficient if pending work is omitted.

## Evolution without runaway demand

Use occupancy feedback, bounded external migration, project cooldowns, and moving-average demand. Require sustained pressure before approval and a lower threshold for cancellation to avoid oscillation. Construction capacity is finite. At saturation, population stabilizes while activities and limited renewal continue.

### Walking surface contract

Local streets and lanes are shared surfaces: pedestrian centers run0.72m inside
the carriageway edge. Arterial walking centers use the raised sidewalk0.72m beyond
the carriageway. Movement, route endpoints and building-access connectors share
this offset calculation. A supported endpoint alone does not establish support
through every junction or station approach; verify complete connectors against
rendered surfaces. Current-format replay remains required; obsolete access paths
may be rejected without migration.

Dated implementation notes for this spec: [validation/spec-notes/03-simulation.md](../validation/spec-notes/03-simulation.md).
