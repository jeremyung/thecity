# Endurance cost baseline — 2026-10-11

The current production WASM completes the seed 1204, development-rate 60,
300,000-tick workload in **56.05 s unprofiled** and **56.61 s with Node CPU
sampling**. This leaves little margin below the existing Vitest 60 s timeout.
These two standalone runs do not invalidate the previously observed isolated
Vitest timeout or establish a passing full suite. No timeout was changed.

Both runs satisfy the endurance collection caps and active-traveler accounting,
restore their saves exactly, and produce the same final-save SHA-256
`e803f8f16b23d28d8dcf8c4cb002367bc1f1263c523108cc9f14ff33fe535ffb`.
The final city has 854 buildings, 214 roads, 8,146 population, and 214 traveling
actors. Save is 2,858,922 bytes. Unprofiled snapshot/save/restore/resave take
8.0/6.8/151.9/4.7 ms, respectively: serialization is not the dominant cost.

Full-process CPU samples, aggregated by function name across call sites, put
17.92% self time in `segments_distance_coords`, 16.58% in `route_avoiding`,
15.11% in `hypotf`, and 12.67% in `rects_overlap`. These are sampled self times,
not inclusive phase timers. Geometry callers span multiple systems; do not
attribute all geometry cost to face preflight. The raw CPU profile is optional
and reproducible with the command below; the compact summary is retained here.

Inspection of `Engine::route_avoiding` finds a full road-vector scan for every
popped route node. A source-order adjacency index is a concrete next candidate;
it must preserve exact routing tie behavior, rail/open/connected filtering,
blocked-node handling, and save/resume determinism. This checkpoint has **no
runtime optimization** and makes no browser performance or visual improvement
claim. Browser worker and frame budgets still need their own measurements.

## Reproduction

After building production WASM with `npm run wasm`:

```sh
node scripts/profile-endurance-wasm.mjs /tmp/endurance.json
node --cpu-prof --cpu-prof-dir=/tmp --cpu-prof-name=endurance.cpuprofile scripts/profile-endurance-wasm.mjs /tmp/endurance-profiled.json
```

The new small runner complements `scripts/profile-mature-wasm.mjs`: it times
only each `step` call in the simulation sum, avoids intermediate snapshots,
and separates final serialization/validation timings without a hard-coded
historical timing baseline. Its assertions mirror the current endurance test.
Reports retain the WASM hash, Node version, launch flags and each batch duration.
Node sampling includes startup and final serialization, whereas batch durations
exclude those phases. Do not compare percentages to wall-clock phase totals.

No simulation, renderer, save format, resource cap, or test timeout changed.
