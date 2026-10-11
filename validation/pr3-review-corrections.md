# PR #3 review corrections — 2026-10-11

## Current-format save correctness

A matching seed/plan digest and vector length did not prove that persisted plan
indices referred to the right world entities. A nonexistent nonzero lot mapping
could suppress that lot's future release. Restore now checks mapping uniqueness,
entity existence, node positions/kinds, road endpoints/kinds, lot geometry,
frontage direction/position, rotation and access node. World entities cannot be
hidden by clearing their mappings. Founding mappings and rail/expressway progress
must agree with realized entities. This is validation at the save/load boundary,
not a change to growth scheduling or a backward-compatibility migration.

The existing mature replay test now continues its original live engine rather
than comparing two restored copies. A new native test and WASM verification
script save during real building, road, rail and expressway construction for seeds
0 and 87231, then compare the live engine with its restored copy every 80 ticks
for 1,600 ticks. No private state mutation is used to create these construction
checkpoints. Eight native construction saves also load and step in WASM. This
checks cross-target plan/restore compatibility, not identical full native/WASM
growth histories.

## Scope and priorities

The user waived backward save compatibility. Current-format validation and
save/resume equivalence remain required. No migration infrastructure was added;
existing compatibility paths were not removed in this focused fix.

The goal, README, architecture and delivery specs now put the existing 800 m
city's visual and measured performance acceptance ahead of larger maps/imports.
The original complete audiovisual scope and beauty gates remain. Cleanup issue
#4 now requires retaining/replacing generic simulation regressions rather than
blanket test deletion. The goal remains paused for review.

## Reproduction

```sh
node scripts/test-engine.mjs --release
MASTERPLAN_REPLAY_FIXTURE_DIR="$PWD/.tools/masterplan-review-fixes" node scripts/test-engine.mjs --release masterplan_replay_tests
npm run wasm
node scripts/verify-masterplan-replay.mjs .tools/masterplan-review-fixes
npm test
npm run build
```

## Results

- Native: **400 passed, 0 failed, 53 ignored**. The previously reported legacy
  frontier replay failure passes on this machine; no test was removed or disabled.
- WASM: rebuilt successfully; both live/reloaded seeds pass all four construction
  kinds and reject invalid mappings. All eight native construction saves restore
  canonically and step successfully.
- TypeScript and production build pass; the existing large-bundle warning remains.
- The full frontend run reports **288 passed, 2 timed out** at the existing
  5-second limit (`lowrise-kit.test.ts` and `wasm.test.ts`). Rerunning both files
  with one worker gives 15 passes and the same movement-test timeout; low-rise
  checks pass unchanged. The movement test passes alone with a diagnostic
  15-second CLI budget (actual test time 4.66 seconds). No committed time limits
  were changed. This is not an all-green default frontend-suite claim.

No visual or hardware frame-rate acceptance is claimed by these correctness fixes.
