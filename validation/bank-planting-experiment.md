# Low bank planting experiment — rejected 2026-10-11

## Deficiency

Locked concept 04 uses layered bank vegetation to connect water, paths and
buildings. Current mature views have bare strips with repeated stones, and most
existing full-size riverside trees are removed by road/building clearance.
Source inspection confirms existing lowrise/midrise massing families; adding
another generic family without checking current rendering would repeat work.
This experiment targeted the visibly bare bank strip instead.

## Two bounded trials

A Luna-authored deterministic candidate helper placed small irregular clumps on
both banks, within z[-390,390], x offset halfWidth+3.2..3.7 m, at most320
instances. Root changed intra-clump spacing to1.1..2m with18..32m breaks and
integrated existing foliage/trunk instance batches. Existing structural
clearance suppressed candidates at crossings/buildings, including expressways.
No textures, simulation changes or new mesh assets were added.

1. Existing tree geometry scaled uniformly by s/1.8, s=.4...6, lowered .45m.
2. Same height with double horizontal scale for broader low canopies.

Primary added a regression checking the actual near/far canopy vertex radius
under any Y rotation remains inside the promenade railing at halfWidth+5.4m.
Eight focused planting/foliage/material tests passed on variant1; four planting
tests passed on variant2. Production builds passed with the existing bundle
warning. These are historical trial results; the rejected helper/tests are removed.

## Browser review

Seed87231, tick160000,1839 buildings, normal UI import of
`.tools/fidelity-review/fair-routes-87231.json`; clear weather, paused,
50mm,DOFoff,Miniature,WebGPU/high detail,1099×1071,Hold waterfront view.
Both geometry queues reached zero through normal scheduling.

- [Variant1 dusk](screenshots/bank-planting/miniature-dusk.jpg)
- [Variant1 noon](screenshots/bank-planting/miniature-noon.jpg)
- [Variant2 noon](screenshots/bank-planting/broad-miniature-noon.jpg)
- [Variant1 queue diagnostics](screenshots/bank-planting/diagnostics.jpg)
- [Pre-experiment dusk](screenshots/water-normals/irregular-miniature-dusk.jpg)

Neither variant makes a meaningful planted edge at this observation scale.
They read mostly as tiny stems. A source-side placement check found229
candidates and no building-clearance exclusions; an approximate road-width
check retained218. This narrows the problem away from wholesale building
suppression but is not an exact renderer visibility or GPU measurement.

## Decision

Removed both variants and their helper/tests; restored only this experiment's
city.ts edits, preserving prior rendering work. Do not enlarge or scatter more
scaled tree cards blindly. Reassess the foliage asset: evaluate a dedicated low
shrub with a readable leaf mass and distance-stable silhouette in a small browser
specimen before placing it city-wide. Alpha-mask minification is a candidate
explanation for the sparse appearance, not yet verified. Preserve promenade and
crossing clearance, bounded instancing and shared-style geometry. Full concept
fidelity remains open, including architecture, planted composition and lighting.
