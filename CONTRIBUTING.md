# Contributing

Read [AGENTS.md](AGENTS.md) for the working rules (they apply to people too) and
[ARCHITECTURE.md](ARCHITECTURE.md) for where code lives.

## Validate

Run the smallest relevant check first, then the full set before pushing:

```sh
cargo test --manifest-path engine/Cargo.toml <test_name>   # one engine test
npx vitest run tests/<file>.test.ts                        # one web test file
npm run test:engine -- --release                           # all engine tests
npm run wasm && npm test                                   # web + WASM integration
npm run build
```

CI also checks formatting (`cargo fmt --check`, `npx prettier --check`) and that the
tree is clean after the build.

## Kinds of change

Keep refactors, behavior changes and visual changes in separate commits. A refactor
must leave engine tests, replay fixtures and save round-trips unchanged. A visual
change needs captures compared with the locked references.
