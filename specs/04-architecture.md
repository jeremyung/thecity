# Browser Architecture

### Movement support presentation

An integer packet associates each pose with its supporting road and
platform identities. Worker protocol v5 transfers it with the presentation token.
The client validates token/epoch and decodes before recycling the buffer. The latest
[validation](../validation/movement-support.md) records transit endpoint fixes
and sampled fresh-world support coverage. Integration must match the packet's
authoritative geometry revision and world epoch to committed infrastructure,
preserve follow-camera continuity, and retain bounded transfer/backpressure.
Entity IDs alone do not prove the supporting geometry version is visible.

Engine snapshots and the cheap WASM `presentation_revision()` getter share an
opaque string token: access geometry generation plus station visual/readiness
phases. The worker publishes a matching structural snapshot before movement
whenever this token changes, bypassing the ordinary 500 ms snapshot throttle.
The token is epoch-scoped and preserves u64 precision. Exhausted generations
fall back to a per-tick token. Actor support gating retains the last rendered pose
only while its exact support remains committed; new actors await their geometry.
Interpolation retains both endpoint dependencies until complete. The camera and
spatial audio consume presented positions; hidden actor IDs suppress structural
follow fallbacks. Integration is implemented, but full motion/performance
acceptance remains open. The token is conservative because
access reservations can advance its geometry component without changing pixels.

Use Rust compiled to WASM for the engine, a dedicated simulation worker, and TypeScript for the application, rendering bridge, camera, and audio. Deploy as static files over HTTPS. Runtime simulation does not require a backend.

## Module layout

```text
engine/                    Cargo workspace
  crates/plan/             pure planning and geometry (master plan, blocks, frontage, kinds)
  crates/wasm/             wasm-bindgen bindings only
  src/core/                engine, world state, clock and RNG, system registry, wire layouts
  src/world/               parcels, river, clearance, spatial indexes
  src/infrastructure/      network, construction, growth, master-plan realization
  src/population/          residents, travelers, errands, demand
  src/transit/             trains, buses, traffic, stations
  src/persistence/         snapshots, save/restore, validation, movement packets
  src/legacy/              retired planners (feature `legacy`)
web/src/
  worker/          WASM loading, scheduler, protocol
  render/          scene coordinator, render layers, kits, water, styles
  camera/          manual controls and scene director
  optics/          lens, focus and tilt-shift math
  audio/           ambience, emitters, music director
  ui/              React controls and inspectors
  storage/         IndexedDB and save coordination
  generated/       types and wire constants generated from Rust
```

React never owns per-frame transforms or simulation entities. The renderer receives a read-only presentation model. The worker is the sole owner of authoritative state. Each `WorldState` sub-state belongs to one domain folder; the tick runs the systems listed in `core/systems.rs` in order. [ARCHITECTURE.md](../ARCHITECTURE.md) maps where each change belongs.

`city-plan`'s `blocks.rs` owns state-free rectangular subdivision with a hard 64-lot result bound. `geometry.rs` supplies oriented-rectangle overlap, bounds, and segment clearance. Engine orchestration reserves interiors and admits parcels only on infrastructure completion. These modules contain no renderer, RNG, or mutable world dependency. Optional parcel rotation and a default-false coordinated-plan marker preserve legacy save behavior; rendering continues to consume the authoritative building rotation.

`city-plan`'s `frontage.rs` owns pure interval subtraction/union and bounded gap subdivision. Engine orchestration projects current reservations into road coordinates, then validates resulting proposals through existing construction and access rules. The helper's global output cap is 64; ordinary infill requests only eight per side. Its deterministic output is derived from saved world state, while scan cursors remain persisted. Per-evaluation caches expire on admission so the next evaluation sees the new parcel.

`city-plan`'s `development.rs` owns state-free building-site selection over a rotating window of at most 32 eligible parcels. Minimal open-building points supply local activity and proximity. The engine retains eligibility, construction mutation, and the persisted cursor. Eligibility collection and the building-point pass remain global; this is not a spatial index or a full fixed-work scheduler.

`infrastructure/road_split.rs` owns the engine's open-road split operation, including precomputed frontage and traveler/bus/train route remaps followed by mutation. It remains an `impl Engine` module with access to authoritative state; this extraction is an ownership boundary, not a state-independent service. `network_split.rs` retains the pure route transformation helpers. Paired splitting now preflights both hosts together, with cumulative actor remaps and combined ID/segment/route caps, before committing either. It preserves the active-plan guard, with an exception only for the validated owning spine plan and its canonical pair of host midpoints. `neighborhood_spine.rs` owns that staged construction lifecycle; `spine_plan.rs` derives the versioned geometry and reservation contract. Bounded deep candidate enumeration is integrated in public WASM. Production keeps the existing nominal-area ranking; a separate test-only site-count comparison has mixed density results and is not promoted. Calling the single-road operation twice remains non-atomic.

## WASM interface

Expose coarse operations: initialize definitions and seed; enqueue command batch; step a tick batch; extract compact changes; query inspector data; serialize; restore. Use wasm-bindgen for the interface and explicit binary layouts for hot data.

Batch boundary crossings. Never invoke WASM once per agent from the renderer. Avoid rebuilding a JavaScript object tree of the entire city each frame.

The structural snapshot includes `road_decks` entries (`road_id`, `deck_height`)
for realized master-plan expressways. They are derived from the regenerated plan
and persisted road mapping, rather than guessed from road bearing in the renderer.
Deck height participates in the plan digest. A changed plan rejects old saves;
no migration is provided. Geometry invalidation includes this metadata so deck,
markings, lamps and piers use a consistent elevation.

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

Elevated movement contract: worker protocol 3 uses
six Float32 values per actor: `[id, x, z, heading, kind, surfaceY]`. Actor kind
continues to identify person/car/train/bus (0/1/2/3). A nonnegative `surfaceY`
is an authoritative absolute pedestrian or vehicle surface height; -1 delegates
to the existing ground/road presentation policy. Do not infer elevated access
from proximity to a station, because other travelers can pass underneath.
Interpolate resolved pedestrian surfaces along with horizontal positions and
use the same elevation for picking geometry, follow targeting and accessories.
Structural snapshots supply `rail_platforms` terminal frames and stair dimensions
for both styles. See [implementation evidence and remaining acceptance](../validation/station-passengers.md).

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

Expressway geometry contract: each `road_decks` entry includes
`controls: [[x,z],[x,z]]` in world metres, oriented from the realized road's
`from` node to its `to` node. These cubic controls and deck height derive from
the complete regenerated plan and participate in its digest. Renderers consume
them directly; they must not infer tangents from the currently realized graph.
Current saves regenerate the same controls; previous plan digests reject without
migration. Curve clearance is enforced during lot admission. Road routing and
construction costs still use the endpoint graph. Regional through-traffic uses
cached arc-length samples of these curves, persisted trips and a 48-car cap.
Only complete open boundary-to-boundary chains admit traffic; their regional
connection is independent of ground-road gateway connectivity. Ramps, lane
changes, merges and ramp grades remain future work.

Dated implementation notes for this spec: [validation/spec-notes/04-architecture.md](../validation/spec-notes/04-architecture.md).
