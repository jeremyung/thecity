# Lot admission and visible grass gaps — 2026-10-13

The fixed seed0 interior comparison showed that narrower roads alone do not produce
convincing compact fabric. This audit traces missing ground coverage before changing
clearance or adding cosmetic fill.

The existing native plan export now includes inset blocks, admitted lot-ground
polygons, and test-only rejection witnesses. Witnesses are excluded from production
WASM/state. Instrumented exports match the earlier candidate's nodes, roads and
lots exactly for seeds87231/0/5. Eight master-plan tests pass. Source order, random
consumption and clearance decisions are unchanged.

Reproduce each input with `PLAN_SEED=SEED PLAN_JSON=ABSOLUTE_OUTPUT node scripts/test-engine.mjs masterplan::export -- --ignored`,
then run `node scripts/audit-lot-admission-gaps.mjs`. Output is
[lot-admission-gaps.json](lot-admission-gaps.json), including input hashes and sample polygons.

| Seed | Lots removed after adding roji | Their polygon area sum | Lots rejected for elevated clearance | Their polygon area sum | Unresolved frontage lots |
| --- | ---: | ---: | ---: | ---: | ---: |
| 87231 | 48 | 7,297 m² | 96 | 14,807 m² | 81 |
| 0 | 68 | 11,273 m² | 109 | 19,255 m² | 76 |
| 5 | 57 | 7,569 m² | 89 | 15,144 m² | 49 |

These are discarded whole partition polygons, **not reclaimable building area**.
Some of that space must remain occupied by roads or legally clear. Earlier partition
losses and rejected pieces with no usable rectangle are not fully traced here.
Unresolved frontage is a separate category that still needs investigation.

## Concrete visible witness

Pixel(365,240) in `screenshots/compact-lanes/seed0-interior-miniature-after.png`
is in the visible green gap. Using its recorded camera, its ground intersection
is(-306.6417,16.0602). This point lies inside an inset block and a lot-ground polygon
removed by the roji step, and outside every admitted lot-ground polygon. This
connects one photographed gap directly to generator deletion, rather than a missing
render batch or building-body setback. The screenshot was taken with all queues0.

A nearby sample at pixel(350,280) is inside a block but neither admitted nor included
in the traced rejections: earlier partition losses remain a possible cause there.
Other samples include admitted ground or points outside inset blocks. Rays use y=0;
they cannot attribute pixels where roofs/decks obscure the ground. Do not generalize
the one confirmed witness to every green patch.

## Next bounded correction

Reserve corridor space before parcel subdivision. Start with the roji witness:
partition usable land around its actual width and terminal clearance, with frontage
labels for the alley, instead of deleting intersected lots after partition. Apply
the same infrastructure-first principle to elevated corridors separately once the
bounded roji correction is evaluated. Keep all access, clearance, connected-growth
and replay requirements; do not relax collision thresholds to admit more buildings.

Retain the current lane candidate as unaccepted evidence, not a successful visual
milestone. Do not stack another width/density tweak onto it. A successful next
intervention must visibly use the leftover land coherently, preserve legitimate
open space and routes, and survive the existing fixed-view multi-seed review.
Neither this diagnosis nor passing plan tests establishes artistic acceptance.
