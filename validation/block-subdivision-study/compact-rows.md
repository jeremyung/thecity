# Compact varied frontage prototype — 2026-10-11

Test-only `subdivide_compact_rectangular_block` starts with7m target frontage
and joins every sixth pair (row slots4/5,10/11,...) into one larger site. The
larger site's width includes the former internal gap; every external gap remains
2.2m. Depth, perimeter inset8m, orientation and outward frontage are retained.
This is a fixed prototype template, not a calibrated district distribution.

The same73 historical admitted block plans are evaluated through the ordinary
exact engine predicate. Existing parcels, road construction clearance, river,
protected land and other reservations remain; no occupied parcel is removed.

| Seed | Recipe | Accepted lots | Nominal area m² |
| --- | --- | ---: | ---: |
| 0 | Baseline11m | 350 | 63,840.70 |
| 0 | Uniform7m | 498 | 59,819.35 |
| 0 | Compact varied | 424 | 61,788.22 |
| 87231 | Baseline11m | 327 | 59,479.99 |
| 87231 | Uniform7m | 461 | 55,288.38 |
| 87231 | Compact varied | 394 | 57,322.22 |

The varied recipe admits21.14% /20.49% more lots, but loses3.22% /3.63% nominal
area. Uniform small lots lose6.30% /7.05% area. Smaller units add frontage rhythm,
but their extra required gaps consume area. **Do not promote this as a density
fix.** It remains useful as one element of a future compact district recipe,
subject to rendered small-building quality, capacity semantics and resource caps.
No production simulation, saves, WASM or renderer change is made.

## Independent review

Root verified all previous three uniform and two mixed variants, plus captured
plan context, match archived reports. The compact prototype's rotated/reversed
winding tests use the actual2m rectangle-overlap predicate, check inset containment,
known merges and finite64-lot bound. All16 block/spine tests pass; the long
73-plan diagnostic passes (32.59s test time). The agent's compact filter also
passes5 tests. The report asserts original source save equality at each observation.
This is not complete construction or multi-seed runtime visual acceptance.

Reproduce:

```
BLOCK_SUBDIVISION_REPORT=/tmp/compact-results.json node scripts/test-engine.mjs --release compare_live_block_subdivision_geometry -- --ignored
node scripts/test-engine.mjs --release blocks::
```

## Next structural question

[Matched crop evidence](../synthetic-morphology/README.md) already showed that
filling current envelopes cannot reach compact-reference coverage. Additional
[land-budget accounting](../synthetic-morphology/land-budget.json) finds the
strongest synthetic inland crop contains37.75% building envelopes and25.65%
open-road5m centerline buffers, leaving36.60% outside both. This remainder is
**not proven buildable**: pending/reserved land, parcel gaps, unmodeled access
and other restrictions are not yet partitioned. Road buffers include rail and
approximate round caps; no clearances should be reduced based on this scalar.
Next locate connected, usable portions of that remainder and compare street/block
layouts that avoid stranding land, while retaining smaller frontage rhythm and
actual engine admission rules. Capacity cannot continue to imply equal demand
for arbitrarily different floorplate areas when recipes are eventually promoted.
