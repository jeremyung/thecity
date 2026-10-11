# Synthetic / PLATEAU crop comparison — 2026-10-11

This diagnostic measures the unchanged production building kit from the mature
seed87231 snapshot at tick242000 (850 buildings). It does not promote new growth
rules. Reference: [PLATEAU crops](../plateau-pilot/README.md).

## Selection and measurements

All crops are250×250m. Station is centered on the actual station node(153,58).
Waterfront is centered(-112,42). Compact deliberately chooses the largest exact
nominal-envelope union among60 east-side50m-lattice crops, giving the generator
a favorable comparison. Exact bounds, source hash and selected IDs are in
`crops.json`. These are scale-matched, not matched land-use or survey samples.

| Crop | Synthetic intersecting buildings | Exact envelope union | Rendered kit union | Reference roof-edge union |
| --- | ---: | ---: | ---: | ---: |
| Station | 113 | 26.38% | 20.11% | 46.04% (140 buildings) |
| Compact | 130 | 37.75% | 28.87% | 48.47% (452 buildings) |
| Waterfront | 63 | 14.53% | 11.15% | 10.05% (25 buildings) |

Synthetic envelopes are oriented rectangles from saved width/depth. Rendered
coverage rasterizes real production kit triangles above1m at0.5m cell centers;
it includes eaves/props and is not exactly the PLATEAU roof-edge semantic.
All850 buildings pass through the kit even when their centers are outside the
crop, so crossing geometry is retained. Gross area includes every crop's water,
roads and protected land. Net developable coverage is not established.

The independent exact polygon envelope union differs from its raster estimate
by−54.63,+27.36,−5.35m² respectively (at most0.088 percentage points of crop area).
This is a sampling diagnostic, not an error bound for rendered roofs. It is much
smaller than the observed compact coverage gap; fine raster convergence remains
necessary for small improvements. No blanket precision claim is made.

[Station envelopes](station-envelopes.svg), [compact envelopes](compact-envelopes.svg),
and [waterfront envelopes](waterfront-envelopes.svg) show nominal lots, not painted
roof geometry. The compact diagram was rendered in Chromium and inspected: the
regular two-row groups leave broad gaps and lack the reference's narrow, varied
fabric. Unbuilt areas are unclassified, not inferred as parks or lawns.

## Implementation consequence

The compact synthetic envelope median is181.90m²; the source roof-edge median is
57.62m². These quantities are different, but the diagram/count/coverage together
show that widening a row is an insufficient main intervention. Rendered coverage
is about76% of nominal-envelope coverage, so filling existing envelopes still
cannot reach this particular reference crop. Avoid relaxing clearance based on
roof-edge gaps or scaling the entire city to this one sample.

Next prototype a compact-neighborhood block recipe with more small frontage
units, occasional larger sites, and a more continuous access network. Preserve
reachable entries, construction ordering and existing exact collision rules.
Compare new and baseline recipes through full multi-seed growth, activity/capacity,
resource counts and rendered camera views. Building capacity currently depends
on floors/use, so changing unit count also requires explicit capacity semantics;
PLATEAU does not supply occupancy or jobs. Keep station and waterfront distinct,
and retain finite caps. This evidence is not approval to ship new density targets.

## Reproduce / validation

```
.tools/plateau-python/bin/python scripts/select-synthetic-crops.py
node scripts/audit-rendered-coverage.mjs --snapshot validation/early-reservation-wasm-87231-snapshot.json --crop 28,-67,278,183 --output validation/synthetic-morphology/station-coverage.json
node scripts/audit-rendered-coverage.mjs --snapshot validation/early-reservation-wasm-87231-snapshot.json --crop 50,-350,300,-100 --output validation/synthetic-morphology/compact-coverage.json
node scripts/audit-rendered-coverage.mjs --snapshot validation/early-reservation-wasm-87231-snapshot.json --crop -237,-83,13,167 --output validation/synthetic-morphology/waterfront-coverage.json
npx vitest run tests/rendered-coverage.test.ts
```

Four focused tests pass, including crop-boundary clipping for both unions,
outside-center processing and out-of-world rejection. Root reran all three
production-kit measurements. No engine changes or browser performance claims.

## Follow-up land budget

`land-budget.json` partitions building envelopes, open-road5m centerline buffers
and their complement. Generate with `scripts/audit-synthetic-land-budget.py`.
The compact crop has25.65% road buffers and36.60% outside envelopes and buffers.
The complement is unclassified, not net developable land or an admission result.
See [compact prototype results](../block-subdivision-study/compact-rows.md) for
the smaller-lot tradeoff and next layout investigation.

The refined descriptive partition of the compact crop's22,877m² remainder is:
11,238m² within2m of existing building envelopes;6,982m² farther from buildings
but within12m of any open-road centerline;4,658m² beyond both distance bands.
These disjoint categories close to the exact gross crop area within1e-6m².
The2m halo uses Euclidean polygon distance, not the engine's SAT rectangle test;
12m is a diagnostic band, not an access certification. Much of the blank land
is narrow spacing or near-road land, so investigate existing-road frontage before
assuming a large inaccessible interior needs another road.

The actual frontage-generator audit led to a [production narrow midpoint
fallback](narrow-midpoint-infill.md), using the existing8m minimum and unchanged
clearances. Its normal WASM continuation constructs18new narrow buildings; visual
and browser timing review remain outstanding.
