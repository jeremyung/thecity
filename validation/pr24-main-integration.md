# PR24 integration with main

Merged main at `5cdf9cb` into the architecture refactor, preserving PR21's
apartment rendering and centered tower entrances. The refactor keeps its extracted
renderer layers, camera injection, generated snapshot types, and development-only
review boundary.

Integration adjustments:
- Apartment fixtures now satisfy the Rust-generated snapshot types.
- Architecture presentation state and lighting synchronization have a separate
  owner; the renderer stays below the 1,500-line limit.
- All building families share the architecture geometry kit.
- Architecture preferences live outside renderer internals; review pages live in
  `dev/` and receive an injected camera director.
- Generated geometry assets retain their generator's exact output and are checked
  by `check:architecture`, rather than being reformatted or counted as handwritten
  source.

Validation on the integrated tree:
- Fresh production and legacy WASM builds.
- Native workspace: 543 passed, 59 ignored, no failures.
- Web: 768 tests across 140 files passed.
- TypeScript and production build passed.
- Generated type regeneration, dependency boundaries, file-size check, architecture
  asset verification, Prettier, and seven review-evidence tests passed.

This merge resolution does not change the separate review findings about the moved
staged-building benchmark root or legacy diagnostic WASM imports.
- Headless Chromium/WebGL smoke attempt was inconclusive: the architecture-city
  page timed out while reading its body after startup. No new visual acceptance
  is claimed by this merge resolution.
