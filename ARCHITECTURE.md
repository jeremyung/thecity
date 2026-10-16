# Architecture

The Rust engine runs inside a Web Worker and owns all simulation state. The browser
main thread renders read-only snapshots with Three.js; React handles controls only and
stays out of per-frame work. Design detail: [specs/04-architecture.md](specs/04-architecture.md).

```text
engine (Rust → WASM)  ──snapshots, movement buffers──▶  worker  ──postMessage──▶  render / camera / audio / ui
        ▲                                                  │
        └──────────────── commands (seed, speed, save) ────┘
```

## Engine (`engine/`)

| Path | Owns |
|---|---|
| `src/lib.rs` | `Engine`, `WorldState`, the tick loop (`step_one`) |
| `src/geometry.rs`, `blocks.rs`, `frontage.rs`, `development.rs`, `masterplan.rs` | Pure, state-free helpers: no engine state, no RNG |
| `src/wasm/` | wasm-bindgen bindings (`CityEngine`) only |
| `src/tests/` | Engine unit tests, split by domain |
| `src/experiments/` | Diagnostics and experiments; compiled only for tests or the `experiments` feature |

The legacy incremental planners (`Engine::new_legacy`) compile only with the `legacy`
feature, which `npm run test:engine` enables. New worlds always use the master plan.

## Web (`web/src/`)

| Path | Owns |
|---|---|
| `worker/` | WASM loading, tick scheduling, the versioned, epoch-tagged protocol |
| `render/` | Turning world data into Three.js geometry; `city.ts` coordinates it |
| `camera/` | Lens, manual control and the automatic Watch director |
| `audio/` | Generated ambience and score |
| `ui/` | React controls and inspectors |
| `storage/` | IndexedDB saves and checkpoints |
| `types.ts` | The snapshot types the worker sends |

Dependency rules: `render/` never imports `worker/`, `storage/` or `ui/`. Only the worker
touches the engine.

## Where to change X

| Change | Where |
|---|---|
| A simulation rule | The engine module that owns that state |
| A new visual | A new `web/src/render/<feature>.ts`, wired in `render/city.ts` |
| A protocol field | Rust snapshot first, then `web/src/types.ts`; bump the protocol version |
| A camera behavior | `web/src/camera/` |
| Test fixtures | `tests/fixtures/` (large ones come from `npm run setup:fixtures`) |
