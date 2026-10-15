# City-wide street hierarchy diagnosis — 2026-10-13

> Historical baseline, superseded for current lane classification. The current
> worktree already makes Compact subdivisions after depth zero into lanes.
> The 3% lane-length figures below precede that change and must not justify
> implementing it again. See `district-street-hierarchy.md` for the reassessment.

The latest user goal makes city-wide urban fabric the primary workstream. This
report replaces a camera-only explanation with a measured generator hypothesis.

`audit-city-corridor-hierarchy.mjs` examines all open roads/buildings in ordinary
seeds87231/0/5 at tick10000 and retained mature checkpoints. It imports production
road-width and base building-envelope helpers. Results: `city-corridor-hierarchy.json`.

| Mature seed | Arterial chord length | Local chord length | Lane chord length | Lane share of ground-street length | Expressway chord length |
| --- | ---: | ---: | ---: | ---: | ---: |
| 87231 | 3,676m | 17,908m | 643m | 2.85% | 1,571m |
| 0 | 3,680m | 17,012m | 649m | 3.00% | 1,570m |
| 5 | 3,673m | 18,357m | 845m | 3.65% | 1,569m |

Production carriageway widths are16m arterial/bridge,6.5m local,4m lane,20m
expressway. Arterials also carry2.5m sidewalks each side. These are current code
values, not inferred reference-photo dimensions. Width×chord sums in the JSON are
not union area or projected screen area; stacked decks must not be double-counted
as consumed ground. All expressway chords and79–86% of arterial chord length lie
within10degrees of world axes. This supports regularity but is not a complete
curvature metric; final expressway curves can differ from endpoint chords.

## Source mechanism and next bounded candidate

`masterplan.rs::split_streets` creates lanes only for a probabilistic subset of
cross-long-axis cuts in the Strips district; ordinary Compact subdivision cuts
always become Local. Other lane sources include short access stubs. The generator
also creates two near-domain-spanning elevated routes above an arterial row and
column, at separate levels. The combination explains why broad repeating corridors
can dominate even with densely realized parcels.

Next candidate to evaluate: district-aware secondary-lane classification for
compact neighborhood subdivisions, keeping primary collectors, station approaches
and required vehicle routes explicit. Check bus/vehicle eligibility, access widths,
lot setbacks, geometry and construction semantics before implementation. Do not
change only rendered widths or randomly downgrade roads. Compare generated plan
and ordinary realized city before/after across the same three seeds, retaining
connectivity, legal parcels and current-save replay. This addresses street
hierarchy; it does not by itself resolve elevated-route composition or river/rail
straightness. Keep those gaps visible and do not declare urban fabric complete.

## Building-gap diagnostic limits

Median unused base-body depth for homes is2.16–2.30m, reflecting the existing
84%depth rule. This is total front+rear allowance, not measured front setback.
Base widths usually reserve0.9m for low-rise homes and1.3m for larger families;
subsequent family-specific geometry changes these envelopes. Negative base-helper
allowances on narrow lots are not a new final-mesh finding; low-rise/massing
helpers must be applied before claiming overhangs. Do not revive another isolated
building-fitting loop from this diagnostic. Actual frontage continuity, final
building gaps and projected road/ground coverage remain unmeasured here.

No production change is made by this audit. The next experiment must visibly
improve ordinary neighborhoods, not merely produce a higher lane count. Reject
inaccessible or implausible routing, collisions, and improvements confined to a
favorable crop. Browser evidence, both styles, daily lighting, motion, performance
and user artistic acceptance remain required.
