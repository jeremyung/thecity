# Neighborhood stage review — 2026-10-15

## Scope and controls

All three seeds (0, 5 and 87231) now have 72 complete captures in total: six fixed poses (corridor, frontage, interior, station, waterfront, overview), two saved stages (ticks 10000 and 83000), and both styles. Every capture waited for object, chunk, sidewalk, road and compile queues to empty. Camera, target, lens and quality match numerically across all four captures of each pose; no page errors occurred.

Renderer: PR19 checkpoint `5dcd457`, Chromium 145, WebGL/ANGLE Metal, Apple M2 Max/macOS. Buffer 1200×800, pixel ratio 1, fixed balanced quality, depth of field off. Presentation copies normalize weather to clear/intensity zero, day to zero, and time to 17:00. The original saved files are unchanged; source hashes and original weather/day/time are in the report. This isolates stage geometry from seasonal/weather differences. These are saved control worlds rendered directly, not a new production scheduler or save-restoration test. All six saves were subsequently restored with current review WASM SHA-256 `77ff97860ed19941cc2130e335c9321d2b07534cccd2d1736b2f028e63b81b3e`. Terrain, nodes, roads, decks, rail platforms/alignments, parcels, buildings, building access, rear extensions, original footprints and ground match the old snapshot exactly. This confirms the captured geometry is still current on restore; it does not prove new-generation replay or future lifecycle behavior. HUD clock/population values are not synchronized by direct renderer injection.

Local harness: `.tools/review-neighborhood-stages.mjs`.
Local captures/reports: `.tools/neighborhood-stage-review/seed{0,5,87231}/` (85.2 MB of PNGs in total; no recordings). Captures remain outside the PR. `matrix-check.json` verifies all 72 captures and their matching controls.

## Findings against the locked concepts

| View | Developing → mature observation | Consequence |
| --- | --- | --- |
| Interior | Much of the large vacant right-hand area becomes streets and buildings. Parallel frontage rows and repeated long roof forms remain. | Early vacancy is partly construction history, not proof of permanent under-allocation. Do not add arbitrary infill there. Street/lot organization still needs review. |
| Frontage | The narrow central lane already has close-set buildings. The apparent lawn to its right becomes an expressway. | Reconfirms the prior authored infill study was invalid. This view needs corridor/frontage composition, not extra invented sites in the reservation. |
| Corridor | The mature expressway and adjacent surface arterial dominate the view even with additional buildings. | Evaluate the hierarchy and spatial relationship of major corridors; do not disguise the issue with fog or more vehicles. Earlier expressway-count experiments remain deferred. |
| Station | New foreground towers obstruct the station from the exact developing camera. | Retain this weak-view evidence, but do not grade detailed station geometry from the obstructed mature image. An additional pose must be validated against mature obstacles before using it for both stages. |
| Waterfront | Broad water, a direct rail crossing and a road bridge divide small building groups; banks repeat sparse tree spacing and hard edges. | The concepts organize a coherent sequence of bank, local street, frontage and curving rail. This is a corridor-relationship problem as well as a later planting/water problem. |
| Overview | Build-out is substantial. Large crossing expressways and repetitive strips dominate the overall composition; the city boundary remains conspicuous. | Building count alone is not the missing target. The next structural hypothesis must explain corridor dominance and repeated block organization. |

Both locked boards were inspected again. They deliberately include open land, especially early in growth, but organize it with banks, paths, vegetation and construction. Filling all green pixels is not fidelity. Current Anime is darker and more outlined; Miniature is cleaner, but their shared geometric issues persist. This is not user visual acceptance.

A local 190×190m world-coordinate window centered at (48,-240) grows from 67 to 79 open buildings. This is a spatial diagnostic, not a projected screen-density measure. It supports distinguishing delayed build-out from unchanged rows; it does not prescribe a target count.

## Next investigation and stop condition

The three-seed review is complete. Seeds 5 and 87231 confirm that substantial developing vacancy fills, while mature interiors retain parallel strips and repeated frontage rows. Major elevated corridors dominate all three overviews. All three station poses become partially obstructed by towers; retain those weak views, but do not use them to certify station detail. A separately labeled station pose would need mature obstacle checks before use in both stages.

Direct mature-save checks show 0 unreleased lots, 0 unbuilt unprotected parcels, 16 unbuilt protected parcels and 0 unfinished project records in every seed (76 completed project records remain stored). This rules out unfinished ordinary construction as the explanation for mature residual spaces in these witnesses. It does not make every unoccupied polygon legally buildable. See `mature-occupancy-check.json` and `current-save-check.json` locally.

Previous compact-expressway and single-route studies were re-read. They already showed limited composition gains without solving local rows and leftover ground. The new matrix does not justify another width/count variant.
Then select one reusable intervention in the relationship between local street hierarchy, frontage blocks and major corridors, supported by cross-seed views and reservation ownership. The planner's rail frontage is initially organized around a horizontal band (`rail_z`); whether that contributes to the observed river/rail composition must be checked against the actual curved alignment and other seeds before changing it. A bank-following rail implementation is not yet justified by this observation alone, and no real-world geographic claim is made.

Do not resume residual-infill fitting, width/depth sweeps, tiny facade additions or contour-budget tuning. No generator or rendering-policy change was made in this review. Urban fabric remains the primary workstream and remains unaccepted.


## Selected next design study

Use one whole, genuinely reserved buildable block to study street and frontage organization together before parcel assignment. Preserve its outer junctions and all major-corridor/protected reservations. This must be an explicit replacement-layout study, with every replaced internal lot/road listed; never portray it as adding legal infill to existing reservations. Establish the ground/clearance ledger and connected entries before rendering buildings. The earlier ten-site counterfactual failed this prerequisite.

Visible target: break the long parallel-strip rhythm while creating a connected local lane and varied, continuous frontage, with less purposeless residual ground. Compare ordinary interior and wider context poses using unchanged material/camera/lighting. A diagram or higher lot count is insufficient. Stop if the design requires encroaching on future infrastructure, merely shifts empty space, or only looks better from a selected close-up. Do not implement a new general planner until the bounded block study demonstrates a plausible scene-level improvement. Existing rejected strip, residual-retrofit and nearest-edge algorithms are controls, not new candidates.
