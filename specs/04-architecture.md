# Browser Architecture

Use Rust compiled to WASM for the engine, a dedicated simulation worker, and TypeScript for the application, rendering bridge, camera, and audio. Deploy as static files over HTTPS. Runtime simulation does not require a backend.

## Module layout

```text
engine/
  core/            identifiers, clocks, RNG, definitions
  world/           geography, parcels, spatial indexes
  infrastructure/  planning, graphs, construction
  population/      demand, activities, travel
  persistence/     snapshots and validation
web/src/
  worker/          WASM loading, scheduler, protocol
  render/          scene bridge, assets, water, styles
  camera/          manual controls and scene director
  audio/           ambience, emitters, music director
  ui/              React controls and inspectors
  storage/         IndexedDB and save coordination
```

React never owns per-frame transforms or simulation entities. The renderer receives a read-only presentation model. The worker is the sole owner of authoritative state.

The layout above is the target decomposition. As of 2026-10-09, browser subsystems are separated, but most Rust systems still live in `engine/src/lib.rs`. Pure scalar distance helpers have begun moving to `engine/src/geometry.rs`, which cannot access engine state. Continue extracting geometry/model boundaries before growth and transit systems. Keep deterministic scheduling and mutation ownership explicit; splitting files alone does not establish independence. Validate refactors separately from behavior corrections against current-format save validation and same-target continuation fixtures.

`engine/src/blocks.rs` now owns state-free rectangular subdivision with a hard 64-lot result bound. `geometry.rs` supplies oriented-rectangle overlap, bounds, and segment clearance. Engine orchestration reserves interiors and admits parcels only on infrastructure completion. These modules contain no renderer, RNG, or mutable world dependency. Optional parcel rotation and a default-false coordinated-plan marker preserve legacy save behavior; rendering continues to consume the authoritative building rotation.

`engine/src/frontage.rs` owns pure interval subtraction/union and bounded gap subdivision. Engine orchestration projects current reservations into road coordinates, then validates resulting proposals through existing construction and access rules. The helper's global output cap is 64; ordinary infill requests only eight per side. Its deterministic output is derived from saved world state, while scan cursors remain persisted. Per-evaluation caches expire on admission so the next evaluation sees the new parcel.

`engine/src/development.rs` owns state-free building-site selection over a rotating window of at most 32 eligible parcels. Minimal open-building points supply local activity and proximity. The engine retains eligibility, construction mutation, and the persisted cursor. Eligibility collection and the building-point pass remain global; this is not a spatial index or a full fixed-work scheduler.

`engine/src/road_split.rs` owns the engine's open-road split operation, including precomputed frontage and traveler/bus/train route remaps followed by mutation. It remains an `impl Engine` module with access to authoritative state; this extraction is an ownership boundary, not a state-independent service. `network_split.rs` retains the pure route transformation helpers. Paired splitting now preflights both hosts together, with cumulative actor remaps and combined ID/segment/route caps, before committing either. It preserves the active-plan guard, with an exception only for the validated owning spine plan and its canonical pair of host midpoints. `neighborhood_spine.rs` owns that staged construction lifecycle; `spine_plan.rs` derives the versioned geometry and reservation contract. Bounded deep candidate enumeration is integrated in public WASM. Production keeps the existing nominal-area ranking; a separate test-only site-count comparison has mixed density results and is not promoted. Calling the single-road operation twice remains non-atomic.

Master-plan checkpoint (2026-10-11): `engine/src/masterplan.rs` is a pure, state-free generator. It produces the transport skeleton, perturbed arterial lattice, recursively split local streets, polygonal blocks and gap-free lots (including flag lots) from the seed and river geometry. Its decisions use only IEEE-exact arithmetic, so native and WASM regenerate identical plans; saves carry a plan digest that restore verifies. It owns no RNG stream shared with the simulation: it uses its own hash of the seed. The engine maps plan nodes to world nodes lazily as roads are realized. The legacy frontier, infill, access-discovery, spine and coordinated-block planners stay compiled only for the legacy test fixtures (`Engine::new_legacy`); new worlds never run them. Delete them once the master-plan path passes acceptance.

## WASM interface

Expose coarse operations: initialize definitions and seed; enqueue command batch; step a tick batch; extract compact changes; query inspector data; serialize; restore. Use wasm-bindgen for the interface and explicit binary layouts for hot data.

Batch boundary crossings. Never invoke WASM once per agent from the renderer. Avoid rebuilding a JavaScript object tree of the entire city each frame.

Rear-extension checkpoint (2026-10-11): `engine/src/rear_extensions.rs` owns
bounded admission, parcel reservation, pending construction, completion and
validation. Save activity schema 4 carries both pending plans and the earliest
original footprint for each extended building. Structural snapshots expose
`rear_extensions` and `building_original_footprints`; the renderer preserves the
original kit and adds the rear volume independently of style. Geometry and
capacity remain authoritative engine fields. Incremental neighborhood batches
report exact descriptor visibility only after their atomic scene commit, allowing
temporary construction geometry to remain until its replacement is visible.
See [the checkpoint and verification limits](../validation/HANDOFF.md#inhabited-rear-extensions--implementation-in-progress-2026-10-11).

## Worker protocol

Every message carries protocol version and world epoch. Epoch changes on load or new world; discard messages from old epochs.

| Message | Fields |
| --- | --- |
| Initialize | seed, engine version, world profile, definitions |
| Commands | sequence, requested tick or next boundary, commands |
| Acknowledgment | applied sequence and tick |
| Structural delta | revision, created, updated, removed IDs |
| Movement snapshot | tick, positions, orientations, activity flags |
| Activity events | bounded event batch, tick, positions, identities |
| Inspector result | request ID, entity ID, current summary |
| Checkpoint | save version, serialized engine payload |
| Metrics | tick cost, routing backlog, memory, active counts |

Publish movement at 10–20 Hz initially. Interpolate between two snapshots with a short presentation delay; avoid extrapolating through intersections. Structural changes are applied in revision order. Reliable deltas are never dropped; request a full resync if a gap is detected.

## Buffer ownership and backpressure

WASM linear memory remains worker-owned. Copy render-relevant fields into pooled transferable ArrayBuffers, send them, and return released buffers to the worker. Do not transfer or detach the engine's live memory. This is one bounded copy, not a zero-copy claim.

Keep a small pool and at most two unconsumed movement snapshots. Coalesce obsolete movement updates under backpressure; retain structural changes and required history. Limit event batches and aggregate repetitive ambience events.

WASM memory growth invalidates cached JavaScript views; recreate views after growth. Prefer predictable reservation with a memory ceiling and allocation metrics.

SharedArrayBuffer and a synchronized ring are later optimizations only after profiling. They require cross-origin isolation headers and disciplined ownership. The transferable-buffer path remains supported. Do not introduce multi-worker authoritative mutation in the first engine.

## Main-thread frame

Process bounded incoming changes, interpolate presentation transforms, update camera and sun uniforms, update visible instance buffers, render. Expensive asset construction is queued and spread across frames. Preload assets for projects before their opening stage. Display a valid low-detail asset until a richer one is ready.

## Extending the world

Building and district definitions use validated versioned schemas. Systems declare cadence, read/write components, and dependencies; engine initialization resolves a stable execution order and rejects cycles. Authoritative systems are compiled modules initially, not arbitrary third-party scripts.

A new renderer implements material creation, style parameters, water treatment, pass construction, and disposal. It cannot import the simulation mutator API. A new building type normally adds definition data and assets, with engine code needed only for genuinely new behavior.

### Offline morphology sources

PLATEAU or other city-model processing belongs in an offline reference pipeline. Produce versioned compact morphology definitions with stable source IDs, local coordinates, source bounds and a provenance/license/transformation manifest. Optional simplified geometry is a separate presentation asset; it must fit authoritative bounds and the existing construction, picking and style contracts. The renderer cannot modify authoritative geography or growth based on downloaded source geometry. Keep CityGML parsing, geospatial conversion and source statistics out of the browser simulation loop. See the [bounded pilot](../validation/plateau-research.md); no dataset or asset is currently cleared for runtime import by this research.

## Saving and lifecycle

Store save payloads and compact metadata in IndexedDB. Keep two rotating checkpoints with transactional metadata updates. Autosave periodically and on important transitions. Request a checkpoint on visibility loss; also keep a recent checkpoint because unload work is not guaranteed to finish.

Hidden tabs pause simulation and suspend or fade sound. Resume restores continuity without background catch-up. Handle storage quota failures visibly and offer export. Saved asset references use stable IDs. Backward save compatibility and migrations are not required (user decision, 2026-10-11). Explicitly reject incompatible formats and offer a fresh world. Current-format restore must validate master-plan mappings against regenerated entity identities, geometry and frontage, as well as existing world invariants; a plan digest alone is insufficient. Replay tests must continue the original live engine against a restored copy, including construction in progress.

## Technical references

- [WASM and JavaScript integration](https://wasm-bindgen.github.io/wasm-bindgen/)
- [WebAssembly concepts and memory](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Concepts)
- [Worker communication and transferables](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Using_web_workers)
- [Shared memory isolation](https://developer.mozilla.org/en-US/docs/Web/API/WorkerGlobalScope/crossOriginIsolated)

Pin tested dependency versions at implementation time.
