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
