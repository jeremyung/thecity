# Live planner worker verification — 2026-10-10

The primary independently ran `node scripts/profile-live-worker.mjs` against the actual Vite-served simulation module worker and production WASM in headless Chromium. The source fixture is seed 87231 at tick 242000. Exact browser version, WASM hash and samples are in [the report](live-worker-profile.json).

- At requested speed 8, the worker advanced 2,789 ticks in 20.002 seconds (about 6.97× effective speed). Buildings increased from 850 to 852.
- The unpaused mid-run save at tick 242701 contained an active live-planning checkpoint: 275,047 bytes within a 3,239,094-byte city save. Restoring it paused in epoch 3 and saving again produced identical bytes.
- The final paused save also restored byte-identically in epoch 2. No worker errors occurred; all received movement buffers were recycled.
- Twenty approximately once-per-second metric samples reported a maximum batch-mean tick cost of 2.975 ms and peak sampled WASM memory of 12,976,128 bytes. This does not measure individual-tick tail latency; metric sampling can miss capture and admission spikes.

The harness intentionally runs without a renderer. This is worker transport, runtime and checkpoint evidence, not visual acceptance, frame timing, a maximum-world benchmark, or proof that newly opened buildings came from the face planner. The mid-run save adds serialization work; it is outside the engine-step timing reported by worker metrics. Requested speed is subject to scheduler overhead and backpressure.

Native integration separately verifies the exact known face route: moving actors retain valid state through admission, ordinary simulation ticks open the connected branch, both promised frontages wait for opening, and repeated save/resume remains equivalent. A second test runs full simulation steps with geometry churn and canonical resume. The focused `access_live` suite passes all seven tests. The prior full native suite passed 307 tests before the additional churn test; the production WASM and web builds pass.

Remaining work: capture/admission worst-case tick timing, larger graphs than the current 512-node/edge capture cap, long-duration worker construction, and comparison of the rendered city with the approved art references. The provisional 4,096 work units per tick are not a measured universal performance guarantee.
