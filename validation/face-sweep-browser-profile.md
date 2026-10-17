# Exhausted-face sweep browser continuation — 2026-10-10

Production WASM in a dedicated Chromium worker continued mature seed 87231 from
242000 to 272000. See [machine-readable evidence](face-sweep-browser-profile.json)
for the WASM hash, browser version, checkpoints, and construction observations.

- 30,000 individual simulation ticks completed; final save restores byte-identically.
- Buildings: 850 → 853. Roads: 208 → 210. Final parcels: 853.
- Sampled 11 distinct selected face identities; exhausted-face history reached
  10 identities. This demonstrates searches reaching other blocks,
  not that those blocks can all admit roads.
- Access branch 4003 was first observed under construction at tick 244500 and
  open/connected at 244700. At 244600 it had two planned lots and zero actual
  parcel frontages. The final report records its two actual frontage parcel IDs,
  4005 and 4006. Snapshots sample transitions every 20 ticks and saves every 200;
  these observations are not exact transition times or proof of every intervening
  construction invariant. Native tests separately cover the opening prerequisite.
- Per-step timing: p95 0.40 ms, p99 10.00 ms,
  max 27.80 ms. Total run 23.831 s
  includes snapshot/save inspection and yields. Timing excludes those probes,
  the application worker scheduler and all rendering. Timer resolution rounds
  many short calls to zero. This is not a frame-rate or maximum-city benchmark.

The sweep fixes repeated selection of an exhausted winning face, but this run's
three added buildings do not establish the requested dense Tokyo fabric.
Continued work must improve useful infill coverage and assess the rendered result.
No graph was truncated and the existing 512-node/edge capture cap was unchanged.

Reproduce with the development server running and rebuilt production WASM:

```sh
LIVE_TICK_COUNT=30000 LIVE_TICK_REPORT_PATH=validation/face-sweep-browser-profile.json LIVE_TICK_TRANSITION_SAMPLE_EVERY=20 LIVE_TICK_OVERALL_TIMEOUT_MS=180000 node scripts/benchmark/profile-live-ticks.mjs
```

Optionally set `LIVE_TICK_FINAL_SAVE_PATH` to export the final city; no final save
is committed by this run. The temporary local export was independently inspected
for the open road and both frontages. Full native release suite: 316 passed,
39 ignored. Production WASM build, web build and script syntax checks pass.
