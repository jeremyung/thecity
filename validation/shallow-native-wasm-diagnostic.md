# Native/WASM shallow-frontage count difference

The 45-versus-44 result is not caused by an extra native replay step or by
different depth-counting rules. Both runs restore
`validation/spine-ranking-seed-87231-by-sites-false.json`, set development
rate 16, and advance exactly 100 calls of 200 ticks from tick 242000 through
262000. The native test's 400-tick save/replay check advances only two separate
restored clones after each checkpoint; it does not advance the measured engine.
The WASM script also checks its 400-tick twin only after recording the 20,000
tick result. The native trace additionally counts only parcels whose depth is
exactly 8.0, matching the WASM script's strict equality; it still gets 45.

Both initial canonical saves have the same FNV-1a-64 hash (`6a6860943fb237a9`).
The first five 200-tick checkpoints remain byte-identical, through tick 243000.
At tick 243200, counts, new parcel IDs, frontage cursors and `rng_growth` still
match, but four saved float fields differ for parcel 3666 on connected road
3424:

| Field | Native | WASM |
| --- | ---: | ---: |
| x | -6.6238756 | -6.6238766 |
| width | 9.091785 | 9.091782 |
| rotation | -1.6543792 | -1.6543791 |
| frontage along | 0.5299522 | 0.52995217 |

Its z coordinate and access node are identical. This is the earliest observed
state divergence, 1,200 simulation ticks into the run. It occurs during candidate
geometry generation for an angled frontage, before either random stream or
planner cursor diverges. The relevant computations are the f32 projection and
interval-width arithmetic in `frontage_gap_candidates_with_layout`, followed
by frontage `atan2` for parcel rotation. The evidence points to a small
floating-point implementation difference; it does not identify one specific
instruction or prove whether the input is `hypot`, projection arithmetic, or
`atan2`.

The first consequential planner difference appears at tick 251600. Both runs
have 22 new shallow lots, but parcel ID 3723 is assigned to different valid
candidates: native places it at `(177.81786, 54.0)`, width `12.864288`, access
node 1651; WASM places it at `(-361.7333, 342.35464)`, width `9.826836`, access
node 1675. At this checkpoint the growth RNG still matches, while the
frontage cursor differs: native `(59, 23)`, WASM `(61, 8)`. By tick 252000,
WASM has one fewer shallow lot and the RNG states also differ. The runs finish
with 45 native lots / 798 total parcels and 44 WASM lots / 797 total parcels.

The public binary tested was SHA-256
`307fbd79e59acc2ab3c75986e3f786e786ea7cc865c460d150893fceb2ad457c`; the
input fixture SHA-256 is
`4d03ed81247fb835129bd5090c815937f3da14d9b29dfb3788221eb9c3233239`.
`engine/src/lib.rs` at trace time was
`a1cb15ee61dd1d7c7f0141d5766aa3f8906506115f7bb7be2aefce878491f56a`.

Reproduce with:

```sh
CARGO_HOME="$PWD/.tools/cargo" RUSTUP_HOME="$PWD/.tools/rustup" \
  .tools/cargo/bin/cargo run --manifest-path engine/Cargo.toml --release \
  --example shallow_trace > validation/shallow-native-trace.jsonl
node scripts/diagnose-shallow-parity.mjs
```

The per-checkpoint traces are `validation/shallow-native-trace.jsonl` and
`validation/shallow-wasm-trace.jsonl`. The WASM run also confirms the original
construction/access/replay assertions: all 44 admitted lots have open
buildings and connected access. This diagnostic isolates a cross-target
floating-point sensitivity in geometry-driven candidate admission; it does
not make native and WASM evolution byte-identical or establish which count is
the desired result.
