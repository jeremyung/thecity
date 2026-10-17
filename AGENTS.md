# Agent rules

## Orient cheaply
- Start from this file and validation/STATUS.md. Do not read README, specs or
  validation logs end to end; open the one spec section the task names.
- Find code with targeted grep (symbol names, -n, a path filter). Never read a
  large file whole; read the matched range only.
- Search tools skip validation/, art-workshop/, concepts/ and public/ unless
  the task is about them (see .ignore).

## Module map (where to change X)
See ARCHITECTURE.md for the full map.
- Simulation rules and state: engine/src/<domain>/ (core, world, infrastructure,
  population, transit, persistence); the tick order is engine/src/core/systems.rs
- Pure geometry / master plan: engine/crates/plan/
- WASM bindings: engine/crates/wasm/
- Retired planners: engine/src/legacy/ (feature `legacy`; do not extend)
- Worker and protocol: web/src/worker/
- Snapshot and wire types: Rust first, then `npm run types` (web/src/generated)
- One visual feature: web/src/render/<feature>.ts, registered in render/layers.ts
- Camera / audio / UI / saves: web/src/camera, audio, ui, storage; wiring in main.tsx

## Scope
- One concern per change: a refactor, a behavior change, or a visual change,
  never two in one commit. Refactors must not change saves or replay output.
- Do not widen the task. Note unrelated problems in the final message instead
  of fixing them.
- New code goes in the owning domain module. Do not add methods to Engine,
  fields to WorldState, or features to CityRenderer directly.
- No new file over 1,500 lines; split before you cross it.
- Production code carries no experiments. Put experiments behind a cargo
  feature or in tests, and delete them when the decision is made.

## Layering
- render never imports worker, storage or ui. ui never imports render internals.
- Only the worker touches the engine. The renderer reads snapshots only.
- Engine types change in Rust first; regenerate TypeScript types, never edit
  them by hand. Bump the protocol version for any wire change.
- Use enums and typed IDs, never string kinds or bare integers.

## Validate the smallest thing first
- Rust change: `cargo test --manifest-path engine/Cargo.toml --features legacy <test_name>`
  (pure planning: `-p city-plan`), then `npm run test:engine -- --release`.
- Web change: `npx vitest run tests/<file>.test.ts`, then `npm test`.
- Run `npm run wasm` (and `npm run wasm:legacy`) only when engine code or bindings
  changed; run `npm run types` when a snapshot type or wire layout changed.
- Before pushing also run `npm run lint:deps` and `npm run check:size`.
- Before pushing: `npm run build` and the full suites once. Report failures with
  the failing test name and first error line, not the whole log.
- Visual changes need a capture compared with the locked references; passing
  tests alone do not accept a visual change.

## Write less, in the right place
- Specs describe the current design only. Dated findings go in
  validation/<topic>.md; current priorities go in validation/STATUS.md (keep
  it under 2 KB, replace rather than append).
- Do not add status paragraphs to README.
- Commit messages: one line of what changed, one line of why.
- Final message: what changed, how it was validated, what is still open.
  No restating the diff.

## Delegation
- Delegate only self-contained tasks with a named file set and a check
  command. Give the module map entry, not the whole repo.
- Sub-agents return a summary and the diff, not file dumps.
