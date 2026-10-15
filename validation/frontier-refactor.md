# Frontier scan extraction validation

The frontier search in `Engine::plan_road` was extracted into a read-only
`propose_frontier_road` scan. It returns the first viable proposal and the
cursor and RNG state reached at that point. An exhausted nonempty search allows
the existing interior-access fallback; an empty candidate set preserves the
old immediate return. Local-chord planning remains before the scan.

## Compatibility comparison

Before editing the engine source, I copied the engine crate and validation
fixtures to `/tmp/thecity-frontier-compare`, appended a test-only capture
harness, and captured the old planner's complete serialized saves. I then
replaced the copied source with the refactored source and captured the same two
cases. Both output pairs compare byte-for-byte:

| Case | Baseline SHA-256 | Refactored SHA-256 | `cmp` |
| --- | --- | --- | --- |
| Mature `ranked-blocks` save; one `plan_road` call, exhausted frontier search | `90deae75bb2c983294032f1f6f9e92ab8e15030449ccde0128229d7a0982c3e2` | same | identical |
| Seed 0, rate 60, 36 × 2,000 ticks; one `plan_road` call, frontier project admitted | `6f9f82311ac3b03a3ecca4073fe2c3999581ea80b5a6d429c72ea7f40e30181a` | same | identical |

The mature input fixture SHA-256 is
`2c4c31f68ee06b24fcc8554dcd145f54e5aef26b65375a6ff26e7b86f9c7e2ea`.
The current source hashes after the refactor are:

- `engine/src/lib.rs`: `bf9da7bb453ba7d6efd2e40682fbb9f36a844504ea757ff8df9e592580d33097`
- `engine/src/frontier.rs`: `b774ce8e8021a7f427a8df6c80054145852ef36b9037fea44e58f7e3f3bbb27d`

The pre-edit source-file SHA was not recorded before the temporary baseline
copy was replaced with the candidate source. The baseline result files remain
available in `/tmp/thecity-frontier-compare/result.json` and
`/tmp/thecity-frontier-compare/result-seed0-72k.json`; candidate files are
`candidate-mature.json` and `candidate-seed0-72k.json` in the same directory.

The baseline and candidate capture commands used the local toolchain:

```sh
CARGO_HOME=./.tools/cargo \
RUSTUP_HOME=./.tools/rustup \
./.tools/cargo/bin/cargo test \
  --manifest-path /tmp/thecity-frontier-compare/engine/Cargo.toml \
  frontier_equivalence_capture -- --nocapture

CARGO_HOME=./.tools/cargo \
RUSTUP_HOME=./.tools/rustup \
./.tools/cargo/bin/cargo test \
  --manifest-path /tmp/thecity-frontier-compare/engine/Cargo.toml \
  frontier_equivalence_candidate_capture -- --nocapture
```

## Tests

`frontier_proposal_scan_is_read_only_and_plan_road_replays` verifies a
nonempty proposal scan, repeatable scan output, no mutation of serialized state,
RNG or cursor, the exhausted-search fallback flag, and deterministic
`plan_road` replay. The full native release command was
`npm run test:engine -- --release`: 126 tests were reported, with 3 ignored and
the remaining tests passing. A final focused debug run of the new test also
passed.
