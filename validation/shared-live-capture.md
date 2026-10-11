# Shared live access capture — 2026-10-11

Starting a live access search now calls `access_capture::capture_pair` at one immutable engine boundary. Discovery and proposal validation reuse one computed viability/collision projection instead of independently building it twice. The two owned inputs still contain separate geometry arrays because their existing contracts own them; a linear clone replaces the second projection and its lookup-map construction.

The shared path retains both adapters' independent caps and source ordering, with discovery failures checked first. A private-field `CapturedViability` token carries the originating immutable engine borrow, so the reuse helper cannot accept an unrelated engine alongside a stale projection. Standalone discovery and fresh-admission capture remain available. Neighborhood segment lookup preserves the first occurrence of duplicate node IDs and skips missing endpoints through an ordered index. No save schema changes.

## Verification

- Full native release suite: **355 passed, zero failed, 43 ignored**, 46.40 seconds.
- Initial, mature, legacy absent-rotation and duplicate/missing-node cases preserve capture results. Both adapters' cap failures leave the city unchanged. Existing frozen geometry oracle checks still pass.
- Root integration test compares the live startup checkpoint with the former two-capture assembly, then verifies byte-identical complete city saves after 200 real simulation ticks.
- Production WASM rebuilt. `node scripts/test-live-capture-wasm.mjs` exercises live startup on the first step, then 200 ticks with JSON save/restore every 17 ticks; complete final saves match uninterrupted execution. Source, WASM and final-save hashes are recorded in `shared-capture-wasm.json`.
- Existing Node WASM discovery profile still matches all **3,180 native reference descriptors**, with unchanged **7,380,416** logical units across quotas and JSON resume. See `access-discovery-wasm-profile.json`.

## Capture-only timing

Reproduce with `node scripts/test-engine.mjs --release profile_shared_capture -- --ignored`. The mature fixture has 160 nodes, 208 roads and 850 parcels. Four alternating-order rounds measure 250 captures per path. The final run was taken after the compilation and other validation processes completed.

Across rounds, native median time was **0.221–0.232 ms** for two independent captures versus **0.121–0.129 ms** for the shared path. See `shared-capture-profile.json` for each round. This is approximately a tenth of a millisecond at this fixture size, not an explanation for the prior 25 ms worker spikes. The benchmark excludes job constructors, output destruction, WASM boundaries and rendering. Machine load is not controlled and this is not a designated reference-device benchmark. No browser timing, frame-rate or maximum-world guarantee follows from it.

## Remaining scaling work

Capture remains synchronous and outside the per-tick planning quota. Larger limits require measuring worst-cap capture, constructor validation and actual worker costs. Making capture incremental needs a coherent frozen source or a validated revision protocol; reading arbitrary mutable engine vectors on different ticks would be incorrect. It must cover reservation/gate changes, save/resume, bounded restart behavior and peak memory, rather than merely charging a counter after an unbounded scan.

The audit confirmed the live trace uses trusted constructors; quadratic duplicate checks in the public topology constructor are not ordinary per-tick work. Trusted handoffs still reserve bounded buffers, and restore still validates/reconstructs whole checkpoints. The face walk already charges nesting one edge at a time, although its total pair work remains quadratic. These distinctions matter before changing their scheduling or caps.

This optimization leaves the 512-node/edge face limits and visual appearance unchanged. Dense urban form, building variety, water and lighting fidelity remain open product requirements.
