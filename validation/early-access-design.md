# Early access streets for coordinated blocks

This note specifies an access spine for the coordinated-block generator. Geometry, reservation, construction and save/resume are implemented and native-tested; candidate enumeration and validation are integrated in public WASM. The unchanged production area-based ranking favors ordinary blocks. A test-only site-count ranking generates completed spines, but matched seeds show mixed density outcomes, so that policy is not promoted. Browser review of an experimental mature save confirms rendered streets while leaving the dense-city visual gate open. See the latest `spec-audit.md` checkpoint and `spine-ranking-experiment.json`.

## Current block geometry

The planner in `engine/src/lib.rs` uses an 82 m by 50 m rectangle. `engine/src/blocks.rs::subdivide_rectangular_block` offsets lot rows 8 m inside each perimeter road centerline, uses an 11 m target frontage, and leaves a 2.2 m gap both between adjacent frontages and between the backs of the two rows. The current recipe therefore produces:

| Measure | Current 82 × 50 block |
| --- | ---: |
| Gross centerline rectangle | 4,100 m² |
| Interior after 8 m perimeter inset | 66 × 34 = 2,244 m² |
| Frontage slots per row | 5 |
| Actual frontage width after distributing remainder | 11.44 m |
| Lot depth per row | (34 − 2.2) / 2 = 15.9 m |
| Nominal lot envelopes | 10 × 11.44 × 15.9 ≈ 1,819 m² |
| Nominal envelopes / gross block area | ≈44.4% |
| Perimeter centerline road length | 264 m |

These are parcel-envelope calculations, not building footprints, floor area, or usable net land. Actual admission still applies existing parcel and road clearance tests. The block module caps subdivisions at 64 lots, well above this recipe's ten.

The renderer assigns ordinary roads a 9 m road width. Current coordinated blocks inset lots 8 m from perimeter road centerlines, leaving a more generous street edge than the 5 m minimum used by some ordinary-parcel checks. The spine design below also uses an 8 m centerline setback for its facing lots so the interior street retains that same visual margin; it must not silently use the smaller minimum to fit more parcels.

## Why an alley does not fit the current block

Adding a centered 9 m road parallel to the 82 m edges of the 82 × 50 block leaves 25 m from either outer road centerline to the alley centerline. One row on each side could have at most 25 − 8 − 5 = 12 m of depth, before any additional visual sidewalk margin; that already reduces the current row depth from 15.9 m. Preserving two back-to-back rows on each side would leave only `(25 − 8 − 5 − 2.2) / 2 = 4.9 m` per row, which is not a useful parcel depth. Thus a centered alley does not add lots at the current block width. It either replaces the current two-row layout with thinner one-row bands or forces four very shallow rows. Existing 82 × 50 blocks should keep the current two-row plan; a post-hoc alley would also collide with already admitted parcels.

## A deeper block recipe that can support an access spine

Use a separate, explicitly selected 82 m × 82 m block recipe where the road graph and terrain allow it. Keep the existing 82 × 50 recipe for shallow sites. The square's centerline alley runs 82 m between its two opposite perimeter roads and joins each at a real junction. It is an ordinary local road so delivery vehicles, residents, and service routes can use it.

For each of the two 41 m bands between the alley and an outer perimeter road, an 8 m setback at both the perimeter street and alley leaves 25 m. With the same 2.2 m rear gap, each row is `(41 − 8 − 8 − 2.2) / 2 = 11.4 m` deep. The 82 m frontage with 8 m endpoint insets remains 66 m, so each row has five 11.44 m lots. Across both sides of the alley, this gives four rows and twenty nominal lots:

| Measure | Proposed 82 × 82 spine block |
| --- | ---: |
| Gross centerline rectangle | 6,724 m² |
| Each side of alley, road-centerline to perimeter-road centerline | 41 m |
| Lot depth per row | (41 − 8 − 8 − 2.2) / 2 = 11.4 m |
| Frontage slots per row | 5 |
| Nominal lot envelopes | 20 × 11.44 × 11.4 = 2,608.32 m² |
| Nominal envelopes / gross block area | ≈38.8% |
| New access-spine length | 82 m |

Compare the alley to an ordinary two-row 82 × 82 block, not only to the smaller current recipe. Without an alley, the same 8 m perimeter inset leaves 66 m for two opposing rows; each row is `(66 − 2.2) / 2 = 31.9 m` deep, producing ten nominal envelopes of about 3,649.36 m² total, or 54.3% of gross block area. The alley version has twenty smaller-frontage sites but about 1,041 m² less nominal parcel envelope and a lower gross envelope share. Its purpose is access and finer-grain frontage down the block interior, not greater parcel area.

For scale, the road-surface proxy inside an 82 × 82 block is roughly 1,395 m² for the 9 m perimeter streets (four 4.5 m inward strips, less corner overlap). The alley adds roughly 657 m² of net in-block surface after its endpoint overlaps, for about 2,052 m² total. These are simplified road-width strips, not a surveyed mesh calculation. The two-row block leaves more area for parcels; the spine adds an internal street while consuming some of that area. If one perimeter edge is reused, three new perimeter edges plus the alley add about 328 m of new centerline; if none is reusable, four perimeter edges plus alley total about 410 m. The benefit must justify that extra network length through actual internal access and occupancy, not nominal lot count alone.

An 82 × 74 m variant would leave `(37 − 8 − 8 − 2.2) / 2 = 9.4 m` per row with the same setbacks. Twenty 11.44 × 9.4 m envelopes total 2,150.72 m². It is not currently a production candidate. Any future comparison must retain the same street margins rather than switching to a smaller alley setback to improve the apparent yield.

## Persisted plan and construction lifecycle

Represent the spine as part of the existing `NeighborhoodPlan`, rather than starting an independent access plan that can overlap a block reservation. Keep the field optional with a serde default so legacy saves continue to load. The candidate must be selected and reserve its corridor and lots before perimeter construction can consume that space. At that point, some perimeter edges may be proposed edges without road IDs yet. The canonical recipe fixes host edges at edge 3 and edge 1, both at their midpoint. Resolve actual host road IDs only after those perimeter edges have materialized. The implemented canonical contract is:

```rust
struct SpinePlan {
    corners: [BlockPoint; 4],
    recipe_version: u8,
    stage: SpineStage,
}
enum SpineStage {
    Reserved,
    PerimeterReady { host_road_ids: [u32; 2] },
    UnderConstruction {
        host_road_ids: [u32; 2],
        split_node_ids: [u32; 2],
        spine_road_id: u32,
        project_id: u32,
    },
}
```

Corners and a versioned recipe reproduce the exact selected layout; do not serialize a second independently mutable list of lots or redundant endpoint coordinates. Recipe versions must retain their geometry semantics across saves. The two split nodes are the spine endpoints. Completion atomically admits the canonical lots and retires the plan.

Reserve all lot envelopes, both split points, the spine corridor, project/history slots, and the IDs needed for both host splits, the road, project, and eventual parcels as one preflight. Do not admit a candidate if any reservation cannot be held. Candidate evaluation remains read-only; only the winning candidate allocates IDs. Pure plan validation checks geometry and stage-ID shape only; engine integration must validate actual world references, terrain, capacity, and connectivity.

The plan proceeds through explicit stages:

1. Find a bounded deep-block candidate before perimeter roads or lots are admitted. Store the planned corners, opposite perimeter edge indices, split parameters, exact selected lots, and reserved spine corridor. Require enough cross-depth for four useful rows, legal split spacing on each host edge, and a clear route corridor. The opposite edges may be existing connected roads or proposed perimeter segments; for proposed edges, do not invent road IDs or split-node IDs yet. Exclude candidates whose full reservation cannot be held.
2. Build the perimeter using the normal visible-road project lifecycle. Once the two opposite host edges are open and connected, resolve their actual road IDs and directed endpoints from the stored edge indices. If an edge was reused in reverse orientation, convert its stored normalized parameter to the actual directed road orientation before splitting.
3. Atomically split both open host edges, preserving each existing road ID on its first half and assigning a new ID to its second half. Remap frontages, access metadata, traveler route distances, bus routes/stops/progress, and connectivity using the already-tested road-split primitive. The two resulting split nodes become the alley junctions; they are the spine endpoints, so separate endpoint-node fields are unnecessary. The perimeter remains traversable during alley construction.
4. Create one visible ordinary-road project between the split nodes. The spine stays disconnected/unusable while the project is in progress; synchronize the road's survey/construction stage and progress with its project. Keep its corridor and lots reserved against every other road, parcel, neighborhood-block, and interior-access admission path.
5. On completion, preflight the full stored layout against current geometry and capacity. Open/connect the spine, classify it as a neighborhood road, and admit all promised frontage records/parcels together. If the stored layout is no longer valid, leave the plan pending and report/retry the failed stage; never silently drop lots or partially open the spine.
6. Clear the plan and release reservations only after the road and all selected lots have been committed. Save/restore at every stage must reproduce the same continuation.

The plan must not reserve a whole district indefinitely: one spine plan at a time remains consistent with the current bounded generator. Block admission still requires demand and local growth eligibility. If demand does not support twenty eventual sites, do not create the 82 × 82 recipe merely to increase geometric capacity. Existing shallow blocks and the current four-edge/shared-edge layouts remain valid alternatives.

## Runtime integration checkpoints

The next implementation must cover the complete reserved-to-open lifecycle:

1. Carry an optional canonical `SpinePlan` in `NeighborhoodPlan`, reserving its twenty lots against the global parcel cap. Legacy plans omit the field.
2. Compare bounded deep-block candidates before allocating nodes or IDs. Reserve corridor, lots, two extra road halves, spine road/project, and final parcel IDs together.
3. Build the perimeter through existing road projects. Preserve the reservation after the fourth road opens; resolve hosts only when connected. Suppress incidental frontage admission into promised lots.
4. Preflight both host splits and cumulative traveler/bus/train route remaps before either commit. An active-plan exception must authorize only this plan's two resolved hosts, rather than disabling the existing guard globally.
5. Synchronize the spine road with its visible project. On completion, preflight all canonical lots, then open the road and admit all lots together. A failed completion remains pending and retryable.
6. Validate actual stage references before restore canonicalization, and verify deterministic continuation from every stage.

After splitting, the original perimeter corner-to-corner host pairs no longer identify single roads. Validate both retained and new half segments through their canonical endpoints and split nodes. The current recipe's new lots face edges 0/2 or the spine, so they do not require assigning new frontage to host-edge halves; existing frontage on those hosts still requires the normal split remaps. Construction readiness after splitting must use the persisted stage, rather than the old four-unsplit-roads predicate.

## Bounded selection and validation

### Ranking limitation under full admission

For the canonical axis-aligned recipes, the current area-per-new-centerline ranking has the following geometric values before world feasibility checks:

| Recipe | Nominal lot area | New centerline | Area / new metre |
| --- | ---: | ---: | ---: |
| Ordinary 82 × 50, all new perimeter | 1,818.96 m² | 264 m | 6.8900 |
| Spine 82 × 82, all new perimeter | 2,608.32 m² | 410 m | 6.3618 |
| Ordinary 82 × 50, reuse 82 m edge | 1,818.96 m² | 182 m | 9.9943 |
| Spine 82 × 82, reuse 82 m edge | 2,608.32 m² | 328 m | 7.9522 |

Therefore a fully feasible ordinary proposal beats its corresponding spine variant. Adding the variant to the candidate list does not establish useful selection frequency. A spine could compete where ordinary geometry or partial lot admission changes the comparison; demonstrate that with representative planner evidence. If this regime is absent or negligible in real seeds, revisit the documented urban-form objective explicitly rather than silently increasing spine scores or constructing artificial obstacles to pass a selection test. These calculations measure nominal envelopes, not rendered density, occupied capacity, or beauty.


Add at most one spine variant evaluation per already-bounded anchor/edge candidate, with a hard candidate budget and persisted cursor. Rank only fully validated, geometry-safe proposals using nominal admitted-envelope area divided by new road length, with deterministic ties by lot count, road length, and stable IDs. Compare it against current shared-edge and full-perimeter candidates before allocating IDs. A spine must not always win: a compact existing-edge block may deliver more usable frontage for less network cost.

Require all existing finite-world constraints: protected land, true river-and-bank reservation, world bounds, slope, connected host roads, endpoint/intersection spacing, exact parcel clearances including both historical and rendered envelopes for legacy parcels, road-road intersection checks, project/history caps, and parcel/ID capacity. For the alley corridor, test both the ordinary road corridor and rendered sidewalk clearance. Keep all existing parcels and their IDs unchanged.

Focused acceptance should establish geometry and behavior rather than a citywide building-count target:

- Pure layout tests verify the twenty lot centers, four frontage rows, setbacks, dimensions, orientation, non-overlap, and equivalent results under rotated or reversed block winding.
- A shallow 82 × 50 candidate is rejected for a spine without affecting the existing two-row plan; a sufficiently deep candidate can be admitted.
- The canonical recipe retains every existing parcel. Any overlap with a reserved lot or street rejects the entire candidate before ID allocation, as do protected/river land, road clearance, or capacity failures. Selectively omitting lots would require a separately specified persisted recipe; this implementation does not silently shrink its promised layout.
- Both host roads are split atomically and retain route, bus, frontage, and building-access continuity. The branch remains disconnected until construction completes, then opens as a connected ordinary road.
- Reserved parcels and corridor cannot be consumed by competing planners; the full stored layout is admitted on completion.
- Pending, completed, legacy, and malformed saves are tested for canonical restore and deterministic twin continuation. Corrupt host references, split positions, duplicate lots, noncanonical layouts, over-cap reservations, and unsafe corridors are rejected.
- Matched public-WASM seeds compare selected recipes, active project progression, usable/occupied frontage, gross parcel-envelope proxy, road length, and runtime. Report those separately; do not describe parcel envelopes as rendered building coverage or treat counts as proof of visual density.

## Limitations

The width/area arithmetic uses rectangular nominal parcel envelopes and the current centerline clearances. It does not prove that buildings fill those envelopes, that sidewalks leave the desired visual margin, or that demand will occupy all twenty sites. An alley is a viable geometric option only for a deeper block whose opposite perimeter edges become real connected host roads; it does not solve density by itself and sacrifices parcel-envelope area relative to the two-row 82 × 82 alternative. Renderer review should verify that the center lane reads as a narrow local street integrated with surrounding buildings and does not turn each block into an oversized road grid.

## Measuring the ranking experiment

The native experiment exports both authoritative saves and render snapshots. To measure the exact current building kit without restoring through an older public WASM, run:

```sh
node scripts/audit-rendered-coverage.mjs --snapshot validation/spine-ranking-seed-0-by-sites-true-snapshot.json --output validation/spine-ranking-seed-0-by-sites-true-coverage.json
```

`--input` continues to mean a save restored through public WASM; `--snapshot` consumes a render snapshot directly. Reports identify the input kind and its SHA-256. Root verified identical coverage, cells, methodology, and counts for a founder world supplied through either route (`coverage-snapshot-input-check.json`). This is a CLI equivalence check, not an experiment result. Projected geometry includes kit surfaces above one metre, uses a 0.5 m cell-center raster, and divides by gross world area; it does not measure net developable land or establish visual acceptance.
