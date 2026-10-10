# Road-opening access: rendered geometry coverage

The matched road-opening experiment was re-audited from its four saved snapshots using the current production `buildingKit`. The table reports the **union of projected kit triangles above 1 m**, sampled on a 0.5 m grid. It includes roof/eave and other authored geometry, removes overlap between buildings, and is not a legal building footprint, floor-area measure, or net developable-land ratio.

| Seed | Access at road opening | Buildings | Projected kit coverage | World-bound share | Paired change |
|---:|:---:|---:|---:|---:|---:|
| 0 | Off | 866 | 120,475.25 m² | 18.824% | — |
| 0 | On | 849 | 118,772.00 m² | 18.558% | −1,703.25 m² (−1.414%) |
| 87,231 | Off | 851 | 120,850.25 m² | 18.883% | — |
| 87,231 | On | 810 | 113,210.00 m² | 17.689% | −7,640.25 m² (−6.322%) |

For context, the separately reported nominal parcel-envelope unions are 157,364 / 155,474.75 m² for seed 0 (off/on) and 157,705.5 / 147,771 m² for seed 87,231. Those envelope proxies are not used as the rendered-coverage result above. Both measures decrease in the treatment runs; the actual projected-kit comparison therefore does not support a density improvement for this two-seed experiment.

All four reports identify the same current kit bundle SHA-256, `cbd0c6cf9c16749e5e2a14543e2701356ac8ee7b96dcde3ece4208196b4596eb`. Each report's `source.sha256` was checked against its referenced snapshot file and matches. The four detailed reports retain per-100 m block coverage and full run provenance:

- [Seed 0, access off](road-opening-access-seed-0-enabled-false-coverage.json)
- [Seed 0, access on](road-opening-access-seed-0-enabled-true-coverage.json)
- [Seed 87,231, access off](road-opening-access-seed-87231-enabled-false-coverage.json)
- [Seed 87,231, access on](road-opening-access-seed-87231-enabled-true-coverage.json)

This is a visual geometry comparison of the saved worlds, not a performance measurement or proof of general behavior. The experiment's treatment and simulation limitations remain documented in [road-opening-access-experiment.md](road-opening-access-experiment.md).
