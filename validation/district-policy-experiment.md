# District-policy matched-seed diagnostic

This is a read-only comparison of the four native experiment saves at tick 242,000, all run at development rate 16. The disabled and enabled runs use the same seed and implementation; they diverge after the first different road admission, so their final city differences are outcomes of a changed simulation path, not a controlled one-project A/B test. The policy remains an experiment and should not be promoted from these two seeds.

The native measurement checkpoints were recomputed into aligned 100 m cells (8 × 8 over the 800 × 800 m world). The analysis uses the saved open-building `OrientedRect` envelope and the same local-axis convention as the engine, clips each polygon against each cell, and allocates capacity by clipped envelope share. The independent projected-kit coverage files use a separate 0.5 m raster of production kit geometry above 1 m. Those kit figures are renderer-grounded but still cover the full world bounds, including water, protected land, and other undevelopable space.

## Paired outcomes

| Seed | Policy | Open buildings | Connected open roads | Ordinary road length | Nominal envelope sum | Floor-area proxy | Projected kit area |
|---|---:|---:|---:|---:|---:|---:|---:|
| 0 | off | 845 | 214 | 14,219.5 m | 161,415.1 m² (25.22% gross) | 469,599.3 m² | 104,770 m² (16.3703%) |
| 0 | on | 813 | 210 | 13,863.9 m | 158,176.7 m² (24.72% gross) | 472,544.5 m² | 102,596.5 m² (16.0307%) |
| 87,231 | off | 753 | 192 | 12,903.4 m | 146,483.5 m² (22.89% gross) | 442,163.8 m² | 95,143.75 m² (14.8662%) |
| 87,231 | on | 802 | 212 | 13,845.9 m | 155,390.6 m² (24.28% gross) | 454,468.4 m² | 101,238.25 m² (15.8185%) |

The nominal envelope and floor figures are sums over saved open buildings, not union footprints or measured net-land coverage. The renderer audit found the nominal envelope union at 25.28%→24.75% for seed 0 and 22.89%→24.26% for seed 87,231, while projected kit area moved in the same direction as the broad engine proxy: down 2,173.5 m² for seed 0 and up 6,094.5 m² for seed 87,231. The seed 0 decrease is real in the rendered geometry, although modest relative to the world area.

The test-only instrumented rerun logged 877 comparisons for seed 87,231 and 879 for seed 0. Successful admissions recorded 6 interior-access and 49 frontier wins for seed 87,231, and 3 interior-access and 49 frontier wins for seed 0. Thus access accounted for 9 of 107 recorded policy admissions (8.4%). The instrumented run took 11.85 s off / 13.72 s on for seed 87,231 and 8.30 s off / 11.09 s on for seed 0; these are single native-run observations, not a benchmark.

## District distribution and capacity

The table below gives the mean nominal envelope coverage in the aligned cells for each profile. Profile targets are Historic Core 32%, Station Core 40%, Station Fringe 35%, and Outer Neighborhood 28%. The experiment rewards only progress toward each minimum; the configured maximum is metadata, not a cap.

| Seed | Policy | Historic Core, 4 cells | Station Core, 5 cells | Station Fringe, 6 cells | Outer Neighborhood, 49 cells |
|---|---:|---:|---:|---:|---:|
| 0 | off | 11.42% | 28.91% | 29.93% | 25.40% |
| 0 | on | 11.06% | 29.57% | 29.95% | 24.69% |
| 87,231 | off | 8.87% | 29.26% | 27.88% | 22.77% |
| 87,231 | on | 16.67% | 25.77% | 29.77% | 24.08% |

At the measurement checkpoint, 43 of 64 cells were below their nominal-envelope minimum in every run: all four Historic Core cells, all five Station Core cells, five of six Station Fringe cells, and 29 of 49 Outer Neighborhood cells. In the separate projected-kit raster, all 64 cells were below the same profile percentages in all four saves. The 100 m cells do not say whether a deficit lies in a usable block interior, on a river edge, or on protected ground, so these counts are not a developable-land estimate.

Open housing and commerce capacity was fully used in these near-final snapshots. Seed 0 moved from 530 housing buildings with 8,039 capacity/occupancy and 315 commercial buildings with 4,030 active capacity, to 514 housing buildings with 7,608 capacity/occupancy and 299 commercial buildings with 4,141 active capacity. Seed 87,231 moved from 469 housing buildings with 7,102 capacity/occupancy and 284 commercial buildings with 4,033 active capacity, to 500 housing buildings with 7,232 capacity/occupancy and 302 commercial buildings with 4,174 active capacity. This weakens citywide vacancy as an explanation for the final outcome. It does not reconstruct every origin catchment at every earlier admission tick; those transient values were not saved per candidate.

## Candidate diagnostics

Test-only counters captured 32 bounded representative records and aggregate totals without serializing state or changing production decisions. The instrumented saves are byte-identical to the earlier matched experiment saves, so these diagnostics did not alter the simulation. Frontier scanning produced 52 proposals in each seed; access scanning produced 33 proposals for seed 87,231 and 26 for seed 0. These are proposal totals across scans, not scan counts.

For seed 87,231, frontier had 52 geometrically feasible proposals, 50 passed live source-vacancy checks, and those candidates represented 170 potential sites / 28,560 m² nominal envelopes. Access had 33 geometric proposals, 28 passed source checks, and represented 117 reserved sites / 18,480.4 m². For seed 0, frontier had 52 geometric proposals, 51 source-feasible, with 154 sites / 25,872 m²; access had 26 geometric and source-feasible proposals, with 97 sites / 15,357.6 m².

Only one seed-87,231 comparison and two seed-0 comparisons had both project types ranked. Access scored higher in all three and was admitted in all three; there were zero paired cases where frontier scored higher. Frontier and access scored candidates in the same comparison only 1 time for seed 87,231 and 2 times for seed 0. Most frontier admissions therefore occurred when no access geometry candidate existed in that comparison. Across all comparisons, no candidate geometry was available 822/877 times (93.7%) and 827/879 times (94.1%). Source-vacancy checks rejected only 2 frontier and 5 access proposals for seed 87,231, and 1 frontier / 0 access proposals for seed 0. There were no recorded forecast failures, score errors, or ranked-admission rejections.

This points to proposal opportunity, not the score comparison, as the immediate limitation in these runs: access seldom had a geometry candidate to compare, and it never lost a paired score comparison. Access nevertheless contributed 6 and 3 admissions, so its opportunities were not absent. These counters do not explain why most proposals had no geometry, and they do not measure unused courtyard area or the benefit of a different access topology. The next diagnostic should classify geometry rejection by cause and location before changing score weights. Forecast area is nominal envelope area and potential sites are not guaranteed occupancy.

## What the score suggests

At these snapshots, demand is 4.0 and `demand_fit` is clamped to 1.0. Every compared feasible project therefore receives the same 0.2 demand contribution before road-cost scaling. The score then adds each candidate's clipped envelope gain divided by that cell's target-minimum envelope area, and divides the project benefit by `1 + road_length / 60 m`. Since the demand term is common and a shorter access road receives a larger absolute share of that term after division, the project-wide demand contribution does not explain why access won only 8.4% of recorded admissions; it tends to help shorter proposals, all else equal.

A remaining calibration risk is asymmetric forecast certainty and volume. A frontier candidate can contain up to 50 compatible frontage-site rectangles; an interior-access proposal carries at most 16 selected lots. The frontier helper explicitly returns “potential lots,” not reservations or guaranteed occupied floor area, while access proposals store the exact selected lots that their plan reserves. Both paths currently forecast 50% of nominal envelope area. However, the instrumented comparison showed no frontier score victory when both categories were ranked, so forecast asymmetry is not supported as the reason for the low access admission count in this sample. It remains a risk to verify once more paired opportunities exist.

The next useful step is to count rejection causes in the bounded geometry enumerators, especially whether potential access sites are blocked by existing parcels, road corridors, protected terrain, or candidate bounds. If no geometry is available because the topology offers few legal access alignments, improving block-scale opportunity is more grounded than adjusting project weights. If many geometries are rejected only by a specific conservative test, audit that rule against its exact rendered envelope before changing it. The current 100 m grid cannot distinguish a vacant courtyard from an already-filled cell edge.

## Provenance and limits

The experiment harness ran `cargo test --release district_policy_experiment_uses_normal_step_for_seed_87231_and_seed_zero -- --ignored --nocapture`. It paired enabled and disabled runs at identical seeds, rates, and horizons, saved each state at tick 242,000, restored each save, and checked a further 2,000 ticks for byte-identical continuation. The full native suite before instrumentation passed 144 tests, with 0 failures and 4 ignored. After instrumentation, `cargo test --release district_policy` passed 7 tests, with 0 failures and 1 intentionally ignored; the matched experiment also passed. No WASM rebuild was performed for this diagnostic.

Source SHA-256 at capture:

| File | SHA-256 |
|---|---|
| `engine/src/lib.rs` | `e7d6fc49efbbfa6c6c58b3da99ac11ce8ee3d0cb1435f014b305ad01e92725aa` |
| `engine/src/district_policy.rs` | `33e3b31189a20ff79c42e4cde8319af061ec879172b09a170a5ffd5e612544ac` |
| `engine/src/project_scoring.rs` | `5662c80166898083a3432fb96a730602e3179373cefea70b8e51215c0d868820` |
| `engine/src/district_density.rs` | `3584f63c79ef4fa976eb1f4026471ecc6f7a5acacadf30390cdc7e5a35d63937` |

The four native saves and their SHA-256 values:

| Save | SHA-256 |
|---|---|
| `validation/district-policy-experiment-seed-0-enabled-false.json` | `e75dc8abd65f94cd0e4f472a4e275081607f54a590fa46ae240e4f40dafb9938` |
| `validation/district-policy-experiment-seed-0-enabled-true.json` | `bfe5947fc09ad89df59a39f621bdc4a043e1b1cf428292b33ce67d246e1499de` |
| `validation/district-policy-experiment-seed-87231-enabled-false.json` | `4d03ed81247fb835129bd5090c815937f3da14d9b29dfb3788221eb9c3233239` |
| `validation/district-policy-experiment-seed-87231-enabled-true.json` | `582a47e27278101737815998d0d6d4a5a7bb9e160b1d7f4542adf83bf282ba93` |

Projected-kit evidence is in `validation/district-policy-experiment-seed-{0,87231}-enabled-{false,true}-coverage.json`. Those reports were generated from the pre-frontage-edit renderer. The two seeds are too small a sample for a general growth claim, and same-seed runs cease to be paired after their first different admission. The score comparison is useful for finding calibration questions, not for establishing that the policy improves Tokyo-like density.
