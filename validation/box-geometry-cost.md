# Box geometry construction — 2026-10-12

## Evidence and decision

The preceding turn retained completed infrastructure, reducing redundant work.
The next section profile showed geometry generation still dominates a full road
rebuild: road kits about25.4ms, retire/bank about4.5ms, merge about3.6ms, station
about1.4ms in the first diagnostic run. Profiling is done in an ignored esbuild
bundle; production source has no timers. Current reproducible section results are
`road-section-cost.json` (legacy box construction) and
`road-section-cost-optimized.json` (candidate). These short independent runs have
noise and do not isolate a reliable improvement in the road-kit subsection.

A delegated direct-transform prototype failed exact normal parity (signed zero)
and was discarded. The accepted second prototype only optimizes creation of
one-segment positive box attributes. It copies immutable template normals, UVs,
indices and groups into independently owned buffers and fills dimensioned
positions. Three's standard rotate/translate operations are retained, including
Float32 rounding. Nonpositive/nonfinite dimensions use the original constructor.
Kit.box now uses this helper. No geometry, materials, textures, light/shadow,
water, FSAA, simulation behavior or scheduling quality was removed.

## Validation

Root inspected the helper and tests. Five hundred seeded dimension/transform
cases compare position/normal/UV/index arrays and groups exactly; edge inputs
exercise the original constructor fallback. Buffer ownership is independent.
The first/last snapshots of the32-snapshot actual-WASM road corpus retain exact
sorted triangle-attribute/material-key/instance-matrix hashes and mesh counts
(`road-box-cost.json`). This is geometry evidence, not pixel or shader proof.

The corpus comparison (retention enabled for both variants) showed median
38.49→35.40ms, aggregate726.57→687.25ms after excluding initial builds. Order and
host load were not controlled, so a paired full-rebuild comparison was added.

`node scripts/audit-road-box-pairs.mjs` reconstructs the legacy Kit.box inside a
diagnostic bundle; no old checkout or generated baseline file is required.
Apple M2 Max, macOS/Node, seed87231 tick18918. Eighteen alternating-order pairs,
first two excluded: candidate faster15/16; median39.53→34.45ms (~13% reduction),
aggregate636.58→564.14ms. Maximum43.74→42.75ms: stalls remain. This is CPU build
cost, not browser frame time or GPU timing. The result supports retaining this
small helper but does not justify further primitive micro-optimization as the
primary workstream.

71tests in15 selected rendering/geometry suites pass. TypeScript and production
build pass; existing large-bundle warning remains. Native/WASM engine unchanged.

## Browser check and limits

Hot reload left the browser on its graphics-startup timeout message. Captured
warn/error logs were empty. A full reload recovered; do not treat that as a
passing hot-reload lifecycle check or assign causality from this observation.
Normal Import restored680buildings,seed87231,tick28434,clear0,day1 10:26.
WebGPU,High,Anime,Overview,50mm,DOFoff,1280×720. Normal scheduling reached zero
building/neighborhood/sidewalk queues. A settled window reported framep9514ms,
CPUp9512ms,max17.9ms; this excludes growth and import hitches.

[Scene](screenshots/ground-depth/box-geometry-anime.png) and
[diagnostics](screenshots/ground-depth/box-geometry-diagnostics.png). No captured
console warnings/errors after recovery. Framing differs from previous captures;
this is a smoke check, not matched artistic acceptance. No fallback or continuous
motion check in this turn. Tab15 retained paused on this checkpoint.

## Next action

Full topology rebuilds are still synchronous and exceed a frame budget. Stage
road geometry generation and final merging with a bounded active job plus latest
snapshot, preserving coherent construction/completed infrastructure and selected
style at commit. Avoid cancel-on-every-update or current-key-only publication:
the sidewalk investigation already demonstrated continuous-growth starvation.
Measure commit lag and same-fixture browser motion before acceptance. A single
long rail/expressway kit and destination-array allocation also need explicit
bounds. Do not take smaller box savings as completion of the performance gate.
Raised junction seams, original artifact camera verification, visual richness
and the remaining living-city requirements remain open.
