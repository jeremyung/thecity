# PR10 integration into codex/living-city-fidelity — 2026-10-12

User requested local integration after review. Integrated the useful mechanisms
from PR10 d6c5664, adapting them to the current dirty worktree instead of applying
whole-file versions or merging its branch.

- Corrected shared shop-sign atlas already integrated:512×528 with complete
  padding, shared material, merged UVs and owner-managed disposal.
- SkylineClusterCache wraps the existing skylineNeighborhoods search and keeps
  its early rejection of losing clusters. Cache holds one eligible-building set;
  geometry, IDs, order, kind and anchor fields invalidate it. Access/stage
  filtering stays in CameraDirector before cache lookup, and framing/camera
  side selection continues to use the current world. No simulation behavior changes.
- Tree clearance broad phase registers conservative rotated building/anchor,
  road and rear-extension bounds. Existing clearance signature early return,
  paved-ground predicate and exact per-pair tests remain. First-two-extension
  and expressway-exclusion behavior are preserved.
- Night-shadow throttling intentionally omitted pending moving-scene/WebGPU
  evidence. Per-frame shadows and the current WebGPU material correction remain.

Root reviewed Luna's tree contribution.61focused tests pass across camera,
skyline neighborhood/cache, tree signature/cache/grid, and sign-atlas suites.
Production build passes with existing bundle warning. No full-suite or new
browser benchmark was run for this integration; unit checks do not establish
full visual/performance acceptance.

Node-only Apple M2 Max /Node26.7.0 probe, seed87231tick27925,723buildings,
717open probe candidates,30rounds: cluster recomputation p50=5.624ms,
cache hit including snapshot clones p50=.165ms. Exact cluster values match.
See pr10-cache-cost.json. This excludes access filtering and measures neither
whole setWorld nor GPU/frame cost. Tree speedup remains unmeasured here.

PR feedback was posted at:
https://github.com/jeremyung/thecity/pull/10#issuecomment-6126605022
PR10 branch itself remains unchanged; local integration does not repair its
atlas or retarget it. No commit, push or main merge performed.

## Larger-city follow-up — 2026-10-12

Previous integration turn was progress. Added reproducible
`scripts/audit-pr10-tree-grid.mjs`: bundles the actual current updateTrees twice,
replacing only its broad-phase candidate selector with a full scan in baseline.
Both sides retain paved-ground clearance and signature caching. Forces cache
invalidation to isolate structural rebuild cost, not unchanged-snapshot hits.
Four current-engine snapshots seed87231ticks27925/28325/28725/29125 grow from
723→733→743→753buildings. Uses1024fixed synthetic tree sites/scales. Exact
trunk/foliage matrices and bounding spheres match every snapshot/round.
Apple M2 Max,Node26.7.0,20measured runs per variant: median25.482→8.038ms,
p9529.411→10.201ms. See pr10-tree-grid-cost.json. This is CPU-only evidence.

Normal Import of growth-723.json into tab22 completed with seed87231tick27925,
723buildings,paused09:55day1,clear. Explore holds post-import camera;50mm,
DOFoff,High,Miniature,WebGPU,1280×720. All rendering queues reached0 through
normal scheduling. [City](screenshots/pr10-integration/city723-miniature.png) and
[diagnostics](screenshots/pr10-integration/diagnostics.png) retained.
120frames,p9516.5ms,CPU p9511.5ms,max18.5ms,824draws,37textures. No console
errors/warnings returned. This is a settled scene observation, not a matched
performance baseline, moving-scene acceptance or whole-city visual approval.
An initial graphics-startup delay notice resolved without Retry.

The screenshot still shows oversized expressway dominance, weak street-level
character and simple water/land transitions. Stop further CPU optimization
without a new measured blocker; next fidelity work should use the locked
concept comparison and resolve the provisional local-light decision.
