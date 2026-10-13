# Living city checkpoint — 2026-10-12

This checkpoint captures accumulated work on `codex/living-city-fidelity` after
main integration. It is a review checkpoint, not final artistic acceptance.

## Captured scope

- Engine movement/access validation, curved expressways and traffic, and height redevelopment.
- Ground and road surface partitioning, sidewalks, rail and train presentation,
  building detail, supported roof services, and daylight fill.
- Incremental rendering and persistent street lighting to reduce growth stalls.
- Camera subject focus, depth-of-field corrections, bounded Watch visibility,
  and review recording controls.
- Updated specifications, reference studies, tests, diagnostic scripts, and
  visual/performance evidence. Historical experiment notes describe their own
  checkpoints; rejected patches are evidence, not active implementation.

## Fresh validation

- `npm run build`: passed (TypeScript and Vite); existing >500 kB chunk warning.
- `npm run test:engine -- --release`: 454 passed, 54 ignored, no failures.
- `npm test`: 594 passed, 2 failed across 107 files. Failures were a contour
  3.5 m edge-threshold assertion and a 5-second movement test timeout.
- Isolated rerun `npx vitest run tests/building-outlines.test.ts tests/wasm.test.ts --maxWorkers=1`:
  7 passed, 1 failed. Movement passes; contour threshold assertion still fails.
  The contour failure remains open; this checkpoint does not claim a green web suite.
- Source/document whitespace checks pass after trimming three trailing blank lines.
  Archived `.patch` files retain required blank context-line prefixes; these
  trigger whitespace warnings when the patch files themselves are added.
- Credential-pattern scan of changed/new text files found no matching private
  keys, OpenRouter keys, or GitHub tokens. This is a bounded scan, not a security audit.

## Review limitations and next work

The locked concepts remain the visual target. Current neighborhood density,
facade variation, street activity and composition still need improvement. Watch
visibility fallback and occlusion beyond building envelopes need review. Roof
and daylight studies are sampled comparisons, not full day/night or motion
acceptance. A previously observed 143 ms orbit hitch did not recur in the bounded
retry; its cause remains unisolated. No full hardware performance matrix is claimed.

Start review with this note, README, specs 01–08, and the later dated entries in
HANDOFF.md. Fix the reproducible contour threshold failure before treating this
checkpoint as release-ready. No save compatibility guarantee is required.
