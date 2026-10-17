# Architecture

The Rust engine runs inside a Web Worker and owns all simulation state. The browser
main thread renders read-only snapshots with Three.js; React handles controls only and
stays out of per-frame work. Design detail: [specs/04-architecture.md](specs/04-architecture.md).

```text
engine (Rust → WASM)  ──snapshots, movement buffers──▶  worker  ──postMessage──▶  app (main.tsx)
        ▲                                                  │                         │
        └──────────── commands (seed, speed, save) ────────┘          render ◀── camera rig, audio
```

## Engine (`engine/`, a Cargo workspace)

| Crate | Owns |
|---|---|
| `crates/plan` (`city-plan`) | Pure, deterministic planning: master plan, geometry, blocks, frontage, rail alignment, road curves, street cross-sections, and the shared kind enums. No engine state, no shared RNG. |
| `.` (`tokyo-city-engine`) | The simulation: `Engine`, `WorldState`, the tick, every domain |
| `crates/wasm` (`city-wasm`) | wasm-bindgen bindings only; the worker's entry point |

Inside the engine crate (`engine/src/`):

| Folder | Owns |
|---|---|
| `core/` | `Engine`, `WorldState`, clock and RNG, limits, the system registry (`systems.rs`), wire layouts (`wire.rs`) |
| `world/` | Parcels, river, clearance checks, spatial indexes; `LandState` |
| `infrastructure/` | Road network, construction, growth, master-plan realization, station towers, renewal; `NetworkState`, `ConstructionState` |
| `population/` | Residents, travelers, errands, demand, district measurement; `PeopleState` |
| `transit/` | Trains, buses, traffic, stations; `TransitState` |
| `persistence/` | Snapshot, save/restore, validation, movement packets |
| `legacy/` | Retired incremental planners. Compiled only with the `legacy` feature; delete once master-plan acceptance passes. |
| `experiments/` | Test-only diagnostics and experiments |
| `tests/` | Unit tests by domain |

`WorldState` holds one sub-state per domain (`network`, `land`, `construction`,
`people`, `transit`, `legacy`). They are flattened into the save, so save keys are
unchanged.

### The tick

`core/systems.rs` lists every system with a name, phase and cadence, in run order.
To add one, write the function in its domain module and add one `System` entry at the
position its phase needs. The order is part of determinism: moving an entry changes
replays.

## Web (`web/src/`)

| Folder | Owns |
|---|---|
| `main.tsx` | The composition root: builds the renderer, camera director, audio and worker client and wires them together |
| `app/` | Glue between layers (audio scene assembly) |
| `worker/` | WASM loading, tick scheduling, the versioned, epoch-tagged protocol |
| `render/` | Turning snapshots into Three.js geometry. `city.ts` coordinates; `roads-layer`, `landscape-layer` and `actors-layer` hold the big subsystems; `layers.ts` registers small self-contained layers |
| `camera/` | The camera director (Watch, Follow, Explore), anchors and framing |
| `optics/` | Lens, focus and tilt-shift math shared by camera and render |
| `audio/` | Generated ambience and score |
| `ui/` | React controls and inspectors |
| `storage/` | IndexedDB saves and checkpoints |
| `generated/` | TypeScript types and wire constants generated from Rust (`npm run types`). Never edit by hand. |
| `dev/` | Development-only routes, loaded only by the dev server |

Import rules (enforced by `npm run lint:deps`): render never imports worker, storage,
ui, camera, audio or app; ui never imports render, worker, camera or audio; the worker
never imports presentation code; optics imports nothing.

## Where to change X

| Change | Where |
|---|---|
| A simulation rule | The domain module that owns the state, plus a `System` entry if it runs every tick |
| A pure planning or geometry rule | `engine/crates/plan` |
| A snapshot field | The Rust type, then `npm run types`; bump `WORKER_PROTOCOL_VERSION` for wire changes |
| The movement buffer layout | `engine/src/core/wire.rs`, then `npm run types` |
| A new visual feature | A `RenderLayer` in its own `web/src/render/` file plus one line in `render/layers.ts` |
| A camera behavior | `web/src/camera/` |
| Test fixtures | `tests/fixtures/` (large ones come from `npm run setup:fixtures`) |
