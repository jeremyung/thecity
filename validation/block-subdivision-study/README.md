# Block subdivision opportunity study — 2026-10-11

**Test-only counterfactual; no production policy promoted.** Based on `cd5a656`.
Two fresh worlds, seeds 0 and 87231, run at development rate 16 to tick 242,000.
Every 200 ticks the diagnostic samples a first-seen active coordinated,
non-spine block before its perimeter completes. The cap is 64 plans per seed;
38 and 35 plans were observed, with neither cap reached. This samples actual
admitted block footprints, not arbitrary rectangles or every rejected candidate.

For each block, a disposable restored copy clears only that block's current
neighborhood reservation. All existing parcels, buildings, roads and other
reservations remain. **No occupied parcel is removed.** All four planned
perimeter lines are added to the collision context as pending roads, including
not-yet-created segments, retaining the conservative 7 m construction clearance.
The ordinary exact oriented-parcel predicate checks each lot sequentially;
accepted footprints immediately block later lots. Street inset stays 8 m and
lot/back gap stays 2.2 m; only target frontage changes from 11 to 14 or 18 m.

| Seed | Target frontage | Feasible lots across sampled blocks | Nominal area |
| --- | ---: | ---: | ---: |
| 0 | 11 m baseline | 350 | 63,840.70 m² |
| 0 | 14 m | 284 | 67,343.80 m² |
| 0 | 18 m | 213 | 69,375.55 m² |
| 87231 | 11 m baseline | 327 | 59,479.99 m² |
| 87231 | 14 m | 264 | 62,334.36 m² |
| 87231 | 18 m | 197 | 64,316.55 m² |

Every sampled block gains nominal lot area with both alternatives. The 14 m
variant gains 5.49% / 4.80% area but loses 18.86% / 19.27% lots. The 18 m
variant gains 8.67% / 8.13% area but loses 39.14% / 39.76% lots. Fewer internal
gaps explain the geometric gain; this is not evidence of more occupied floor
area, richer streets, or an improved skyline. Summed opportunity area is not
final-world coverage and may include footprints at different historical times.

An implementation obstacle is explicit in the current building policy:
`choose_building_profile` assigns capacity from use and floor count (`home`: 6+2×floors;
apartment/office: 5×floors), without footprint area. Simply replacing small lots
with fewer wide lots therefore does not automatically increase housing/jobs and
could reduce activity and facade variety. There is no justification here for
promoting wider lots globally or relaxing collision clearances.

The next experiment should evaluate a **mixed subdivision** and its building
program jointly: retain small shop/house frontages, use larger floorplates where
district type and demand justify them, and compare actual construction,
occupied capacity, rendered coverage and street variety over matched seeds.
Capacity changes must be physically explained and bounded, with save/replay and
legacy capacity semantics addressed; multiplying capacity only to improve a
density statistic would not satisfy the intended world.

## Verification and reproduction

The focused synthetic test verifies a legal lot becomes rejected when its exact
footprint is inserted as an existing parcel, without changing the source save.
The long diagnostic asserts source save equality at every observation and uses
the real production clearance predicate. Both tests pass. Root repeated the
two-seed diagnostic and its full JSON report matches byte-for-byte. Production source
changes only register a `#[cfg(test)]` module; runtime growth is unchanged.

```sh
BLOCK_SUBDIVISION_REPORT=/tmp/subdivision.json node scripts/test-engine.mjs --release block_subdivision_diagnostic -- --include-ignored
```

The report records each block's corners, road identities, observation tick,
retained parcel/footprint counts, zero excluded parcels, and per-variant
generation/admission totals. Observation can miss a plan entirely born and
completed within 200 ticks. This is not a lifecycle, reservation replacement,
performance, browser, or visual-acceptance test. The alternate subdivisions are
never admitted into the evolving source world.
