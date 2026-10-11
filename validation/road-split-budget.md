# Synchronous road-split budget

This report measures native Rust `Engine::try_split_open_road` directly. It includes both the read-only preflight and commit/connectivity refresh, and excludes fixture construction, JSON restore, and construction-timer work. These are native timings, not browser or WASM frame measurements.

The harness is `engine/src/road_split_profile.rs` (enabled by a root-owned `#[cfg(test)] mod road_split_profile;` declaration in `engine/src/lib.rs`). Run its measurements with `node scripts/test-engine.mjs --release road_split_profile -- --ignored --nocapture`. It profiles a fresh-world road, the 2,000-traveler cap with 128-node routes, and two 4,999-road cases. In the route-cap case, 1,999 actors use valid back-and-forth routes on another connected road; one actor is actively on the host edge with a 127-node route, which expands to exactly the 128-node cap. The geometry cases use either detached unique-endpoint survey roads or a connected open-road chain; both preserve a valid host split point. To emit a canonical 4,999-road/2,000-traveler save for a future WASM probe, run that command with `EXPORT_ROAD_SPLIT_PROFILE=1`; the fixed output is `validation/road-split-profile-stress.json`, and ordinary test runs never write it.

## Results

Pre-index baseline, release build on the local native host (25 representative samples, 15 traveler-cap samples, 5 combined-cap samples):

| Case | Input before split | Mean | Median | P95 | Max |
| --- | --- | ---: | ---: | ---: | ---: |
| Representative | 6 roads, 7 nodes, 4 travelers; affected route 15→16 nodes | 5 µs | 3 µs | 10 µs | 11 µs |
| Traveler cap | 6 roads, 7 nodes, 2,000 travelers; 1,999 routes at 128 nodes and affected route 127→128 | 2.75 ms | 2.53 ms | 4.65 ms | 4.65 ms |
| Combined caps | 4,999 roads, 9,993 nodes, 2,000 travelers and the same route mix | 40.75 ms | 40.50 ms | 46.66 ms | 46.66 ms |

The capped combined case restored and validated through the engine’s ordinary save loader. The timing harness prints the same fields when rerun. The small representative case is a fresh `Engine::new(87231)` world; no construction timer or simulation tick ran during a sample.

The harness also checks that actor world positions survive the split and that a late route-validation failure leaves the saved engine state byte-for-byte unchanged. A preceding five-sample capped run on the same pre-index implementation ranged from 43 ms median to 82 ms max, so the 25-sample result should be read as a local native distribution, not a hard worst-case bound.

## Indexed implementation comparison

The parent added first-source-match node and parcel indexes during preflight and an adjacency index for connectivity recomputation. Rerunning the same native release harness after those changes (25 samples per road-cap case) produced:

| Case | Input before split | Mean | Median | P95 | Max |
| --- | --- | ---: | ---: | ---: | ---: |
| Representative | 6 roads, 7 nodes, 4 travelers | 4 µs | 2 µs | 7 µs | 32 µs |
| Traveler cap | 6 roads, 7 nodes, 2,000 travelers; 1,999×128-node routes and one 127→128 affected route | 2.87 ms | 2.91 ms | 4.18 ms | 4.18 ms |
| Road/traveler caps, detached geometry | 4,999 roads, 9,993 nodes, 2,000 travelers | 3.66 ms | 3.34 ms | 6.61 ms | 6.74 ms |
| Road cap, connected chain | 4,999 roads, 5,000 nodes, 2,000 travelers | 5.90 ms | 5.72 ms | 8.31 ms | 9.91 ms |

The connected case uses 4,993 unique open segments extending from the seeded gateway across an empty north-edge strip. It restores successfully and makes connectivity refresh traverse the large graph. Both cap cases stay below the 5,000-road limit before splitting and reach exactly 5,000 roads afterward. The 32 µs representative maximum is a single small-sample outlier; the median and p95 are more useful here.

## Interpretation

After indexed lookups and connectivity refresh, the measured cap cases are comfortably below a 50 ms 20 Hz tick on this native host. That supports keeping the transaction synchronous and atomic for these bounded inputs; it does not establish a WASM frame budget, which needs a separate runtime measurement. The original pre-index result shows why the indexes matter: the same detached cap case fell from 40.75 ms mean / 46.66 ms p95 to 3.66 ms / 6.61 ms.

The transaction still stages traveler routes and cursor positions from current state. A geometry revision alone cannot make those values safe to retain across ticks, because actor route and progress can change independently. If a future multi-tick implementation is needed, it must either add a route/movement generation to its freshness check or rebase the actor remaps from current routes and cursors at commit. No resumable split protocol is implemented here.
