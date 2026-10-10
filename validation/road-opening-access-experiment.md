# Road-opening access experiment

Matched native runs compare the current scheduler with a test-only treatment that, when a connected ordinary road opens, tries up to 24 access-street proposals on that road before admitting its frontage parcels. Both sides use the same source, seed, rate, and horizon. The earlier access scheduler and district policy experiments were disabled; the normal late access planner remains enabled. The branch and selected lots use the existing proposal validators, atomic split/admission path, construction project, and save/restore logic.

| Seed | Opening-time treatment | Access plans admitted at opening | All access completions | Open buildings | Connected open roads | Ordinary road length | Building envelope proxy | Floor-area proxy |
|---:|:---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | off | 0 | 1 | 866 | 212 | 14,015.8 m | 157,293.9 m² | 430,666.8 m² |
| 0 | on | 30 | 33 | 849 | 246 | 14,063.6 m | 155,376.0 m² | 442,314.1 m² |
| 87,231 | off | 0 | 1 | 851 | 198 | 13,405.5 m | 157,667.5 m² | 463,532.9 m² |
| 87,231 | on | 36 | 39 | 810 | 245 | 13,296.2 m | 147,720.2 m² | 399,294.1 m² |

The treatment consistently opened more connected roads, while both seeds ended with fewer buildings and less nominal envelope area. Floor-area proxy rose slightly for seed 0 and fell substantially for seed 87,231. This is mixed evidence and does not establish a density improvement. The opening-time plan also needs an explicit frontage-reservation guard: without it, a legacy axis-aligned host-frontage candidate could overlap an access lot after its rendered frontage rotation was applied. The guard runs only while an access plan is pending and checks the actual frontage orientation, including tower candidates.

The focused lifecycle tests verify that a new road can admit an access plan before frontage creation, the pending branch appears in the ordinary road-collision context, frontage lots avoid both the branch and reserved lots, and the normal construction path completes identically after canonical save/restore. A mature no-op case verifies an infeasible 24-proposal host leaves serialized state and the global planner cursor unchanged. The matched-run harness validates each world and checks canonical restore plus a 2,000-tick twin continuation.

The exact metrics, elapsed time, save/snapshot hashes, and raw artifacts are in [`road-opening-access-experiment.json`](road-opening-access-experiment.json). The paired save and snapshot files use `road-opening-access-seed-{0,87231}-enabled-{false,true}-{save,snapshot}.json`.

Run provenance: `Engine::new(seed)`, development rate 16, exactly 121 batches of 2,000 ticks, with `district_policy_experiment=false` and `early_access_experiment=false`. Treatment differs only by `road_opening_access_experiment`. Source SHA-256: `engine/src/lib.rs` `adbe68aa61dc9617a59ae095ddb7b19f8600aabc4621d59f4c6216746ea9bc65`; `engine/src/road_opening_access_experiment.rs` `a2046ca258e256409586b3188d46fe6fb3033338cd812c9f44b15c67a8974a53`.

## Frontage-guard verification

The full native release suite passed after the pending-plan guard was frozen: `npm run test:engine -- --release` — 176 passed, 0 failed, 10 ignored. The ignored tests are opt-in experiment/profile runs, including the matched road-opening treatment above.

The admission-path audit found no unguarded production frontage path. `add_frontage_parcels` applies the orientation-aware reservation check to both ordinary lots and station-tower lots before insertion. It then checks the pending road project through `parcel_geometry_clear`'s centerline clearance; that clearance is conservative for the supported lot envelopes. Progressive infill uses `progressive_frontage_geometry_clear`, which checks both selected-lot reservations and the exact pending-road corridor. Coordinated-block completion cannot coexist with a pending access plan, and access completion preflights its own reserved lots against the branch, then clears that plan immediately before inserting those lots. Initial founding frontage has no pending reservation to honor.

This verifies native behavior and save/replay coverage only. The road-opening scheduling experiment remains test-only and was not promoted; its matched density results are mixed as recorded above. WASM/browser integration and visual review remain with the parent task.

| Artifact | SHA-256 |
|---|---|
| seed 0, treatment off, save | `fb74d841dec5612dbd3e9ae418711d375fd212e441c6f0e094c6bde38c87590b` |
| seed 0, treatment on, save | `b02900eca9786881645957fbcd38981914abdcac652f74b42923d6b306521906` |
| seed 0, treatment off, snapshot | `04b82b4f2dc72069d910d1f7c268991e511e3ccfbcb426b21aeef54d0e95e103` |
| seed 0, treatment on, snapshot | `89a0062477f7144426ff43128db7618ee8d1212092446797aa9e19838d147591` |
| seed 87,231, treatment off, save | `e3ea0b5c097d89c31621a20f4625dab1e3f56bef8f4f9bb14b217ad3d0df707d` |
| seed 87,231, treatment on, save | `5ccde5537389699fadcc862830e8bf01eac34fede7b62e8adb67eac4c4e543a9` |
| seed 87,231, treatment off, snapshot | `bf98610f977ce20bc9c47658bb10a2f432185bfdd99dcba25fac4f62cd2c1a65` |
| seed 87,231, treatment on, snapshot | `6d620e0d1e4285aac1c3b41c3d73f7d13bdb5cf8cb79da3543ab11098436398a` |
