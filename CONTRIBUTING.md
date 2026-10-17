# Contributing

Read [AGENTS.md](AGENTS.md) for the working rules (they apply to people too) and
[ARCHITECTURE.md](ARCHITECTURE.md) for where code lives.

## Validate

Run the smallest relevant check first, then the full set before pushing:

```sh
cargo test --manifest-path engine/Cargo.toml <test_name>   # one engine test
npx vitest run tests/<file>.test.ts                        # one web test file
npm run test:engine -- --release                           # all engine tests
npm run wasm && npm run wasm:legacy && npm test             # web + WASM integration
npm run build
```

CI also checks formatting (`npm run format:check`), import boundaries
(`npm run lint:deps`), the 1,500-line file limit (`npm run check:size`), that
`npm run types` leaves `web/src/generated` unchanged, and that the tree is clean after
the build.

## Adding things

- **A simulation system:** write the function in the owning domain module, then add a
  `System` entry in `engine/src/core/systems.rs` at the right phase. Replays change, so
  record the new baseline in the same PR.
- **A render layer:** implement `RenderLayer` (`web/src/render/layer.ts`) in a new file
  and add its factory to `web/src/render/layers.ts`.
- **A snapshot field:** add it to the Rust type, run `npm run types`, and bump
  `WORKER_PROTOCOL_VERSION` if the wire changes.

## Kinds of change

Keep refactors, behavior changes and visual changes in separate commits. A refactor
must leave engine tests, replay fixtures and save round-trips unchanged. A visual
change needs captures compared with the locked references.
