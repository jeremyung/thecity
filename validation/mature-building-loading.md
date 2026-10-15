# Mature building loading

## Change and reason

Mature snapshot loading spent most static-chunk updates extracting contours under a fixed 2,000-operation allowance. The phase diagnostic found 1,694 of 1,860 updates ending in contour work, with no commit-gate waits. This delayed complete scene review by several minutes.

Contour preparation now processes 500-operation slices, checking a cooperative 1.5 ms deadline between slices, with an absolute 8,000-operation ceiling per preparation update. Geometry transformation, merging, source regeneration and mesh-count quotas remain unchanged. The contour extraction algorithm and output are unchanged. The deadline is not a hard frame-time guarantee: one slice, allocations and garbage collection can overrun it.

## Complete-load comparison

One sequential pair on Apple M2 Max/macOS, Chromium 145 headless, actual ANGLE Metal backend. Both use the same seed-0/tick-83000 renderer snapshot, paused worker, Anime style, 17:00 lighting preview, 1200×800 buffer, pixel ratio 1, fixed balanced quality and depth of field disabled. The renderer-only load does not test production save restoration or subsequent simulation growth. UI clock/population fields are not synchronized by this harness.

| Measurement | Baseline | Candidate |
| --- | ---: | ---: |
| All loading queues empty | 339.5 s | 164.4 s |
| Building-update p50 / p95 / p99 | 0.9 / 3.4 / 5.4 ms | 1.9 / 5.2 / 7.6 ms |
| Building-update maximum | 44.8 ms | 38.7 ms |
| Loading frame interval p95 / p99 | 36.3 / 38.1 ms | 36.7 / 38.9 ms |
| Loading frame intervals over 50 ms | 24 of 12,871 | 24 of 6,083 |
| Post-load interval p95 / p99 | 35.7 / 43.3 ms | 35.2 / 42.9 ms |

Loading finished about 52% sooner in this pair, at the cost of more building-preparation CPU per frame. The long-frame count stayed the same over fewer frames, so its proportion increased. Shader compilation and other startup work produced multi-second outliers in both runs. The 15-second post-load observation includes the move to the fixed review pose and geometry hashing; it is not a clean steady-state benchmark. Browser cache differences and single-run variance prevent a general speedup or performance-budget claim.

All 1,007 committed building attribute/index buffers match by SHA-256, including contour buffers. The input world, final camera, target, lens, quality, tick and style also match. Both loading queues completed without page errors. Matched captures were inspected: the mature expressway traverses the earlier apparent lawn, confirming that the rejected authored infill study was not buildable land.

Input SHA-256: `97ede53c53db3487ba29ddbaa1138c5622926cfeb2154a6d464387c500a8dbaa`.
Sorted geometry-record SHA-256: `a5cd0a74ad789f4d160b4a1b6500f0434d14d1965bb495edfb058d6558cfe867`.

## Verification and boundaries

Targeted tests cover contour equivalence against Three.js EdgesGeometry, the operation ceiling, clock-driven yielding, cancellation, construction transitions and style resolution. Existing visual assertions and geometry detail are retained. Full web and TypeScript results are recorded in the checkpoint review document.

This is a loading improvement, not visual acceptance or a solution to all browser performance issues. WebGPU, mobile, both-style full-load comparisons, production growth/renewal, memory stability and user visual acceptance remain open. The benchmark does not justify raising the ceiling further.

Local diagnostic scripts and raw results are retained under `.tools/compare-mature-load.mjs`, `.tools/summarize-mature-load.py` and `.tools/mature-load-comparison/`; these are not distributed test dependencies. No recordings or new binary evidence are included in the PR.
