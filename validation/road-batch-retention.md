# Retaining completed infrastructure — 2026-10-12

Deficiency: live growth rebuilds completed road furniture on every construction
progress bucket. The prior browser profile measured road-stage median 61.9 ms.

## Change and acceptance

CityRenderer now keeps two attached, material-batched groups: completed road
infrastructure and active construction. Only construction is replaced on progress
changes. Completed geometry invalidates on nodes, all crossing/reservation road
endpoints/kinds/open/connected state, deck controls/heights, rail layouts, river
shape/width and station canopy threshold. Shared materials remain owned by Art;
retired geometry/owned resources use the existing compile-aware disposal path.
The unused roadGroups map was removed. Twelve-step construction presentation
cadence is unchanged. Simulation and save contracts are untouched.

Accept only if retained geometry stays identical, dependency changes invalidate,
completion transfers geometry, style switching works and measured work declines.
Reject loss of furniture, stale crossings, resource leaks or unbounded draw growth.
This is a bounded two-batch split, not per-road meshes or a general cache.

## CPU evidence

`road-batch-cost.json`: Apple M2 Max, macOS arm64, 32 actual WASM snapshots at
80-tick intervals from seed87231/tick18918/growth16. Calls exercise real geometry
builders directly, not the production scheduler; each snapshot invokes rebuild.
Initial build excluded from timing summary. Compared with locally captured
pre-change renderer, 14 of32 updates retained completed geometry. Aggregate
subsequent build time fell1267→712ms (44%). Median39.26→38.36ms and p9550.73→46.56ms:
there is no claim that topology-changing updates are now fast.

At first/last snapshots, sorted triangle vertex attributes/material keys and
instance matrices hash identically; triangles95258 and113978 respectively.
Road/station mesh counts increase43→47 initially and43→45 finally because
construction is a separate batch. Hashing does not verify pixels or shader output.

Reproducible isolation command:
`node scripts/audit-road-batches.mjs .tools/fidelity-review/sidewalk-live-growth.json`
compares current code with forced completed-batch regeneration. Its report,
`road-batch-retention-cost.json`, repeats14 retained updates and geometry parity.
Aggregate1413→825ms (42%), but p9557.72→64.41ms in that run. Retention reduces total
work; tail-latency improvement is **not established**. Optional third argument
`before` uses the ignored local pre-change bundle captured before editing; it is
not required for the default reproducible comparison. No GPU timings here.

## Browser smoke check

Normal WebGPU import of the same tick18918/441-building save, growth16/speed8,
High detail, Miniature, Overview,50mm,DOFoff,1280×720. City advanced normally to
680buildings,tick28434,day1 10:26,clear0. Export is retained at
`.tools/fidelity-review/road-batches-live-growth.json`.

One running window at591buildings: framep95126.6ms,CPUp9525.3ms,maxframe178.6ms,
6neighborhood/1sidewalk jobs pending. This differs from the earlier baseline's
city/time/window; **not a matched browser speedup claim**. Hitching remains.

[Running diagnostics](screenshots/ground-depth/road-batches-running.png),
[Miniature](screenshots/ground-depth/road-batches-miniature.png),
[Anime](screenshots/ground-depth/road-batches-anime.png).
Same paused clock/settings for style captures; overview camera continued settling
between captures, so framing is not pixel-matched. Both render infrastructure,
and switching completed with no captured warn/error logs. No fallback/motion or
long-run leak acceptance from this smoke check. Tab15 retained paused Anime.

Root reviewed delegated tests, replaced an initially vacuous group.geometry
assertion, and ensured construction vertex data is captured before disposal.
Tests cover real geometry retention/change/transfer, dependency signatures,
station canopy and deferred owned-resource disposal. Six focused batch tests
pass; broader six-suite pass and production build recorded in handoff.

## Remaining work

Topology changes still synchronously rebuild completed infrastructure. Trees and
transit/access remain measured structural costs. Next inspect finer invalidation
or stage full-road rebuilding with a same-fixture browser comparison before
claiming performance acceptance. Preserve visual details. Raised junction seams,
original artifact camera reproduction and full concept fidelity remain open.
