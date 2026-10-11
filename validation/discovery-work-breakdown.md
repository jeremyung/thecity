# Native and WASM discovery work accounting

Both runs restore `validation/early-reservation-wasm-87231-save.json` (SHA-256
`f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e`) and
produce the same 2,069 descriptor bit tuples. The native total is 6,232,851
work units; WASM reports 6,233,215, a difference of 364 (0.0058%).

The exact top-level stage split localizes all of the difference to
`DescriptorSearch`:

| Stage | Native | WASM | Difference |
| --- | ---: | ---: | ---: |
| CopyGraph | 207 | 207 | 0 |
| Trace | 144,009 | 144,009 | 0 |
| Selection | 3,079,689 | 3,079,689 | 0 |
| CopyRankInputs | 1,518 | 1,518 | 0 |
| Ranking | 51,853 | 51,853 | 0 |
| BuildDescriptors | 4,815 | 4,815 | 0 |
| Descriptors | 2,950,760 | 2,951,124 | +364 |

The native and WASM breakdowns are in
`access-discovery-native-work-breakdown.json` and
`access-discovery-wasm-work-breakdown.json`. Each diagnostic advances in
65,536-unit chunks and binary-searches the exact work unit where a
top-level stage changes. It intentionally does not attribute work to
`DescriptorSearch` subphases: those phases repeat, so end-state phase sampling
cannot yield sound per-phase totals.

This does not indicate live simulation timing divergence: `DiscoveryJob` is a
separate diagnostic preparation API and is not called by `Engine::step`. It
does mean total work to finish this diagnostic is platform-dependent. Same-
target quota and save/resume tests pass, and native/WASM final descriptor bits
match exactly. A checkpoint resumed on the opposite target has not been tested;
the 364-unit difference alone neither proves nor rules out cross-target resume
compatibility.
