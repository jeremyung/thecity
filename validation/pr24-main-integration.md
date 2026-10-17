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

The subsequent review fix corrects the moved benchmark root, points the legacy
diagnostics and profilers at `.tools/wasm-legacy`, and uses the maintained save
fixture. Setup commands are documented in CONTRIBUTING.md.
- Headless Chromium/WebGL smoke attempt was inconclusive: the architecture-city
  page timed out while reading its body after startup. No new visual acceptance
  is claimed by this merge resolution.

## Review tooling follow-up

- Corrected the staged-building benchmark's repository root after its move into
  `scripts/benchmark/`; its default input is now the maintained 850-building save.
- Switched discovery/proposal diagnostics and profilers to the isolated legacy
  WASM build, including recorded binary hashes, and current-schema save inputs.
- Proposal descriptor inputs use their relocated fixture directory; its native
  oracle report remains a locally generated prerequisite.
- Replaced stale work/count constants in the breakdown diagnostic with an
  independent uninterrupted-run comparison and the current descriptor fixture.

Validation: the 850-building benchmark completed; discovery input parity,
work-breakdown accounting (7,506,942 work units, 3,180 descriptors), quota/checkpoint
profiling, and native/WASM proposal profiling completed. The native proposal oracle
passed. All 768 web tests and the production build passed again. Dependency, size,
formatting, and whitespace checks passed. No generated timing reports are committed.

## Main integration after PR36

Merged main at `778cf87` after the business architecture work reached main.
Preserved the business families, district preferences, media displays and glass
lighting while retaining PR24's architecture owner, extracted geometry helpers,
injected camera and actor layer. Moved the expanded architecture study into the
development-only entry point and adapted its controls to those interfaces.
Commercial preference parsing now lives outside renderer internals, and the new
fixtures use the generated snapshot enums.

Validation: all 807 web tests across 148 files passed; TypeScript and production
build passed, as did dependency boundaries, source-size limits and generated
architecture asset verification. The engine is unchanged by this merge; the
existing fresh production and legacy WASM builds were used. No new browser visual
acceptance is claimed.
