# Organic street-face layout diagnosis

This read-only diagnosis uses the mature seed 87231 browser-review save at tick
242,839 and the three largest connected empty-space masks in
`density-void-audit.json`. It narrows the earlier hypothesis: these masks do not
show 50–150 m road-isolated interiors. Their large bounding boxes describe
irregular connected shapes, not distance from the street network.

Reproduce the measurements from the repository root with:

```sh
python3 scripts/audit-void-road-distance.py
```

The script decodes each saved 2 m mask cell, constructs connected open local
and arterial road segments from the saved node endpoints, and computes exact
point-to-segment distance. It records hashes for both the mask report and its
source save. It does not infer road spacing from segment length. The input
hashes are `1d28c0ef61f1d2740461211c6df634333e4b07d6e1eb0b1cbc8bcf6343f15ee0`
for the audit and `c3b273bdbfb693683b4fbce4eb974cf25021bc8faa0834025f7f39e5dcaef646`
for the save.

## Measured pattern

The snapshot contains 836 buildings and 836 parcels, with no unbuilt parcels.
Of 215 open connected ordinary roads, 140 carry the `neighborhood_road_ids`
tag (65.1%); they account for 8,720 m of 14,030.8 m centerline (62.2%). Of 834
frontage records, 590 are tagged coordinated (70.7%) and 244 are on other roads
(29.3%). That is 67.66 frontage records per centerline kilometre for tagged
roads and 45.94 for other roads. The tag is a proxy for coordinated-plan
membership, not proof that each road borders a completed block interior.

| Mask | Empty cells / area | Road-face area | Nearest road p50 / p90 / max | Boundary tagged / other |
| --- | ---: | ---: | ---: | ---: |
| East-south | 3,144 / 12,576 m² | 28,828.5 m² | 12.49 / 25.81 / 40.50 m | 12 / 7 |
| West-north | 1,482 / 5,928 m² | 11,633.2 m² | 12.08 / 28.08 / 39.21 m | 6 / 6 |
| West-center | 1,461 / 5,844 m² | 11,289.1 m² | 13.31 / 28.47 / 38.34 m | 2 / 5 |

The three empty masks total 24,348 m², approximately 47% of their associated
road-face areas. Their characteristic face widths, computed as twice area over
polygon perimeter, are about 38–40 m. No mask cell is over 50 m from an open
ordinary-road centerline. The boundaries are mixed: coordinated tags dominate
the first face, split the second evenly, and are a minority on the third.
Consequently, the evidence does not support blaming only organic roads or
claiming these are 50–150 m deep roadless holes.

The stronger measured constraint is parcel lock-in. The mature access diagnostic
tested 5,160 bounded branch proposals and found zero feasible; 4,368 (84.7%)
first failed exact parcel-corridor clearance. An adaptive 4,668-branch sample
intersecting the masks also found zero feasible, with 4,527 (97.0%) first
rejected by parcel clearance. The masks are open space after subtracting the
current parcels and road corridors, but the available branch routes still cross
existing parcel envelopes. More proposal timing or finer sampling will not
remove those saved collisions.

In `engine/src/lib.rs`, connected road completion calls
`add_frontage_parcels(rid)` at line 2319. The ordinary frontage candidate path
creates 14 × 12 m envelopes at a fixed 15 m centerline offset (lines 3384–3438);
subsequent progressive infill admits more sites in the same roadside bands.
Those admitted parcel envelopes become hard collision reservations. By
contrast, a coordinated plan reserves its selected block lots before its
perimeter roads are completed. This difference in admission order is a concrete
way the ordinary-road path can leave irregular face interiors without planned
access, even though the city overall is mostly served by coordinated-tagged
roads.

## Bounded next change to evaluate

Use the new bounded road-face geometry groundwork to propose one persisted
interior-access cut-set as part of a face reservation **before the first
frontage parcels are admitted on any of that face's boundary roads**. The plan
should contain only validated 9 m connected local-road corridors and the
canonical remaining parcel layout; all corridor and parcel capacity is reserved
together, and the roads still use the ordinary visible construction and
completion lifecycle. Face eligibility should be limited to geometries that
actually contain useful unserved interior area and have a collision-free
connected cut-set; ineligible or too-shallow faces retain current frontage
behavior. Preserve the existing 82 × 50 and 82 × 82 recipes rather than
replacing their validated layouts.

This is a proposal, not a proven density improvement. In particular, detecting
a face only when its closing edge is about to open may be too late: earlier
boundary roads may already have created parcel reservations. The implementation
must either discover and reserve the intended face before its first boundary
frontage admission, or reject that face for this path without deleting or
moving existing parcels. Validate on the three mask-source neighborhoods and
matched fresh seeds: show reservations precede their boundary frontage,
accepted corridors are connected and clear, shallow faces stay unchanged,
all projects remain visible, save/restore is deterministic, and road/building/
parcel/project/history caps remain valid. Compare interior frontage, occupied
use capacity, projected kit coverage, road length, and construction history;
do not treat added roads, lots, or buildings alone as success.

## Limitations

The road-face masks use 2 m sampling and omit terrain/protected geometry. The
point-to-centerline distances say nothing by themselves about parcel access,
legal buildability, or net developable land. `neighborhood_road_ids` is a
membership tag, not an independent topology proof. Mature-snapshot statistics
cannot reconstruct the exact state when each frontage was admitted. Finally,
reserving a cut-set before frontage is a geometry-grounded hypothesis; it has
not been implemented or shown to increase occupied or rendered density.
