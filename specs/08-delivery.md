# Delivery and Validation

Build a complete small slice before increasing city size. Visual quality and simulation correctness need separate measurements; WASM improves neither automatically.

## Milestones

### One Rendered Waterfront

Create an authored river, bridge, shop, station, and small street in Last Train, First Light. Include moving water, sun progression, night lighting, one train, pedestrian motion, FSAA, and a basic audio mix. This proves the renderer and mood before procedural complexity.

Acceptance: coherent dawn, noon, dusk, and night; stable edges during camera motion; no distracting texture shimmer; measured WebGPU and fallback behavior.

### Autonomous Settlement

Introduce Rust WASM, the worker protocol, finite seeded geography, active road access, parcels, aggregate demand, travelers, and staged construction. A founding settlement grows without viewer input.

Acceptance: occupied buildings have valid access; visible trips have destinations; same seed reproduces engine outcomes; pause and save/resume work.

### Expanding Neighborhoods

Add procedural road projects, corridor reservations, bridges, rail construction, station catchments, infrastructure limits, and project competition. Add camera footprint tracking and scene anchors.

Acceptance: expansion visibly follows infrastructure; camera reveals new neighborhoods; routes update without teleportation or permanent queues.

### Mature Living City

Introduce soft saturation, bounded population, infill, renovation, business turnover, character preservation, and complete spatial sound. Scale only after profiling.

Acceptance: a long-running city remains bounded and interesting; memory plateaus; original anchors remain recognizable; mature quiet periods are acceptable.

### Selectable Little Tokyo

Add physical miniature materials, photographic lighting treatment, and stronger optional tilt-shift. Keep the world and geometry identical.

Acceptance: style switching preserves tick, seed, selected entity, camera, and simulation hash; no persistent GPU resource leak.

## Proposed budgets

Choose and record one desktop integrated-GPU reference machine and one mobile reference device before adopting hard performance claims. Profile the actual city and effects on both.

| Metric | Initial target |
| --- | --- |
| Desktop render rate | Near 60 fps at agreed resolution |
| Mobile reduced quality | Near 30 fps |
| Main-thread application CPU | Typically under 4 ms per frame |
| Simulation step | p95 under 5 ms at 20 Hz on desktop reference |
| Movement publication | 10–20 Hz with bounded backlog |
| Desktop WASM memory | Under 128 MiB initially |
| Desktop application CPU memory | Under 300 MiB, measured separately from GPU resources |
| Estimated GPU allocations | Under 256 MiB initially; browsers may not expose complete usage |
| First-scene compressed download | Under 15 MiB initially, with later assets streamed |

GPU time, CPU time, and overall frame intervals are distinct metrics and may overlap; do not add them as a guaranteed total. Temporal reconstruction and high-resolution reflections need their own measured budgets.

## Graceful load handling

Use a bounded tick batch and yield between batches. If sustained tick throughput falls below the requested fast-forward rate, visibly reduce achievable speed rather than changing dt. Render quality adapts with hysteresis; authoritative world rules and caps remain fixed for a world profile.

Fixed work quotas schedule expensive searches across ticks. Under active-traveler limits, aggregate additional trip demand consistently and preserve destination flow counts. Never decide economic outcomes from camera visibility or elapsed CPU milliseconds.

## Tests

- Deterministic seed and command replay hashes for versioned engine fixtures.
- Save/resume equivalence including pending road projects and route jobs.
- Invariants: valid references, bounded occupancy, connected active networks, no occupied inaccessible parcel.
- Small corridor scenarios for pedestrian crossings, lane queues, rerouting, and station transfers.
- Boundary and reservation conflicts, including bridge construction.
- Saturation tests: budgets reject new expansion and renewal remains possible.
- Camera and style independence of engine state.
- Worker backpressure, stale epoch rejection, and structural resync.
- Buffer reuse, repeated loads, and repeated style switches without growth in retained resources.
- Representative visual sequences across AA, rain, water, dawn, night, and camera cuts.
- Audio mute, pause, loops, resume, and emitter caps.

Run headless accelerated worlds across a fixed seed corpus, then several multi-hour browser sessions at normal pace. Accelerated tests expose state errors but do not replace observing the audiovisual experience.

## Implementation decisions to resolve during the first slice

Pin Three.js and Rust tool versions; record exact reference devices; select the primary AA technique; measure the reflection strategy; tune camera angles and focus; validate modular asset density; select initial music and sound assets. These do not block implementing the contracts above.

## PLATEAU calibration pilot tasks

The [research report](../validation/plateau-research.md) defines a proposed, bounded pilot; these are outstanding tasks, not measured results or replacement release gates.

| Task | Deliverable and dependency |
| --- | --- |
| PLATEAU-01 | Verify resource-level availability, vintage, rights and sizes for three 250 m-square crops; measure footprint/height/frontage/gap distributions with clear denominators. Limit compressed source retrieval to 50 MiB; record a source limitation if distribution granularity prevents this. No whole-ward download. |
| PLATEAU-02 | After 01, build a separate synthetic benchmark using existing kits and compare fixed cameras in both styles at noon/dusk/night. Keep normal seeded trajectories unchanged while calibrating. |
| PLATEAU-03 | Package compact versioned definitions and source/transformation/attribution manifests. Optional imported geometry only after 02 proves a need; report proposes at most 3 silhouettes and a 2 MiB compressed pack, subject to existing resource budgets. |
| PLATEAU-04 | After reference and access validation, compare calibrated growth across seeds 0, 87231 and a new seed, preserving connectivity/construction/save invariants. |
| PLATEAU-05 | Before promoting 03/04, record hardware, matching camera/effect settings, browser frame timing and residency across style cycles; retain all existing visual and multi-hour gates. |

The report proposes crop-matching tolerances and a 10% frame-interval regression ceiling for the pilot. These are provisional experiment criteria, not established Tokyo statistics or permission to reduce beauty, water quality, anti-aliasing, audio or simulation scope. Verify and retain resource-specific attribution and processing disclosures before distributing derived data/assets.

## Release criterion

### Visual quality is a release gate

Before judging a loaded city, verify that renderer diagnostics show zero queued buildings and zero queued neighborhood batches. A populated simulation snapshot does not prove all geometry is on screen. Save a diagnostics capture alongside the unobstructed scene; record seed, tick, camera, style, weather, and depth-of-field state.

Beauty and fidelity to the approved artistic direction are requirements, not
optional polish. Functional tests and performance numbers cannot substitute for
visual review. The primary agent owns this review even when implementation is
delegated to Luna.

Compare captured browser views against `concepts/04-last-train-first-light.png`
for the first style and `concepts/01-little-tokyo.png` for the second. Review the
founding waterfront, a developing neighborhood, and a mature scene at dawn,
noon, dusk, and night, including a narrower viewport. Inspect camera movement as
well as still frames.

Acceptance requires readable compositions, convincing Tokyo street detail,
layered foliage and atmosphere, coherent light and shadow, expressive water,
restrained night emission, and stable anti-aliased edges. Last Train, First Light
must carry the environmental painting direction in its materials and lighting;
Little Tokyo must convey a tactile miniature through surface response and focus.
The difference must remain visible with depth of field disabled.

Reject generic box cities, uniformly scattered scenery, washed-out grading,
unreadable subjects, abrupt camera reframing, and distracting repetitive motion.
Improve composition, assets, shaders, or batching when these fail. Under load,
reduce optional effect resolution and distant detail before compromising the
focal scene or removing required water and anti-aliasing.

Fabric acceptance (2026-10-11): at maturity, rendered-footprint crops of compact and station districts meet the [fabric targets](02-world-growth.md#fabric-targets) for at least three seeds, with no overlapping lots, no inaccessible occupied building, and connected junctions. Run `node scripts/audit-masterplan-fabric.mjs` on a mature snapshot to report gross coverage, footprint median/p90, floor median and road share per crop.

Procedural-generation research acceptance (proposed work, 2026-10-09): compare fixed-seed, fixed-tick scenes with matched camera, time, style, and focus settings. Measure rendered building footprint and net developable block coverage separately from parcel-envelope proxies. Check parcel overlap, real street access, connected junctions, bounded proposal queues, and save/resume of pending planning work. Verify infrastructure construction precedes adjacent opening. Evaluate several seeds and stages; one denser screenshot does not establish general quality. Architectural variation should remain coherent within districts and survive style switching without geometry changes. Prioritize compact blocks and street detail before using extra towers, blur, or grading to imply density.

A viewer can watch infrastructure open new land, see homes and businesses grow around it, follow purposeful activity through a full daily cycle, and observe a mature city that continues changing within finite bounds. Last Train, First Light must feel complete before adding more art modes or larger maps.
