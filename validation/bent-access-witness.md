# Bounded bent-access search — 2026-10-11

No candidate in the tested two-segment route family can serve two valid lots
on this exhausted face under the existing clearance rules. This is a negative
offline result, not a production change or a proof that the face is inaccessible.

## Controlled comparisons

All three treatments restore the identical tick-272000 save from
[the rejection audit](exhausted-face-rejections.md), using its recorded SHA-256.
Each tests all 2,440 available descriptors with 12 possible bends: 10/20/30 m
along the original ray, offset perpendicularly by −20/−10/+10/+20 m. The host
attachment and original endpoint remain fixed. The absolute search cap is
2,500 descriptors × 12 bends = 30,000 routes. Each leg is at most 60 m, the first
at least 10 m, the final at least 24 m, and the combined length at most 120 m.

| Lot template | Center offset from road | Routes checked | Witnesses |
| --- | ---: | ---: | ---: |
| 12 m depth, target width 14 m | 15 m | 29,280 | 0 |
| Same dimensions, closer frontage | 11.25 m | 29,280 | 0 |
| 8 m depth, target width 10 m | 9.5 m | 29,280 | 0 |

Actual widths come from the existing frontage subdivision, with minimum width
8 m and 2.2 m gaps. Every lot must retain the existing 5 m road clearance and
2 m parcel separation. Smaller lot dimensions do not reduce collision clearance.

All three runs have the same first-failure route classification:

| First failing gate | Routes |
| --- | ---: |
| Diagnostic leg lengths | 10,832 |
| Target face containment | 3,070 |
| First-leg road clearance | 1,885 |
| Final-leg road clearance | 3,577 |
| Parcel/reservation clearance | 9,904 |
| Split or new-node clearance | 8 |
| Fewer than two viable final-leg lots | 4 |

These are ordered checks, not independent counts of all obstacles affecting a
route. Duplicate entrances and overlapping paths are possible. JSON evidence:
[standard](bent-access-witness.json),
[closer frontage](bent-access-compact-frontage.json), and
[small lots](bent-access-small-lots.json).

## Review correction

A provisional small-lot positive was invalid. Its bend was tested using the
1 m split-point rule, even though the bend creates a new node. Root review changed
that check to the existing 18 m new-endpoint clearance and reran every treatment.
The positive disappears. The provisional diagram was removed, and a dedicated
regression now rejects an otherwise clear route whose bend is 10 m from an
existing node. Do not cite the earlier positive as evidence for implementation.
The reports retain bounded node-rejection samples for inspection.

## Scope and controls

This test-only code checks both centerlines inside the traced face, including
concave-boundary crossing; full lot edges/corners must also remain in the face.
It checks parcel/reserved-lot envelopes, exact protected rectangle intersection,
river clearance including the 4.5 m road allowance, world margins, existing and
pending streets, reserved access and neighborhood corridors, host split checks,
and new bend/end nodes. Only the first leg is exempt from its attachment host.
The intended shared bend is allowed; collinear reversal is rejected. Every lot
uses `ViabilityQuery` against each new leg plus the captured world and is checked
against other selected lots.

This duplicates some branch checks in test-only code and is deliberately not an
admission API. Some checks are more conservative than live straight planning
(e.g. exact protected-rectangle intersection and checking all captured roads).
It does not test live demand/capacity admission, turning radii, ranking,
construction, traveler routing, building-kit fit, or save migration. Lots are
sampled only on the final leg; it does not enumerate arbitrary lot layouts,
all bend locations, additional legs or changed destinations. Such limitations
must accompany any interpretation of the negative result.

Six synthetic tests cover a complete positive route/lot case, blocked second
leg, second-leg host contact, collinear reversal, concave escape, rotated-street
lot clearance and the corrected 18 m bend-node rule. Root reran those tests and
all three fixture treatments. The module is excluded from production WASM by
`#[cfg(test)]`. This is not a browser-performance measurement.

## Reproduce

Regenerate the input save as described in the rejection audit, then:

```sh
node scripts/test-engine.mjs --release access_bend_diagnostic
DENSITY_AUDIT_SAVE=/tmp/thecity-fragmented-final.json \
BEND_AUDIT_REPORT=../validation/bent-access-witness.json \
node scripts/test-engine.mjs --release diagnose_two_segment_bend_witness -- --ignored
DENSITY_AUDIT_SAVE=/tmp/thecity-fragmented-final.json BEND_LOT_OFFSET_M=11.25 \
BEND_AUDIT_REPORT=../validation/bent-access-compact-frontage.json \
node scripts/test-engine.mjs --release diagnose_two_segment_bend_witness -- --ignored
DENSITY_AUDIT_SAVE=/tmp/thecity-fragmented-final.json BEND_LOT_PROFILE=small \
BEND_AUDIT_REPORT=../validation/bent-access-small-lots.json \
node scripts/test-engine.mjs --release diagnose_two_segment_bend_witness -- --ignored
```

## Consequence

Do not promote this bend grid, shrink production lots, or weaken node clearance
based on these results. Another route search should choose destinations together
with viable lot layouts rather than retaining failed straight-ray endpoints.
It needs a valid geometric witness before introducing a multi-segment save and
construction lifecycle. Upstream neighborhood generation and explicit mature
redevelopment remain separate paths; earlier failed scheduling/ranking experiments
are recorded in early-reservation-gap-diagnosis.md. Artistic density acceptance
is still unmet and cannot be inferred from planner tests.
