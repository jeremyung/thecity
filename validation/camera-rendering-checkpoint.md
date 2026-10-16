# Camera and rendering checkpoint — 2026-10-16

## PR scope

**Title:** Add configurable tilt-shift and stabilize camera rendering

**Base:** main `56f5486` (merged PR19). **Branch:** `codex/camera-rendering-checkpoint`.

The city needs controllable miniature photography while retaining clear inspection views. This checkpoint adds depth-aware tilted focus controls and stabilizes scene submission when effects are toggled. It also improves Watch subject variety and includes retained daylight, horizon, foliage and bounded architectural corrections from the reference study.

## Review order

1. `web/src/camera/tilt-shift.ts`, `miniature-scale.ts`, Controls and renderer uniforms: optional optical controls and unblurred inspection.
2. `web/src/render/scene-submission.ts`, `city.ts`, postprocessing ownership adapters and instance reuse: stable scene submission, cached pipeline selection and resource retirement. The adapters deliberately depend on Three r183 internals and must be revisited on upgrades.
3. Watch selection and founder visibility in the camera director: representative subjects, bounded interest and obstruction-aware framing. Crowded views can still obscure subjects.
4. Building massing/kit/access changes: bounded terrace homes, facade/roof construction and shared-street landing levels. These do not alter simulation land use or density.
5. README/specification priorities and the material-study decision summary. Rejected experiments are excluded, not pending implementation.

## Validation

- `npm run build`: passes TypeScript and production build. Existing Vite warning remains: main JS chunk about 1.52 MB / 456 kB gzip.
- `npm test -- --maxWorkers=2`: all 133 files / 728 tests pass.
- `npm run check:textures`: fails the existing proposed budget: 35.996 MiB estimated mip storage versus 32 MiB and a 1254px ivory-plaster development tile versus the 1024px tile limit. Texture files and the audit script are unchanged from main. This includes development-only images and is not measured runtime GPU usage; no new texture budget regression is introduced here.
- Rust/WASM implementation is unchanged; no new native rebuild claimed.
- Local WebGPU motion, lifecycle and timing evidence is summarized with limitations in [street-building-material-study.md](street-building-material-study.md).

## Scope and remaining risks

This is a bounded technical/visual checkpoint, not completion of the living-city goal. Broad reference fidelity, temporal artifact review, end-to-end GPU latency validation and live-production acceptance remain open. Detailed fallback gable end-wall visual confirmation is still obstructed, despite ray-tested geometry. Browser diagnostics do not establish a universal 60 fps claim.

No recordings, bulk captures/reports, fixture archives, dependencies or rejected prototype modules belong in this PR. Existing reference images are preserved. Local raw evidence remains ignored; the checked-in tests and build are reproducible, while historical screenshot diagnostics require local assets/harnesses.

## Manual reviewer checks

Run the normal app, compare Anime and Miniature with focus off/on, then vary tilt angle, band position, width and blur. Check near/far subject separation, switching style, resizing, day/night/rain, and ordinary Watch tours. Inspect frontage and roof construction unblurred. Repeated effects toggles should not trigger the former multi-second whole-city shader compilation stall.

## Pasted PR12 review reconciliation — 2026-10-16

The supplied review examined `codex/living-city-fidelity` at `fb039797`, not this checkpoint. Remote verification still reports PR12 open/conflicting. Current checkpoint HEAD is main `56f5486`, which merged the reconciled PR19; the current source has no divergence from main before staged changes. Do not rebase the superseded 1,038-file branch into this checkpoint.

1. **Fixture offsets:** already corrected in the pinned current-format bundle. Verified all 23 manifest hashes and restored all 18 city saves through WASM without errors. The old refresh script is not shipped here; fixed the stale README command to the supported hash-checked fixture hydrator. No fixture regeneration or bulk additions are needed.
2. **Legacy migration test:** already replaced with current-format generation, history/occupancy checks and deterministic continuation. The existing native `incompatible_activity_saves_are_rejected_without_migration` test explicitly covers incompatible schemas. Obsolete migration is not restored.
3. **Main conflict:** applies to old PR12 only. This branch starts at current main. PR12 was not modified or closed by this review response.
4. **Connector tolerance:** retained the 1 cm current-format invariant and documented its scope beside validation. Silently snapping changes path length/progress and would undermine deterministic restore. A future offset-policy change requires an explicit save-schema decision; current requirements do not promise obsolete-save compatibility.
5. **Follow ternary:** the premise is incorrect: `updateFollow` calls `setMode("watch")` after the missing-subject grace period, clearing `followId`. Named the post-update condition and documented it; retained the correct transition easing. The existing disappearing-subject regression exercises this path.

Fresh targeted validation: camera and business-activity WASM suites pass, 57 tests in two files. Full web suite previously passed 728 tests; the new change preserves behavior and only clarifies the branch condition. Native source change is a comment only; no fresh native rebuild claimed. No review comment was posted externally.
