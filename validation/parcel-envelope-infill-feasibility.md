# Existing parcel envelope infill feasibility

**Spatial selection caveat (2026-10-10):** the 15 parcels and their rendered kits are measured from the stated tick-242000 checkpoint. The large-face outline used to select representative blockers, however, comes from an older tick-242839 raster. Its quoted area/bounds are historical context, not a measured current-world free-space component. The per-parcel no-fit result remains limited to those sampled parcels; it does not rule out access through the larger spaces between parcels.

## Finding

The measured parcels leave visible ground around their rendered building kits, but the samples do not support subdividing those leftover patches into independent lots. A conservative raster search found no axis-aligned 8 m × 12 m rectangle inside any of the 15 blocker parcels after buffering the rendered kit by 2.2 m. That search did not yet check frontage or a door-to-curb route, so a passing rectangle would only have been a necessary geometry condition; none passed even that screen.

This is a legal-envelope issue as well as a visual one. Current development admission reserves the whole parent parcel and rejects new parcels that overlap its stored/rendered envelope with the required gap. The renderer's uncovered grass is not currently a separate parcel or buildable site. Replatting a parent would require changing that parcel reservation while preserving the building's identity and its existing frontage/access contract.

## Evidence

The analysis uses the public-WASM checkpoint `validation/early-reservation-wasm-87231-save.json` (seed 87231, tick 242,000; SHA-256 `f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e`) and its matching authoritative snapshot. It measures the production `buildingKit` meshes from `web/src/render/kit.ts`, projects triangles above 1 m onto XZ, and rasterizes them at 0.25 m. The script and machine-readable results are `scripts/diagnose-parcel-envelope-infill.mjs` and `validation/parcel-envelope-infill-feasibility.json`.

| Parcel | Building | Reserved envelope | Kit-covered area inside envelope | Uncovered raster area | Access |
|---|---|---:|---:|---:|---|
| 68 | 2-floor shop | 14 × 12 m (168 m²) | 128.06 m² | 39.94 m² | Open connected local road 66; access node 3 |
| 707 | 3-floor apartment | 16 × 16 m (256 m²) | 222.38 m² | 33.63 m² | Open connected local road 706; access node 585 |
| 2608 | 7-floor office | 16 × 16 m (256 m²) | 191.75 m² | 64.25 m² | Open connected local road 2607; access node 2595 |
| 2614 | 2-floor home | 16 × 16 m (256 m²) | 194.13 m² | 61.88 m² | Open connected local road 2612; access node 2596 |
| 2627 | 2-floor shop | 11.44 × 15.90 m (181.90 m²) | 136.38 m² | 45.52 m² | Open connected local road 2599; access node 2595 |

Parcel 68 is centered at `(-215.41, 28.95)`; its existing shop sits on connected road 66. Parcel 707 is centered at `(-285.92, 160.84)`; its occupied apartment fronts road 706. In the first large east/north void, 13 representative blocking parcels have 40.96–64.25 m² of uncovered raster area each (mean 49.70 m²). Across all 15 sampled blockers, the mean is 47.98 m², and the conservative subdivision probe found zero 8 × 12 m placements in every parcel.

The first void itself is approximately `x=91..297, z=-351..-103`, with about 12,576 m² of rasterized unbuilt face area. The engine's current frontage scan identified 65 unique candidate centers in that face; all 65 failed the authoritative parcel-admission geometry check because of existing parcel envelopes. Examples include a proposed 16 × 16 m lot at `(252.86, -114.86)` on road 2607 blocked by parcel 2608, and a candidate at `(260.86, -154.86)` on road 2729 blocked by parcel 2762. These candidates have connected street access, so their blocker is the reserved lot geometry rather than a lack of road connectivity. The remaining area between such lots is real open ground, but it is fragmented by existing envelopes and setbacks.

## Interpretation and limits

The kit measurement is a visual projection, not a legal footprint: it includes roof/eave and facade/site details above 1 m, uses a 0.25 m raster, and may count decorations that should not constrain future use. Conversely, the search checks only axis-aligned rectangles; it could miss a rotated residual shape. It also does not test frontage or walking access. The no-fit result therefore says that a simple retained-building subdivision into a conventional 8 × 12 m lot is not supported by these samples; it does not prove every smaller or irregular accessory use impossible.

There is not enough evidence to implement parcel subdivision as the next density mechanism. The visible remainders are smaller than a typical minimum lot after even a modest separation buffer, and subdividing the parent would conflict with the engine's whole-envelope parcel reservations. Any later retained-building replat feature should be a separate, bounded operation that creates an independently fronted/accessed lot only when the exact rotated footprint, legal separation, parcel boundary, and door-to-curb path all fit. It must preserve the existing building and frontage IDs, update the parent parcel into non-overlapping sub-parcels transactionally, and pass save/restore and replay validation. If no such plot exists, growth should use a planned interior street or explicit redevelopment rather than treating decorative ground around an occupied lot as available land.
