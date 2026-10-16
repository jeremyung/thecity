# Deep center-street candidate — not promoted

The 871-building visual review exposed large open block interiors. Source review
found that the planner only offers an approximately 82×82 m center-street block.
Its four rows have 11.4 m depth; its nominal area/new-road score is worse than the
ordinary 82×50 m block. A larger rectangle can support deeper lots without changing
street setbacks, parcel gaps, minimum depth, floor counts or capacity semantics.

## Trial

`candidate.patch` adds an 82×110 m option before the old square at both shared-edge
and new-perimeter sites. Both consume the existing 24-spine-candidate budget; the
budget is not doubled. Consequently, this changes which spine variants fit in each scan.
It does not shorten the ordinary-block scan: `any_limit_reached()` only checks
the shared/full ordinary budgets. Candidate ranking and exact feasibility/admission checks remain unchanged.
The patch includes the changed selection expectation in one regression; it is
archived, **not applied** to production. Existing saved corner-based recipes do not
need a version change to represent the rectangle.

Pure layout checks cover cross-depths 82/100/110/114 m, rotation and winding, lot
containment, pairwise non-overlap, frontage mapping and canonical recipe validation.
All yield 20 lots of width 11.44 m; depths are 11.4/15.9/18.4/19.4 m. The exact
minimum cross-depth for four 8 m rows is 68.4 m. The 110 m recipe supplies
4,209.92 m² of nominal lots across 466 m of all-new road centerline, versus
1,818.96 m² across 264 m for the ordinary 82×50 layout. With a shared 82 m edge,
the denominators are 384 and 182 m. Geometry arithmetic alone does not establish
world feasibility or actual city benefit.

## Matched fresh-world WASM result

Both variants start fresh seeds at development rate 16 and run 121 batches of
2,000 ticks, ending at tick 242,000. Baseline is commit `b6c5979` (including narrow
midpoint infill), not the older 850-building save. CPU production-kit projection
uses the same bundle hash, 0.5 m raster and full 800×800 m domain for all four cases.

| Seed | Baseline buildings → candidate | Baseline projected m² → candidate | Change |
|---|---:|---:|---:|
| 0 | 868 → 866 | 122,139.25 → 125,139.50 | +2.46% |
| 87231 | 904 → 850 | 129,157.50 → 124,799.00 | −3.37% |

Candidate saves restored at tick 232,000 match uninterrupted saves every 2,000
ticks through 242,000. Complete hashes and coverage reports are adjacent. Raw
saves/snapshots are retained locally under `.tools/deep-spine-review/` (ignored).
No matched screenshots or browser performance measurements were taken.

The candidate full native run had 368 passes, 3 failures and 45 ignored tests.
Failures were `coordinated_block_lots_wait_for_perimeter_and_preserve_access_on_restore`,
`exhausted_infill_quota_does_not_block_road_growth_or_skip_available_lots`, and
`neighborhood_plan_reserves_connected_corridors_and_replays_pending_work`.
Some assertions assume ordinary-block selection, but those failures were not
all adjudicated or weakened: the mixed growth result already fails to establish
a robust improvement. Production source and browser WASM were restored.

## Decision and next work

Retain the pure rectangular-layout tests and normal-tick replay coverage for both
82 and 110 m plans. Do not promote the wider-first search. Larger blocks improve
local area efficiency yet can displace useful expansion elsewhere; a local score
is insufficient. Before another ranking change, measure which ordinary projects
are displaced and when the larger reservation prevents development. Avoid tuning
only to the favorable seed or claiming that access alone increases city coverage.

Restored production verification: full native release suite passes 371 tests,
0 failures, 45 ignored. The browser WASM rebuild succeeds. These results include
the retained geometric regressions and 82/110 m normal-tick replay tests; they
do not validate the rejected candidate as a production policy.

## Spatial follow-up

`layout-87231.svg` compares nominal building envelopes from the two saved worlds
on the same 800 m domain. It is a map, not a rendered screenshot. Reproduce with:

```
python3 scripts/audit/compare-growth-layouts.py --baseline .tools/deep-spine-review/baseline-87231-snapshot.json --candidate .tools/deep-spine-review/candidate-87231-snapshot.json --output validation/deep-spine-review/layout-87231.svg
```

Seed87231 loses 9,722 m² across 31 fixed 100 m cells and gains 5,363.5 m² across
33 cells. The x=−400 column accounts for −3,256.75 m²; the z=−400 row for
−3,075 m² (these overlap, so must not be added). The candidate has 80 buildings
with the distinctive 11.44×18.40 m recipe envelope. Their cells include both
gains and losses. Ordinary road length drops from 13,885.159 to 12,930.743 m.
Candidate extant-building completions end on day7; baseline has 57 completions
on days8–10. Completion day is a per-building field, not a causal plan trace.

These data show earlier saturation and altered spatial development. They do
**not** identify a specific rejected ordinary project or prove reservation
displacement. Stable IDs cannot match causal identity after divergent growth.
The initial hypothesis needs a trace at planning/admission boundaries, including
winning layout, rejected alternatives and reservation geometry, before being
used to justify a production policy change.
