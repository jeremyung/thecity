# Master-plan generator review — 2026-10-11

This review replaces the incremental frontier/infill planner with the seeded master plan described in [world growth](../../specs/02-world-growth.md#master-plan). New worlds (`Engine::new`, WASM `new CityEngine(seed)`) use it. The legacy planner remains only behind `Engine::new_legacy` / `CityEngine.newLegacy` for the existing legacy fixtures.

## What changed

- `engine/src/masterplan.rs` generates the plan from the seed and river geometry. It is pure and target-independent: plan decisions use only IEEE-exact arithmetic, and saves store a bit-level plan digest that restore verifies. The plan contains:
  - a transport skeleton: an elevated rail corridor with a station, an elevated expressway over the river with an arterial spur, and gateways;
  - a perturbed arterial lattice;
  - local streets made by recursive jittered splitting (T-junctions, polygonal blocks);
  - gap-free lots by street-aligned row and frontage cuts, with flag lots (旗竿地) in deep rows.
- `engine/src/masterplan_growth.rs` realizes the plan. Founding opens the gateway arterial to the waterfront market and a street path to the station. Growth then:
  - starts streets that touch the open network, scored by frontage lots, nearby demand, station access and kind;
  - releases lots as parcels when their street opens;
  - lets demand- and vacancy-gated building projects fill them;
  - sets use and height by district (station core, arterial frontage, waterfront);
  - builds the western rail extension and the expressway at population milestones.
- Streets are narrowed to Tokyo widths: local 6.5 m with painted edge lines, lanes 4 m, and sidewalks only on arterials.
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
| Compact, rendered coverage | 28.87% | 51.55 / 51.08 / 49.64% | 40–50% | Taito 48.47% |
| Station, rendered coverage | 20.11% | 39.92 / 42.13 / 42.17% | 38–48% | Shinjuku edge 46.04% |
| Whole map, rendered coverage | ~19.9% | 34.77 / 34.27 / 35.01% | > 30% | — |
| Compact, median body footprint | 181.9 m² (lot) | 133.8 / 121.9 / 104.6 m² | 55–140 m² | Taito 57.6 m² (roof edge) |
| Station, median floors | — | 5 / 5 / 5 | 4–7 | Shinjuku 5 |
| Compact, road share | 25.65% (5 m buffers) | 24.88 / 25.42 / 27.73% | 18–28% | — |
| Station, road share | — | 35.30 / 34.55 / 33.69% | 20–30% | — |

Mature cities have 1,784 / 1,805 / 1,873 buildings, roughly double the legacy planner's 850–900 at tick 242,000. Full details are in [fabric-audit.json](fabric-audit.json).

Outside the targets:

- The two densest compact crops exceed the 50% upper bound by up to 1.6 points. The densest window is selected on purpose.
- Station road share is 4–5 points high, because the crop includes both rail-corridor frontage streets and the station-access arterials.
- Body footprints are still about twice Taito's median roof-edge area. Methods differ (body box versus roof edge), but the grain remains coarser than the reference.

## Captures

[Overview](captures/overview.png), [neighborhood](captures/neighborhood.png), [station](captures/station.png) and [street](captures/street.png) views of seed 87231 at tick 160,000, made with `scripts/capture-masterplan-city.mjs`. They use software WebGL with the post pipeline detached while geometry queues drain, preview hour 15, Anime style, and depth of field off. They are visual evidence only, not performance evidence. The [plan diagram](plan-87231.svg) shows lots by district, the elevated layers and the station. For comparison, the earlier mature legacy city is [mature-871-software-webgl.png](../screenshots/narrow-infill/mature-871-software-webgl.png).

## Validation

- Native: 392 passed, 1 failed, 52 ignored. The failure (`frontier_proposal_scan_is_read_only_and_plan_road_replays`) already fails on the PR #2 head before these changes. New tests cover:
  - plan determinism, lot disjointness and density;
  - street setbacks and frontage facing;
  - gateway connectivity;
  - founding;
  - growth with no overlaps, accessible buildings and save/restore replay;
  - same-seed determinism.
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
