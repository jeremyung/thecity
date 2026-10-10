# Interior access-first scheduling experiment

This experiment asked whether the existing bounded interior-access planner should run before local-chord and ordinary frontier planning. The normal growth scheduler still gives usable existing parcels priority; the experiment only changed `plan_road` ordering.

To protect the city's first bridgehead, the candidate used the existing bridgehead eligibility calculation: when an open connected bridge has an active opposite-bank catchment and no ordinary service at that endpoint, `plan_road` first tries the bank-entry frontier. Once the bank has ordinary service, an interior-access plan can compete normally. This was required by the bridge regression: without the exception, a candidate branch split an east-side street while the west bank had no approach road.

I compared matched native release runs from the same source revision except for this scheduling decision. Both used seed `87231` and `0`, development rate `16`, exactly `242000` ticks, and `2000`-tick steps. The candidate scan stayed capped at 48 proposals per call. It wrote no fixtures and was not built to WASM.

| Seed | Scheduler | Buildings / parcels | Connected open roads | Road length | Nominal parcel envelope | Parcel-area × floors proxy | Access projects in history | Simulation time |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 87231 | Existing fallback | 753 / 753 | 192 | 13,227.1 m | 146,483.7 m² | 442,163.6 m² | 0 | 12.17 s |
| 87231 | Access-first candidate | 793 / 793 | 204 | 13,894.7 m | 153,411.7 m² | 456,982.8 m² | 2 | 17.12 s |
| 0 | Existing fallback | 845 / 845 | 214 | 14,543.1 m | 161,415.1 m² | 469,599.2 m² | 1 | 8.24 s |
| 0 | Access-first candidate | 789 / 789 | 208 | 14,100.4 m | 152,736.6 m² | 462,695.8 m² | 2 | 10.28 s |

The candidate adds 40 buildings and 6,928 m² of nominal parcel envelope for seed `87231`, while seed `0` loses 56 buildings and 8,679 m². It also takes longer in both runs. The access-first ordering therefore has seed-sensitive outcomes and higher observed runtime in these two samples; they do not establish a net density improvement. These metrics correspond exactly to the candidate diff below: access was tried first only when no eligible bridgehead bootstrap was present, and the old no-frontier fallback call was removed. A later correction restored that fallback for scans where no frontier road is admitted, so a blocked bridgehead cannot suppress a viable access scan; that correction was not part of the measured candidate. The final shared source restores the old scheduling order and fallback. The next policy should compare access opportunities with district and frontier need instead of giving all access proposals unconditional priority.

“Nominal parcel envelope” sums saved parcel width × depth; it is not net land coverage or rendered footprint. “Parcel-area × floors proxy” multiplies that nominal area by each building's saved floor count and is not gross floor area. These figures are useful for matched relative comparison only.

## Candidate patch

The experiment moved one bounded access evaluation ahead of chords and frontier growth when no bridgehead bootstrap was pending, then removed its former fallback call:

```diff
@@ plan_road, after bridgehead eligibility is computed
+        if bridgehead_headings.is_empty() && self.try_plan_interior_access() {
+            return;
+        }
@@ plan_road, when no frontier road was selected
         let Some((start, target_id, x, z)) = selected else {
             self.state.road_origin_cursor = next_cursor;
-            self.try_plan_interior_access();
             return;
         };
```

The candidate passed the full native release suite at that point (107 passed, 3 ignored). Focused tests covered access admission, frontier fallback after a rejected scan, and bridgehead bootstrap. After restoring the old scheduler, a stale-quota test was updated to account for the corrected 12,000-tick horizon: when its only original site is a reserved offline tower lot, the scheduler first adds connected ordinary frontage and develops it on the next evaluation. The restored source passes the full native release suite: 106 passed, 0 failed, and 3 ignored. No public WASM rebuild was run.
