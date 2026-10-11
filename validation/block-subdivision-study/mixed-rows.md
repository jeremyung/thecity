# Mixed-row follow-up — 2026-10-11

The test-only subdivision keeps one canonical frontage row at the 11 m target
and substitutes a 14 or 18 m target on the opposite row. Narrow-side positions,
perimeter roads, inset and gaps remain unchanged. Same-frontage inputs exactly
reproduce the original subdivision. Invalid or unfillable rows reject the whole
layout; the 64-lot cap remains. No runtime policy or save format changes.

| Seed | Targets, two sides | Feasible lots | Nominal area |
| --- | --- | ---: | ---: |
| 0 | 11 / 11 baseline | 350 | 63,840.70 m² |
| 0 | 11 / 14 | 316 | 65,680.64 m² |
| 0 | 11 / 18 | 279 | 66,613.44 m² |
| 87231 | 11 / 11 baseline | 327 | 59,479.99 m² |
| 87231 | 11 / 14 | 294 | 60,959.64 m² |
| 87231 | 11 / 18 | 259 | 62,003.21 m² |

Both mixed alternatives gain nominal area in all 73 observed blocks. The
11/14 option gains 2.5–2.9% area and loses roughly 10% of lots; 11/18 gains
4.2–4.3% and loses roughly 20%. This halves the lot-count penalty of uniform
widening, but does not eliminate it. It preserves geometric frontage detail on
one side; it does **not** assign that side to shops/houses or prove visual variety.

Root verified every observation's identifying/context fields and the previous
three uniform variants remain exactly equal to the archived report. The
mixed-only artifact references that baseline hash and includes the full new
report hash. Reproduce with the same `BLOCK_SUBDIVISION_REPORT` command in the
parent README; the output now includes all five variants.

Four new mixed-layout tests plus the existing block/spine tests pass (14 total).
Root added exact production collision-predicate checks at 2 m clearance, rotated
and reversed winding cases, and rejection when one row cannot fit. The ignored
two-seed geometry diagnostic passes. No frontend or production WASM changes;
full engine/browser suites were not rerun for this test-only study.

**Not promoted.** A fixed wide side is not yet district-aware urban design.
Building capacity still ignores footprint area, so these counts cannot establish
more occupied floor area or activity. Before an end-to-end growth treatment,
define distinct residential/job capacity units, footprint-to-usable-floor-area
assumptions, and renewal/save compatibility; then test mixed programs with
unchanged resource caps and inspect actual rendered streets. The PLATEAU
reference-calibration requirement remains open. Do not silently invent measured
Tokyo occupancy densities or retroactively multiply saved capacities.
