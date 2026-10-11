# Shallower shared-edge fallback — rejected

Tested against checkpoint `cfdbaeb` on 2026-10-11. This experiment is archived
as `experiment.patch`; its candidate hook is not enabled or retained in the
engine source. It does not establish improved density or artistic acceptance.

## Question

Does a 42 m-deep coordinated block recover space when the existing 50 m-deep
block cannot fit along an open 70–90 m street? The fallback keeps the current
8 m inset, 11 m target frontage and 2.2 m lot gap, all collision predicates,
ranking, demand, capacity, and construction rules. It uses one additional slot
from the existing shared-candidate budget and runs only when the 50 m candidate
is rejected. Production and control builds keep the experiment disabled.

The canonical 82×50 layout reserves 1,818.96 m² across ten lots; its 42 m-deep
alternative reserves 1,361.36 m² across ten shallower lots. This is a geometric
fit fallback, not an assertion that reducing block depth increases coverage.
Most non-parcel block area is street allowance and spacing, not automatically
developable vacant land. The prior spine/site-count experiments are separate.

## Matched result

Fresh seeds 0 and 87231, development rate 16, capture tick 242,000. Projected
coverage uses the actual current building kit above 1 m, unioned on the same
0.5 m grid. It is neither floor area nor a browser performance measurement.

| Seed | Open buildings, control → fallback | Projected kit area, m² | Change | Fallback attempts / winners |
| --- | --- | --- | --- | --- |
| 0 | 886 → 874 | 127,026.50 → 125,063.25 | −1.55% | 19,330 / 0 |
| 87231 | 812 → 789 | 119,950.75 → 116,284.75 | −3.06% | 22,689 / 0 |

No fallback won admission. Nevertheless the world trajectories changed because
extra attempts consumed the existing candidate budget, altering which ordinary
candidates were considered. Do not interpret this as a test where the two
worlds differ only by constructed shallow blocks. More candidates are not
automatically a better search when they displace existing candidates.

Both treatments had more occupied housing but less rendered coverage; commerce
changed in opposite directions. These mixed metrics do not meet the density
goal. Detailed measured counts and source hashes are in `results.json`.

The geometry test proves an obstructed 50 m candidate can have a feasible 42 m
alternative. It also exercises save/restore and completion of a **manually
installed** canonical 42 m plan through the existing road completion hooks.
It does not prove that the planner selected it or validate construction timing.
All four matched runs assert world validity, then compare uninterrupted and
restored saves after 2,000 further ticks. Root independently reran the test and
the paired experiment, comparing saved world bytes, and audited rendered
coverage. Elapsed native times are recorded only as provenance.

## Consequence and reproduction

Do not promote this policy or increase its weight. The counter does not record
how many fallback candidates were feasible before ranking, so zero winners is
not proof that all 42 m candidates were impossible. A future adaptive block
search must preserve fair access to baseline candidates and compare complete
prospective layouts across several seeds; it must not blindly add shapes to
the same crowded budget. No further morphology change is justified by these
results alone.

To reproduce in a disposable checkout based on `cfdbaeb`, first copy this
experiment directory into that checkout:

```sh
git apply validation/shallow-block-experiment/experiment.patch
node scripts/test-engine.mjs --release shallow_shared_edge_geometry_and_saved_lifecycle_support_42m_blocks
SHALLOW_BLOCK_EXPERIMENT_DIR=/tmp/thecity-shallow-block \
node scripts/test-engine.mjs --release compare_shallow_shared_edge_fallback_at_mature_seeds -- --ignored
node scripts/audit-rendered-coverage.mjs \
  --snapshot /tmp/thecity-shallow-block/seed-0-baseline-snapshot.json \
  --output /tmp/thecity-shallow-block/seed-0-baseline-coverage.json
```

Repeat the coverage command for each seed and treatment. The patch includes the
test harness; large generated city saves remain reproducible temporary artifacts.
