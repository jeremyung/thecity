# Living-city fidelity workstream — 2026-10-11

## Baseline and scope

The resumed goal requires the living city and concept fidelity together. This
workstream compares the production browser renderer with concepts 04 and 01;
it does not replace the growth, audio, performance or later map requirements.
PR #3 was still open at inspection; its corrections are present at f386f45.
Work continues separately on `codex/living-city-fidelity`.

A fresh WASM seed 87231 was advanced through 160,000 ordinary engine ticks at
16× development pace, producing 1,936 buildings. The save was imported through
the normal World → Import UI. The live WebGPU rendering loop remained enabled;
no private queue drain or detached post-processing was used. Zero building and
neighborhood queues were observed before the mature baseline captures.

Baseline media: [day](screenshots/fidelity-baseline/mature-noon-anime.jpg),
[night](screenshots/fidelity-baseline/mature-night-anime.jpg), and
[queue diagnostics](screenshots/fidelity-baseline/mature-loading-diagnostics.jpg).
1280×720 browser viewport, Anime, 50 mm; noon lighting preview has DOF disabled,
night at authoritative 22:00 has DOF enabled. Seed/tick match, but these are
exploratory views rather than registered concept-comparison compositions.
The `founder-before.jpg` file records a later Watch view of the founding state;
the first shop close-up was inspected live before the director moved on.

The mature scene is substantially denser than the retired planner. It still
reads as procedural geometry rather than the locked atmospheric city concepts.
Diagnostic triangle/draw counters are observations, not a hardware benchmark.

## Ranked perceptual gaps

1. **Transport realism:** expressways are broad blank slabs, with no lane
   markings and lamps based at ground height. Elevated crossings are also a
   flat cross rather than a convincing interchange. Fixing furniture does not
   solve interchange topology or provide traffic.
2. **Architectural identity and surface depth:** repeated clean window grids,
   plain side walls, oversized bare anchor forecourts, and dark gridded gables.
   The founder lacks the ceramic roof/storefront richness of the concepts.
3. **Night light and water:** dark, weakly articulated river and limited local
   light variation compared with the luminous banks, bridges and reflections.
4. **Grounded activity:** raised-platform passengers and physical flag-lot
   access remain known gaps; inspect motion in the same accepted compositions.
   [The station baseline](station-passengers.md) now proves the queue is also
   on the wrong side of the tracks. This is the next bounded workstream;
   layout, traversal, and presentation elevation must be corrected together.
5. **Composition and atmosphere:** controlled reference compositions, layered
   foreground vegetation and the distinction between the two styles need a
   complete dawn/day/dusk/night review.

## First bounded intervention: expressway deck furniture

Cause verified in `CityRenderer.roadKit`: expressway falls into the ordinary
road fallback, bypasses arterial markings and receives ground-height lamps.
A dedicated shared-style furniture kit adds lane and shoulder markings, a
central barrier, and inward-facing lamps mounted on the deck. Completed
expressways return before the ground-road furniture path. Construction does
not acquire completed furniture prematurely. No simulation geometry, traffic,
network connectivity, extra light objects or per-frame work is added.

Acceptance: markings read as lanes on the real city deck; supports remain on
the deck for rotated/reversed segments; no ground lamps poke through the deck;
shared geometry works in both styles. A blank slab should no longer dominate
solely because its road furniture is missing. This is not complete expressway
realism or artistic acceptance.

A roof experiment was considered first from the opening view, but the mature
view exposed the more consequential transport defect. The unintegrated roof
prototype also failed root geometry review (inverted ridge and undersampled
rolls), despite passing its narrow tests, so it was removed. Revisit roof design
with silhouette and sampling assertions; do not retain failed experimental code.

## Verification

Three focused geometry tests pass across expressway furniture and existing
bridge elevations. TypeScript and production build pass (existing bundle warning).
Browser acceptance and matched cross-style follow-up are recorded below as
completed, rather than inferred from these tests.

### Live follow-up

The [updated Anime view](screenshots/fidelity-baseline/expressway-after-anime.jpg)
shows lane markings, median and deck-mounted lamps. Both geometry queues reached
zero through ordinary animation frames; [diagnostics](screenshots/fidelity-baseline/expressway-after-diagnostics.jpg).
The same seed/tick/noon/50 mm/DOF-off settings were used, but the Watch-to-Explore
camera was held at a different pose after reload, so this is not a pixel-matched
before/after or performance comparison.

**Next bounded intervention:** resolve the flat cross of elevated corridors.
Markings currently cross each other there, making the invalid junction obvious.
Do not claim expressway realism until its physical junction and intended network
behavior agree. Retain the narrow furniture fix as an in-progress improvement;
no wider artistic acceptance is claimed.

[Little Tokyo follow-up](screenshots/fidelity-baseline/expressway-after-miniature.jpg)
uses the same held camera, noon preview and disabled DOF as the updated Anime
view. The live style switch completes and preserves the visible scene; browser
console inspection reports no errors/warnings. This verifies furniture appears
in both styles, not the full style-fidelity or motion gates.

## Second intervention: separate the crossing routes

The flat cross was present in both plan topology and geometry: row and column
expressways reused a lifted node. The plan now has separate nodes for each route,
with 11 m east–west and 20 m north–south decks. Both remain independent regional
corridors; this does not invent ramps or a turn connection. Deck heights are
included in the plan digest and published through `road_decks` for realized roads.

The renderer consumes those heights for deck, barriers, road furniture and piers.
Upper supports skip the lower transverse carriageway/rail envelope plus 2 m;
parallel ground streets retain the median-pier reservation. This is geometric
clearance, not structural-engineering certification. The 9 m deck separation also
clears the lower road's 6.6 m lamp poles beneath the upper deck.

The new mature WASM checkpoint retains seed 87231, tick 160000 and 1,936 buildings.
Its 16 expressway segments form two disjoint sets of nine nodes at heights 11/20.
WASM restore preserves deck metadata, and the old master-plan checkpoint rejects
explicitly as intended. Native snapshot verification uses ordinary 160,000-tick
growth and restore, rather than fabricating open road state. Native/WASM live
construction continuation tests remain part of verification.

### Layered crossing verification

- Native full release suite: **402 passed, 0 failed, 53 ignored**.
- Five focused frontend geometry checks pass for deck furniture and crossing
  supports, including reversed and skew road segments and both heights.
- WASM rebuilt; two seeds replay all four live construction kinds; eight newly
  exported native construction saves restore and step in WASM.
- TypeScript and production build pass (existing bundle warning).
- Full frontend suite with two workers: **292 passed, 1 timed out**. The movement
  test in `wasm.test.ts` hits its existing 5-second limit, as recorded before this
  intervention. No time limits were changed and no all-green suite is claimed.

Matched current views: [Anime](screenshots/fidelity-baseline/layered-crossing-anime.jpg)
and [Miniature](screenshots/fidelity-baseline/layered-crossing-miniature.jpg).
Same held camera, seed 87231/tick 160000, 50 mm, noon preview, DOF off; browser
viewport changed since the previous baseline, so compare these two styles with
each other, not as registered before/after images. The [diagnostics view](screenshots/fidelity-baseline/layered-crossing-diagnostics.jpg)
records zero geometry queues, with no manual draining. Browser console reports
no graphics errors or warnings after the final render. An earlier startup warning
cleared when rendering finished; startup latency is still an open requirement.

The previously flat cross is now visibly an overpass, and the graph agrees.
Ambient regional expressway traffic is still absent; this checkpoint does not
claim complete transport behavior. The next visual workstream should address
architectural/material richness against the concepts rather than extending the
highway feature indefinitely. Large-map, art-fidelity, hardware-performance and
full motion acceptance remain open.
