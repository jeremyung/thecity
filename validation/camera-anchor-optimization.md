# Camera anchor neighborhood search — 2026-10-12

The preceding browser trace measured CameraDirector.world at median 20.5 ms per
snapshot. A bounded CPU harness isolates buildAnchors as the expensive pure
snapshot method; obstacle-envelope generation and developed extent are sub-ms.

The skyline search previously scanned all eligible buildings for every seed.
It now creates temporary 90 m spatial bins, checks the adjacent nine cells with
the identical Math.hypot <=90 predicate, and sorts accepted indices back into
original building order. This preserves floating-point reduction order and tie
selection. Candidate framing is calculated only after its score beats the
current winner. No cached world state, changed scoring, fewer anchors, changed
collision checks, or reduced visual quality is introduced.

Luna supplied the CPU harness; root reviewed its actual prototype invocations,
restored WASM inputs and exact output comparison, then reran after refinements.
`camera-anchor-cost.json` records five worlds, seed87231 ticks22026–30026,
575→775 buildings, three measured rounds each after warmup, M2 Max/Node26.7.
Final buildAnchors median 11.151→5.840 ms; p95 14.539→7.250 ms. All anchor
fields, vector coordinates and output order match the baseline on all five
worlds. Initial binning improved only ~17%; filtering before sorting and avoiding
losing-candidate framing produced the retained result (~48% median reduction).

52 focused tests pass: camera behavior, follow visibility/presentation, and new
exact neighborhood-order/boundary tests across dense, dispersed and negative
coordinate layouts. TypeScript and production build pass (existing bundle-size
warning). These establish CPU/behavior evidence, not browser frame acceptance.

The reproducible script requires the preserved baseline bundle
`.tools/camera-cost/before.mjs` and current save listed in its report. Those are
local diagnostic inputs; the new neighborhood regression tests are self-contained.
Benchmark versions run in fixed order and use different dependency bundling
(baseline includes Three, candidate resolves it externally); sub-ms differences
in unchanged methods must not be treated as improvements. No camera movement,
GPU, long-session memory or complete world() benchmark claim is made.

Next validate normal browser growth and camera modes at this checkpoint. Tree
clearance remains another measured cost; do not call growth stalls fixed from
this isolated anchor benchmark.
