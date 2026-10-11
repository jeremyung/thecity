# Master-plan generator review — 2026-10-11

This review replaces the incremental frontier/infill planner with the seeded master plan described in [world growth](../../specs/02-world-growth.md#master-plan). New worlds (`Engine::new`, WASM `new CityEngine(seed)`) use it. The legacy planner remains only behind `Engine::new_legacy` / `CityEngine.newLegacy` for the existing legacy fixtures.

## What changed

- `engine/src/masterplan.rs` generates the plan from the seed and river geometry. It is pure and target-independent: plan decisions use only IEEE-exact arithmetic, and saves store a bit-level plan digest that restore verifies. The plan contains:
  - a transport skeleton: an elevated rail corridor with a station, an elevated expressway over the river with an arterial spur, and gateways;
  - a perturbed arterial lattice;
  - local streets made by recursive jittered splitting along a per-district orientation (T-junctions, polygonal blocks at angles to the arterials), plus dead-end roji;
  - gap-free lots by street-aligned row and frontage cuts, with flag lots (旗竿地) in deep rows.
- `engine/src/masterplan_growth.rs` realizes the plan. Founding opens the gateway arterial to the waterfront market and a street path to the station. Growth then:
  - starts streets that touch the open network, scored by frontage lots, nearby demand, station access and kind;
  - releases lots as parcels when their street opens;
  - lets demand- and vacancy-gated building projects fill them;
  - sets use and height by district (station core, arterial frontage, waterfront);
  - builds the western rail extension and the expressway at population milestones.
- Road hierarchy: elevated expressway 20 m, four-lane arterials 16 m with 2.5 m sidewalks, local streets 6.5 m with painted edge lines, alleys 4 m. Only five lattice lines are arterials, matching Tokyo's wide arterial spacing; the rest are local collectors.
- The expressway runs elevated above an arterial row and column, crossing the river once on a bridge row, with its own grade-separated nodes.
- Heights: compact fabric has almost no single-storey buildings plus scattered 5–6 storey blocks, and the station core gets up to eight 16–32 floor towers.
- Rendering:
  - buildings run to shared lot lines with a fixed detail margin instead of the old 84% inset;
  - developed blocks are paved, not lawn;
  - rail runs on a viaduct and trains run at deck height;
  - the expressway has a deck, piers and barriers.
- `MAX_BUILDINGS` is 3,000.

## Measured fabric

`node scripts/audit-masterplan-fabric.mjs` on three seeds at tick 160,000 (development rate 16). Rendered coverage is the production-kit plan-view union, the same method used for the [PLATEAU comparison](../synthetic-morphology/README.md). The compact crop is the densest 250 m window on a 25 m lattice; the station crop is centered on the station.

| Crop | Legacy (seed 87231) | Master plan (seeds 87231 / 0 / 7) | Target | Tokyo reference |
| --- | --- | --- | --- | --- |
| Compact, rendered coverage | 28.87% | 50.08 / 50.28 / 47.72% | 40–50% | Taito 48.47% |
| Station, rendered coverage | 20.11% | 40.51 / 40.28 / 42.06% | 38–48% | Shinjuku edge 46.04% |
| Whole map, rendered coverage | ~19.9% | 32.70 / 31.89 / 31.42% | > 30% | — |
| Compact, median body footprint | 181.9 m² (lot) | 101.80 / 113.00 / 109.20 m² | 55–140 m² | Taito 57.6 m² (roof edge) |
| Compact, road share | 25.65% (5 m buffers) | 24.77 / 24.52 / 28.48% | 18–28% | — |
| Station, road share | — | 35.82 / 35.36 / 33.55% | 20–30% | — |

Street-network shape is compared with PLATEAU and OpenStreetMap in the [street reference](../street-reference/README.md). Orientation order is now 0.55–0.58 (Togoshi 0.52, Taito 0.64), down from 0.79–0.83.

Heights (walls only; PLATEAU heights include roofs):

| Crop | Ours p10 / p50 / p90 | PLATEAU p10 / p50 / p90 |
| --- | --- | --- |
| Compact | 7.1 / 10.3 / 16.8 m | Taito 7.6 / 12.1 / 22.1 m |
| Station | 7.1 / 16.8–18.5 / 29.8–37.0 m | Shinjuku 6.3 / 17.5 / 32.9 m |

Each city has 2–8 station towers of 16–32 floors, up to 105 m.

Mature cities have 1,936 / 1,834 / 1,976 buildings, roughly double the legacy planner's 850–900 at tick 242,000. Full details are in [fabric-audit.json](fabric-audit.json).

Outside the targets:

- Seed 0's densest compact crop exceeds the 50% upper bound by 0.28 points. The densest window is selected on purpose.
- Station road share is 3.5–6 points high, because the crop includes the rail-corridor frontage streets, an arterial row and the station approach.
- Compact p90 height is about 5 m below Taito's, and median footprints (78–100 m²) are still above Taito's roof-edge median.

## Captures

[Overview](captures/overview.png), [neighborhood](captures/neighborhood.png), [station](captures/station.png) and [street](captures/street.png) views of seed 87231 at tick 160,000, made with `scripts/capture-masterplan-city.mjs`. They use software WebGL with the post pipeline detached while geometry queues drain, preview hour 15, Anime style, and depth of field off. They are visual evidence only, not performance evidence. The [plan diagram](plan-87231.svg) shows lots by district, the elevated layers and the station. For comparison, the earlier mature legacy city is [mature-871-software-webgl.png](../screenshots/narrow-infill/mature-871-software-webgl.png).

## Validation

- Native: 393 passed, 1 failed, 53 ignored. The failure (`frontier_proposal_scan_is_read_only_and_plan_road_replays`) already fails on the PR #2 head before these changes. New tests cover:
  - plan determinism, lot disjointness and density;
  - street setbacks and frontage facing;
  - gateway connectivity;
  - founding;
  - growth with no overlaps, accessible buildings and save/restore replay;
  - same-seed determinism;
  - the expressway crossing the river without running along it.
- Web: 290 of 290 pass. Generic WASM rail, bus, camera and caps tests run against master-plan worlds; the block-subdivision and shared-edge tests use the legacy constructor.
- Cross-target: saves produced natively for all three seeds restore in WASM with matching plan digests.
- `npm run build` and `tsc --noEmit` pass.

## Known limitations

- Rail service still runs one station-to-gateway segment. The western extension is built but has no trains; multi-station service is the next transit slice.
- Waiting rail passengers render at ground-level platform positions under the raised platform.
- Flag-lot access is abstract: pedestrians walk straight from the door to the street.
- Densification by lot assembly and height renewal is specified but not yet implemented; the existing renewal system still runs.
- Browser frame rate at 1,800–2,000 buildings is unmeasured. The spec requires distant low-rise LOD before raising detail further.
- Full build-out takes about 60,000–90,000 ticks at development rate 16. Pacing has not been reviewed against the experience targets.
- The legacy planner code is still compiled for its fixtures; delete it once this path is accepted.

## Reproduce

```
cd engine
MASTERPLAN_SEED=87231 MASTERPLAN_TICKS=160000 MASTERPLAN_SNAPSHOT=$PWD/../.tools/masterplan-review/mp-87231.json \
  cargo test --release masterplan_growth::tests::long_run -- --ignored --nocapture
PLAN_SVG=$PWD/../validation/masterplan-review/plan-87231.svg cargo test --release masterplan::svg -- --ignored
cd .. && npm run wasm
node scripts/audit-masterplan-fabric.mjs --output validation/masterplan-review/fabric-audit.json .tools/masterplan-review/mp-*.json
npx vite --host 127.0.0.1 --port 5173 &   # then:
node scripts/capture-masterplan-city.mjs .tools/masterplan-review/mp-87231.json.save.json validation/masterplan-review/captures 87231
```
