# Checkpoint notes moved from specs/04-architecture.md

Dated candidate, checkpoint and status notes, moved verbatim on 2026-10-16 so the
spec describes current design only. Newest decisions win over older notes.

Master-plan checkpoint (2026-10-11): `engine/src/masterplan.rs` is a pure, state-free generator. It produces the transport skeleton, perturbed arterial lattice, recursively split local streets, polygonal blocks and gap-free lots (including flag lots) from the seed and river geometry. Its decisions use only IEEE-exact arithmetic, so native and WASM regenerate identical plans; saves carry a plan digest that restore verifies. It owns no RNG stream shared with the simulation: it uses its own hash of the seed. The engine maps plan nodes to world nodes lazily as roads are realized. The legacy frontier, infill, access-discovery, spine and coordinated-block planners stay compiled only for the legacy test fixtures (`Engine::new_legacy`); new worlds never run them. Delete them once the master-plan path passes acceptance.

Rear-extension checkpoint (2026-10-11): `engine/src/rear_extensions.rs` owns
bounded admission, parcel reservation, pending construction, completion and
validation. Save activity schema 4 carries both pending plans and the earliest
original footprint for each extended building. Structural snapshots expose
`rear_extensions` and `building_original_footprints`; the renderer preserves the
original kit and adds the rear volume independently of style. Geometry and
capacity remain authoritative engine fields. Incremental neighborhood batches
report exact descriptor visibility only after their atomic scene commit, allowing
temporary construction geometry to remain until its replacement is visible.
See [the checkpoint and verification limits](../validation/archive/2026-10/HANDOFF.md#inhabited-rear-extensions--implementation-in-progress-2026-10-11).

## Train door state checkpoint — 2026-10-11

`TrainView.door_open_fraction` and `door_terminal` are derived from existing
terminal position and dwell countdown, rather than a second persisted clock.
Station/gateway dwell lasts 220/180 ticks, with 20-tick opening and closing phases
at 20 Hz. Boarding and alighting require the fully open interval. Boarding cadence
is three ticks to retain the existing 48-passenger capacity within that interval.
Quiet-hour station holds keep doors closed. Worker protocol v4 includes a compact
`trainDoors` Float32Array alongside each movement message: `[id, terminal, fraction]`
with terminal -1 away from a stop. At most eight trainsets use 96 bytes. The WASM
export shares the snapshot phase calculation without constructing a full world.
Doors and positions use the same pair of updates and interpolation fraction;
coarse world snapshots cannot rewind animated door state. The first movement
sample displays the restored phase immediately. A platform-position guard keeps
fast-forward skips from displaying open doors while moving. Full passenger
approach choreography and visual motion acceptance remain outstanding.
