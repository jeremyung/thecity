# Destination-first access diagnostic — 2026-10-11

The tested face has no two-lot layout in this bounded search family, even before
attempting a connecting street. This changes the next investigation: more bends
alone cannot rescue these particular layouts. It does not prove the whole face
is undevelopable or justify reducing clearance rules.

## Method and result

Restore the identical tick-272000 fixture from the earlier rejection audit:
SHA-256 `5207f37c688888d9e6d49e293ba07a626e9bc5faba72019b86ab21ea8e77e9e3`.
The test asserts that the engine save remains unchanged. Trace face
`[2656,2657,2772,2773,3131,3010,3011,2895]`, then enumerate a 5 m lattice,
eight lane directions, and 28/40 m lane lengths. Unlike the previous bend test,
lane endpoints do not come from failed straight-road proposals.

Each lane must fit inside the face and clear existing roads, parcels,
reservations, water, protected land, and world bounds. Existing frontage
subdivision generates nominal 12 m-deep, target 14 m-wide lots at an 11.25 m
center offset. Actual widths vary with subdivision. Full rectangles must fit
the face and pass `ViabilityQuery`, including 5 m road and 2 m parcel clearance.
Only lanes with at least two viable lots proceed to host connection checks.

| Measurement | Result |
| --- | ---: |
| Grid positions examined / inside face | 891 / 561 |
| Lane layouts checked | 8,976 |
| Outside face | 3,744 |
| Existing/reserved road conflict | 2,627 |
| Parcel/reservation conflict | 2,531 |
| Clear lanes, zero viable lots | 34 |
| Clear lanes, one viable lot | 40 |
| Clear lanes, at least two viable lots | 0 |
| Host connections attempted | 0 |
| Viability-query work | 219,806 |

The 48 deduplicated host split positions therefore remain unused. Grid, layout,
connection, and query limits were not reached. Counts describe layouts, not
distinct land parcels; many overlap. Evidence: [machine report](destination-first-access.json).

## Validation and limits

Root reran the diagnostic independently; its report exactly matched the agent
run. Nine synthetic checks pass, including a viable destination with a legal
connection, occupied lots, concave-face escape, bend-node clearance and a
nonzero query budget that stops without overshooting. Two expensive diagnostics
are ignored in the default test suite. This module is excluded from production
WASM; no runtime behavior, construction, rendering or browser performance changed.

The query work cap is separate from bounded grid and route counts; it is not
a measurement of all CPU operations. Existing capture/trace ceilings still
apply. The search is limited to this face, seed, lattice, angles, lane lengths,
and lot subdivision. A coarser template can miss useful packing. In particular,
the result does not exclude a different subdivision, short single-lot access,
lots along both connecting legs, or coordinated redevelopment. None is approved
for production by this evidence.

Do not add more route variants for these rejected layouts. The next density
decision should compare usable lot packing and early block subdivision across
faces/seeds, while preserving occupied parcels and coherent infrastructure.
Any new admission policy still needs deterministic construction, save/resume,
matched city coverage and visual verification.

## Reproduce

Regenerate the fixture using the instructions in `exhausted-face-rejections.md`, then:

```sh
node scripts/test-engine.mjs --release access_bend_diagnostic
DENSITY_AUDIT_SAVE=/tmp/thecity-fragmented-final.json \
DESTINATION_AUDIT_REPORT=../validation/destination-first-access.json \
node scripts/test-engine.mjs --release diagnose_destination_first_access_witness -- --ignored
```
