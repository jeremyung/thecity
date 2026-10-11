# Independent fallback budget — no geometry benefit

Follow-up to the rejected shared-budget experiment, based on `d3822ad`.
The runtime hooks were again removed after the experiment. `experiment.patch`
is a reproducible test-only implementation, not an enabled engine policy.

The 42 m fallback now has at most eight attempts per neighborhood evaluation,
independent of the baseline shared/full/spine budgets. The anchor cursor,
baseline candidate order, stopping conditions, geometry checks and ranking are
unchanged. This adds bounded work; it does not claim that extra work is free or
browser-safe. Feasible candidates, transient best candidates, and admitted
winners have separate counters.

| Seed | Attempts | Feasible / transient best / admitted | Max attempts per evaluation | Saved world compared with control |
| --- | ---: | --- | ---: | --- |
| 0 | 13,737 | 0 / 0 / 0 | 8 | Byte-identical |
| 87231 | 16,021 | 0 / 0 / 0 | 8 | Byte-identical |

Both runs use development rate 16 and capture at tick 242,000. The paired
harness asserts exact control/treatment save equality when no alternative wins,
world validity, the eight-attempt maximum, and save/resume equivalence after
another 2,000 ticks. The geometry/manual-plan lifecycle test passes. Root reran
that test, reviewed the independent budget code, and compared every generated
save and snapshot with its control and the previous independently rerun control:
all bytes match. Root did not rerun the four long simulations again this pass.
Results, counters and hashes are in `results.json`.

Identical snapshots imply identical input to the existing kit-coverage audit:
127,026.50 m² for seed 0 and 119,950.75 m² for seed 87231. Those measurements
come from the preceding control audit; they were not remeasured as new geometry
or presented as a new visual review.

This isolates the earlier coverage regression to candidate-budget displacement.
It also shows that the sampled shallow fallback cannot help these trajectories:
none passed the full geometry predicate before ranking. It does not prove all
shallower block placements are impossible; only the first eight eligible
fallbacks per evaluation are tested. Do not tune the score to make infeasible
geometry win, or expand this sampling family without a concrete legal layout
that it currently misses. No production change is promoted.

## Reproduce

In a disposable checkout based on `d3822ad`, copy this directory in, then:

```sh
git apply validation/shallow-block-experiment/fair-budget/experiment.patch
node scripts/test-engine.mjs --release shallow_shared_edge_geometry_and_saved_lifecycle_support_42m_blocks
SHALLOW_BLOCK_EXPERIMENT_DIR=/tmp/thecity-fair-shallow \
node scripts/test-engine.mjs --release compare_shallow_shared_edge_fallback_at_mature_seeds -- --ignored
```

This patch replaces the earlier experiment; do not apply both patches together.
