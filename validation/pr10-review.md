# PR #10 review — 2026-10-12

Reviewed `d6c566450af97088f3c9ec06b4c575007583b256` (perf/render-cost).
PR: https://github.com/jeremyung/thecity/pull/10
Recommendation: useful work, request changes before merging wholesale.
No GitHub review/comment, merge, cherry-pick, or production source edit performed.

## Confirmed finding

P2, web/src/render/materials.ts:213–215: the three padded sign cells require
3*(160+2*8)=528 pixels, but the atlas is 512 pixels tall. The third cell begins
at360 and ends520. Its bottom UV is -0.015625; canvas clipping loses the bottom
8 pixels, including part of the border, and clamp sampling stretches the edge.
Derive atlas height from cell count/stride (or repack cells within512) and add
bounds/padding coverage. This is a small but real fidelity regression.

A focused regression importing the actual PR ArtMaterials and checking every
SHOP_SIGN_LABELS cell's UV interval failed for 商店 with -0.015625. The test
lives in /private/tmp/thecity-pr10-review/tests/pr10-review.test.ts for review;
it was excluded from the existing-suite run below, not counted as a baseline failure.

## Validation

Exported the exact fetched PR head with git archive into
/private/tmp/thecity-pr10-review, using installed locked dependency versions.
Rebuilt its own WASM with npm run wasm; did not reuse current worktree WASM.
Initial build/camera check lacked generated WASM; reran after generating it.
- npm run build passes (existing bundle-size warning).
- Existing vitest suite:289passed,1failed of290 across55files. Failure is
  tests/wasm.test.ts movement validity at the default5000ms timeout.
- Isolated diagnostic rerun of that test passes in4.47s. This does not turn
  the default full-suite run into a pass. Engine tree and that test are identical
  to origin/main, verified with git diff --exit-code.
- Focused new atlas bounds regression fails as above.
- git diff --check passes.
- GitHub showed no status checks, comments or reviews at inspection time.
- Did not independently reproduce PR performance/pixel-comparison claims or
  run the PR on WebGPU. Those remain author-reported evidence.

## Integration recommendation

PR currently targets generator/lot-partition. PR3 has merged to main as48ed560;
main containsf386f45. The main-to-PR change set is the same five-file renderer
optimization; retarget PR10 to main before eventual merge.

Current codex/living-city-fidelity HEAD8e9f5e1 has substantial uncommitted work.
Read-only git apply --check fails in director.ts, city.ts and materials.ts.
Do not overwrite those files with PR versions.

- Shared shop-sign atlas: valuable draw reduction; port after fixing bounds.
  Keep the separate storefront-poster atlas and its WebGPU shadow-binding fix.
- Skyline: current worktree already has a tested spatial neighborhood helper
  and skips framing work for losing clusters. Add memoization around that
  implementation if profiling supports it; preserve cache invalidation fields
  and world-order ties rather than reintroducing a second inline spatial grid.
- Trees: current worktree caches clearance and handles paved ground. PR broad
  phase is complementary on structural updates; integrate inside that existing
  path, preserving pavement/rear-extension checks and instance-array parity.
- Night shadows: defer or separate. All actor instances cast shadows, so the
  city is not static at night. A four-frame cadence needs moving-actor, growth,
  camera, style-switch, quality-change and WebGPU review. No observed GPU bug
  is claimed here; current evidence is insufficient for that quality tradeoff.

PR adds no regression tests for the new atlas, skyline cache/grid, tree grid,
or shadow invalidation. Add targeted tests before calling these independently
validated; existing290tests demonstrably miss the atlas boundary defect.

## Follow-up

User-authorized feedback posted at
https://github.com/jeremyung/thecity/pull/10#issuecomment-6126605022 .
Useful changes subsequently adapted into the local working branch; see
[local integration](pr10-integration.md). The reviewed PR head is unchanged.
