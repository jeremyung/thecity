# PR 17 / PR 18 inclusion review — 2026-10-14

Reviewed main `fca58ca`, PR17 `29b077dc402f77a384f1fb1b9057d72d7c937021`, and PR18 `1bb72dd7815a2492a13d2a4f6119a67647b71353`. Both GitHub PRs were open and mergeable with no reported status checks. Mergeability is not validation.

## Decisions

- PR17 (utility poles and overhead rail wires): useful and no blocking defect found for the straight-rail main baseline in the inspected scope. Hold inclusion in the living-city checkpoint until the overhead geometry consumes authoritative curved rail samples and committed infrastructure. Retain the checkpoint's TrainFleet/TrainPlacement reconciliation and platform geometry rather than replacing these with the PR's old city.ts integration. Main-only acceptance remains conditional on browser/backend verification and evidence packaging.
- PR18 (cladding and room windows): request a small correctness fix before merge. An ordinary apartment's occupancy is baked into windowInfo by windowLived, but buildingRenderSignature only tracks apartment occupancy when an original footprint exists. A vacant-to-occupied transition therefore leaves the cached panes dark; the reverse leaves occupied rooms behind. Include all apartments in the binary occupancy signature. Update the test that incorrectly classifies apartments as occupancy-independent.
- Both features address reference fidelity, but neither fixes the primary street-layout, density, building-adjacency or activity gaps. Include them as supporting presentation work, not milestone completion.

## Reproductions and validation

PR17's overhead/train focused tests: 16 passed. PR17 TypeScript: passed (generated WASM declarations supplied for typechecking; no WASM runtime test claimed). A synthetic authoritative rail curve with a 30 m midpoint bow leaves the PR's nearest wire point 29.895 m from the midpoint. Its roadSegments/railOverhead API ignores rail_alignments and follows node chords. This is a checkpoint-integration blocker, not a new straight-rail-main regression.

PR18 existing focused tests initially passed 51 assertions in six suites; the signature suite was rerun after completing the isolated source copy. A new ordinary-apartment occupancy regression fails on the exact PR head. The proposed fix plus adjusted signature expectations passes 57 tests across eight suites (surfaces, lowrise, midrise, towers, extensions, chunks, signatures and regression). TypeScript passes. Fix is prepared locally in `.tools/pr-update-2026-10-14/pr18-occupancy-fix.patch`; it has not been pushed to PR18.

These runs verify CPU geometry and state contracts, not shader compilation or visual quality. WebGPU/mobile, motion shimmer, both styles, and the new material GPU cost need actual browser review. No blanket approval is implied.

## Evidence packaging

PR17 adds 26 image captures totaling 20,712,136 bytes. PR18 adds 27 totaling 16,287,735 bytes. Exclude those raw captures from the source checkpoint and use a separately authorized evidence location. This does not remove the retained reference collection. No recordings, references, caches or release archives were deleted or uploaded during this review.

## Prepared checkpoint update

The clean source checkpoint is `codex/city-runtime-review` at `3ec7d9c`, distinct from the old large PR12 branch. Reconstructed the earlier root export and compared subsequent changes so newer main/train reconciliation is preserved. The pending update contains:

1. Narrow-site lowrise envelope correction in kit.ts/lowrise-massing.ts, with 486 geometry specimens inside the lowrise regression and the updated size guard.
2. Coverage-audit implementation and declarations already expected by the clean branch's tests; these were missing from the earlier source split.

Prepared six-file patch: `.tools/pr-update-2026-10-14/checkpoint-source.patch`. Against a copy of the clean branch: 37 building tests and 7 coverage tests pass; TypeScript passes. Patch application check passes. Existing engine and train changes remain as reconciled; rejected city-layout experiments are not promoted. This is a bounded checkpoint, not a new full native/WASM/browser validation. The last full native result remains historical (504 passed, 0 failed, 59 ignored), not rerun for this preparation.

PR12 should remain draft. Do not update it by blindly pushing the dirty root: its history contains the bulk evidence that motivated the clean source split. The clean replacement still requires reproducible fixture delivery before merge. No public fixture/archive publication is authorized by this review request.
