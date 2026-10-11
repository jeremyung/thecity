# Individual WASM tick timings — 2026-10-10

Primary verification ran `node scripts/profile-live-ticks.mjs` against the production WASM in a dedicated headless Chromium module worker. It measures each `CityEngine.step(1)` call rather than the application worker's sampled batch means. The [JSON report](live-tick-profile.json) records browser version, WASM hash, fixture identity, 30 checkpoint observations and the slowest 20 ticks.

The mature seed-87231 fixture advanced from tick 242000 to 248000. Across 6,000 calls: p95 **0.5 ms**, p99 **10.3 ms**, maximum **25.2 ms**. The median rounded to zero at the browser timer's resolution; it does not mean those calls take no time. The initial calls are included, with no warmup exclusion. The run took 2.991 seconds including periodic yields and 671.5 ms of save/snapshot inspection outside the timed calls.

Both discovery and proposal-validation phases appeared in the samples. The largest sampled live job serialized to 888,622 bytes. Final save/restore was byte-identical. Buildings increased from 850 to 853 and roads from 208 to 210; these counts alone do not attribute growth to the face planner.

The slowest calls cluster at growth/scheduling cadences, but sampling checkpoint phases every 200 ticks cannot identify the responsible function. These measurements include all engine work inside each call. They exclude the application's worker scheduler, renderer, UI, message transfer and GPU workload. The fixture meets the proposed 5 ms p95 target on this host, while its tail is substantially slower. This is not maximum-world or device-wide performance acceptance.

## Scaling audit

The live adapter refuses graphs above 512 participating nodes or ordinary roads. Matching caps also exist in discovery, preflight, bridge detection, topology, face walk, selection and sampling. Raising only the adapter limit would fail downstream.

At 5,000 edges, all-pairs planarity alone examines 12,497,500 pairs; duplicate checks, endpoint lookup and face nesting add more work. The current 131,072 pair cap and 1,180,160 preflight-work cap are sized around 512 edges. Walk/output limits (1,024 half-edges, 512 faces) and selection limits must change together. Truncating the graph is not acceptable because it can change face boundaries.

Before expansion to the engine's 5,000-road limit, implement deterministic indexed endpoint/duplicate lookup and a resumable spatial broad phase with exact final geometry checks; align complete-graph and face-output bounds across stages. Also reconcile the 5,000 candidate-node cap with legal states containing more road endpoints, and the 4,000 projected-footprint cap with legacy parcels that contribute two envelopes. Keep larger-graph support pending until those cases, checkpoint memory and restore latency are measured. The current cap remains an explicit limitation, not a completed metropolis-scale requirement.
