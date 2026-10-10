# Exact access-corridor check for the largest mature face

This diagnostic tests the geometry reported by the 2 m raster audit against the engine’s current Rust predicates. It uses the public checkpoint `validation/early-reservation-wasm-87231-save.json` (seed 87231, tick 242000, SHA-256 `f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e`). The world is restored into an in-memory engine; no source save or production world is mutated.

The face has 22,936.24 m² of polygon area and 12,400 m² of raster-classified free component. The engine’s current exact proposal enumerator examined 4,944 candidates (206 eligible ordinary roads × 24 proposals per host) and produced zero proposals in this world. The scan is tied to this checkpoint and its fixed proposal recipe; it does not establish that all conceivable street alignments fail.

## Raster candidate, checked by the engine

The proposed host split on road 3317 at `t=0.72` passes split preflight. The suggested straight branch from `(237.859, -3.862)` to `(207, 1)` is 31.24 m long. Its bounds, endpoint spacing, junction heading, river and protected-land checks, and separation from other roads pass. Its exact road-to-parcel clearance is only 4.0529 m, below the 5 m requirement, so the branch is rejected. The candidate frontage lots consequently fail parcel admission; this is the decisive blocker for that direct chord.

A two-segment route through `(229, 1)` has tested leg lengths of 10.11 m and 22 m. Both legs pass the checked parcel-corridor, river, and protected-land predicates, and two 8×8 m lots at `(224, 13)` and `(214, 13)` pass the checked parcel/frontage geometry when both legs are included as roads. This is only a geometric diagnostic: current `InteriorAccessPlan` represents a single branch, so it cannot reserve or construct the two-leg route. The first leg’s short length and junction spacing also need validation in any multi-segment design.

## Existing one-branch lifecycle constraints

A bounded endpoint search sampled 855 points in the target face for branches 26–60 m long from the same split. Of these, 632 failed exact parcel-corridor clearance, 93 failed junction-angle rules, 66 failed junction spacing, and 62 left fewer than two exact-valid lots. One straight geometry candidate runs from `(237.859, -3.862)` to `(181, 15)`, length 59.906 m. Its two 12.502×12 m lots at `(204.707, -8.668)` and `(190.752, -4.039)` pass exact geometry and frontage checks, with 300.05 m² combined nominal envelope area. The road split passes preflight.

The earlier endpoint-grid diagnostic had a loop post-increment error: it computed candidate length at one z sample but used the next z sample for later checks and JSON output. I corrected that and reran the scan. A focused lifecycle test now submits the corrected `(181,15)` geometry through normal admission, restores the pending plan, completes it, creates the promised parcels, and compares resumed completion byte-for-byte. It passes under the generalized validator below.

A separate scan of the currently supported split fractions and lengths evaluated 128 post-split options (136 were rejected during host-split preflight). It found no saved-lifecycle candidate in the target face. For host 3317, inward-facing options at `t=0.5` and `t=0.75` fail parcel-corridor clearance; the opposite side lies outside the selected face. The fixed proposal recipe does not fit this particular void.

## Interpretation and next design

The production frontage normal calculation is correct: the planner’s `nx,nz` are the branch tangent, and rotating that vector yields the perpendicular lot offset. No production change was made for the earlier suspected normal bug.

The evidence points to candidate parameterization, not a general clearance defect. A next one-branch planner could derive split positions from exact safe intervals along a host road and derive endpoint/heading candidates from free-face geometry, instead of trying only fixed fractions and perpendicular branches. The save validator now accepts any finite interior split with at least 10 m on both host halves and any 24–60 m branch that meets ordinary junction-angle rules. It still checks endpoint spacing, bounds, river/protected land, all-road clearance, reserved geometry, and every lot’s exact frontage and collision constraints. The corrected straight candidate is now fully admitted, restored, completed, and replayed in a native test. The two-leg route would still require a distinct multi-segment persisted lifecycle.

Diagnostics and focused test source are in `engine/src/face_access_exact_tests.rs`; machine-readable results are `validation/face-access-exact-gates.json`, `validation/face-access-exact-planner-scan.json`, `validation/face-access-legal-candidates.json`, and `validation/face-access-straight-candidates.json`.

Focused checks passed:

- `npm run test:engine -- --release audit_largest_face_direct_access_candidate -- --ignored --nocapture`
- `npm run test:engine -- --release scan_exact_planner_for_access_into_largest_face -- --ignored --nocapture`
- `npm run test:engine -- --release geometry_valid_noncanonical_interior_access_plan_restores_and_completes -- --nocapture`
- `npm run test:engine -- --release` (198 passed, 18 ignored)

The focused exact candidate lifecycle test and full native release suite pass (198 passed, 18 ignored). The generator remains unchanged, so no naturally generated growth result is claimed. No public WASM rebuild was requested.
