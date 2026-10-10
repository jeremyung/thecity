# Mature void interior-access diagnostic

This read-only probe evaluated the mature browser-review save at tick 242,839 (seed 87,231). The source save SHA-256 is `c3b273bdbfb693683b4fbce4eb974cf25021bc8faa0834025f7f39e5dcaef646`; the void-mask audit SHA-256 is `1d28c0ef61f1d2740461211c6df634333e4b07d6e1eb0b1cbc8bcf6343f15ee0`. Reproduce with:

```sh
CARGO_HOME="$PWD/.tools/cargo" RUSTUP_HOME="$PWD/.tools/rustup" \
  .tools/cargo/bin/cargo test --manifest-path engine/Cargo.toml --release \
  diagnose_access_proposals_for_mature_browser_voids -- --ignored --nocapture
```

The world had no active construction, pending interior-access plan, or neighborhood plan; demand was 4.0, project admission was available, and it had 215 open connected local/arterial host roads. The production scan's bounded 48-proposal windows covered all 5,160 fixed positions (three host fractions × two sides × four branch lengths per host) in 110 read-only calls. It returned zero feasible proposals. Resetting the persisted cursor restored byte-identical save JSON. Thus these voids have no proposal being hidden solely by the current candidate ranking.

The diagnostic re-evaluated each candidate in production gate order, calling the engine's river, protected-corridor, parcel-corridor, lot-geometry, and capacity checks. The earliest rejection counts across the fixed cycle were: 4,368 parcel-corridor conflicts, 629 bounds/river failures, 64 host-endpoint margins, 32 protected-area conflicts, 20 endpoint-spacing conflicts, 13 existing-road/project conflicts, and 34 geometrically clear corridors that yielded fewer than two lots. The parcel-corridor check here tests the current 5 m exact clearance against all 836 parcel envelopes (including unbuilt parcel reservations); with no pending access or neighborhood plan, there were no extra plan reservations to account for.

To test whether the fixed fractions alone miss openings, the probe also sampled `t = 0.025..0.975` at 0.025 increments and ran the same exact validators for branches intersecting each UI-derived 2 m void mask. It evaluated 4,668 targeted branches and found no feasible access-and-lot proposal. First rejection counts were 4,527 parcel-corridor conflicts, 62 bounds/river failures, 49 insufficient-lot yields, 24 endpoint-spacing conflicts, and 11 existing-road/project conflicts. For example, the closest east/south probe to the UI sample was host road 2009 at `t≈0.475`, a 60 m branch from `(175.45,-182)` to `(175.45,-242)`: it crosses the void but fails parcel clearance. The closest west/north candidate was road 1461 at `t=.85`, length 24 m, also rejected by parcel clearance. The west/center candidates near roads 159 and 200 had the same result. The exact representative records are in [mature-void-access-proposals.json](mature-void-access-proposals.json).

This indicates the remaining voids are not simply an unvisited sub-grid split location: sampled branches into them run through existing parcel envelopes, while the few corridor-clear candidates cannot fit the required two lots. Lowering clearance or forcing a road through existing plots would violate the current geometry contract. A viable next design needs to preserve future access corridors before parcels are admitted, or create a coordinated block layout that reserves access at the same time as its lots. This probe does not prove no feasible geometry exists at every continuous `t`, and the UI's 2 m masks are approximate; it does show that finer sampling alone is unlikely to unlock these three gaps without changing how access corridors are reserved.

The diagnostic lives in `engine/src/mature_void_diagnostic.rs` behind an ignored test. It changes no simulation behavior or save schema.
